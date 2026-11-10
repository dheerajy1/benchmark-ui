use dioxus::prelude::*;

use super::{
    metrics::{newest, requests_chip, series_points, short_day, TrendMetric},
    models::TrendPoint,
};
use crate::{
    components::{Card, CardError, CardHeader, GhostChart},
    features::{
        charts::{BarChart, BarGroup, BarSeries, ChartTone, TrendChart},
        run_detail::{fetch_run, pairs_of, BenchmarkResult, NativeVsDocker},
    },
    format::ValueFormat,
};

// Card with a chart inside, `class` can resize the chart through `--chart-height`
#[component]
fn ChartCard(
    #[props(into)] title: String,
    #[props(into)] subtitle: String,
    #[props(into, default)] class: String,
    // Small text at the right end of the header, empty for none
    #[props(into, default)] chip: String,
    children: Element,
) -> Element {
    let trailing = (!chip.is_empty()).then(|| {
        rsx! {
            span { class: "shrink-0 text-(length:--fs-xs) text-(color:--clr-text-muted) tabular-nums",
                "{chip}"
            }
        }
    });

    rsx! {
        Card { class: "fade-up {class}",
            CardHeader { title, subtitle, trailing }
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
                ChartCard {
                    title: "Native vs Docker",
                    subtitle: "Peak memory of the latest run.",
                    GhostChart {}
                }
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
                    title: "Couldn't load native vs Docker",
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

            let pairs = pairs_of(&run.metrics.workspaces);
            let containers = if pairs.is_empty() {
                rsx! {
                    ChartCard {
                        title: "Native vs Docker",
                        subtitle: "Peak memory of the latest run.",
                        {note("The latest run had no workspace measured in both modes.")}
                    }
                }
            } else {
                rsx! {
                    for (name , native , docker) in pairs {
                        NativeVsDocker { key: "{name}", name, native, docker }
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
                class: "lg:col-span-2",
                chip: requests_chip(&trends),
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
        }
        {containers}
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
