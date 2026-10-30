use dioxus::prelude::*;

use super::{
    frame::{
        baseline_y, inner_height, inner_width, view_box, y_scale, ChartEmpty, ChartGrid,
        ChartLegend, SvgTooltip, TooltipLine, HEIGHT, MARGIN_LEFT, MARGIN_TOP,
    },
    palette::ChartTone,
    scale::nice_domain,
};
use crate::format::{format_optional, ValueFormat};

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

// Drawn bar with every coordinate already formatted for the SVG attributes
struct Bar {
    x: String,
    y: String,
    width: String,
    height: String,
    fill_class: &'static str,
}

struct GroupZone {
    index: usize,
    x: String,
    width: String,
    label_x: String,
    label: String,
    aria: String,
}

// Vertical bars on a zero baseline, several series are drawn side by side
#[component]
pub fn BarChart(
    #[props(into)] title: String,
    groups: Vec<BarGroup>,
    series: Vec<BarSeries>,
    format: ValueFormat,
) -> Element {
    let mut hovered = use_signal(|| None::<usize>);

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
    let zero_y = scale.map(0.0).clamp(MARGIN_TOP, baseline_y());

    let group_width = inner_width() / groups.len() as f64;
    let group_padding = group_width * 0.18;
    let usable_width = group_width - group_padding * 2.0;
    let series_count = series.len();
    let bar_gap = if series_count > 1 { 3.0 } else { 0.0 };
    let bar_width =
        ((usable_width - bar_gap * (series_count - 1) as f64) / series_count as f64).max(1.0);

    let mut bars: Vec<Bar> = Vec::new();
    for (group_index, group) in groups.iter().enumerate() {
        let group_left = MARGIN_LEFT + group_width * group_index as f64;

        for (series_index, value) in group.values.iter().enumerate() {
            let (Some(value), Some(entry)) = (value, series.get(series_index)) else {
                continue;
            };

            let value_y = scale.map(*value);
            let top = value_y.min(zero_y);
            let height = (zero_y - value_y).abs();
            let left = group_left + group_padding + series_index as f64 * (bar_width + bar_gap);

            bars.push(Bar {
                x: format!("{left:.2}"),
                y: format!("{top:.2}"),
                width: format!("{bar_width:.2}"),
                height: format!("{height:.2}"),
                fill_class: entry.tone.fill_class(),
            });
        }
    }

    let zones: Vec<GroupZone> = groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            let left = MARGIN_LEFT + group_width * index as f64;
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

            GroupZone {
                index,
                x: format!("{left:.2}"),
                width: format!("{group_width:.2}"),
                label_x: format!("{:.2}", left + group_width / 2.0),
                label: group.label.clone(),
                aria: format!("{}: {summary}", group.label),
            }
        })
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
                MARGIN_LEFT + group_width * index as f64 + group_width / 2.0,
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

    let view_box_value = view_box();
    let plot_top = MARGIN_TOP;
    let plot_height = inner_height();
    let axis_y = HEIGHT - 8.0;

    rsx! {
        div { class: "w-full",
            if show_legend {
                ChartLegend { entries: legend }
            }
            svg {
                class: "h-auto w-full",
                view_box: "{view_box_value}",
                "role": "group",
                "aria-label": "{title}",
                ChartGrid { domain: domain.clone(), format }
                for bar in bars {
                    rect {
                        x: "{bar.x}",
                        y: "{bar.y}",
                        width: "{bar.width}",
                        height: "{bar.height}",
                        rx: "3",
                        class: "{bar.fill_class}",
                    }
                }
                for zone in zones {
                    text {
                        x: "{zone.label_x}",
                        y: "{axis_y}",
                        text_anchor: "middle",
                        class: "fill-(--clr-text-faint) text-(length:--fs-xs)",
                        "{zone.label}"
                    }
                    rect {
                        x: "{zone.x}",
                        y: "{plot_top}",
                        width: "{zone.width}",
                        height: "{plot_height}",
                        class: "fill-transparent",
                        "tabindex": "0",
                        "role": "img",
                        "aria-label": "{zone.aria}",
                        onmouseenter: move |_| hovered.set(Some(zone.index)),
                        onmouseleave: move |_| hovered.set(None),
                        onfocus: move |_| hovered.set(Some(zone.index)),
                        onblur: move |_| hovered.set(None),
                    }
                }
                if let Some((anchor_x , heading , lines)) = tooltip {
                    SvgTooltip { anchor_x, heading, lines }
                }
            }
        }
    }
}
