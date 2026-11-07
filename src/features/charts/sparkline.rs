use dioxus::prelude::*;

use super::{frame::HoverZones, geometry, palette::ChartTone};
use crate::format::{format_optional, ValueFormat};

// Share of the height kept free above and below the line so the stroke is never cut off
const VERTICAL_PADDING: f64 = 10.0;

// More room when the Peak and Low labels are drawn next to the line
const LABELED_PADDING: f64 = 24.0;

// Positions of the highest and the lowest value, `None` when fewer than two different values
// exist. Gaps and non-finite values are skipped, the first of equal values wins.
fn extremes(values: &[Option<f64>]) -> Option<(usize, usize)> {
    let mut highest: Option<(usize, f64)> = None;
    let mut lowest: Option<(usize, f64)> = None;

    for (index, value) in values.iter().enumerate() {
        let Some(value) = value.filter(|value| value.is_finite()) else {
            continue;
        };

        if highest.is_none_or(|(_, best)| value > best) {
            highest = Some((index, value));
        }
        if lowest.is_none_or(|(_, best)| value < best) {
            lowest = Some((index, value));
        }
    }

    let (high_index, high) = highest?;
    let (low_index, low) = lowest?;

    (high - low > f64::EPSILON).then_some((high_index, low_index))
}

// Maps each value to a position in the 0 to 100 box. Every point owns an equal column, so the
// hover zones line up with the marks. Missing values stay gaps, a flat series sits in the middle.
fn spark_points(values: &[Option<f64>], padding: f64) -> Vec<Option<(f64, f64)>> {
    let present: Vec<f64> = values
        .iter()
        .flatten()
        .copied()
        .filter(|value| value.is_finite())
        .collect();

    let low = present.iter().copied().fold(f64::INFINITY, f64::min);
    let high = present.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = high - low;
    let count = values.len().max(1) as f64;

    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let value = (*value).filter(|value| value.is_finite())?;
            let x = (index as f64 + 0.5) / count * 100.0;
            let y = if span.abs() < f64::EPSILON {
                50.0
            } else {
                let ratio = (value - low) / span;
                100.0 - padding - ratio * (100.0 - 2.0 * padding)
            };

            Some((x, y))
        })
        .collect()
}

// Small trend graph for a tile. Grey at rest, it takes its tone and thickens while the parent
// element (which must carry the `group` class) is hovered. The line draws in on load, a dot
// follows the hovered column with a label above it, and the end dot pulses on hover.
#[component]
pub fn Sparkline(
    values: Vec<Option<f64>>,
    // One label per value, for example `3 runs ago`
    labels: Vec<String>,
    format: ValueFormat,
    #[props(default)] tone: ChartTone,
    // Marks the highest and the lowest point with a small `Peak` and `Low` label
    #[props(default)] mark_extremes: bool,
    #[props(into, default = "h-14".to_string())] class: String,
) -> Element {
    let hovered = use_signal(|| None::<usize>);
    let padding = if mark_extremes {
        LABELED_PADDING
    } else {
        VERTICAL_PADDING
    };
    let points = spark_points(&values, padding);
    let runs = geometry::segments(&points);

    if runs.is_empty() {
        return rsx! {
            div { class: "w-full {class}", "aria-hidden": "true" }
        };
    }

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

    let color_style = format!(
        "--spark-color: {}; --spark-soft: {}",
        tone.color_var(),
        tone.soft_var()
    );

    let end_dot = points
        .iter()
        .rev()
        .find_map(|point| *point)
        .map(|(x, y)| format!("left: {x:.2}%; top: {y:.2}%"));

    let active = hovered();
    let hover_dot = active.and_then(|index| {
        let (x, y) = points.get(index).copied().flatten()?;
        let value = format_optional(values.get(index).copied().flatten(), format);
        let label = labels.get(index).cloned().unwrap_or_default();
        let text = if label.is_empty() {
            value
        } else {
            format!("{label} · {value}")
        };
        // Near the edges the label grows inward so it stays inside the tile
        let shift = if x < 25.0 {
            "translate-x-0"
        } else if x > 75.0 {
            "-translate-x-full"
        } else {
            "-translate-x-1/2"
        };

        Some((format!("left: {x:.2}%; top: {y:.2}%"), text, shift))
    });

    // (position, text, vertical placement) of the Peak and Low labels
    let extreme_labels: Vec<(String, String, &'static str, &'static str)> = if mark_extremes {
        extremes(&values)
            .map(|(high, low)| {
                [
                    (high, "Peak", "-translate-y-full -mt-1.5"),
                    (low, "Low", "mt-1.5"),
                ]
                .into_iter()
                .filter_map(|(index, word, vertical)| {
                    let (x, y) = points.get(index).copied().flatten()?;
                    let value = format_optional(values.get(index).copied().flatten(), format);
                    let shift = if x < 25.0 {
                        "translate-x-0"
                    } else if x > 75.0 {
                        "-translate-x-full"
                    } else {
                        "-translate-x-1/2"
                    };

                    Some((
                        format!("left: {x:.2}%; top: {y:.2}%"),
                        format!("{word} {value}"),
                        vertical,
                        shift,
                    ))
                })
                .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let zone_labels: Vec<String> = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let label = labels.get(index).cloned().unwrap_or_default();
            format!("{label}: {}", format_optional(*value, format))
        })
        .collect();

    rsx! {
        div {
            class: "relative w-full {class}",
            style: "{color_style}",
            svg {
                class: "draw-in absolute inset-0 h-full w-full",
                view_box: "0 0 100 100",
                "preserveAspectRatio": "none",
                "aria-hidden": "true",
                for d in area_paths {
                    path { d: "{d}", class: "spark-area stroke-none" }
                }
                for d in line_paths {
                    path { d: "{d}", class: "vector-fixed spark-line" }
                }
            }
            for (index , (style , text , vertical , shift)) in extreme_labels.into_iter().enumerate() {
                span {
                    key: "{index}",
                    class: "pointer-events-none absolute text-(length:--fs-2xs) whitespace-nowrap tabular-nums text-(color:--clr-text-faint) {vertical} {shift}",
                    style: "{style}",
                    "{text}"
                }
            }
            if let Some(style) = end_dot {
                span {
                    class: "spark-end pointer-events-none absolute size-2 -translate-x-1/2 -translate-y-1/2 rounded-(--round-full)",
                    style: "{style}",
                }
            }
            if let Some((style , text , shift)) = hover_dot {
                span {
                    class: "pointer-events-none absolute size-2.5 -translate-x-1/2 -translate-y-1/2 rounded-(--round-full) border-2 border-(color:--clr-surface) bg-(--spark-color)",
                    style: "{style}",
                }
                span {
                    class: "pointer-events-none absolute z-10 -mt-3 rounded-(--round-sm) bg-(--clr-ink) px-(--space-2) py-px text-(length:--fs-2xs) whitespace-nowrap tabular-nums text-(color:--clr-ink-contrast) -translate-y-full {shift}",
                    style: "{style}",
                    "{text}"
                }
            }
            HoverZones { labels: zone_labels, active: hovered }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highest_value_is_nearest_the_top() {
        let points = spark_points(&[Some(1.0), Some(5.0), Some(3.0)], VERTICAL_PADDING);
        let ys: Vec<f64> = points.iter().flatten().map(|point| point.1).collect();

        assert!(ys[1] < ys[2] && ys[2] < ys[0]);
        assert!(ys[1] >= VERTICAL_PADDING);
        assert!(ys[0] <= 100.0 - VERTICAL_PADDING);
    }

    #[test]
    fn flat_series_sits_in_the_middle() {
        let points = spark_points(&[Some(4.0), Some(4.0)], VERTICAL_PADDING);

        assert!(points.iter().flatten().all(|point| point.1 == 50.0));
    }

    #[test]
    fn gaps_and_non_finite_values_stay_missing() {
        let points = spark_points(
            &[Some(1.0), None, Some(f64::NAN), Some(2.0)],
            VERTICAL_PADDING,
        );

        assert!(points[1].is_none());
        assert!(points[2].is_none());
        assert!(points[0].is_some() && points[3].is_some());
    }

    #[test]
    fn empty_input_gives_no_points() {
        assert!(spark_points(&[], VERTICAL_PADDING).is_empty());
    }

    #[test]
    fn finds_the_peak_and_the_low() {
        assert_eq!(
            extremes(&[Some(3.0), Some(9.0), None, Some(1.0), Some(5.0)]),
            Some((1, 3))
        );
    }

    #[test]
    fn a_flat_or_tiny_series_has_no_extremes() {
        assert_eq!(extremes(&[Some(2.0), Some(2.0)]), None);
        assert_eq!(extremes(&[Some(2.0)]), None);
        assert_eq!(extremes(&[None, Some(f64::NAN)]), None);
    }
}
