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

// A dashed line across the chart at a known value, for example the peak of the last run
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub value: f64,
    // Printed on the line
    pub label: String,
}

// Dot on the line, positions are percentages of the plot area
struct Dot {
    left: String,
    top: String,
    class: String,
}

// Half the width of a `Peak ...` label, so the label can stay inside the plot at any chart width
const PEAK_LABEL_HALF_WIDTH: &str = "3rem";

// Side of the plot the dashed-line label sits on. It takes the side the highest point is not on,
// so the `Peak ...` label and the dashed-line label never land on top of each other.
fn reference_label_class(peak_percent: Option<f64>) -> &'static str {
    match peak_percent {
        Some(percent) if percent >= 50.0 => "left-1",
        _ => "right-1",
    }
}

// Line and area mark over evenly spaced points, gaps stay gaps and are never drawn as zero
#[component]
pub fn TrendChart(
    #[props(into)] title: String,
    points: Vec<SeriesPoint>,
    format: ValueFormat,
    #[props(default)] tone: ChartTone,
    // Dashed line at a value from another run
    #[props(default)] reference: Option<Reference>,
    // A grey dashed line of readings from an earlier phase, stretched over the same width
    #[props(default)] ghost: Vec<f64>,
    // The newest point pulses, for a chart that is still being filled
    #[props(default)] live: bool,
    // Names the highest point
    #[props(default)] mark_peak: bool,
    // Grows the plot into the height its parent leaves over, the parent has to be a flex column
    #[props(default)] fill: bool,
) -> Element {
    let hovered = use_signal(|| None::<usize>);

    let present: Vec<f64> = points.iter().filter_map(|point| point.value).collect();

    if present.is_empty() {
        return rsx! {
            ChartEmpty { message: "No measurements recorded for this metric yet." }
        };
    }

    // The reference and the ghost stay inside the plot too
    let extra: Vec<f64> = reference
        .iter()
        .map(|line| line.value)
        .chain(ghost.iter().copied())
        .collect();
    let low = present
        .iter()
        .chain(extra.iter())
        .copied()
        .fold(f64::INFINITY, f64::min);
    let high = present
        .iter()
        .chain(extra.iter())
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
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

    let ghost_paths: Vec<String> = if ghost.len() >= 2 {
        let ghost_coordinates: Vec<Option<(f64, f64)>> = ghost
            .iter()
            .enumerate()
            .map(|(index, value)| {
                Some((
                    (index as f64 + 0.5) / ghost.len() as f64 * 100.0,
                    scale.map(*value),
                ))
            })
            .collect();

        geometry::segments(&ghost_coordinates)
            .iter()
            .map(|run| geometry::line_path(run))
            .collect()
    } else {
        Vec::new()
    };

    // Index and value of the highest point
    let peak_point = if mark_peak {
        points
            .iter()
            .enumerate()
            .filter_map(|(index, point)| point.value.map(|value| (index, value)))
            .fold(None::<(usize, f64)>, |best, current| match best {
                Some(best) if best.1 >= current.1 => Some(best),
                _ => Some(current),
            })
    } else {
        None
    };

    // Top position, label and side of the reference line
    let reference_class = reference_label_class(peak_point.map(|(index, _)| x_percent(index)));
    let reference_view = reference.as_ref().map(|line| {
        (
            format!("{:.2}%", scale.map(line.value)),
            line.label.clone(),
            reference_class,
        )
    });

    // Position of the newest point, where the pulse sits
    let ping = if live {
        coordinates
            .iter()
            .rposition(|coordinate| coordinate.is_some())
            .and_then(|index| coordinates[index])
            .map(|(x, y)| (format!("{x:.2}%"), format!("{y:.2}%")))
    } else {
        None
    };

    // Position and text of the highest point. The label is centered on the point but kept inside
    // the plot, so a peak at the far right or left never spills over the edge.
    let peak = peak_point.map(|(index, value)| {
        (
            format!(
                "clamp({PEAK_LABEL_HALF_WIDTH}, {:.2}%, calc(100% - {PEAK_LABEL_HALF_WIDTH}))",
                x_percent(index)
            ),
            format!("{:.2}%", scale.map(value)),
            format!("Peak {}", format_optional(Some(value), format)),
        )
    });

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
    let wrapper_class = if fill {
        "flex w-full flex-1 flex-col"
    } else {
        "w-full"
    };

    rsx! {
        div { class: "{wrapper_class}", "role": "group", "aria-label": "{title}",
            PlotFrame { domain: domain.clone(), format, x_labels, fill,
                svg {
                    class: "absolute inset-0 h-full w-full",
                    view_box: "0 0 100 100",
                    "preserveAspectRatio": "none",
                    "aria-hidden": "true",
                    for d in area_paths {
                        path { d: "{d}", class: "{soft} stroke-none" }
                    }
                    for d in ghost_paths {
                        path {
                            d: "{d}",
                            stroke_linejoin: "round",
                            class: "vector-fixed fill-none stroke-1 stroke-(--clr-text-faint) [stroke-dasharray:4_3]",
                        }
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
                if let Some((top , label , side)) = reference_view {
                    div {
                        class: "pointer-events-none absolute inset-x-0 border-t border-dashed border-(color:--clr-text-muted)",
                        style: "top: {top}",
                    }
                    span {
                        class: "pointer-events-none absolute {side} -translate-y-full whitespace-nowrap text-(length:--fs-2xs) text-(color:--clr-text-muted)",
                        style: "top: {top}",
                        "{label}"
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
                if let Some((left , top)) = ping {
                    span {
                        class: "pointer-events-none absolute size-3 -translate-x-1/2 -translate-y-1/2 animate-ping rounded-(--round-full) opacity-60 {swatch}",
                        style: "left: {left}; top: {top}",
                    }
                }
                if let Some((left , top , text)) = peak {
                    span {
                        class: "pointer-events-none absolute -translate-x-1/2 -translate-y-[170%] whitespace-nowrap rounded-(--round-sm) bg-(--clr-surface) px-1 text-(length:--fs-2xs) font-(weight:--fw-medium) text-(color:--clr-text)",
                        style: "left: {left}; top: {top}",
                        "{text}"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dashed_line_label_takes_the_side_the_peak_is_not_on() {
        assert_eq!(reference_label_class(Some(97.0)), "left-1");
        assert_eq!(reference_label_class(Some(50.0)), "left-1");
        assert_eq!(reference_label_class(Some(12.0)), "right-1");
    }

    #[test]
    fn without_a_marked_peak_the_dashed_line_label_stays_on_the_right() {
        assert_eq!(reference_label_class(None), "right-1");
    }
}
