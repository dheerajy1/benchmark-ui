use dioxus::prelude::*;

use super::palette::ChartTone;
use crate::format::{format_optional, ValueFormat};

// One part of a stacked bar
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub name: String,
    pub tone: ChartTone,
    pub value: Option<f64>,
}

// One stacked bar, for example one run
#[derive(Debug, Clone, PartialEq)]
pub struct StackRow {
    pub label: String,
    pub segments: Vec<Segment>,
}

impl StackRow {
    fn total(&self) -> f64 {
        self.segments
            .iter()
            .filter_map(|segment| segment.value)
            .filter(|value| value.is_finite() && *value > 0.0)
            .sum()
    }
}

// Width of every segment in percent of the longest bar. A segment that exists is never thinner
// than 1%, so the total can pass the track by a hair, the track clips it.
fn segment_widths(rows: &[StackRow]) -> Vec<Vec<f64>> {
    let longest = rows.iter().map(StackRow::total).fold(0.0_f64, f64::max);

    rows.iter()
        .map(|row| {
            row.segments
                .iter()
                .map(|segment| match segment.value {
                    Some(value) if value.is_finite() && value > 0.0 && longest > 0.0 => {
                        (value / longest * 100.0).max(1.0)
                    }
                    _ => 0.0,
                })
                .collect()
        })
        .collect()
}

// A stacked bar with its parts already sized and its total formatted
struct StackLine {
    label: String,
    parts: Vec<StackPart>,
    total: String,
}

struct StackPart {
    name: String,
    style: String,
    title: String,
}

// Stacked bars on one shared scale, a legend names the parts. Hovering a row dims the others.
#[component]
pub fn StackedBars(rows: Vec<StackRow>, format: ValueFormat) -> Element {
    let widths = segment_widths(&rows);
    let legend: Vec<(String, &'static str)> = rows
        .first()
        .map(|row| {
            row.segments
                .iter()
                .map(|segment| (segment.name.clone(), segment.tone.swatch_class()))
                .collect()
        })
        .unwrap_or_default();
    let lines: Vec<StackLine> = rows
        .into_iter()
        .zip(widths)
        .map(|(row, widths)| {
            let total = format_optional(Some(row.total()).filter(|total| *total > 0.0), format);
            let parts = row
                .segments
                .iter()
                .zip(widths)
                .filter(|(_, width)| *width > 0.0)
                .map(|(segment, width)| StackPart {
                    name: segment.name.clone(),
                    style: format!(
                        "width: {width:.1}%; background-color: {}",
                        segment.tone.color_var()
                    ),
                    title: format!(
                        "{}: {}",
                        segment.name,
                        format_optional(segment.value, format)
                    ),
                })
                .collect();

            StackLine {
                label: row.label,
                parts,
                total,
            }
        })
        .collect();

    rsx! {
        div { class: "flex flex-col gap-(--space-3)",
            div { class: "group/stack flex flex-col gap-(--space-3)", role: "list",
                for line in lines {
                    div {
                        key: "{line.label}",
                        role: "listitem",
                        class: "flex flex-col gap-(--space-1) transition-opacity duration-150 group-hover/stack:opacity-40 hover:opacity-100! sm:flex-row sm:items-center sm:gap-(--space-3)",
                        span { class: "w-32 shrink-0 text-(length:--fs-sm) text-(color:--clr-text-muted)",
                            "{line.label}"
                        }
                        div { class: "flex h-4 min-w-0 flex-1 overflow-hidden rounded-(--round-full) bg-(--clr-surface-hover)",
                            for part in line.parts {
                                span {
                                    key: "{part.name}",
                                    class: "grow-x block h-full shrink-0",
                                    style: "{part.style}",
                                    title: "{part.title}",
                                }
                            }
                        }
                        span { class: "w-28 shrink-0 text-right text-(length:--fs-sm) font-(weight:--fw-medium) tabular-nums text-(color:--clr-text)",
                            "{line.total}"
                        }
                    }
                }
            }
            div { class: "flex flex-wrap items-center gap-(--space-4) text-(length:--fs-xs) text-(color:--clr-text-muted)",
                for (name , swatch) in legend {
                    span { key: "{name}", class: "inline-flex items-center gap-(--space-2)",
                        span { class: "inline-block size-(--dot-size) rounded-(--round-full) {swatch}" }
                        "{name}"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(source: Option<f64>, build: Option<f64>) -> StackRow {
        StackRow {
            label: "r".to_string(),
            segments: vec![
                Segment {
                    name: "Source".to_string(),
                    tone: ChartTone::Violet,
                    value: source,
                },
                Segment {
                    name: "Build".to_string(),
                    tone: ChartTone::Cyan,
                    value: build,
                },
            ],
        }
    }

    #[test]
    fn scales_every_segment_to_the_longest_bar() {
        let rows = [row(Some(50.0), Some(50.0)), row(Some(25.0), Some(25.0))];

        assert_eq!(
            segment_widths(&rows),
            vec![vec![50.0, 50.0], vec![25.0, 25.0]]
        );
    }

    #[test]
    fn a_missing_part_takes_no_width() {
        let rows = [row(Some(100.0), None)];

        assert_eq!(segment_widths(&rows), vec![vec![100.0, 0.0]]);
    }

    #[test]
    fn totals_only_present_positive_parts() {
        assert_eq!(row(Some(3.0), None).total(), 3.0);
        assert_eq!(row(None, None).total(), 0.0);
    }
}
