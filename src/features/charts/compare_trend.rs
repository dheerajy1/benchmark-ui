use dioxus::prelude::*;

use super::{
    frame::{
        y_scale, ChartEmpty, ChartLegend, ChartTooltip, HoverZones, PlotFrame, TooltipLine, XAlign,
    },
    geometry,
    palette::ChartTone,
    scale::nice_domain,
};
use crate::format::{format_optional, ValueFormat};

// One line of the chart, `values` has one entry per label and is `None` where nothing was measured
#[derive(Debug, Clone, PartialEq)]
pub struct CompareSeries {
    pub name: String,
    pub tone: ChartTone,
    pub values: Vec<Option<f64>>,
    // Shown after the name in the legend only, empty for none
    pub note: String,
}

// The legend text of a series: its name, then the note when it has one
fn legend_label(line: &CompareSeries) -> String {
    if line.note.is_empty() {
        line.name.clone()
    } else {
        format!("{} · {}", line.name, line.note)
    }
}

// Dot on a line, positions are percentages of the plot area
struct Dot {
    left: String,
    top: String,
    class: String,
}

// Line paths and dots of one series
struct Drawing {
    stroke: &'static str,
    paths: Vec<String>,
    dots: Vec<Dot>,
}

// Pads the front with gaps so that two series of different length end at the same position
pub fn align_end(values: Vec<Option<f64>>, len: usize) -> Vec<Option<f64>> {
    let missing = len.saturating_sub(values.len());
    let mut aligned = vec![None; missing];

    aligned.extend(values);
    aligned
}

// Several lines over the same positions with one shared scrub line. The tooltip lists every
// series at the hovered position, gaps stay gaps and are never drawn as zero.
#[component]
pub fn CompareTrendChart(
    #[props(into)] title: String,
    labels: Vec<String>,
    series: Vec<CompareSeries>,
    format: ValueFormat,
) -> Element {
    let hovered = use_signal(|| None::<usize>);

    let present: Vec<f64> = series
        .iter()
        .flat_map(|line| line.values.iter().flatten().copied())
        .collect();

    if present.is_empty() {
        return rsx! {
            ChartEmpty { message: "No measurements recorded for this metric yet." }
        };
    }

    let low = present.iter().copied().fold(f64::INFINITY, f64::min);
    let high = present.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let domain = nice_domain(low, high, false, 5);
    let scale = y_scale(&domain);

    let count = labels.len().max(1);
    let x_percent = |index: usize| (index as f64 + 0.5) / count as f64 * 100.0;
    let active = hovered();

    let drawings: Vec<Drawing> = series
        .iter()
        .map(|line| {
            let coordinates: Vec<Option<(f64, f64)>> = line
                .values
                .iter()
                .enumerate()
                .map(|(index, value)| value.map(|value| (x_percent(index), scale.map(value))))
                .collect();
            let paths = geometry::segments(&coordinates)
                .iter()
                .filter(|run| run.len() >= 2)
                .map(|run| geometry::line_path(run))
                .collect();
            let swatch = line.tone.swatch_class();
            let dots = coordinates
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

            Drawing {
                stroke: line.tone.stroke_class(),
                paths,
                dots,
            }
        })
        .collect();

    let zone_labels: Vec<String> = labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let values: Vec<String> = series
                .iter()
                .map(|line| {
                    format!(
                        "{} {}",
                        line.name,
                        format_optional(line.values.get(index).copied().flatten(), format)
                    )
                })
                .collect();

            format!("{label}: {}", values.join(", "))
        })
        .collect();

    let guide_left = active.map(|index| format!("{:.2}%", x_percent(index)));
    let tooltip = active.and_then(|index| {
        labels.get(index).map(|label| {
            let lines = series
                .iter()
                .map(|line| TooltipLine {
                    label: line.name.clone(),
                    value: format_optional(line.values.get(index).copied().flatten(), format),
                    swatch: Some(line.tone),
                })
                .collect::<Vec<_>>();

            (x_percent(index), label.clone(), lines)
        })
    });

    let x_labels: Vec<(String, XAlign)> = labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            if labels.len() == 1 {
                (label.clone(), XAlign::Center)
            } else if index == 0 {
                (label.clone(), XAlign::Start)
            } else if index + 1 == labels.len() {
                (label.clone(), XAlign::End)
            } else {
                (String::new(), XAlign::Center)
            }
        })
        .collect();

    let legend: Vec<(String, ChartTone)> = series
        .iter()
        .map(|line| (legend_label(line), line.tone))
        .collect();

    rsx! {
        div { class: "w-full", "role": "group", "aria-label": "{title}",
            ChartLegend { entries: legend }
            PlotFrame { domain: domain.clone(), format, x_labels,
                svg {
                    class: "absolute inset-0 h-full w-full",
                    view_box: "0 0 100 100",
                    "preserveAspectRatio": "none",
                    "aria-hidden": "true",
                    for (series_index , drawing) in drawings.iter().enumerate() {
                        for (path_index , d) in drawing.paths.iter().enumerate() {
                            path {
                                key: "{series_index}-{path_index}",
                                d: "{d}",
                                stroke_linejoin: "round",
                                stroke_linecap: "round",
                                class: "vector-fixed fill-none stroke-2 {drawing.stroke}",
                            }
                        }
                    }
                }
                if let Some(left) = guide_left {
                    div {
                        class: "pointer-events-none absolute inset-y-0 w-px bg-(--clr-border-strong)",
                        style: "left: {left}",
                    }
                }
                for (series_index , drawing) in drawings.into_iter().enumerate() {
                    for (dot_index , dot) in drawing.dots.into_iter().enumerate() {
                        span {
                            key: "{series_index}-{dot_index}",
                            class: "{dot.class}",
                            style: "left: {dot.left}; top: {dot.top}",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_legend_adds_the_note_after_the_name() {
        let mut line = CompareSeries {
            name: "Baseline".to_string(),
            tone: ChartTone::Slate,
            values: Vec::new(),
            note: String::new(),
        };

        assert_eq!(legend_label(&line), "Baseline");

        line.note = "1,200 requests avg".to_string();

        assert_eq!(legend_label(&line), "Baseline · 1,200 requests avg");
    }

    #[test]
    fn pads_the_front_so_series_end_together() {
        assert_eq!(
            align_end(vec![Some(1.0), Some(2.0)], 4),
            vec![None, None, Some(1.0), Some(2.0)]
        );
        assert_eq!(align_end(vec![Some(1.0)], 1), vec![Some(1.0)]);
    }

    #[test]
    fn never_shortens_a_series() {
        assert_eq!(align_end(vec![Some(1.0), Some(2.0)], 1).len(), 2);
    }
}
