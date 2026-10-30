use dioxus::prelude::*;

use super::{
    metrics::{series_points, TrendMetric},
    models::TrendPoint,
};
use crate::{
    components::Card,
    features::charts::TrendChart,
    format::{format_optional, MISSING},
};

// One metric over the newest runs with its latest value
#[component]
pub fn TrendCard(metric: TrendMetric, trends: Vec<TrendPoint>) -> Element {
    let points = series_points(&trends, metric);
    let format = metric.format();
    let latest = format_optional(trends.first().and_then(|point| metric.value(point)), format);
    let latest_class = if latest == MISSING {
        "text-(color:--clr-text-faint)"
    } else {
        "text-(color:--clr-text)"
    };
    let run_count = trends.len();
    let label = metric.label();

    rsx! {
        Card {
            div { class: "flex items-start justify-between gap-(--space-3) px-(--space-5) pt-(--space-4)",
                div { class: "min-w-0",
                    h3 { class: "text-(length:--fs-base) font-(weight:--fw-semibold) text-(color:--clr-text)",
                        "{label}"
                    }
                    p { class: "text-(length:--fs-xs) text-(color:--clr-text-faint)", "Last {run_count} runs" }
                }
                span { class: "text-(length:--fs-xl) font-(weight:--fw-semibold) tabular-nums {latest_class}",
                    "{latest}"
                }
            }
            div { class: "p-(--space-4)",
                TrendChart {
                    title: label,
                    points,
                    format,
                    tone: metric.tone(),
                }
            }
        }
    }
}
