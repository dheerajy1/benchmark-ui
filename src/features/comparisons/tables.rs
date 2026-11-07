use dioxus::prelude::*;

use super::rows::MetricRow;
use crate::components::DeltaBadge;

const HEADER_CELL: &str = "px-(--space-3) py-(--space-2) text-(length:--fs-2xs) font-(weight:--fw-medium) uppercase tracking-wider whitespace-nowrap text-(color:--clr-text-faint)";

// Table line with every cell already formatted
struct TableLine {
    group: String,
    label: &'static str,
    baseline: String,
    comparison: String,
    difference: String,
    change: Option<f64>,
    lower_is_better: bool,
}

// Rows of one group or more, `show_group` adds the group name in front of each metric
#[component]
pub fn MetricTable(rows: Vec<MetricRow>, #[props(default)] show_group: bool) -> Element {
    let lines: Vec<TableLine> = rows
        .iter()
        .map(|row| TableLine {
            group: row.group.clone(),
            label: row.label,
            baseline: row.baseline_text(),
            comparison: row.comparison_text(),
            difference: row.difference_text(),
            change: row.diff.percentage_change,
            lower_is_better: row.diff.lower_is_better(),
        })
        .collect();

    rsx! {
        div { class: "-mx-(--space-4) overflow-x-auto",
            table { class: "w-full min-w-(--table-min-width) border-collapse text-left text-(length:--fs-sm)",
                thead { class: "bg-(--clr-surface-raised)",
                    tr {
                        if show_group {
                            th { class: "{HEADER_CELL}", "Group" }
                        }
                        th { class: "{HEADER_CELL}", "Metric" }
                        th { class: "{HEADER_CELL} text-right", "Baseline" }
                        th { class: "{HEADER_CELL} text-right", "Comparison" }
                        th { class: "{HEADER_CELL} text-right", "Difference" }
                        th { class: "{HEADER_CELL} text-right", "Change" }
                    }
                }
                tbody {
                    for line in lines {
                        tr {
                            key: "{line.group}-{line.label}",
                            class: "border-t border-(color:--clr-border) transition-colors hover:bg-(--clr-surface-hover)",
                            if show_group {
                                td { class: "px-(--space-3) py-(--space-2) text-(length:--fs-xs) text-(color:--clr-text-faint)",
                                    "{line.group}"
                                }
                            }
                            td { class: "px-(--space-3) py-(--space-2)", "{line.label}" }
                            td { class: "px-(--space-3) py-(--space-2) text-right whitespace-nowrap tabular-nums",
                                "{line.baseline}"
                            }
                            td { class: "px-(--space-3) py-(--space-2) text-right whitespace-nowrap tabular-nums",
                                "{line.comparison}"
                            }
                            td { class: "px-(--space-3) py-(--space-2) text-right whitespace-nowrap tabular-nums",
                                "{line.difference}"
                            }
                            td { class: "px-(--space-3) py-(--space-2) text-right whitespace-nowrap",
                                DeltaBadge {
                                    change: line.change,
                                    lower_is_better: line.lower_is_better,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
