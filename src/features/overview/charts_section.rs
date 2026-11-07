use dioxus::prelude::*;

use super::{
    metrics::{newest, series_points, short_day, TrendMetric},
    models::TrendPoint,
};
use crate::{
    components::{Card, CardError, CardHeader, GhostChart},
    features::{
        charts::{BarChart, BarGroup, BarSeries, ChartTone, TrendChart},
        run_detail::{fetch_run, BenchmarkResult, WorkspaceResult},
    },
    format::ValueFormat,
};

// Card with a chart inside, `class` can resize the chart through `--chart-height`
#[component]
fn ChartCard(
    #[props(into)] title: String,
    #[props(into)] subtitle: String,
    #[props(into, default)] class: String,
    children: Element,
) -> Element {
    rsx! {
        Card { class: "fade-up {class}",
            CardHeader { title, subtitle }
            div { class: "p-(--space-4) pb-(--space-3)", {children} }
        }
    }
}

fn note(text: &str) -> Element {
    rsx! {
        p { class: "flex h-(--chart-height) items-center justify-center text-center text-(length:--fs-sm) text-(color:--clr-text-faint)",
            "{text}"
        }
    }
}

fn single(name: &str, tone: ChartTone) -> Vec<BarSeries> {
    vec![BarSeries {
        name: name.to_string(),
        tone,
    }]
}

// One group per workspace with its native and its Docker peak memory side by side
pub fn native_vs_docker(workspaces: &[WorkspaceResult]) -> Vec<BarGroup> {
    let mut groups: Vec<BarGroup> = Vec::new();

    for workspace in workspaces {
        let index = match groups
            .iter()
            .position(|group| group.label == workspace.name)
        {
            Some(index) => index,
            None => {
                groups.push(BarGroup {
                    label: workspace.name.clone(),
                    values: vec![None, None],
                });
                groups.len() - 1
            }
        };
        let slot = usize::from(workspace.mode == "docker");
        let peak = workspace
            .memory
            .as_ref()
            .map(|memory| memory.peak.rss_bytes as f64);

        groups[index].values[slot] = peak;
    }

    groups
}

// Charts of the Overview: throughput over the newest runs, latency percentiles and native
// against Docker memory of the latest run, then peak memory, startup and build time per run
#[component]
pub fn OverviewCharts(trends: Vec<TrendPoint>, latest_run_id: String) -> Element {
    let mut detail = use_resource(use_reactive!(|(latest_run_id,)| async move {
        fetch_run(&latest_run_id).await
    }));

    let recent = newest(&trends);
    let chronological: Vec<&TrendPoint> = recent.iter().rev().collect();

    let throughput = series_points(recent, TrendMetric::Throughput);
    let memory_groups: Vec<BarGroup> = chronological
        .iter()
        .map(|point| BarGroup {
            label: short_day(&point.created_at),
            values: vec![TrendMetric::PeakMemory.value(point)],
        })
        .collect();
    let time_groups: Vec<BarGroup> = chronological
        .iter()
        .map(|point| BarGroup {
            label: short_day(&point.created_at),
            values: vec![
                TrendMetric::Startup.value(point),
                TrendMetric::Build.value(point),
            ],
        })
        .collect();

    let result: Option<Result<BenchmarkResult, ()>> =
        detail().map(|outcome| outcome.map_err(|_| ()));

    let (latency, containers): (Element, Element) = match result {
        None => (
            rsx! {
                GhostChart {}
            },
            rsx! {
                GhostChart {}
            },
        ),
        Some(Err(())) => (
            rsx! {
                CardError {
                    title: "Couldn't load latency by percentile",
                    on_retry: move |_| detail.restart(),
                }
            },
            rsx! {
                CardError {
                    title: "Couldn't load native vs Docker memory",
                    on_retry: move |_| detail.restart(),
                }
            },
        ),
        Some(Ok(run)) => {
            let latency = match run.metrics.http_latency {
                Some(http) => rsx! {
                    BarChart {
                        title: "Latency by percentile",
                        groups: vec![
                            BarGroup {
                                label: "p50".to_string(),
                                values: vec![http.p50_ms],
                            },
                            BarGroup {
                                label: "p95".to_string(),
                                values: vec![http.p95_ms],
                            },
                            BarGroup {
                                label: "p99".to_string(),
                                values: vec![http.p99_ms],
                            },
                            BarGroup {
                                label: "max".to_string(),
                                values: vec![http.max_ms],
                            },
                        ],
                        series: single("Latency", ChartTone::Amber),
                        format: ValueFormat::LatencyMs,
                    }
                },
                None => note("The latest run had no workload."),
            };

            let groups = native_vs_docker(&run.metrics.workspaces);
            let containers = if groups.is_empty() {
                note("The latest run had no workspace.")
            } else {
                rsx! {
                    BarChart {
                        title: "Peak memory, native and Docker",
                        groups,
                        series: vec![
                            BarSeries {
                                name: "Native".to_string(),
                                tone: ChartTone::Violet,
                            },
                            BarSeries {
                                name: "Docker".to_string(),
                                tone: ChartTone::Cyan,
                            },
                        ],
                        format: ValueFormat::Bytes,
                    }
                }
            };

            (latency, containers)
        }
    };

    rsx! {
        div { class: "grid grid-cols-1 gap-(--space-4) lg:grid-cols-3",
            ChartCard {
                title: "Throughput",
                subtitle: "Requests per second over the last 12 runs.",
                class: "lg:col-span-2 lg:row-span-2 lg:[--chart-height:27rem]",
                TrendChart {
                    title: "Throughput",
                    points: throughput,
                    format: ValueFormat::PerSecond,
                    tone: ChartTone::Blue,
                }
            }
            ChartCard {
                title: "Latency by percentile",
                subtitle: "Response time of the latest run.",
                {latency}
            }
            ChartCard {
                title: "Native vs Docker",
                subtitle: "Peak memory of the latest run.",
                {containers}
            }
        }
        div { class: "grid grid-cols-1 gap-(--space-4) lg:grid-cols-2",
            ChartCard {
                title: "Peak memory by run",
                subtitle: "Oldest to newest, the last 12 runs.",
                BarChart {
                    title: "Peak memory by run",
                    groups: memory_groups,
                    series: single("Peak memory", ChartTone::Violet),
                    format: ValueFormat::Bytes,
                }
            }
            ChartCard {
                title: "Startup and build time",
                subtitle: "Oldest to newest, the last 12 runs.",
                BarChart {
                    title: "Startup and build time",
                    groups: time_groups,
                    series: vec![
                        BarSeries {
                            name: "Startup".to_string(),
                            tone: ChartTone::Teal,
                        },
                        BarSeries {
                            name: "Build".to_string(),
                            tone: ChartTone::Amber,
                        },
                    ],
                    format: ValueFormat::DurationMs,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(name: &str, mode: &str, peak: Option<u64>) -> WorkspaceResult {
        let memory = peak.map(|bytes| {
            format!(
                r#","memory":{{"baseline":{{"rss_bytes":1,"rss_size":"1 B"}},"peak":{{"rss_bytes":{bytes},"rss_size":"x"}},"delta":{{"rss_bytes":0,"rss_size":"0 B"}}}}"#
            )
        });

        serde_json::from_str(&format!(
            r#"{{"name":"{name}","mode":"{mode}","status":"ok"{}}}"#,
            memory.unwrap_or_default()
        ))
        .expect("parses")
    }

    #[test]
    fn pairs_native_and_docker_per_workspace() {
        let groups = native_vs_docker(&[
            workspace("api", "native", Some(100)),
            workspace("api", "docker", Some(150)),
            workspace("worker", "native", Some(70)),
            workspace("worker", "docker", None),
        ]);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].values, vec![Some(100.0), Some(150.0)]);
        assert_eq!(groups[1].values, vec![Some(70.0), None]);
    }

    #[test]
    fn no_workspace_gives_no_groups() {
        assert!(native_vs_docker(&[]).is_empty());
    }
}
