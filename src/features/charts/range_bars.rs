use dioxus::prelude::*;

use super::palette::ChartTone;
use crate::format::{format_optional, ValueFormat};

// One run: a bar floating from its low value to its high value, with an optional tick between
#[derive(Debug, Clone, PartialEq)]
pub struct RangeRow {
    pub label: String,
    pub tone: ChartTone,
    pub low: Option<f64>,
    pub high: Option<f64>,
    pub mark: Option<f64>,
}

// Left edge and width in percent of the track, a range that exists is never thinner than 1.5%
fn range_geometry(low: f64, high: f64, scale_max: f64) -> Option<(f64, f64)> {
    if !(low.is_finite() && high.is_finite()) || scale_max <= 0.0 {
        return None;
    }

    let (low, high) = if low <= high {
        (low, high)
    } else {
        (high, low)
    };
    let left = (low / scale_max * 100.0).clamp(0.0, 100.0);
    let width = ((high - low) / scale_max * 100.0).max(1.5);

    Some((left.min(100.0 - width.min(100.0)), width.min(100.0)))
}

// A row with its numbers already placed on the track
struct RangeLine {
    row: RangeRow,
    geometry: Option<(f64, f64)>,
    mark: Option<f64>,
    text: String,
}

fn position(value: f64, scale_max: f64) -> f64 {
    (value / scale_max * 100.0).clamp(0.0, 100.0)
}

// Floating bars on one shared scale that starts at zero. Hovering a row dims the others.
#[component]
pub fn RangeBars(rows: Vec<RangeRow>, format: ValueFormat) -> Element {
    let scale_max = rows
        .iter()
        .flat_map(|row| [row.low, row.high, row.mark])
        .flatten()
        .filter(|value| value.is_finite())
        .fold(0.0_f64, f64::max);

    let lines: Vec<RangeLine> = rows
        .into_iter()
        .map(|row| {
            let geometry = row
                .low
                .zip(row.high)
                .and_then(|(low, high)| range_geometry(low, high, scale_max));
            let mark = row.mark.map(|value| position(value, scale_max));
            let text = format!(
                "{} → {}",
                format_optional(row.low, format),
                format_optional(row.high, format)
            );

            RangeLine {
                row,
                geometry,
                mark,
                text,
            }
        })
        .collect();

    rsx! {
        div { class: "group/range flex flex-col gap-(--space-3)", role: "list",
            for line in lines {
                {
                    let RangeLine { row, geometry, mark, text } = line;
                    let swatch = row.tone.swatch_class();
                    let color = row.tone.color_var();

                    rsx! {
                        div {
                            key: "{row.label}",
                            role: "listitem",
                            class: "flex flex-col gap-(--space-1) transition-opacity duration-150 group-hover/range:opacity-40 hover:opacity-100! sm:flex-row sm:items-center sm:gap-(--space-3)",
                            span { class: "flex w-32 shrink-0 items-center gap-(--space-2) text-(length:--fs-sm) text-(color:--clr-text-muted)",
                                span { class: "inline-block size-(--dot-size) rounded-(--round-full) {swatch}" }
                                "{row.label}"
                            }
                            div { class: "relative h-3 min-w-0 flex-1 rounded-(--round-full) bg-(--clr-surface-hover)",
                                if let Some((left , width)) = geometry {
                                    span {
                                        class: "grow-x absolute top-0 bottom-0 rounded-(--round-full)",
                                        style: "left: {left:.1}%; width: {width:.1}%; background-color: {color}",
                                    }
                                }
                                if let Some(at) = mark {
                                    span {
                                        class: "absolute -top-0.5 -bottom-0.5 w-0.5 rounded-(--round-full) bg-(--clr-ink)",
                                        style: "left: {at:.1}%",
                                        title: "Average",
                                    }
                                }
                            }
                            span { class: "w-44 shrink-0 text-right text-(length:--fs-sm) tabular-nums text-(color:--clr-text)",
                                "{text}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_the_range_on_the_scale() {
        assert_eq!(range_geometry(25.0, 75.0, 100.0), Some((25.0, 50.0)));
        assert_eq!(range_geometry(0.0, 100.0, 100.0), Some((0.0, 100.0)));
    }

    #[test]
    fn keeps_an_empty_range_visible_inside_the_track() {
        assert_eq!(range_geometry(100.0, 100.0, 100.0), Some((98.5, 1.5)));
        assert_eq!(range_geometry(50.0, 50.0, 100.0), Some((50.0, 1.5)));
    }

    #[test]
    fn swaps_a_reversed_range_and_rejects_bad_input() {
        assert_eq!(range_geometry(75.0, 25.0, 100.0), Some((25.0, 50.0)));
        assert_eq!(range_geometry(1.0, 2.0, 0.0), None);
        assert_eq!(range_geometry(f64::NAN, 2.0, 10.0), None);
    }

    #[test]
    fn clamps_a_mark_to_the_track() {
        assert_eq!(position(150.0, 100.0), 100.0);
        assert_eq!(position(20.0, 100.0), 20.0);
    }
}
