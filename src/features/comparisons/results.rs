use dioxus::prelude::*;

use super::{
    models::{ComparisonData, MetricDiff, RunReference},
    rows::metric_rows,
};
use crate::{
    app::Route,
    components::{Badge, Card, CardHeader, DeltaBadge, StatusBadge, Tone},
    features::charts::{BarChart, BarGroup, BarSeries, ChartTone},
    format::{format_timestamp, short_id, ValueFormat, MISSING},
};

const HEADER_CELL: &str = "px-(--space-3) py-(--space-2) text-(length:--fs-xs) font-(weight:--fw-medium) uppercase tracking-wider text-(color:--clr-text-faint)";

fn pair_group(label: &str, diff: &MetricDiff) -> BarGroup {
    BarGroup {
        label: label.to_string(),
        values: vec![diff.baseline_value, diff.comparison_value],
    }
}

fn pair_series() -> Vec<BarSeries> {
    vec![
        BarSeries {
            name: "Baseline".to_string(),
            tone: ChartTone::Blue,
        },
        BarSeries {
            name: "Comparison".to_string(),
            tone: ChartTone::Green,
        },
    ]
}

// Identity of one side of the comparison
#[component]
fn RunCard(#[props(into)] role: String, run: RunReference) -> Element {
    let short = short_id(&run.run_id);
    let created = format_timestamp(&run.created_at);

    rsx! {
        Card { class: "p-(--space-5)",
            div { class: "flex items-center justify-between gap-(--space-3)",
                span { class: "text-(length:--fs-xs) font-(weight:--fw-medium) uppercase tracking-wider text-(color:--clr-text-faint)",
                    "{role}"
                }
                if let Some(runtime) = run.runtime.clone() {
                    Badge { label: runtime, tone: Tone::Accent }
                }
            }
            Link {
                to: Route::RunDetail {
                    run_id: run.run_id.clone(),
                },
                class: "mt-(--space-2) inline-block font-(family-name:--font-code) text-(length:--fs-lg) text-(color:--clr-accent) hover:underline",
                "{short}"
            }
            p { class: "mt-(--space-1) break-all font-(family-name:--font-code) text-(length:--fs-xs) text-(color:--clr-text-muted)",
                "{run.target.path}"
            }
            p { class: "text-(length:--fs-xs) text-(color:--clr-text-faint)", "{created}" }
        }
    }
}

// Grouped bars of baseline against comparison
#[component]
fn PairChart(#[props(into)] title: String, groups: Vec<BarGroup>, format: ValueFormat) -> Element {
    rsx! {
        Card {
            CardHeader { title: title.clone() }
            div { class: "p-(--space-4)",
                BarChart {
                    title,
                    groups,
                    series: pair_series(),
                    format,
                }
            }
        }
    }
}

// Why a side of a metric may be missing, the backend reports the step statuses
#[component]
fn StatusRow(
    #[props(into)] label: String,
    baseline: Option<String>,
    comparison: Option<String>,
) -> Element {
    rsx! {
        div { class: "flex flex-wrap items-center gap-(--space-2)",
            span { class: "w-20 text-(length:--fs-sm) text-(color:--clr-text-muted)", "{label}" }
            if let Some(status) = baseline {
                StatusBadge { status }
            } else {
                span { class: "text-(color:--clr-text-faint)", "{MISSING}" }
            }
            span { class: "text-(color:--clr-text-faint)", "→" }
            if let Some(status) = comparison {
                StatusBadge { status }
            } else {
                span { class: "text-(color:--clr-text-faint)", "{MISSING}" }
            }
        }
    }
}

// Table row with every cell already formatted
struct TableRow {
    group: &'static str,
    label: &'static str,
    baseline: String,
    comparison: String,
    difference: String,
    change: Option<f64>,
    lower_is_better: bool,
}

// Run identities, step statuses, charts and the full metric table of one comparison
#[component]
pub fn ComparisonResults(data: ComparisonData) -> Element {
    let metrics = &data.metrics;

    let throughput = vec![pair_group(
        "Requests/s",
        &metrics.http_latency.requests_per_second,
    )];
    let latency = vec![
        pair_group("p50", &metrics.http_latency.p50_ms),
        pair_group("p95", &metrics.http_latency.p95_ms),
        pair_group("p99", &metrics.http_latency.p99_ms),
    ];
    let memory = vec![
        pair_group("baseline", &metrics.memory.baseline_rss_bytes),
        pair_group("peak", &metrics.memory.peak_rss_bytes),
        pair_group("delta", &metrics.memory.delta_rss_bytes),
    ];
    let cpu = vec![
        pair_group("baseline", &metrics.cpu.baseline_percent),
        pair_group("average", &metrics.cpu.average_percent),
        pair_group("peak", &metrics.cpu.peak_percent),
    ];
    let timing = vec![
        pair_group("Build", &metrics.build_time.duration_ms),
        pair_group("Startup", &metrics.startup_time.duration_ms),
    ];

    let rows: Vec<TableRow> = metric_rows(&data)
        .into_iter()
        .map(|row| TableRow {
            group: row.group,
            label: row.label,
            baseline: row.baseline_text(),
            comparison: row.comparison_text(),
            difference: row.difference_text(),
            change: row.diff.percentage_change,
            lower_is_better: row.diff.lower_is_better(),
        })
        .collect();

    rsx! {
        div { class: "flex flex-col gap-(--space-6)",
            div { class: "grid grid-cols-1 gap-(--space-4) md:grid-cols-2",
                RunCard { role: "Baseline", run: data.baseline.clone() }
                RunCard { role: "Comparison", run: data.comparison.clone() }
            }
            Card {
                CardHeader {
                    title: "Measurement status",
                    subtitle: "Baseline → comparison. A value is only compared when both sides succeeded.",
                }
                div { class: "flex flex-col gap-(--space-3) p-(--space-5)",
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
            div { class: "grid grid-cols-1 gap-(--space-4) lg:grid-cols-2",
                PairChart { title: "Throughput", groups: throughput, format: ValueFormat::PerSecond }
                PairChart { title: "Latency", groups: latency, format: ValueFormat::LatencyMs }
                PairChart { title: "Memory", groups: memory, format: ValueFormat::Bytes }
                PairChart { title: "CPU", groups: cpu, format: ValueFormat::Percent }
                PairChart { title: "Build and startup", groups: timing, format: ValueFormat::DurationMs }
            }
            Card {
                CardHeader {
                    title: "All metrics",
                    subtitle: "Change is relative to the baseline. Green is an improvement.",
                }
                div { class: "overflow-x-auto",
                    table { class: "w-full min-w-(--table-min-width) border-collapse text-left text-(length:--fs-sm)",
                        thead {
                            tr {
                                th { class: "{HEADER_CELL}", "Group" }
                                th { class: "{HEADER_CELL}", "Metric" }
                                th { class: "{HEADER_CELL} text-right", "Baseline" }
                                th { class: "{HEADER_CELL} text-right", "Comparison" }
                                th { class: "{HEADER_CELL} text-right", "Difference" }
                                th { class: "{HEADER_CELL} text-right", "Change" }
                            }
                        }
                        tbody {
                            for row in rows {
                                tr { class: "border-t border-(color:--clr-border) hover:bg-(--clr-surface-hover)",
                                    td { class: "px-(--space-3) py-(--space-3) text-(color:--clr-text-faint)", "{row.group}" }
                                    td { class: "px-(--space-3) py-(--space-3)", "{row.label}" }
                                    td { class: "px-(--space-3) py-(--space-3) text-right tabular-nums", "{row.baseline}" }
                                    td { class: "px-(--space-3) py-(--space-3) text-right tabular-nums", "{row.comparison}" }
                                    td { class: "px-(--space-3) py-(--space-3) text-right tabular-nums", "{row.difference}" }
                                    td { class: "px-(--space-3) py-(--space-3) text-right",
                                        DeltaBadge { change: row.change, lower_is_better: row.lower_is_better }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
