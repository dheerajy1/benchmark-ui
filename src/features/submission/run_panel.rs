use dioxus::prelude::*;

use super::{checks::Check, estimate::TimelineRow};
use crate::components::{copy_text, use_toasts, Card, Icon, IconKind};

// Sticky right panel: what will run, the checks that must pass, and the Run button. The button
// belongs to the form with id `form_id`, so it works from outside the form.
#[component]
pub fn RunPanel(
    lines: Vec<(String, String)>,
    checks: Vec<Check>,
    form_id: &'static str,
    ready: bool,
    running: bool,
    #[props(default)] message: Option<String>,
    // The request as JSON, offered for copying when the form is valid
    #[props(default)] copy_json: Option<String>,
    // The steps of the run, those that are off greyed out, and the estimated time
    #[props(default)] timeline: Vec<TimelineRow>,
    #[props(into, default)] estimate: String,
) -> Element {
    let toasts = use_toasts();
    let button_class = crate::components::button_class(crate::components::ButtonVariant::Primary);
    let rows: Vec<(&'static str, &'static str, String)> = checks
        .into_iter()
        .map(|check| {
            let (icon, class) = if check.ok {
                ("ok", "text-(color:--clr-success-text)")
            } else {
                ("open", "text-(color:--clr-text-muted)")
            };

            (icon, class, check.label)
        })
        .collect();

    rsx! {
        Card { class: "fade-up flex flex-col gap-(--space-4) p-(--space-4) shadow-(--shadow-card)",
            div { class: "flex flex-col gap-(--space-2)",
                h2 { class: "text-(length:--fs-base) font-(weight:--fw-semibold) text-(color:--clr-text)",
                    "What will run"
                }
                if lines.is_empty() {
                    p { class: "text-(length:--fs-sm) text-(color:--clr-text-faint)", "Nothing chosen yet." }
                }
                dl { class: "flex flex-col gap-(--space-2)",
                    for (label , value) in lines {
                        div { key: "{label}", class: "flex flex-col",
                            dt { class: "text-(length:--fs-2xs) font-(weight:--fw-medium) uppercase tracking-wider text-(color:--clr-text-faint)",
                                "{label}"
                            }
                            dd { class: "text-(length:--fs-sm) break-all text-(color:--clr-text)", "{value}" }
                        }
                    }
                }
            }
            if !timeline.is_empty() {
                div { class: "flex flex-col gap-(--space-2) border-t border-(color:--clr-border) pt-(--space-3)",
                    div { class: "flex flex-col",
                        span { class: "text-(length:--fs-2xs) font-(weight:--fw-medium) uppercase tracking-wider text-(color:--clr-text-faint)",
                            "Estimated time"
                        }
                        span { class: "text-(length:--fs-sm) text-(color:--clr-text)", "{estimate}" }
                    }
                    ul { class: "flex flex-col gap-(--space-2)",
                        for row in timeline {
                            li {
                                key: "{row.label}",
                                class: "flex flex-col gap-1 transition-opacity duration-300 {row.class}",
                                div { class: "flex items-baseline justify-between gap-(--space-2) text-(length:--fs-xs)",
                                    span { class: "text-(color:--clr-text)", "{row.label}" }
                                    span { class: "tabular-nums text-(color:--clr-text-muted)", "{row.time}" }
                                }
                                div { class: "h-1 overflow-hidden rounded-(--round-full) bg-(--clr-border)",
                                    div {
                                        class: "h-full rounded-(--round-full) bg-(--clr-accent) transition-[width] duration-500 ease-(--ease-out)",
                                        style: "width: {row.share}%",
                                    }
                                }
                            }
                        }
                    }
                }
            }
            ul { class: "flex flex-col gap-(--space-2) border-t border-(color:--clr-border) pt-(--space-3)",
                for (icon , class , label) in rows {
                    li {
                        key: "{label}",
                        class: "flex items-start gap-(--space-2) text-(length:--fs-sm) {class}",
                        if icon == "ok" {
                            Icon { kind: IconKind::Check, class: "mt-px size-(--icon-sm) pop-in" }
                        } else {
                            span { class: "mx-1.5 mt-1.5 inline-block size-1.5 shrink-0 rounded-(--round-full) bg-(--clr-border-strong)" }
                        }
                        span { "{label}" }
                    }
                }
            }
            button {
                r#type: "submit",
                form: "{form_id}",
                class: "{button_class}",
                disabled: running || !ready,
                Icon { kind: IconKind::Play, class: "size-(--icon-sm)" }
                if running {
                    "Running benchmark…"
                } else {
                    "Run benchmark"
                }
            }
            if let Some(json) = copy_json {
                button {
                    r#type: "button",
                    class: "inline-flex cursor-pointer items-center justify-center gap-(--space-2) text-(length:--fs-sm) text-(color:--clr-text-muted) transition-colors hover:text-(color:--clr-accent)",
                    onclick: move |_| {
                        copy_text(&json);
                        toasts.show("Copied the request as JSON");
                    },
                    Icon { kind: IconKind::Copy, class: "size-(--icon-sm)" }
                    "Copy as JSON"
                }
            }
            if running {
                div { class: "flex items-start gap-(--space-3)", role: "status",
                    span { class: "mt-(--space-1) inline-block size-4 shrink-0 animate-spin rounded-(--round-full) border-2 border-(color:--clr-border-strong) border-t-(color:--clr-accent)" }
                    p { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)",
                        "Starting the run. You will be taken to its live view in a moment."
                    }
                }
            }
            if let Some(message) = message {
                p {
                    class: "rounded-(--round-md) border border-(color:--clr-danger) bg-(--clr-danger-soft) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-danger)",
                    role: "alert",
                    "{message}"
                }
            }
        }
    }
}
