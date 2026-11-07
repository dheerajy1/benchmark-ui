use std::collections::HashMap;

use super::models::RunSummary;
use crate::format::{format_optional, ValueFormat, MISSING};

// Local storage key of the column choice, kept in this browser only
const STORAGE_KEY: &str = "benchmark.runs.columns";

// Every column the Runs table can show
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Column {
    Throughput,
    P95,
    PeakMemory,
    CpuAverage,
    Startup,
    BuildTime,
    BuildOutput,
    ImageSize,
    DiskUsage,
    Status,
}

impl Column {
    pub const ALL: [Column; 10] = [
        Self::Throughput,
        Self::P95,
        Self::PeakMemory,
        Self::CpuAverage,
        Self::Startup,
        Self::BuildTime,
        Self::BuildOutput,
        Self::ImageSize,
        Self::DiskUsage,
        Self::Status,
    ];

    // Shown until the person chooses otherwise
    pub const DEFAULT: [Column; 5] = [
        Self::Throughput,
        Self::P95,
        Self::PeakMemory,
        Self::CpuAverage,
        Self::Startup,
    ];

    // Groups of the Columns menu, in display order
    pub const GROUPS: [(&'static str, &'static [Column]); 4] = [
        ("Performance", &[Self::Throughput, Self::P95]),
        ("Resources", &[Self::PeakMemory, Self::CpuAverage]),
        (
            "Build and size",
            &[
                Self::Startup,
                Self::BuildTime,
                Self::BuildOutput,
                Self::ImageSize,
                Self::DiskUsage,
            ],
        ),
        ("Run", &[Self::Status]),
    ];

    fn key(self) -> &'static str {
        match self {
            Self::Throughput => "throughput",
            Self::P95 => "p95",
            Self::PeakMemory => "peak_memory",
            Self::CpuAverage => "cpu_average",
            Self::Startup => "startup",
            Self::BuildTime => "build_time",
            Self::BuildOutput => "build_output",
            Self::ImageSize => "image_size",
            Self::DiskUsage => "disk_usage",
            Self::Status => "status",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|column| column.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Throughput => "Throughput",
            Self::P95 => "p95 latency",
            Self::PeakMemory => "Peak memory",
            Self::CpuAverage => "CPU average",
            Self::Startup => "Startup",
            Self::BuildTime => "Build time",
            Self::BuildOutput => "Build output",
            Self::ImageSize => "Image size",
            Self::DiskUsage => "Unpacked on disk",
            Self::Status => "Status",
        }
    }

    // `None` for the status column, which is a badge and not a number
    fn format(self) -> Option<ValueFormat> {
        match self {
            Self::Throughput => Some(ValueFormat::PerSecond),
            Self::P95 => Some(ValueFormat::LatencyMs),
            Self::PeakMemory | Self::BuildOutput | Self::ImageSize | Self::DiskUsage => {
                Some(ValueFormat::Bytes)
            }
            Self::CpuAverage => Some(ValueFormat::Percent),
            Self::Startup | Self::BuildTime => Some(ValueFormat::DurationMs),
            Self::Status => None,
        }
    }

    pub fn is_numeric(self) -> bool {
        self.format().is_some()
    }

    // Only throughput is better when higher, every other measurement is better when lower
    fn higher_is_better(self) -> bool {
        matches!(self, Self::Throughput)
    }

    // Sum over the Docker workspaces of a run, `None` when none reported the size
    fn docker_total(
        run: &RunSummary,
        pick: fn(&super::models::WorkspaceSummary) -> Option<u64>,
    ) -> Option<f64> {
        let sizes: Vec<u64> = run
            .summary
            .workspaces
            .iter()
            .filter(|workspace| workspace.mode == "docker")
            .filter_map(pick)
            .collect();

        if sizes.is_empty() {
            None
        } else {
            Some(sizes.iter().sum::<u64>() as f64)
        }
    }

    pub fn value(self, run: &RunSummary) -> Option<f64> {
        let summary = &run.summary;

        match self {
            Self::Throughput => summary.requests_per_second,
            Self::P95 => summary.p95_ms,
            Self::PeakMemory => summary.memory_peak_bytes.map(|value| value as f64),
            Self::CpuAverage => summary.cpu_average_percent,
            Self::Startup => summary.startup_time_ms.map(|value| value as f64),
            Self::BuildTime => summary.build_time_ms.map(|value| value as f64),
            Self::BuildOutput => summary.build_output_bytes.map(|value| value as f64),
            Self::ImageSize => Self::docker_total(run, |workspace| workspace.image_bytes),
            Self::DiskUsage => Self::docker_total(run, |workspace| workspace.disk_usage_bytes),
            Self::Status => None,
        }
    }

    // Number of a run in this column. A run that did not finish shows a dash.
    pub fn measured(self, run: &RunSummary) -> Option<f64> {
        if run.status == "done" {
            self.value(run)
        } else {
            None
        }
    }

    pub fn text(self, run: &RunSummary) -> String {
        match self.format() {
            Some(format) => format_optional(self.measured(run), format),
            None => MISSING.to_string(),
        }
    }
}

// Best value of every shown numeric column among the finished runs, only when at least two runs
// have a value, because a lone value is not better than anything
pub fn best_values(runs: &[RunSummary], shown: &[Column]) -> HashMap<Column, f64> {
    let mut best = HashMap::new();

    for column in shown.iter().copied().filter(|column| column.is_numeric()) {
        let values: Vec<f64> = runs.iter().filter_map(|run| column.measured(run)).collect();

        if values.len() < 2 {
            continue;
        }

        let pick = if column.higher_is_better() {
            values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        } else {
            values.iter().copied().fold(f64::INFINITY, f64::min)
        };

        best.insert(column, pick);
    }

    best
}

// Width of the thin throughput bar, as a percentage of the highest throughput on the page
pub fn bar_percent(value: Option<f64>, highest: f64) -> f64 {
    match value {
        Some(value) if highest > 0.0 => (value / highest * 100.0).clamp(2.0, 100.0),
        _ => 0.0,
    }
}

// Which columns are shown and in what order. Every column keeps a place in the order, so a
// hidden column returns where it was.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnPrefs {
    entries: Vec<(Column, bool)>,
}

impl ColumnPrefs {
    // The default five in their order, then the others hidden
    pub fn standard() -> Self {
        let mut entries: Vec<(Column, bool)> = Column::DEFAULT
            .into_iter()
            .map(|column| (column, true))
            .collect();

        entries.extend(
            Column::ALL
                .into_iter()
                .filter(|column| !Column::DEFAULT.contains(column))
                .map(|column| (column, false)),
        );

        Self { entries }
    }

    // Shown columns in table order
    pub fn shown(&self) -> Vec<Column> {
        self.entries
            .iter()
            .filter(|(_, shown)| *shown)
            .map(|(column, _)| *column)
            .collect()
    }

    pub fn shown_count(&self) -> usize {
        self.entries.iter().filter(|(_, shown)| *shown).count()
    }

    // Every column in order, shown or not
    pub fn ordered(&self) -> Vec<Column> {
        self.entries.iter().map(|(column, _)| *column).collect()
    }

    pub fn is_shown(&self, column: Column) -> bool {
        self.entries
            .iter()
            .any(|(entry, shown)| *entry == column && *shown)
    }

    // Switches a column, the last shown column cannot be hidden
    pub fn toggle(&mut self, column: Column) {
        let only_one_left = self.shown_count() == 1;

        if let Some((_, shown)) = self.entries.iter_mut().find(|(entry, _)| *entry == column) {
            if *shown && only_one_left {
                return;
            }

            *shown = !*shown;
        }
    }

    // Moves `dragged` to the place of `target`
    pub fn move_to(&mut self, dragged: Column, target: Column) {
        if dragged == target {
            return;
        }

        let Some(from) = self
            .entries
            .iter()
            .position(|(column, _)| *column == dragged)
        else {
            return;
        };
        let entry = self.entries.remove(from);
        let to = self
            .entries
            .iter()
            .position(|(column, _)| *column == target)
            .map_or(self.entries.len(), |position| {
                if from < position + 1 {
                    position + 1
                } else {
                    position
                }
            });

        self.entries.insert(to, entry);
    }

    // `throughput:1,p95:1,build_time:0`
    pub fn encode(&self) -> String {
        self.entries
            .iter()
            .map(|(column, shown)| format!("{}:{}", column.key(), u8::from(*shown)))
            .collect::<Vec<_>>()
            .join(",")
    }

    // Unknown names are skipped and missing columns are added hidden, so an older or newer
    // saved value still works. `None` when nothing would be shown.
    pub fn decode(text: &str) -> Option<Self> {
        let mut entries: Vec<(Column, bool)> = Vec::new();

        for part in text.split(',') {
            let Some((key, flag)) = part.split_once(':') else {
                continue;
            };
            let Some(column) = Column::from_key(key.trim()) else {
                continue;
            };

            if entries.iter().all(|(entry, _)| *entry != column) {
                entries.push((column, flag.trim() == "1"));
            }
        }

        for column in Column::ALL {
            if entries.iter().all(|(entry, _)| *entry != column) {
                entries.push((column, false));
            }
        }

        let prefs = Self { entries };

        (prefs.shown_count() > 0).then_some(prefs)
    }

    // The saved choice of this browser, or the standard one
    pub fn load() -> Self {
        web_sys::window()
            .and_then(|window| window.local_storage().ok().flatten())
            .and_then(|storage| storage.get_item(STORAGE_KEY).ok().flatten())
            .and_then(|text| Self::decode(&text))
            .unwrap_or_else(Self::standard)
    }

    pub fn save(&self) {
        if let Some(storage) =
            web_sys::window().and_then(|window| window.local_storage().ok().flatten())
        {
            let _ = storage.set_item(STORAGE_KEY, &self.encode());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(status: &str, rps: Option<f64>, memory: Option<u64>) -> RunSummary {
        let body = format!(
            r#"{{
                "run_id": "a", "status": "{status}", "display_name": "p",
                "target": {{ "path": "/p" }}, "discovery": {{ "runtime": "rust" }},
                "created_at": "2026-10-08T05:11:22Z",
                "summary": {{
                    "requests_per_second": {},
                    "memory_peak_bytes": {},
                    "workspaces": [
                        {{ "mode": "docker", "image_bytes": 100, "disk_usage_bytes": 250 }},
                        {{ "mode": "docker", "image_bytes": 50 }},
                        {{ "mode": "native" }}
                    ]
                }}
            }}"#,
            rps.map_or("null".to_string(), |value| value.to_string()),
            memory.map_or("null".to_string(), |value| value.to_string()),
        );

        serde_json::from_str(&body).expect("parses")
    }

    #[test]
    fn starts_with_the_five_default_columns() {
        let prefs = ColumnPrefs::standard();

        assert_eq!(prefs.shown(), Column::DEFAULT.to_vec());
        assert_eq!(prefs.shown_count(), 5);
        assert_eq!(prefs.ordered().len(), 10);
        assert!(!prefs.is_shown(Column::BuildTime));
    }

    #[test]
    fn the_last_column_stays_on() {
        let mut prefs = ColumnPrefs::standard();

        for column in Column::DEFAULT {
            prefs.toggle(column);
        }

        assert_eq!(prefs.shown(), vec![Column::Startup]);

        prefs.toggle(Column::BuildTime);
        prefs.toggle(Column::Startup);

        assert_eq!(prefs.shown(), vec![Column::BuildTime]);
    }

    #[test]
    fn moves_a_column_before_and_after_another() {
        let mut prefs = ColumnPrefs::standard();

        prefs.move_to(Column::Startup, Column::Throughput);
        assert_eq!(prefs.shown()[0], Column::Startup);

        prefs.move_to(Column::Startup, Column::CpuAverage);
        assert_eq!(
            prefs.shown(),
            vec![
                Column::Throughput,
                Column::P95,
                Column::PeakMemory,
                Column::CpuAverage,
                Column::Startup
            ]
        );
    }

    #[test]
    fn a_saved_choice_round_trips() {
        let mut prefs = ColumnPrefs::standard();
        prefs.toggle(Column::Status);
        prefs.move_to(Column::Status, Column::Throughput);

        assert_eq!(ColumnPrefs::decode(&prefs.encode()), Some(prefs));
    }

    #[test]
    fn an_odd_saved_value_still_works() {
        let prefs = ColumnPrefs::decode("p95:1, nonsense, gone:1,throughput:0").expect("decodes");

        assert_eq!(prefs.shown(), vec![Column::P95]);
        assert_eq!(prefs.ordered().len(), 10);
        assert_eq!(ColumnPrefs::decode("p95:0"), None);
        assert_eq!(ColumnPrefs::decode(""), None);
    }

    #[test]
    fn sums_the_docker_sizes_of_a_run() {
        let finished = run("done", Some(1.0), Some(1));

        assert_eq!(Column::ImageSize.value(&finished), Some(150.0));
        assert_eq!(Column::DiskUsage.value(&finished), Some(250.0));
    }

    #[test]
    fn a_run_that_did_not_finish_shows_a_dash() {
        let failed = run("failed", Some(5.0), Some(7));

        assert_eq!(Column::Throughput.text(&failed), MISSING);
        assert_eq!(Column::Status.text(&failed), MISSING);
    }

    #[test]
    fn marks_the_best_value_of_each_column() {
        let runs = [
            run("done", Some(100.0), Some(300)),
            run("done", Some(150.0), Some(200)),
            run("failed", Some(900.0), Some(1)),
        ];

        let best = best_values(
            &runs,
            &[Column::Throughput, Column::PeakMemory, Column::Status],
        );

        assert_eq!(best.get(&Column::Throughput), Some(&150.0));
        assert_eq!(best.get(&Column::PeakMemory), Some(&200.0));
        assert_eq!(best.get(&Column::Status), None);
    }

    #[test]
    fn a_lone_value_is_not_the_best() {
        let runs = [run("done", Some(100.0), None), run("done", None, None)];

        assert!(best_values(&runs, &[Column::Throughput]).is_empty());
    }

    #[test]
    fn scales_the_throughput_bar() {
        assert_eq!(bar_percent(Some(50.0), 100.0), 50.0);
        assert_eq!(bar_percent(Some(0.1), 100.0), 2.0);
        assert_eq!(bar_percent(None, 100.0), 0.0);
        assert_eq!(bar_percent(Some(5.0), 0.0), 0.0);
    }
}
