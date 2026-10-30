use dioxus::prelude::*;

use super::{
    latest_run::LatestRun,
    metrics::{change_hint, summary_value, TrendMetric},
    models::OverviewData,
    trend_card::TrendCard,
};
use crate::{
    components::{StatTile, Tone},
    format::{format_number, format_optional, path_tail, MISSING},
};

// Headline tiles, newest run and trend charts of a non-empty history
#[component]
pub fn Dashboard(data: OverviewData) -> Element {
    let Some(latest) = data.latest.clone() else {
        return rsx! {};
    };

    let tiles: Vec<(String, String, String, Tone)> = TrendMetric::ALL
        .iter()
        .map(|metric| {
            let value = format_optional(summary_value(*metric, &latest.summary), metric.format());
            let (hint, tone) = change_hint(&data.trends, *metric);

            (metric.label().to_string(), value, hint, tone)
        })
        .collect();

    let total_runs = format_number(data.totals.total_runs as f64, 0);
    let runtime = latest
        .discovery
        .runtime
        .clone()
        .unwrap_or_else(|| MISSING.to_string());
    let target_tail = path_tail(&latest.target.path);
    let previous_run_id = data.trends.get(1).map(|point| point.run_id.clone());

    rsx! {
        div { class: "flex flex-col gap-(--space-6)",
            div { class: "grid grid-cols-2 gap-(--space-4) lg:grid-cols-4",
                StatTile {
                    label: "Total runs",
                    value: total_runs,
                    hint: "Stored by the API",
                }
                for (label , value , hint , tone) in tiles {
                    StatTile { label, value, hint, tone }
                }
                StatTile { label: "Runtime", value: runtime, hint: target_tail }
            }
            LatestRun { run: latest, previous_run_id }
            div { class: "grid grid-cols-1 gap-(--space-4) lg:grid-cols-2 2xl:grid-cols-3",
                for (index , metric) in TrendMetric::ALL.into_iter().enumerate() {
                    TrendCard {
                        key: "{index}",
                        metric,
                        trends: data.trends.clone(),
                    }
                }
            }
        }
    }
}
