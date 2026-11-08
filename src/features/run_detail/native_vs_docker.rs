use dioxus::prelude::*;

use super::models::WorkspaceResult;
use crate::{
    components::{Badge, Card, Tone},
    features::charts::ChartTone,
    format::{format_optional, ValueFormat},
};

// Native and Docker of the same workspace, in the order of the native ones
pub fn pairs_of(workspaces: &[WorkspaceResult]) -> Vec<(String, WorkspaceResult, WorkspaceResult)> {
    workspaces
        .iter()
        .filter(|native| native.mode == "native")
        .filter_map(|native| {
            workspaces
                .iter()
                .find(|docker| docker.mode == "docker" && docker.name == native.name)
                .map(|docker| (native.name.clone(), native.clone(), docker.clone()))
        })
        .collect()
}

// Under this change in percent two readings count as the same
const SAME_LIMIT: f64 = 1.0;

// One measurement of the same workspace in both modes
#[derive(Debug, Clone, PartialEq)]
struct Row {
    label: &'static str,
    native: Option<f64>,
    docker: Option<f64>,
    format: ValueFormat,
}

fn peak_memory(mode: &WorkspaceResult) -> Option<f64> {
    mode.memory
        .as_ref()
        .map(|memory| memory.peak.rss_bytes as f64)
}

// Every row is a number where less is better
fn rows_for(native: &WorkspaceResult, docker: &WorkspaceResult) -> Vec<Row> {
    let row =
        |label: &'static str, format: ValueFormat, read: fn(&WorkspaceResult) -> Option<f64>| Row {
            label,
            native: read(native),
            docker: read(docker),
            format,
        };

    vec![
        row("Peak memory", ValueFormat::Bytes, peak_memory),
        row("CPU average", ValueFormat::Percent, |mode| {
            mode.cpu.as_ref().map(|cpu| cpu.average_percent)
        }),
        row("CPU peak", ValueFormat::Percent, |mode| {
            mode.cpu.as_ref().map(|cpu| cpu.peak_percent)
        }),
        row("Startup", ValueFormat::DurationMs, |mode| {
            mode.startup
                .as_ref()
                .map(|startup| startup.duration_ms as f64)
        }),
        row("Build", ValueFormat::DurationMs, |mode| {
            mode.build.as_ref().map(|build| build.duration_ms as f64)
        }),
    ]
}

// How far Docker is from Native in percent, `None` when Native has nothing to compare with
fn change_percent(native: f64, docker: f64) -> Option<f64> {
    if native <= 0.0 {
        return None;
    }

    Some((docker - native) / native * 100.0)
}

// The sentence on top of the card, about memory because that is what a container changes most
fn headline_for(native_peak: Option<f64>, docker_peak: Option<f64>) -> String {
    match native_peak
        .zip(docker_peak)
        .and_then(|(native, docker)| change_percent(native, docker))
    {
        Some(change) if change.abs() < SAME_LIMIT => {
            "Docker and Native used about the same memory".to_string()
        }
        Some(change) if change < 0.0 => format!("Docker used {:.0}% less memory", -change),
        Some(change) => format!("Docker used {change:.0}% more memory"),
        None => "Native and Docker side by side".to_string(),
    }
}

// The pill at the end of a row: how Docker compares with Native, green when it is lower
fn pill_for(native: Option<f64>, docker: Option<f64>) -> (String, Tone) {
    match (native, docker) {
        (Some(native), Some(docker)) => match change_percent(native, docker) {
            Some(change) if change.abs() < SAME_LIMIT => ("≈ same".to_string(), Tone::Neutral),
            Some(change) if change < 0.0 => (format!("{change:+.0}%"), Tone::Success),
            Some(change) => (format!("{change:+.0}%"), Tone::Danger),
            None => ("no baseline".to_string(), Tone::Neutral),
        },
        (None, Some(_)) => ("Docker only".to_string(), Tone::Neutral),
        (Some(_), None) => ("Native only".to_string(), Tone::Neutral),
        (None, None) => ("no data".to_string(), Tone::Neutral),
    }
}

// Length of a bar in percent of the longer one of the two
fn bar_percent(value: Option<f64>, longest: f64) -> f64 {
    match value {
        Some(value) if longest > 0.0 => (value / longest * 100.0).clamp(0.0, 100.0),
        _ => 0.0,
    }
}

// Native and Docker of one workspace in one card: a sentence that says what changed, then one
// pair of bars per measurement with a pill for the difference.
#[component]
pub fn NativeVsDocker(name: String, native: WorkspaceResult, docker: WorkspaceResult) -> Element {
    let headline = headline_for(peak_memory(&native), peak_memory(&docker));
    let rows = rows_for(&native, &docker);
    let native_dot = ChartTone::Violet.swatch_class();
    let docker_dot = ChartTone::Cyan.swatch_class();

    rsx! {
        Card { class: "flex flex-col gap-(--space-4) p-(--space-4)",
            div { class: "flex flex-col gap-(--space-2)",
                h3 { class: "text-(length:--fs-xl) font-(weight:--fw-semibold) text-(color:--clr-text)",
                    "{headline}"
                }
                div { class: "flex flex-wrap items-center gap-(--space-3) text-(length:--fs-xs) text-(color:--clr-text-muted)",
                    span { class: "inline-flex items-center gap-(--space-1)",
                        span { class: "size-2 rounded-(--round-full) {native_dot}" }
                        "Native"
                    }
                    span { class: "inline-flex items-center gap-(--space-1)",
                        span { class: "size-2 rounded-(--round-full) {docker_dot}" }
                        "Docker"
                    }
                    span { class: "text-(color:--clr-text-faint)", "Workspace {name}" }
                }
            }
            div { class: "flex flex-col divide-y divide-(color:--clr-border)",
                for row in rows {
                    VerdictRow { key: "{row.label}", row }
                }
            }
        }
    }
}

#[component]
fn VerdictRow(row: Row) -> Element {
    let longest = row
        .native
        .into_iter()
        .chain(row.docker)
        .fold(0.0_f64, f64::max);
    let (pill, tone) = pill_for(row.native, row.docker);

    rsx! {
        div { class: "grid grid-cols-1 items-center gap-(--space-2) py-(--space-3) sm:grid-cols-[7rem_minmax(0,1fr)_7rem] sm:gap-(--space-4)",
            span { class: "text-(length:--fs-sm) text-(color:--clr-text)", "{row.label}" }
            div { class: "flex min-w-0 flex-col gap-(--space-1)",
                BarLine {
                    tone: ChartTone::Violet,
                    percent: bar_percent(row.native, longest),
                    text: format_optional(row.native, row.format),
                }
                BarLine {
                    tone: ChartTone::Cyan,
                    percent: bar_percent(row.docker, longest),
                    text: format_optional(row.docker, row.format),
                }
            }
            div { class: "sm:text-right",
                Badge { label: pill, tone }
            }
        }
    }
}

#[component]
fn BarLine(tone: ChartTone, percent: f64, text: String) -> Element {
    let swatch = tone.swatch_class();
    let width = format!("{percent:.1}");

    rsx! {
        div { class: "flex items-center gap-(--space-2)",
            div { class: "h-2 min-w-0 flex-1 overflow-hidden rounded-(--round-full) bg-(--clr-surface-raised)",
                div {
                    class: "h-full rounded-(--round-full) transition-[width] duration-700 {swatch}",
                    style: "width: {width}%",
                }
            }
            span { class: "w-20 shrink-0 text-right text-(length:--fs-xs) tabular-nums text-(color:--clr-text-muted)",
                "{text}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(name: &str, mode: &str) -> WorkspaceResult {
        serde_json::from_str(&format!(
            r#"{{"name":"{name}","mode":"{mode}","status":"ok"}}"#
        ))
        .expect("parses")
    }

    #[test]
    fn a_pair_needs_both_modes_of_the_same_workspace() {
        let pairs = pairs_of(&[
            workspace("api", "native"),
            workspace("api", "docker"),
            workspace("worker", "native"),
            workspace("site", "docker"),
        ]);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "api");
        assert!(pairs_of(&[]).is_empty());
    }

    #[test]
    fn the_headline_says_whether_docker_used_less_or_more() {
        assert_eq!(
            headline_for(Some(100.0), Some(93.0)),
            "Docker used 7% less memory"
        );
        assert_eq!(
            headline_for(Some(100.0), Some(130.0)),
            "Docker used 30% more memory"
        );
        assert_eq!(
            headline_for(Some(100.0), Some(100.4)),
            "Docker and Native used about the same memory"
        );
    }

    #[test]
    fn without_both_readings_the_headline_does_not_guess() {
        assert_eq!(
            headline_for(None, Some(10.0)),
            "Native and Docker side by side"
        );
        assert_eq!(
            headline_for(Some(0.0), Some(10.0)),
            "Native and Docker side by side"
        );
    }

    #[test]
    fn the_pill_is_green_when_docker_is_lower_and_red_when_it_is_higher() {
        assert_eq!(
            pill_for(Some(100.0), Some(80.0)),
            ("-20%".to_string(), Tone::Success)
        );
        assert_eq!(
            pill_for(Some(100.0), Some(140.0)),
            ("+40%".to_string(), Tone::Danger)
        );
        assert_eq!(
            pill_for(Some(100.0), Some(100.5)),
            ("≈ same".to_string(), Tone::Neutral)
        );
    }

    #[test]
    fn a_missing_reading_is_named_and_never_counted_as_zero() {
        assert_eq!(
            pill_for(None, Some(966.0)),
            ("Docker only".to_string(), Tone::Neutral)
        );
        assert_eq!(
            pill_for(Some(861.0), None),
            ("Native only".to_string(), Tone::Neutral)
        );
        assert_eq!(pill_for(None, None), ("no data".to_string(), Tone::Neutral));
    }

    #[test]
    fn bars_are_drawn_against_the_longer_one_and_a_missing_value_has_no_bar() {
        assert_eq!(bar_percent(Some(50.0), 100.0), 50.0);
        assert_eq!(bar_percent(Some(100.0), 100.0), 100.0);
        assert_eq!(bar_percent(None, 100.0), 0.0);
        assert_eq!(bar_percent(Some(5.0), 0.0), 0.0);
    }
}
