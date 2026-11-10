use dioxus::prelude::*;

use super::{cards::DataCard, models::RunReference};
use crate::{
    components::{GhostChart, GhostNote},
    features::{
        charts::{ago_labels, align_end, ChartTone, CompareSeries, CompareTrendChart},
        connection::ApiError,
        history::{fetch_page, Column, HistoryFilters, RunSummary},
        run_detail::{column_values, window_ending_at},
    },
    format::{format_number, ValueFormat, MISSING},
};

const HISTORY_LIMIT: u32 = 100;

// What the chart can plot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Throughput,
    P95,
    PeakMemory,
    ImageSize,
    DockerBuild,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Self::Throughput => "Throughput",
            Self::P95 => "p95 latency",
            Self::PeakMemory => "Peak memory",
            Self::ImageSize => "Image size",
            Self::DockerBuild => "Docker build",
        }
    }

    // The history list column behind the tab, the Docker build time is not in the list
    fn column(self) -> Option<(Column, ValueFormat)> {
        match self {
            Self::Throughput => Some((Column::Throughput, ValueFormat::PerSecond)),
            Self::P95 => Some((Column::P95, ValueFormat::LatencyMs)),
            Self::PeakMemory => Some((Column::PeakMemory, ValueFormat::Bytes)),
            Self::ImageSize => Some((Column::ImageSize, ValueFormat::Bytes)),
            Self::DockerBuild => None,
        }
    }
}

// The tabs to offer, the two Docker ones only when a run had a Docker workspace
fn tabs(has_docker: bool) -> Vec<Tab> {
    let mut tabs = vec![Tab::Throughput, Tab::P95, Tab::PeakMemory];

    if has_docker {
        tabs.push(Tab::ImageSize);
        tabs.push(Tab::DockerBuild);
    }

    tabs
}

// The newest runs of a target path, newest first
async fn fetch_runs(target_path: &str) -> Result<Vec<RunSummary>, ApiError> {
    let filters = HistoryFilters {
        target_path: target_path.to_string(),
        ..HistoryFilters::default()
    };

    fetch_page(&filters, None, HISTORY_LIMIT)
        .await
        .map(|page| page.items)
}

// Values of one column for the two windows, both padded to end at the newest run
fn aligned_values(
    baseline: &[RunSummary],
    comparison: &[RunSummary],
    column: Column,
) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let len = baseline.len().max(comparison.len());

    (
        align_end(column_values(baseline, column), len),
        align_end(column_values(comparison, column), len),
    )
}

// The average requests the runs of a window sent, "—" when none of them sent any
fn requests_note(window: &[RunSummary]) -> String {
    let counts: Vec<u64> = window
        .iter()
        .filter_map(|run| run.summary.total_requests)
        .filter(|count| *count > 0)
        .collect();

    if counts.is_empty() {
        return format!("{MISSING} requests avg");
    }

    let average = counts.iter().sum::<u64>() as f64 / counts.len() as f64;

    format!("{} requests avg", format_number(average, 0))
}

// Lines of the last runs of each run's target, one tab per measurement. Chart only.
#[component]
pub fn TrendsCard(baseline: RunReference, comparison: RunReference, has_docker: bool) -> Element {
    let mut tab = use_signal(|| Tab::Throughput);

    let baseline_path = baseline.target.path.clone();
    let comparison_path = comparison.target.path.clone();
    let history = use_resource(use_reactive!(
        |(baseline_path, comparison_path)| async move {
            let first = fetch_runs(&baseline_path).await?;
            let second = if comparison_path == baseline_path {
                first.clone()
            } else {
                fetch_runs(&comparison_path).await?
            };

            Ok::<_, ApiError>((first, second))
        }
    ));

    let options = tabs(has_docker);
    let current = if options.contains(&tab()) {
        tab()
    } else {
        Tab::Throughput
    };

    let buttons: Vec<(Tab, &'static str, &'static str, bool)> = options
        .iter()
        .map(|option| {
            let class = if *option == current {
                "bg-(--clr-accent-soft) text-(color:--clr-accent) font-(weight:--fw-medium)"
            } else {
                "text-(color:--clr-text-muted) hover:bg-(--clr-surface-hover)"
            };

            (*option, option.label(), class, *option == current)
        })
        .collect();

    let body: Element = match history() {
        None => rsx! {
            GhostChart {}
        },
        Some(Err(_)) => rsx! {
            p { class: "flex h-(--chart-height) items-center justify-center text-center text-(length:--fs-sm) text-(color:--clr-text-faint)",
                "The history of these targets could not be loaded."
            }
        },
        Some(Ok((first, second))) => match current.column() {
            None => rsx! {
                p { class: "rounded-(--round-md) bg-(--clr-warning-soft) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-warning-text)",
                    "The history list does not carry the Docker build time yet, so it cannot be drawn. It needs a change in benchmark-api."
                }
            },
            Some((column, format)) => {
                let first = window_ending_at(&first, &baseline.run_id);
                let second = window_ending_at(&second, &comparison.run_id);
                let (baseline_values, comparison_values) = aligned_values(&first, &second, column);
                let labels = ago_labels(baseline_values.len());
                let (baseline_note, comparison_note) = if current == Tab::Throughput {
                    (requests_note(&first), requests_note(&second))
                } else {
                    (String::new(), String::new())
                };

                if baseline_values.len() < 2 {
                    rsx! {
                        GhostNote { title: "Trends need at least 2 runs", text: "" }
                    }
                } else {
                    rsx! {
                    CompareTrendChart {
                        title: current.label(),
                        labels,
                        series: vec![
                            CompareSeries {
                                name: "Baseline".to_string(),
                                tone: ChartTone::Slate,
                                values: baseline_values,
                                note: baseline_note,
                            },
                            CompareSeries {
                                name: "Comparison".to_string(),
                                tone: ChartTone::Blue,
                                values: comparison_values,
                                note: comparison_note,
                            },
                        ],
                        format,
                    }
                    }
                }
            }
        },
    };

    rsx! {
        DataCard {
            title: "Over the last 12 runs",
            subtitle: "The last runs of each run's target, ending at the compared run.",
            chart: Some(rsx! {
                div { class: "flex flex-col gap-(--space-3)",
                    div { class: "flex flex-wrap gap-(--space-1)", role: "tablist",
                        for (option , label , class , selected) in buttons {
                            button {
                                key: "{label}",
                                r#type: "button",
                                role: "tab",
                                "aria-selected": "{selected}",
                                class: "cursor-pointer rounded-(--round-full) px-(--space-3) py-(--space-1) text-(length:--fs-sm) transition-colors {class}",
                                onclick: move |_| tab.set(option),
                                "{label}"
                            }
                        }
                    }
                    {body}
                }
            }),
        }
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

    fn run_with_requests(id: &str, requests: Option<u64>) -> RunSummary {
        let sent = requests.map_or("null".to_string(), |count| count.to_string());

        serde_json::from_str(&format!(
            r#"{{
                "run_id": "{id}", "status": "done", "display_name": "p",
                "target": {{ "path": "/p" }}, "discovery": {{ "runtime": "bun" }},
                "created_at": "2026-10-08T05:11:22Z",
                "summary": {{ "total_requests": {sent} }}
            }}"#
        ))
        .expect("parses")
    }

    #[test]
    fn the_note_averages_the_runs_that_sent_requests() {
        let window = [
            run_with_requests("a", Some(1000)),
            run_with_requests("b", None),
            run_with_requests("c", Some(0)),
            run_with_requests("d", Some(1400)),
        ];

        assert_eq!(requests_note(&window), "1,200 requests avg");
        assert_eq!(requests_note(&[]), "— requests avg");
    }

    #[test]
    fn offers_the_docker_tabs_only_with_docker() {
        assert_eq!(tabs(false).len(), 3);
        assert_eq!(tabs(true).len(), 5);
        assert_eq!(tabs(true)[4].column(), None);
    }

    #[test]
    fn pads_the_shorter_window_at_the_front() {
        let baseline = [run("b", 2.0), run("a", 1.0)];
        let comparison = [run("c", 9.0)];

        let (first, second) = aligned_values(
            &window_ending_at(&baseline, "b"),
            &window_ending_at(&comparison, "c"),
            Column::Throughput,
        );

        assert_eq!(first, vec![Some(1.0), Some(2.0)]);
        assert_eq!(second, vec![None, Some(9.0)]);
    }
}
