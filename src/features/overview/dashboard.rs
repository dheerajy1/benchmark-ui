use dioxus::prelude::*;

use super::{
    charts_section::OverviewCharts, latest_run::LatestRun, metric_tile::MetricTile,
    metrics::TrendMetric, models::OverviewData,
};

// Latest run against the one before it, the six tiles and the charts of one project
#[component]
pub fn Dashboard(data: OverviewData) -> Element {
    let Some(latest) = data.latest.clone() else {
        return rsx! {};
    };

    let latest_run_id = latest.run_id.clone();

    rsx! {
        div { class: "flex flex-col gap-(--space-4)",
            LatestRun { run: latest, previous: data.previous.clone() }
            div { class: "grid grid-cols-1 gap-(--space-4) sm:grid-cols-2 lg:grid-cols-3",
                for metric in TrendMetric::ALL {
                    MetricTile {
                        key: "{metric.label()}",
                        metric,
                        trends: data.trends.clone(),
                    }
                }
            }
            OverviewCharts { trends: data.trends.clone(), latest_run_id }
        }
    }
}
