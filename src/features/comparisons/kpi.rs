use dioxus::prelude::*;

use super::models::{ComparisonData, MetricDiff};
use crate::{
    components::DeltaBadge,
    features::charts::split_unit,
    format::{format_optional, ValueFormat},
};

// One big number of the strip: the comparison value, the change, and the baseline it came from
#[component]
pub fn KpiTile(#[props(into)] label: String, diff: MetricDiff, format: ValueFormat) -> Element {
    let (number, unit) = split_unit(&format_optional(diff.comparison_value, format));
    let baseline = format_optional(diff.baseline_value, format);

    rsx! {
        div { class: "group fade-up flex min-w-0 flex-col gap-(--space-2) rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) px-(--space-4) py-(--space-3) shadow-(--shadow-card)",
            div { class: "flex items-center justify-between gap-(--space-2)",
                span { class: "text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text-muted)",
                    "{label}"
                }
                DeltaBadge {
                    change: diff.percentage_change,
                    lower_is_better: diff.lower_is_better(),
                }
            }
            div { class: "flex items-baseline gap-(--space-1)",
                span { class: "text-(length:--fs-kpi) font-(weight:--fw-semibold) tabular-nums text-(color:--clr-text)",
                    "{number}"
                }
                if !unit.is_empty() {
                    span { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)", "{unit}" }
                }
            }
            p { class: "flex items-center gap-(--space-1) text-(length:--fs-xs) text-(color:--clr-text-faint)",
                "baseline {baseline}"
                span { class: "inline-block transition-transform duration-200 ease-(--ease-out) group-hover:translate-x-1",
                    "aria-hidden": "true",
                    "→"
                }
            }
        }
    }
}

// Four numbers that matter most, repeated from the hero on purpose as big figures
#[component]
pub fn KpiStrip(data: ComparisonData) -> Element {
    let metrics = &data.metrics;

    rsx! {
        div { class: "grid grid-cols-1 gap-(--space-3) sm:grid-cols-2 xl:grid-cols-4",
            KpiTile {
                label: "Throughput",
                diff: metrics.http_latency.requests_per_second.clone(),
                format: ValueFormat::PerSecond,
            }
            KpiTile {
                label: "p95 latency",
                diff: metrics.http_latency.p95_ms.clone(),
                format: ValueFormat::LatencyMs,
            }
            KpiTile {
                label: "Peak memory",
                diff: metrics.memory.peak_rss_bytes.clone(),
                format: ValueFormat::Bytes,
            }
            KpiTile {
                label: "Build time",
                diff: metrics.build_time.duration_ms.clone(),
                format: ValueFormat::DurationMs,
            }
        }
    }
}
