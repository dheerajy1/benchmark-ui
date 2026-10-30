use dioxus::prelude::*;

use super::{
    frame::{
        baseline_y, inner_height, inner_width, view_box, y_scale, ChartEmpty, ChartGrid,
        SvgTooltip, TooltipLine, HEIGHT, MARGIN_LEFT, MARGIN_RIGHT, MARGIN_TOP, WIDTH,
    },
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

// Invisible hit area that drives the hover tooltip and keyboard focus
struct HoverZone {
    index: usize,
    x: String,
    width: String,
    label: String,
}

// Line and area mark over evenly spaced points, gaps stay gaps and are never drawn as zero
#[component]
pub fn TrendChart(
    #[props(into)] title: String,
    points: Vec<SeriesPoint>,
    format: ValueFormat,
    #[props(default)] tone: ChartTone,
) -> Element {
    let mut hovered = use_signal(|| None::<usize>);

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
    let step = if count > 1 {
        inner_width() / (count - 1) as f64
    } else {
        0.0
    };
    let x_at = |index: usize| {
        if count > 1 {
            MARGIN_LEFT + step * index as f64
        } else {
            MARGIN_LEFT + inner_width() / 2.0
        }
    };

    let coordinates: Vec<Option<(f64, f64)>> = points
        .iter()
        .enumerate()
        .map(|(index, point)| point.value.map(|value| (x_at(index), scale.map(value))))
        .collect();

    let runs = geometry::segments(&coordinates);
    let baseline = baseline_y();

    let line_paths: Vec<String> = runs
        .iter()
        .filter(|run| run.len() >= 2)
        .map(|run| geometry::line_path(run))
        .collect();
    let area_paths: Vec<String> = runs
        .iter()
        .filter(|run| run.len() >= 2)
        .map(|run| geometry::area_path(run, baseline))
        .collect();

    let dots: Vec<(String, String)> = coordinates
        .iter()
        .flatten()
        .map(|(x, y)| (format!("{x:.2}"), format!("{y:.2}")))
        .collect();

    let zone_width = if count > 1 { step } else { inner_width() };
    let zones: Vec<HoverZone> = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let left = (x_at(index) - zone_width / 2.0).max(MARGIN_LEFT);

            HoverZone {
                index,
                x: format!("{left:.2}"),
                width: format!("{zone_width:.2}"),
                label: format!(
                    "{}: {}",
                    point.detail_label,
                    format_optional(point.value, format)
                ),
            }
        })
        .collect();

    let active = hovered();

    let guide_x = active.map(|index| format!("{:.2}", x_at(index)));
    let active_dot = active
        .and_then(|index| coordinates.get(index).copied().flatten())
        .map(|(x, y)| (format!("{x:.2}"), format!("{y:.2}")));
    let tooltip = active.and_then(|index| {
        points.get(index).map(|point| {
            let lines = vec![TooltipLine {
                label: title.clone(),
                value: format_optional(point.value, format),
                swatch: Some(tone),
            }];

            (x_at(index), point.detail_label.clone(), lines)
        })
    });

    let first_label = points
        .first()
        .map(|point| point.axis_label.clone())
        .unwrap_or_default();
    let last_label = points
        .last()
        .map(|point| point.axis_label.clone())
        .unwrap_or_default();

    let view_box_value = view_box();
    let plot_top = MARGIN_TOP;
    let plot_height = inner_height();
    let axis_y = HEIGHT - 8.0;
    let axis_end_x = WIDTH - MARGIN_RIGHT;
    let center_x = MARGIN_LEFT + inner_width() / 2.0;
    let guide_bottom = baseline;
    let stroke = tone.stroke_class();
    let fill = tone.fill_class();
    let soft = tone.soft_fill_class();

    rsx! {
        svg {
            class: "h-auto w-full",
            view_box: "{view_box_value}",
            "role": "group",
            "aria-label": "{title}",
            ChartGrid { domain: domain.clone(), format }
            for d in area_paths {
                path { d: "{d}", class: "{soft} stroke-none" }
            }
            for d in line_paths {
                path {
                    d: "{d}",
                    stroke_linejoin: "round",
                    stroke_linecap: "round",
                    class: "fill-none stroke-2 {stroke}",
                }
            }
            for (cx , cy) in dots {
                circle {
                    cx: "{cx}",
                    cy: "{cy}",
                    r: "3",
                    class: "{fill} stroke-(color:--clr-surface) stroke-1",
                }
            }
            if count > 1 {
                text {
                    x: "{MARGIN_LEFT}",
                    y: "{axis_y}",
                    text_anchor: "start",
                    class: "fill-(--clr-text-faint) text-(length:--fs-xs)",
                    "{first_label}"
                }
                text {
                    x: "{axis_end_x}",
                    y: "{axis_y}",
                    text_anchor: "end",
                    class: "fill-(--clr-text-faint) text-(length:--fs-xs)",
                    "{last_label}"
                }
            } else {
                text {
                    x: "{center_x}",
                    y: "{axis_y}",
                    text_anchor: "middle",
                    class: "fill-(--clr-text-faint) text-(length:--fs-xs)",
                    "{first_label}"
                }
            }
            if let Some(x) = guide_x {
                line {
                    x1: "{x}",
                    x2: "{x}",
                    y1: "{plot_top}",
                    y2: "{guide_bottom}",
                    class: "stroke-(color:--clr-border-strong)",
                }
            }
            if let Some((cx , cy)) = active_dot {
                circle {
                    cx: "{cx}",
                    cy: "{cy}",
                    r: "5",
                    class: "{fill} stroke-(color:--clr-surface) stroke-2",
                }
            }
            for zone in zones {
                rect {
                    x: "{zone.x}",
                    y: "{plot_top}",
                    width: "{zone.width}",
                    height: "{plot_height}",
                    class: "fill-transparent",
                    "tabindex": "0",
                    "role": "img",
                    "aria-label": "{zone.label}",
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
