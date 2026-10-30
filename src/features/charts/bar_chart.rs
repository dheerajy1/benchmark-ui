use dioxus::prelude::*;

use super::{
    frame::{
        y_scale, ChartEmpty, ChartLegend, ChartTooltip, HoverZones, PlotFrame, TooltipLine, XAlign,
    },
    palette::ChartTone,
    scale::nice_domain,
};
use crate::format::{format_optional, ValueFormat};

// Widest a single bar may grow, as a percentage of the plot width
const MAX_BAR_PERCENT: f64 = 12.0;

// Space between the bars of one cluster, as a percentage of the plot width
const BAR_GAP_PERCENT: f64 = 1.0;

// Thinnest a bar is drawn, in percent of the plot height, so tiny values stay visible
const MIN_BAR_HEIGHT: f64 = 0.4;

#[derive(Debug, Clone, PartialEq)]
pub struct BarSeries {
    pub name: String,
    pub tone: ChartTone,
}

// One cluster of bars on the x axis, `values` has one entry per series
#[derive(Debug, Clone, PartialEq)]
pub struct BarGroup {
    pub label: String,
    pub values: Vec<Option<f64>>,
}

// Drawn bar in the 0 to 100 plot box, every coordinate already formatted for the SVG attributes
struct Bar {
    x: String,
    y: String,
    width: String,
    height: String,
    fill_class: &'static str,
}

// Vertical bars on a zero baseline, several series are drawn side by side
#[component]
pub fn BarChart(
    #[props(into)] title: String,
    groups: Vec<BarGroup>,
    series: Vec<BarSeries>,
    format: ValueFormat,
) -> Element {
    let hovered = use_signal(|| None::<usize>);

    let present: Vec<f64> = groups
        .iter()
        .flat_map(|group| group.values.iter().flatten().copied())
        .collect();

    if present.is_empty() || series.is_empty() {
        return rsx! {
            ChartEmpty { message: "No measurements available for this run." }
        };
    }

    let low = present
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min)
        .min(0.0);
    let high = present
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        .max(0.0);
    let domain = nice_domain(low, high, true, 5);
    let scale = y_scale(&domain);
    let zero_y = scale.map(0.0).clamp(0.0, 100.0);

    let group_count = groups.len();
    let group_width = 100.0 / group_count as f64;
    let series_count = series.len();
    let gaps = BAR_GAP_PERCENT * (series_count - 1) as f64;
    let fitted = (group_width * 0.7 - gaps) / series_count as f64;
    let bar_width = fitted.clamp(0.5, MAX_BAR_PERCENT);
    let cluster_width = bar_width * series_count as f64 + gaps;

    let mut bars: Vec<Bar> = Vec::new();
    for (group_index, group) in groups.iter().enumerate() {
        let group_left = group_width * group_index as f64;
        let cluster_left = group_left + (group_width - cluster_width) / 2.0;

        for (series_index, value) in group.values.iter().enumerate() {
            let (Some(value), Some(entry)) = (value, series.get(series_index)) else {
                continue;
            };

            let value_y = scale.map(*value);
            let top = value_y.min(zero_y);
            let height = (zero_y - value_y).abs().max(MIN_BAR_HEIGHT);
            let left = cluster_left + series_index as f64 * (bar_width + BAR_GAP_PERCENT);

            bars.push(Bar {
                x: format!("{left:.2}"),
                y: format!("{top:.2}"),
                width: format!("{bar_width:.2}"),
                height: format!("{height:.2}"),
                fill_class: entry.tone.fill_class(),
            });
        }
    }

    let zone_labels: Vec<String> = groups
        .iter()
        .map(|group| {
            let summary = group
                .values
                .iter()
                .enumerate()
                .map(|(series_index, value)| {
                    let name = series
                        .get(series_index)
                        .map(|entry| entry.name.as_str())
                        .unwrap_or("");
                    format!("{name} {}", format_optional(*value, format))
                })
                .collect::<Vec<_>>()
                .join(", ");

            format!("{}: {summary}", group.label)
        })
        .collect();

    let x_labels: Vec<(String, XAlign)> = groups
        .iter()
        .map(|group| (group.label.clone(), XAlign::Center))
        .collect();

    let active = hovered();
    let tooltip = active.and_then(|index| {
        groups.get(index).map(|group| {
            let lines = group
                .values
                .iter()
                .enumerate()
                .filter_map(|(series_index, value)| {
                    series.get(series_index).map(|entry| TooltipLine {
                        label: entry.name.clone(),
                        value: format_optional(*value, format),
                        swatch: Some(entry.tone),
                    })
                })
                .collect::<Vec<_>>();

            (
                (index as f64 + 0.5) * group_width,
                group.label.clone(),
                lines,
            )
        })
    });

    let legend: Vec<(String, ChartTone)> = series
        .iter()
        .map(|entry| (entry.name.clone(), entry.tone))
        .collect();
    let show_legend = series.len() > 1;

    rsx! {
        div { class: "w-full", "role": "group", "aria-label": "{title}",
            if show_legend {
                ChartLegend { entries: legend }
            }
            PlotFrame { domain: domain.clone(), format, x_labels,
                svg {
                    class: "absolute inset-0 h-full w-full",
                    view_box: "0 0 100 100",
                    "preserveAspectRatio": "none",
                    "aria-hidden": "true",
                    for (index , bar) in bars.into_iter().enumerate() {
                        rect {
                            key: "{index}",
                            x: "{bar.x}",
                            y: "{bar.y}",
                            width: "{bar.width}",
                            height: "{bar.height}",
                            class: "{bar.fill_class}",
                        }
                    }
                }
                HoverZones { labels: zone_labels, active: hovered }
                if let Some((anchor_percent , heading , lines)) = tooltip {
                    ChartTooltip { anchor_percent, heading, lines }
                }
            }
        }
    }
}
