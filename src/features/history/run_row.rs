use std::collections::HashMap;

use dioxus::prelude::*;

use super::{
    columns::{bar_percent, Column},
    models::{runtime_label, RunSummary},
    selection::Role,
};
use crate::{
    app::Route,
    components::{Badge, Icon, IconKind, RuntimeTag, Tone},
    format::{relative_time, short_id},
};

const CELL: &str = "px-(--space-3) py-(--space-3) align-middle";
const NUMBER_CELL: &str =
    "px-(--space-3) py-(--space-3) text-right align-middle whitespace-nowrap tabular-nums";

// Label and tone of how a run ended
fn status_badge(status: &str) -> (String, Tone) {
    match status {
        "done" => ("Done".to_string(), Tone::Success),
        "failed" => ("Failed".to_string(), Tone::Danger),
        "cancelled" => ("Cancelled".to_string(), Tone::Neutral),
        other => (other.to_string(), Tone::Neutral),
    }
}

// One table row of the Runs list. A click anywhere on the row selects it for Compare, the
// arrow at the end opens the run.
#[component]
pub fn RunRow(
    run: RunSummary,
    columns: Vec<Column>,
    best: HashMap<Column, f64>,
    highest_throughput: f64,
    role: Option<Role>,
    dimmed: bool,
    index: usize,
    on_toggle: EventHandler<String>,
) -> Element {
    let toggle_id = run.run_id.clone();
    let button_id = run.run_id.clone();
    let short = short_id(&run.run_id);
    let when = relative_time(&run.created_at, js_sys::Date::now() as i64);
    let runtime = run.runtime_code().map(runtime_label);
    let (status_text, status_tone) = status_badge(&run.status);
    let selected = role.is_some();

    let row_class = if selected {
        "bg-(--clr-accent-soft)"
    } else if dimmed {
        "opacity-45 hover:opacity-80 hover:bg-(--clr-hover-wash)"
    } else {
        "hover:bg-(--clr-hover-wash)"
    };
    let box_class = if selected {
        "border-(color:--clr-accent) bg-(--clr-accent) text-white"
    } else {
        "border-(color:--clr-border-strong) bg-(--clr-surface) text-transparent"
    };
    let delay = format!("animation-delay: {}ms", (index % 20) * 25);

    let role_class = match role {
        Some(Role::Baseline) => "bg-(--clr-accent)",
        _ => "bg-(--clr-chart-5)",
    };

    let cells: Vec<(Column, String, &'static str, f64)> = columns
        .iter()
        .map(|column| {
            let value = column.measured(&run);
            let is_best = value.is_some() && value == best.get(column).copied();
            let bar = if *column == Column::Throughput {
                bar_percent(value, highest_throughput)
            } else {
                0.0
            };

            let value_class = if is_best {
                "font-(weight:--fw-semibold) text-(color:--clr-success-text)"
            } else {
                "text-(color:--clr-text)"
            };

            (*column, column.text(&run), value_class, bar)
        })
        .collect();

    rsx! {
        tr {
            class: "group/row fade-up cursor-pointer transition-[background-color,opacity] duration-150 {row_class}",
            style: "{delay}",
            onclick: move |_| on_toggle.call(toggle_id.clone()),
            td { class: "w-10 {CELL} rounded-l-(--round-md)",
                button {
                    r#type: "button",
                    class: "flex size-5 cursor-pointer items-center justify-center rounded-(--round-sm) border transition-colors {box_class}",
                    role: "checkbox",
                    "aria-checked": "{selected}",
                    "aria-label": "Select run {short} for comparison",
                    onclick: move |event| {
                        event.stop_propagation();
                        on_toggle.call(button_id.clone());
                    },
                    if selected {
                        Icon { kind: IconKind::Check, class: "pop-in size-3.5" }
                    }
                }
            }
            td { class: "{CELL}",
                div { class: "flex items-center gap-(--space-2)",
                    span { class: "max-w-(--cell-max) truncate font-(weight:--fw-medium) text-(color:--clr-text)",
                        title: "{run.target.path}",
                        "{run.display_name}"
                    }
                    if let Some(runtime) = runtime {
                        RuntimeTag { runtime }
                    }
                    if let Some(role) = role {
                        span { class: "rounded-(--round-full) px-(--space-2) py-px text-(length:--fs-2xs) font-(weight:--fw-medium) text-white {role_class}",
                            "{role.label()}"
                        }
                    }
                }
                div { class: "flex items-center gap-(--space-1) text-(length:--fs-xs) whitespace-nowrap text-(color:--clr-text-faint)",
                    "{when} · "
                    span { class: "font-(family-name:--font-code)", "{short}" }
                    Icon {
                        kind: IconKind::ChevronRight,
                        class: "size-3 -translate-x-1 opacity-0 transition-all duration-200 group-hover/row:translate-x-0 group-hover/row:opacity-100",
                    }
                }
            }
            for (column , text , value_class , bar) in cells {
                if column == Column::Status {
                    td { key: "{column.label()}", class: "{CELL}",
                        Badge { label: status_text.clone(), tone: status_tone }
                    }
                } else {
                    td { key: "{column.label()}", class: "{NUMBER_CELL}",
                        span { class: "{value_class}", "{text}" }
                        if column == Column::Throughput {
                            div { class: "mt-1 ml-auto h-1 w-24 overflow-hidden rounded-(--round-full) bg-(--clr-surface-hover)",
                                div {
                                    class: "h-full rounded-(--round-full) bg-(--clr-border-strong) transition-colors group-hover/row:bg-(--clr-accent)",
                                    style: "width: {bar:.1}%",
                                }
                            }
                        }
                    }
                }
            }
            td { class: "w-10 {CELL} rounded-r-(--round-md) text-right",
                Link {
                    to: Route::RunDetail {
                        run_id: run.run_id.clone(),
                    },
                    class: "inline-flex size-8 items-center justify-center rounded-(--round-md) text-(color:--clr-text-muted) transition-colors hover:bg-(--clr-surface-hover) hover:text-(color:--clr-accent)",
                    onclick: move |event: MouseEvent| event.stop_propagation(),
                    "aria-label": "Open run {short}",
                    Icon { kind: IconKind::ChevronRight, class: "size-4" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_how_a_run_ended() {
        assert_eq!(status_badge("done"), ("Done".to_string(), Tone::Success));
        assert_eq!(status_badge("failed").1, Tone::Danger);
        assert_eq!(status_badge("cancelled").0, "Cancelled");
    }
}
