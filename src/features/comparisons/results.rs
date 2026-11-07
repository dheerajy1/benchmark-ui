use dioxus::prelude::*;

use super::{
    cards::{use_view_mode_provider, DataCard, ViewSwitch},
    models::{ComparisonData, MetricDiff},
    rows::{metric_rows, MetricRow},
    subscriber::{has_docker, SubscriberSection},
    summary::{diverge_rows, summary_rows},
    tables::MetricTable,
    trends::TrendsCard,
};
use crate::{
    components::{use_section_spy, Card, CardHeader, SectionBar, SectionBlock, StatusBadge},
    features::charts::{
        BarChart, BarGroup, BarSeries, ChartTone, DivergingBars, RangeBars, RangeRow, Segment,
        StackRow, StackedBars,
    },
    format::{ValueFormat, MISSING},
};

pub fn pair_group(label: &str, diff: &MetricDiff) -> BarGroup {
    BarGroup {
        label: label.to_string(),
        values: vec![diff.baseline_value, diff.comparison_value],
    }
}

pub fn pair_series() -> Vec<BarSeries> {
    vec![
        BarSeries {
            name: "Baseline".to_string(),
            tone: ChartTone::Slate,
        },
        BarSeries {
            name: "Comparison".to_string(),
            tone: ChartTone::Blue,
        },
    ]
}

// The rows of the given groups, in the order `metric_rows` lists them
fn rows_of(rows: &[MetricRow], groups: &[&str]) -> Vec<MetricRow> {
    rows.iter()
        .filter(|row| groups.contains(&row.group.as_str()))
        .cloned()
        .collect()
}

// One floating bar per run from its low value to its high value
fn range_rows(low: &MetricDiff, high: &MetricDiff, mark: Option<&MetricDiff>) -> Vec<RangeRow> {
    vec![
        RangeRow {
            label: "Baseline".to_string(),
            tone: ChartTone::Slate,
            low: low.baseline_value,
            high: high.baseline_value,
            mark: mark.and_then(|diff| diff.baseline_value),
        },
        RangeRow {
            label: "Comparison".to_string(),
            tone: ChartTone::Blue,
            low: low.comparison_value,
            high: high.comparison_value,
            mark: mark.and_then(|diff| diff.comparison_value),
        },
    ]
}

// Source and build output of each run as one stacked bar
fn size_rows(data: &ComparisonData) -> Vec<StackRow> {
    let artifact = &data.metrics.artifact_size;
    let row = |label: &str, source: Option<f64>, build: Option<f64>| StackRow {
        label: label.to_string(),
        segments: vec![
            Segment {
                name: "Source".to_string(),
                tone: ChartTone::Violet,
                value: source,
            },
            Segment {
                name: "Build output".to_string(),
                tone: ChartTone::Cyan,
                value: build,
            },
        ],
    };

    vec![
        row(
            "Baseline",
            artifact.source.bytes.baseline_value,
            artifact.build.bytes.baseline_value,
        ),
        row(
            "Comparison",
            artifact.source.bytes.comparison_value,
            artifact.build.bytes.comparison_value,
        ),
    ]
}

// Why a side of a metric may be missing, the backend reports the step statuses
#[component]
fn StatusRow(
    #[props(into)] label: String,
    baseline: Option<String>,
    comparison: Option<String>,
) -> Element {
    rsx! {
        div { class: "flex flex-col gap-(--space-2) rounded-(--round-md) border border-(color:--clr-border) bg-(--clr-surface-raised) px-(--space-3) py-(--space-2)",
            span { class: "text-(length:--fs-2xs) font-(weight:--fw-medium) uppercase tracking-wider text-(color:--clr-text-faint)",
                "{label}"
            }
            div { class: "flex flex-wrap items-center gap-(--space-2)",
                if let Some(status) = baseline {
                    StatusBadge { status }
                } else {
                    span { class: "text-(color:--clr-text-faint)", "{MISSING}" }
                }
                span { class: "text-(color:--clr-text-faint)", "aria-hidden": "true", "→" }
                if let Some(status) = comparison {
                    StatusBadge { status }
                } else {
                    span { class: "text-(color:--clr-text-faint)", "{MISSING}" }
                }
            }
        }
    }
}

// Sections of one comparison below the hero: summary, latency, resources, build and size, and
// the subscriber when either run had one
#[component]
pub fn ComparisonResults(data: ComparisonData) -> Element {
    use_view_mode_provider();
    let spy = use_section_spy();

    let metrics = &data.metrics;
    let all = metric_rows(&data);
    let has_workspaces = !metrics.workspaces.is_empty();

    let mut titles = vec!["Summary", "Latency", "Resources", "Build and size"];
    if has_workspaces {
        titles.push("Subscriber");
    }
    titles.push("Trends");

    let trends_index = titles.len() - 1;
    let summary = summary_rows(&data);
    let bars = diverge_rows(&summary);
    let latency = vec![
        pair_group("p50", &metrics.http_latency.p50_ms),
        pair_group("p95", &metrics.http_latency.p95_ms),
        pair_group("p99", &metrics.http_latency.p99_ms),
    ];
    let timing = vec![
        pair_group("Build", &metrics.build_time.duration_ms),
        pair_group("Startup", &metrics.startup_time.duration_ms),
    ];
    let memory = range_rows(
        &metrics.memory.baseline_rss_bytes,
        &metrics.memory.peak_rss_bytes,
        None,
    );
    let cpu = range_rows(
        &metrics.cpu.baseline_percent,
        &metrics.cpu.peak_percent,
        Some(&metrics.cpu.average_percent),
    );

    rsx! {
        SectionBar {
            titles,
            spy,
            trailing: Some(rsx! {
                ViewSwitch {}
            }),
        }
        div { class: "flex flex-col gap-(--space-6)",
            SectionBlock { index: 0, spy, title: "Summary",
                DataCard {
                    title: "What changed",
                    subtitle: "Better to the right, worse to the left. Bars past 150% are cut and marked.",
                    chart: Some(rsx! {
                        DivergingBars { rows: bars }
                    }),
                    table: Some(rsx! {
                        MetricTable { rows: summary, show_group: true }
                    }),
                }
            }
            SectionBlock { index: 1, spy, title: "Latency",
                DataCard {
                    title: "Latency by percentile",
                    subtitle: "Time to answer a request, lower is better.",
                    chart: Some(rsx! {
                        BarChart {
                            title: "Latency by percentile",
                            groups: latency,
                            series: pair_series(),
                            format: ValueFormat::LatencyMs,
                        }
                    }),
                    table: Some(rsx! {
                        MetricTable { rows: rows_of(&all, &["HTTP"]) }
                    }),
                }
            }
            SectionBlock { index: 2, spy, title: "Resources",
                DataCard {
                    title: "Memory and CPU range",
                    subtitle: "Each bar runs from the baseline to the peak. The dark tick is the CPU average.",
                    chart: Some(rsx! {
                        div { class: "flex flex-col gap-(--space-5)",
                            div { class: "flex flex-col gap-(--space-3)",
                                h4 { class: "text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text)",
                                    "Memory"
                                }
                                RangeBars { rows: memory, format: ValueFormat::Bytes }
                            }
                            div { class: "flex flex-col gap-(--space-3)",
                                h4 { class: "text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text)",
                                    "CPU"
                                }
                                RangeBars { rows: cpu, format: ValueFormat::Percent }
                            }
                        }
                    }),
                    table: Some(rsx! {
                        MetricTable { rows: rows_of(&all, &["Memory", "CPU"]), show_group: true }
                    }),
                }
            }
            SectionBlock { index: 3, spy, title: "Build and size",
                div { class: "grid grid-cols-1 gap-(--space-4) lg:grid-cols-2",
                    DataCard {
                        title: "Build and startup",
                        subtitle: "How long each step took.",
                        chart: Some(rsx! {
                            BarChart {
                                title: "Build and startup",
                                groups: timing,
                                series: pair_series(),
                                format: ValueFormat::DurationMs,
                            }
                        }),
                        table: Some(rsx! {
                            MetricTable { rows: rows_of(&all, &["Timing"]) }
                        }),
                    }
                    DataCard {
                        title: "Shipped size",
                        subtitle: "Source plus build output, excluded files left out.",
                        chart: Some(rsx! {
                            StackedBars { rows: size_rows(&data), format: ValueFormat::Bytes }
                        }),
                        table: Some(rsx! {
                            MetricTable { rows: rows_of(&all, &["Artifacts"]) }
                        }),
                    }
                }
                Card {
                    CardHeader {
                        title: "Measurement status",
                        subtitle: "Baseline → comparison. A value is only compared when both sides succeeded.",
                    }
                    div { class: "grid grid-cols-1 gap-(--space-3) p-(--space-4) sm:grid-cols-3",
                        StatusRow {
                            label: "Build",
                            baseline: Some(metrics.build_time.baseline_status.clone()),
                            comparison: Some(metrics.build_time.comparison_status.clone()),
                        }
                        StatusRow {
                            label: "Startup",
                            baseline: Some(metrics.startup_time.baseline_status.clone()),
                            comparison: Some(metrics.startup_time.comparison_status.clone()),
                        }
                        StatusRow {
                            label: "HTTP",
                            baseline: metrics.http_latency.baseline_status.clone(),
                            comparison: metrics.http_latency.comparison_status.clone(),
                        }
                    }
                }
            }
            if has_workspaces {
                SectionBlock { index: 4, spy, title: "Subscriber",
                    SubscriberSection { data: data.clone() }
                }
            }
            SectionBlock { index: trends_index, spy, title: "Trends",
                TrendsCard {
                    baseline: data.baseline.clone(),
                    comparison: data.comparison.clone(),
                    has_docker: has_docker(&data),
                }
            }
        }
    }
}
