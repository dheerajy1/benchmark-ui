use dioxus::prelude::*;

use super::{fields::WasMark, form::WorkloadMode, target_run::TargetForm};
use crate::{
    components::{
        copy_text, use_toasts, Badge, Card, FormField, Icon, IconKind, Popover, PopoverAlign,
        RuntimeTag, SegmentSwitch, Stepper, Tone, INPUT_CLASS,
    },
    features::{history::runtime_label, targets::TargetView},
};

// Names of headers, `None` when there are no headers
fn names_text(names: &[String]) -> String {
    if names.is_empty() {
        "None".to_string()
    } else {
        names.join(", ")
    }
}

// Read-only facts about a target, as label and value. Header names only, never values.
pub fn target_facts(target: &TargetView) -> Vec<(&'static str, String)> {
    let mut lines = vec![("Path", target.path.clone())];

    if let Some(startup) = &target.startup {
        let command = format!(
            "{} {}",
            startup.command.program,
            startup.command.args.join(" ")
        );
        let readiness = &startup.readiness;

        lines.push(("Start command", command.trim().to_string()));
        lines.push((
            "Ready when",
            format!(
                "{}:{} {} ({} → {})",
                readiness.host,
                readiness.port,
                readiness.path,
                readiness.method,
                readiness.expected_status
            ),
        ));
        lines.push((
            "Readiness headers",
            match &readiness.header_names {
                Some(names) => names_text(names),
                None => "Server health credentials".to_string(),
            },
        ));
    }

    if let Some(workload) = &target.workload {
        lines.push(("Workload", format!("{} {}", workload.method, workload.path)));
        lines.push(("Workload headers", names_text(&workload.header_names)));
    }

    for workspace in &target.workspaces {
        let mut modes = Vec::new();

        if workspace.native.is_some() {
            modes.push("native");
        }
        if workspace.docker.is_some() {
            modes.push("Docker");
        }

        lines.push((
            "Workspace",
            format!(
                "{} in {} ({})",
                workspace.name,
                workspace.directory,
                modes.join(", ")
            ),
        ));
    }

    lines
}

// One option of the picker with everything it shows
struct PickerRow {
    id: String,
    name: String,
    path: String,
    runtime: Option<String>,
    selected: bool,
}

// Dropdown that shows each target with its name, runtime tag and path
#[component]
fn TargetPicker(
    targets: Vec<TargetView>,
    runtimes: Vec<(String, String)>,
    selected: String,
    disabled: bool,
    on_pick: EventHandler<String>,
) -> Element {
    let mut open = use_signal(|| false);

    let rows: Vec<PickerRow> = targets
        .iter()
        .map(|target| PickerRow {
            id: target.id.clone(),
            name: target.name.clone(),
            path: target.path.clone(),
            runtime: runtimes
                .iter()
                .find(|(path, _)| *path == target.path)
                .map(|(_, runtime)| runtime_label(runtime)),
            selected: target.id == selected,
        })
        .collect();
    let current = rows
        .iter()
        .find(|row| row.selected)
        .map(|row| (row.name.clone(), row.runtime.clone(), row.path.clone()));

    rsx! {
        div { class: "relative",
            button {
                r#type: "button",
                class: "flex w-full cursor-pointer items-center justify-between gap-(--space-3) rounded-(--round-md) border border-(color:--clr-border-strong) bg-(--clr-canvas) px-(--space-3) py-(--space-2) text-left transition-colors hover:border-(color:--clr-text-faint) focus-visible:shadow-(--halo-focus) focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50",
                disabled,
                "aria-haspopup": "listbox",
                "aria-expanded": "{open()}",
                onclick: move |_| open.set(!open()),
                div { class: "flex min-w-0 flex-col gap-px",
                    if let Some((name , runtime , path)) = current {
                        span { class: "flex items-center gap-(--space-2) text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text)",
                            "{name}"
                            if let Some(runtime) = runtime {
                                RuntimeTag { runtime }
                            }
                        }
                        span { class: "truncate font-(family-name:--font-code) text-(length:--fs-xs) text-(color:--clr-text-muted)",
                            "{path}"
                        }
                    } else {
                        span { class: "text-(length:--fs-sm) text-(color:--clr-text-faint)", "Choose a target" }
                    }
                }
                Icon {
                    kind: IconKind::ChevronRight,
                    class: "size-(--icon-sm) rotate-90 text-(color:--clr-text-muted)",
                }
            }
            if open() {
                Popover {
                    align: PopoverAlign::Left,
                    rise: true,
                    width: "w-full",
                    on_close: move |_| open.set(false),
                    ul { role: "listbox", class: "flex max-h-72 flex-col overflow-y-auto p-(--space-1)",
                        for row in rows {
                            li { key: "{row.id}", role: "option", "aria-selected": "{row.selected}",
                                button {
                                    r#type: "button",
                                    class: "flex w-full cursor-pointer items-center justify-between gap-(--space-3) rounded-(--round-md) px-(--space-3) py-(--space-2) text-left transition-colors hover:bg-(--clr-surface-hover)",
                                    onclick: {
                                        let id = row.id.clone();
                                        move |_| {
                                            on_pick.call(id.clone());
                                            open.set(false);
                                        }
                                    },
                                    div { class: "flex min-w-0 flex-col gap-px",
                                        span { class: "flex items-center gap-(--space-2) text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text)",
                                            "{row.name}"
                                            if let Some(runtime) = row.runtime.clone() {
                                                RuntimeTag { runtime }
                                            }
                                        }
                                        span { class: "truncate font-(family-name:--font-code) text-(length:--fs-xs) text-(color:--clr-text-muted)",
                                            "{row.path}"
                                        }
                                    }
                                    if row.selected {
                                        Icon { kind: IconKind::Check, class: "size-(--icon-sm) text-(color:--clr-accent)" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn mode_name(mode: WorkloadMode) -> &'static str {
    match mode {
        WorkloadMode::Duration => "Duration",
        WorkloadMode::Requests => "Requests",
    }
}

// The editable card with the workload of this run, marked where it differs from the target
#[component]
fn WorkloadCard(form: Signal<Option<TargetForm>>, defaults: TargetForm, running: bool) -> Element {
    let mut form = form;
    let Some(current) = form() else {
        return rsx! {};
    };

    let mode_changed = current.mode != defaults.mode;
    let amount_changed = current.amount.trim() != defaults.amount.trim();
    let concurrency_changed = current.concurrency.trim() != defaults.concurrency.trim();
    let amount_label = match current.mode {
        WorkloadMode::Requests => "Requests",
        WorkloadMode::Duration => "Duration (seconds)",
    };
    let amount_tint = if amount_changed {
        "bg-(--clr-accent-soft)!"
    } else {
        ""
    };
    let selected = usize::from(current.mode == WorkloadMode::Requests);
    let default_mode = defaults.mode;
    let default_amount = defaults.amount.clone();
    let default_concurrency = defaults.concurrency.clone();
    let reset_amount = default_amount.clone();
    let reset_concurrency = default_concurrency.clone();

    rsx! {
        Card { class: "fade-up flex flex-col gap-(--space-4) p-(--space-4)",
            div {
                h2 { class: "text-(length:--fs-base) font-(weight:--fw-semibold) text-(color:--clr-text)",
                    "Workload for this run"
                }
                p { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)",
                    "Starts from the target's values. Changes apply to this run only."
                }
            }
            div { class: "flex flex-col gap-(--space-1)",
                span { class: "text-(length:--fs-xs) font-(weight:--fw-medium) text-(color:--clr-text-muted)",
                    "Run until"
                }
                SegmentSwitch {
                    label: "Run until",
                    options: vec![("Duration", running), ("Requests", running)],
                    selected,
                    width_rem: 6.5,
                    on_pick: move |index: usize| {
                        let mode = if index == 0 {
                            WorkloadMode::Duration
                        } else {
                            WorkloadMode::Requests
                        };
                        form.with_mut(|filled| {
                            if let Some(filled) = filled {
                                filled.mode = mode;
                            }
                        });
                    },
                }
                if mode_changed {
                    WasMark {
                        was: mode_name(default_mode),
                        on_reset: move |_| {
                            form.with_mut(|filled| {
                                if let Some(filled) = filled {
                                    filled.mode = default_mode;
                                }
                            });
                        },
                    }
                }
            }
            div { class: "grid grid-cols-1 gap-(--space-4) sm:grid-cols-2",
                FormField {
                    label: amount_label,
                    hint: if current.mode == WorkloadMode::Duration { "Up to 300 seconds" } else { "" },
                    input {
                        r#type: "text",
                        inputmode: "numeric",
                        class: "{INPUT_CLASS} {amount_tint}",
                        value: "{current.amount}",
                        disabled: running,
                        oninput: move |event| {
                            form.with_mut(|filled| {
                                if let Some(filled) = filled {
                                    filled.amount = event.value();
                                }
                            });
                        },
                    }
                    if amount_changed {
                        WasMark {
                            was: default_amount.clone(),
                            on_reset: move |_| {
                                let value = reset_amount.clone();
                                form.with_mut(|filled| {
                                    if let Some(filled) = filled {
                                        filled.amount = value;
                                    }
                                });
                            },
                        }
                    }
                }
                div { class: "flex flex-col gap-(--space-1)",
                    span { class: "text-(length:--fs-xs) font-(weight:--fw-medium) text-(color:--clr-text-muted)",
                        "Concurrency"
                    }
                    Stepper {
                        label: "concurrency",
                        value: current.concurrency.clone(),
                        changed: concurrency_changed,
                        disabled: running,
                        on_change: move |value: String| {
                            form.with_mut(|filled| {
                                if let Some(filled) = filled {
                                    filled.concurrency = value;
                                }
                            });
                        },
                    }
                    if concurrency_changed {
                        WasMark {
                            was: default_concurrency.clone(),
                            on_reset: move |_| {
                                let value = reset_concurrency.clone();
                                form.with_mut(|filled| {
                                    if let Some(filled) = filled {
                                        filled.concurrency = value;
                                    }
                                });
                            },
                        }
                    }
                }
            }
        }
    }
}

// The saved target view: a picker, what the server sets, and the workload of this run
#[component]
pub fn SavedTargetForm(
    targets: Vec<TargetView>,
    runtimes: Vec<(String, String)>,
    form: Signal<Option<TargetForm>>,
    running: bool,
) -> Element {
    let mut form = form;
    let toasts = use_toasts();

    let current = form();
    let selected_id = current
        .as_ref()
        .map(|filled| filled.target_id.clone())
        .unwrap_or_default();
    let selected = targets
        .iter()
        .find(|target| target.id == selected_id)
        .cloned();
    let facts: Vec<(&'static str, String)> =
        selected.as_ref().map(target_facts).unwrap_or_default();
    let defaults = selected.as_ref().map(TargetForm::for_target);
    let lookup = targets.clone();

    rsx! {
        Card { class: "fade-up flex flex-col gap-(--space-3) p-(--space-4)",
            h2 { class: "text-(length:--fs-base) font-(weight:--fw-semibold) text-(color:--clr-text)",
                "Target"
            }
            TargetPicker {
                targets,
                runtimes,
                selected: selected_id,
                disabled: running,
                on_pick: move |id: String| {
                    if let Some(target) = lookup.iter().find(|target| target.id == id) {
                        form.set(Some(TargetForm::for_target(target)));
                    }
                },
            }
        }
        if selected.is_some() {
            Card { class: "fade-up flex flex-col gap-(--space-3) p-(--space-4)",
                div { class: "flex items-center justify-between gap-(--space-3)",
                    h2 { class: "text-(length:--fs-base) font-(weight:--fw-semibold) text-(color:--clr-text)",
                        "What this target does"
                    }
                    Badge { label: "Set on the server", tone: Tone::Neutral }
                }
                dl { class: "flex flex-col",
                    for (label , value) in facts {
                        div {
                            key: "{label}-{value}",
                            class: "group/row flex items-start justify-between gap-(--space-3) border-t border-(color:--clr-border) py-(--space-2) first:border-t-0",
                            div { class: "min-w-0",
                                dt { class: "text-(length:--fs-2xs) font-(weight:--fw-medium) uppercase tracking-wider text-(color:--clr-text-faint)",
                                    "{label}"
                                }
                                dd { class: "text-(length:--fs-sm) break-all text-(color:--clr-text)", "{value}" }
                            }
                            button {
                                r#type: "button",
                                class: "inline-flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-(--round-md) text-(color:--clr-text-muted) opacity-0 transition-opacity hover:bg-(--clr-surface-hover) hover:text-(color:--clr-text) focus-visible:opacity-100 group-hover/row:opacity-100",
                                "aria-label": "Copy {label}",
                                onclick: {
                                    let value = value.clone();
                                    move |_| {
                                        copy_text(&value);
                                        toasts.show("Copied");
                                    }
                                },
                                Icon { kind: IconKind::Copy, class: "size-(--icon-sm)" }
                            }
                        }
                    }
                }
                p { class: "text-(length:--fs-xs) text-(color:--clr-text-faint)",
                    "Targets are changed by editing the server's targets file, not here."
                }
            }
        }
        {
            match (current, defaults) {
            (Some(filled), Some(defaults)) if filled.has_workload => rsx! {
                WorkloadCard { form, defaults, running }
            },
            (Some(_), Some(_)) => rsx! {
                p {
                    class: "rounded-(--round-md) border border-(color:--clr-info) bg-(--clr-info-soft) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-info)",
                    role: "status",
                    "This target defines no workload, so the run only starts the target and checks its readiness."
                }
            },
            _ => rsx! {},
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn target(startup: bool) -> TargetView {
        let value = json!({
            "id": "api", "name": "API", "path": "/p",
            "startup": if startup { json!({
                "command": { "program": "bun", "args": ["run", "start"] },
                "readiness": { "host": "127.0.0.1", "port": 5001, "path": "/health",
                    "method": "GET", "expected_status": 200, "header_names": null }
            }) } else { json!(null) },
            "workload": { "method": "GET", "path": "/items", "mode": "requests",
                "requests": 100, "duration_seconds": null, "concurrency": 4,
                "header_names": ["X-Client-Id"] },
            "workspaces": []
        });

        serde_json::from_value(value).expect("parses")
    }

    #[test]
    fn lists_the_facts_in_the_decided_order() {
        let labels: Vec<&str> = target_facts(&target(true))
            .into_iter()
            .map(|(label, _)| label)
            .collect();

        assert_eq!(
            labels,
            [
                "Path",
                "Start command",
                "Ready when",
                "Readiness headers",
                "Workload",
                "Workload headers"
            ]
        );
    }

    #[test]
    fn shows_header_names_and_never_values() {
        let facts = target_facts(&target(true));

        assert!(facts
            .iter()
            .any(|(label, value)| *label == "Workload headers" && value == "X-Client-Id"));
        assert!(facts
            .iter()
            .any(|(label, value)| *label == "Readiness headers"
                && value == "Server health credentials"));
    }

    #[test]
    fn a_target_without_startup_has_no_start_facts() {
        let labels: Vec<&str> = target_facts(&target(false))
            .into_iter()
            .map(|(label, _)| label)
            .collect();

        assert_eq!(labels, ["Path", "Workload", "Workload headers"]);
    }

    #[test]
    fn names_none_when_a_list_is_empty() {
        assert_eq!(names_text(&[]), "None");
        assert_eq!(names_text(&["A".to_string(), "B".to_string()]), "A, B");
    }
}
