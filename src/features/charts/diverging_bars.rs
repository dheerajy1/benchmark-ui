use dioxus::prelude::*;

use crate::components::DeltaBadge;

// Bars longer than this many percent are cut at the edge and marked
const CAP_PERCENT: f64 = 150.0;

// One metric: `improvement` is positive when the comparison is better, whatever the direction
// of the metric, `change` is the plain change in percent shown in the pill
#[derive(Debug, Clone, PartialEq)]
pub struct DivergeRow {
    pub label: String,
    pub improvement: f64,
    pub change: f64,
    pub lower_is_better: bool,
}

// Width of the bar in percent of the whole track (half of it is one side) and whether it was cut
fn bar_geometry(improvement: f64) -> (f64, bool) {
    let size = improvement.abs();
    let width = size.min(CAP_PERCENT) / CAP_PERCENT * 50.0;
    let width = if size > 0.0 { width.max(0.8) } else { 0.0 };

    (width, size > CAP_PERCENT)
}

// Diverging bars around a center line: better to the right in green, worse to the left in red.
// Hovering one row dims the others.
#[component]
pub fn DivergingBars(rows: Vec<DivergeRow>) -> Element {
    rsx! {
        div { class: "group/diverge flex flex-col gap-(--space-2)", role: "list",
            for row in rows {
                {
                    let (width, cut) = bar_geometry(row.improvement);
                    let better = row.improvement >= 0.0;
                    let side = if better { "left: 50%;" } else { "right: 50%;" };
                    let color = if better { "var(--clr-success)" } else { "var(--clr-danger)" };
                    let origin = if better { "left center" } else { "right center" };
                    let mark = if better { "»" } else { "«" };
                    let mark_side = if better { "right-0" } else { "left-0" };
                    let style = format!(
                        "{side} width: {width:.1}%; background-color: {color}; transform-origin: {origin}"
                    );

                    rsx! {
                        div {
                            key: "{row.label}",
                            role: "listitem",
                            class: "flex items-center gap-(--space-3) transition-opacity duration-150 group-hover/diverge:opacity-40 hover:opacity-100!",
                            span { class: "w-32 shrink-0 truncate text-(length:--fs-sm) text-(color:--clr-text-muted)",
                                title: "{row.label}",
                                "{row.label}"
                            }
                            div { class: "relative h-4 min-w-0 flex-1",
                                span { class: "absolute top-0 bottom-0 left-1/2 w-px bg-(--clr-border-strong)" }
                                span {
                                    class: "grow-x absolute top-0.5 bottom-0.5 rounded-(--round-sm)",
                                    style: "{style}",
                                }
                                if cut {
                                    span { class: "absolute top-1/2 -translate-y-1/2 text-(length:--fs-xs) font-(weight:--fw-semibold) text-(color:--clr-text-muted) {mark_side}",
                                        "aria-label": "cut at 150%",
                                        "{mark}"
                                    }
                                }
                            }
                            div { class: "w-28 shrink-0 text-right",
                                DeltaBadge {
                                    change: Some(row.change),
                                    lower_is_better: row.lower_is_better,
                                }
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
    fn scales_to_half_the_track_at_the_cap() {
        assert_eq!(bar_geometry(75.0), (25.0, false));
        assert_eq!(bar_geometry(-150.0), (50.0, false));
    }

    #[test]
    fn cuts_and_marks_bars_past_the_cap() {
        assert_eq!(bar_geometry(400.0), (50.0, true));
        assert_eq!(bar_geometry(-151.0), (50.0, true));
    }

    #[test]
    fn keeps_a_tiny_change_visible_but_not_a_zero_one() {
        assert_eq!(bar_geometry(0.1).0, 0.8);
        assert_eq!(bar_geometry(0.0), (0.0, false));
    }
}
