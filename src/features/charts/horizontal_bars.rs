use dioxus::prelude::*;

use super::palette::ChartTone;
use crate::format::{format_optional, ValueFormat};

// One labelled bar
#[derive(Debug, Clone, PartialEq)]
pub struct BarRow {
    pub label: String,
    pub value: Option<f64>,
}

// Width of each bar in percent of the longest one, a bar that exists is never thinner than 1.5%
fn widths(rows: &[BarRow]) -> Vec<f64> {
    let highest = rows
        .iter()
        .filter_map(|row| row.value)
        .filter(|value| value.is_finite())
        .fold(0.0_f64, f64::max);

    rows.iter()
        .map(|row| match row.value {
            Some(value) if highest > 0.0 && value.is_finite() => {
                (value / highest * 100.0).clamp(1.5, 100.0)
            }
            _ => 0.0,
        })
        .collect()
}

// Horizontal bars to scale. Hovering one bar dims the others.
#[component]
pub fn HorizontalBars(
    rows: Vec<BarRow>,
    format: ValueFormat,
    #[props(default)] tone: ChartTone,
) -> Element {
    let widths = widths(&rows);
    let color = tone.color_var();
    let lines: Vec<(String, String, f64)> = rows
        .iter()
        .zip(widths)
        .map(|(row, width)| (row.label.clone(), format_optional(row.value, format), width))
        .collect();

    rsx! {
        div { class: "group/bars flex flex-col gap-(--space-3)", role: "list",
            for (label , text , width) in lines {
                div {
                    key: "{label}",
                    class: "flex items-center gap-(--space-3) transition-opacity duration-150 group-hover/bars:opacity-40 hover:opacity-100!",
                    role: "listitem",
                    span { class: "w-12 shrink-0 text-(length:--fs-sm) text-(color:--clr-text-muted)",
                        "{label}"
                    }
                    div { class: "h-3 min-w-0 flex-1 overflow-hidden rounded-(--round-full) bg-(--clr-surface-hover)",
                        div {
                            class: "grow-x h-full rounded-(--round-full)",
                            style: "width: {width:.1}%; background-color: {color}",
                        }
                    }
                    span { class: "w-24 shrink-0 text-right text-(length:--fs-sm) font-(weight:--fw-medium) tabular-nums text-(color:--clr-text)",
                        "{text}"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(value: Option<f64>) -> BarRow {
        BarRow {
            label: "p".to_string(),
            value,
        }
    }

    #[test]
    fn scales_bars_to_the_longest() {
        assert_eq!(
            widths(&[row(Some(50.0)), row(Some(100.0)), row(None)]),
            vec![50.0, 100.0, 0.0]
        );
    }

    #[test]
    fn a_tiny_bar_stays_visible() {
        assert_eq!(widths(&[row(Some(0.1)), row(Some(100.0))])[0], 1.5);
    }

    #[test]
    fn no_values_gives_empty_bars() {
        assert_eq!(widths(&[row(None)]), vec![0.0]);
        assert_eq!(widths(&[row(Some(0.0))]), vec![0.0]);
    }
}
