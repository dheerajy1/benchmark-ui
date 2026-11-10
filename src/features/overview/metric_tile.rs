use dioxus::prelude::*;

use super::{
    metrics::{newest, requests_chip, TrendMetric},
    models::TrendPoint,
};
use crate::features::charts::TrendTile;

// One of the six overview tiles, fed with the newest runs of the project
#[component]
pub fn MetricTile(metric: TrendMetric, trends: Vec<TrendPoint>) -> Element {
    let chip = if metric == TrendMetric::Throughput {
        requests_chip(&trends)
    } else {
        String::new()
    };
    let values: Vec<Option<f64>> = newest(&trends)
        .iter()
        .rev()
        .map(|point| metric.value(point))
        .collect();

    rsx! {
        TrendTile {
            label: metric.label(),
            values,
            format: metric.format(),
            tone: metric.tone(),
            lower_is_better: metric.lower_is_better(),
            chip,
        }
    }
}
