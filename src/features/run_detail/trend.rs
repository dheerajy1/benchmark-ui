use crate::features::history::{Column, RunSummary};

use super::models::BenchmarkResult;

// How many runs of the same target the tiles and graphs show
pub const WINDOW: usize = 12;

// The runs up to and including `run_id`, oldest first and at most `WINDOW` long, from a list of
// the target's runs with the newest first. Empty when the run is not in the list.
pub fn window_ending_at(runs: &[RunSummary], run_id: &str) -> Vec<RunSummary> {
    let Some(position) = runs.iter().position(|run| run.run_id == run_id) else {
        return Vec::new();
    };

    runs[position..runs.len().min(position + WINDOW)]
        .iter()
        .rev()
        .cloned()
        .collect()
}

// Values of one column over the window, a run that did not finish has none
pub fn column_values(window: &[RunSummary], column: Column) -> Vec<Option<f64>> {
    window.iter().map(|run| column.measured(run)).collect()
}

// The run's own number for a column, used until the history of its target has loaded
pub fn own_value(result: &BenchmarkResult, column: Column) -> Option<f64> {
    if result.status != "done" {
        return None;
    }

    let metrics = &result.metrics;

    match column {
        Column::Throughput => metrics
            .http_latency
            .as_ref()
            .map(|http| http.requests_per_second),
        Column::P95 => metrics.http_latency.as_ref().and_then(|http| http.p95_ms),
        Column::PeakMemory => metrics
            .memory
            .as_ref()
            .map(|memory| memory.peak.rss_bytes as f64),
        Column::CpuAverage => metrics.cpu.as_ref().map(|cpu| cpu.average_percent),
        Column::Startup => metrics
            .startup_time
            .duration_ms
            .map(|duration| duration as f64),
        Column::BuildTime => metrics
            .build_time
            .duration_ms
            .map(|duration| duration as f64),
        Column::BuildOutput => Some(metrics.artifact_size.build.bytes as f64),
        Column::ImageSize | Column::DiskUsage | Column::Status => None,
    }
}

// Values of a column for the tiles: the window when the history is known, otherwise only the
// run itself
pub fn tile_values(
    window: &[RunSummary],
    result: &BenchmarkResult,
    column: Column,
) -> Vec<Option<f64>> {
    if window.is_empty() {
        vec![own_value(result, column)]
    } else {
        column_values(window, column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(id: &str, rps: f64) -> RunSummary {
        serde_json::from_str(&format!(
            r#"{{
                "run_id": "{id}", "status": "done", "display_name": "p",
                "target": {{ "path": "/p" }}, "discovery": {{ "runtime": "bun" }},
                "created_at": "2026-10-08T05:11:22Z",
                "summary": {{ "requests_per_second": {rps} }}
            }}"#
        ))
        .expect("parses")
    }

    fn ids(runs: &[RunSummary]) -> Vec<&str> {
        runs.iter().map(|run| run.run_id.as_str()).collect()
    }

    #[test]
    fn ends_the_window_at_the_run_and_reads_oldest_first() {
        let runs = [run("d", 4.0), run("c", 3.0), run("b", 2.0), run("a", 1.0)];

        assert_eq!(ids(&window_ending_at(&runs, "c")), ["a", "b", "c"]);
        assert_eq!(ids(&window_ending_at(&runs, "d")), ["a", "b", "c", "d"]);
    }

    #[test]
    fn keeps_at_most_twelve_runs() {
        let runs: Vec<RunSummary> = (0..30).map(|n| run(&format!("r{n}"), 1.0)).collect();

        let window = window_ending_at(&runs, "r5");

        assert_eq!(window.len(), WINDOW);
        assert_eq!(window.last().map(|run| run.run_id.as_str()), Some("r5"));
        assert_eq!(window[0].run_id, "r16");
    }

    #[test]
    fn a_run_outside_the_list_has_no_window() {
        assert!(window_ending_at(&[run("a", 1.0)], "zzz").is_empty());
    }

    #[test]
    fn reads_a_column_over_the_window() {
        let window = [run("a", 10.0), run("b", 20.0)];

        assert_eq!(
            column_values(&window, Column::Throughput),
            vec![Some(10.0), Some(20.0)]
        );
    }
}
