use dioxus::prelude::*;

use super::{
    frame::{y_scale, ChartEmpty, ChartTooltip, HoverZones, PlotFrame, TooltipLine, XAlign},
    geometry,
    palette::ChartTone,
    scale::nice_domain,
};
use crate::format::{format_optional, ValueFormat};

// One position on the x axis, `value` is `None` when the run did not measure it
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesPoint {
    // Short label printed under the axis, only the first and last are shown
    pub axis_label: String,
    // Longer label shown in the tooltip
    pub detail_label: String,
    pub value: Option<f64>,
}

// Dot on the line, positions are percentages of the plot area
struct Dot {
    left: String,
    top: String,
    class: String,
}

// Line and area mark over evenly spaced points, gaps stay gaps and are never drawn as zero
#[component]
pub fn TrendChart(
    #[props(into)] title: String,
    points: Vec<SeriesPoint>,
    format: ValueFormat,
    #[props(default)] tone: ChartTone,
) -> Element {
    let hovered = use_signal(|| None::<usize>);

    let present: Vec<f64> = points.iter().filter_map(|point| point.value).collect();

    if present.is_empty() {
        return rsx! {
            ChartEmpty { message: "No measurements recorded for this metric yet." }
        };
    }

    let low = present.iter().copied().fold(f64::INFINITY, f64::min);
    let high = present.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let domain = nice_domain(low, high, false, 5);
    let scale = y_scale(&domain);

    let count = points.len();
    // Every point owns an equal column, so hover zones and marks always line up
    let x_percent = |index: usize| (index as f64 + 0.5) / count as f64 * 100.0;

    let coordinates: Vec<Option<(f64, f64)>> = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            point
                .value
                .map(|value| (x_percent(index), scale.map(value)))
        })
        .collect();

    let runs = geometry::segments(&coordinates);

    let line_paths: Vec<String> = runs
        .iter()
        .filter(|run| run.len() >= 2)
        .map(|run| geometry::line_path(run))
        .collect();
    let area_paths: Vec<String> = runs
        .iter()
        .filter(|run| run.len() >= 2)
        .map(|run| geometry::area_path(run, 100.0))
        .collect();

    let active = hovered();

    let swatch = tone.swatch_class();
    let dots: Vec<Dot> = coordinates
        .iter()
        .enumerate()
        .filter_map(|(index, coordinate)| {
            let (x, y) = (*coordinate)?;
            let size_class = if active == Some(index) {
                "size-3 border-2"
            } else {
                "size-2 border"
            };

            Some(Dot {
                left: format!("{x:.2}%"),
                top: format!("{y:.2}%"),
                class: format!(
                    "pointer-events-none absolute -translate-x-1/2 -translate-y-1/2 rounded-(--round-full) border-(color:--clr-surface) {size_class} {swatch}"
                ),
            })
        })
        .collect();

    let zone_labels: Vec<String> = points
        .iter()
        .map(|point| {
            format!(
                "{}: {}",
                point.detail_label,
                format_optional(point.value, format)
            )
        })
        .collect();

    let guide_left = active.map(|index| format!("{:.2}%", x_percent(index)));
    let tooltip = active.and_then(|index| {
        points.get(index).map(|point| {
            let lines = vec![TooltipLine {
                label: title.clone(),
                value: format_optional(point.value, format),
                swatch: Some(tone),
            }];

            (x_percent(index), point.detail_label.clone(), lines)
        })
    });

    // Only the first and last point carry an axis label, the rest stay empty
    let x_labels: Vec<(String, XAlign)> = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            if count == 1 {
                (point.axis_label.clone(), XAlign::Center)
            } else if index == 0 {
                (point.axis_label.clone(), XAlign::Start)
            } else if index + 1 == count {
                (point.axis_label.clone(), XAlign::End)
            } else {
                (String::new(), XAlign::Center)
            }
        })
        .collect();

    let stroke = tone.stroke_class();
    let soft = tone.soft_fill_class();

    rsx! {
        div { class: "w-full", "role": "group", "aria-label": "{title}",
            PlotFrame { domain: domain.clone(), format, x_labels,
                svg {
                    class: "absolute inset-0 h-full w-full",
                    view_box: "0 0 100 100",
                    "preserveAspectRatio": "none",
                    "aria-hidden": "true",
                    for d in area_paths {
                        path { d: "{d}", class: "{soft} stroke-none" }
                    }
                    for d in line_paths {
                        path {
                            d: "{d}",
                            stroke_linejoin: "round",
                            stroke_linecap: "round",
                            class: "vector-fixed fill-none stroke-2 {stroke}",
                        }
                    }
                }
                if let Some(left) = guide_left {
                    div {
                        class: "pointer-events-none absolute inset-y-0 w-px bg-(--clr-border-strong)",
                        style: "left: {left}",
                    }
                }
                for (index , dot) in dots.into_iter().enumerate() {
                    span {
                        key: "{index}",
                        class: "{dot.class}",
                        style: "left: {dot.left}; top: {dot.top}",
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
