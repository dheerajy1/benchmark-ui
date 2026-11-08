use dioxus::prelude::*;

use super::form::FormState;
use crate::components::{Card, Collapse, FormField, Stepper, Switch, INPUT_CLASS, TEXTAREA_CLASS};

// Longest stretch of an old value that `was ...` shows
const WAS_LIMIT: usize = 28;

// One text input of the custom form. It names where the value lives, so one component can read
// and write any of them and compare it with the value of the run the form was filled from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Path,
    Program,
    Args,
    Host,
    Port,
    ReadinessPath,
    ExpectedStatus,
    ReadinessHeaders,
    WorkloadPath,
    WorkloadHeaders,
    Amount,
    Concurrency,
    WorkspaceName,
    WorkspaceDirectory,
    EnvFilePath,
    EnvPasted,
    ReadyLogLine,
    ReadyTimeout,
    LoadCommand,
    IdleSeconds,
    NativeBuild,
    NativeStart,
    DockerBuild,
    DockerBuildTimeout,
    DockerImage,
    DockerContainer,
    DockerRun,
}

impl Slot {
    pub fn get(self, state: &FormState) -> String {
        let workspace = &state.workspace;

        match self {
            Slot::Path => state.path.clone(),
            Slot::Program => state.program.clone(),
            Slot::Args => state.args.clone(),
            Slot::Host => state.host.clone(),
            Slot::Port => state.port.clone(),
            Slot::ReadinessPath => state.readiness_path.clone(),
            Slot::ExpectedStatus => state.expected_status.clone(),
            Slot::ReadinessHeaders => state.readiness_headers.clone(),
            Slot::WorkloadPath => state.workload_path.clone(),
            Slot::WorkloadHeaders => state.workload_headers.clone(),
            Slot::Amount => state.amount.clone(),
            Slot::Concurrency => state.concurrency.clone(),
            Slot::WorkspaceName => workspace.name.clone(),
            Slot::WorkspaceDirectory => workspace.directory.clone(),
            Slot::EnvFilePath => workspace.env_file_path.clone(),
            Slot::EnvPasted => workspace.env_pasted.clone(),
            Slot::ReadyLogLine => workspace.ready_log_line.clone(),
            Slot::ReadyTimeout => workspace.ready_timeout_seconds.clone(),
            Slot::LoadCommand => workspace.load_command.clone(),
            Slot::IdleSeconds => workspace.idle_seconds.clone(),
            Slot::NativeBuild => workspace.native_build_command.clone(),
            Slot::NativeStart => workspace.native_command.clone(),
            Slot::DockerBuild => workspace.docker_build_command.clone(),
            Slot::DockerBuildTimeout => workspace.docker_build_timeout_seconds.clone(),
            Slot::DockerImage => workspace.docker_image.clone(),
            Slot::DockerContainer => workspace.docker_container_name.clone(),
            Slot::DockerRun => workspace.docker_run_command.clone(),
        }
    }

    pub fn set(self, state: &mut FormState, value: String) {
        let workspace = &mut state.workspace;

        match self {
            Slot::Path => state.path = value,
            Slot::Program => state.program = value,
            Slot::Args => state.args = value,
            Slot::Host => state.host = value,
            Slot::Port => state.port = value,
            Slot::ReadinessPath => state.readiness_path = value,
            Slot::ExpectedStatus => state.expected_status = value,
            Slot::ReadinessHeaders => state.readiness_headers = value,
            Slot::WorkloadPath => state.workload_path = value,
            Slot::WorkloadHeaders => state.workload_headers = value,
            Slot::Amount => state.amount = value,
            Slot::Concurrency => state.concurrency = value,
            Slot::WorkspaceName => workspace.name = value,
            Slot::WorkspaceDirectory => workspace.directory = value,
            Slot::EnvFilePath => workspace.env_file_path = value,
            Slot::EnvPasted => workspace.env_pasted = value,
            Slot::ReadyLogLine => workspace.ready_log_line = value,
            Slot::ReadyTimeout => workspace.ready_timeout_seconds = value,
            Slot::LoadCommand => workspace.load_command = value,
            Slot::IdleSeconds => workspace.idle_seconds = value,
            Slot::NativeBuild => workspace.native_build_command = value,
            Slot::NativeStart => workspace.native_command = value,
            Slot::DockerBuild => workspace.docker_build_command = value,
            Slot::DockerBuildTimeout => workspace.docker_build_timeout_seconds = value,
            Slot::DockerImage => workspace.docker_image = value,
            Slot::DockerContainer => workspace.docker_container_name = value,
            Slot::DockerRun => workspace.docker_run_command = value,
        }
    }
}

// The old value of a field as `was` shows it: the first line, cut short, or `empty`
pub fn shorten(text: &str) -> String {
    let first = text.lines().next().unwrap_or("");
    let cut = first.chars().count() > WAS_LIMIT || text.lines().nth(1).is_some();
    let mut short: String = first.chars().take(WAS_LIMIT).collect();

    if short.is_empty() && !cut {
        return "empty".to_string();
    }

    if cut {
        short.push('…');
    }

    short
}

// The value of a field in the run the form was filled from, when it differs from the form now
fn changed_from(reference: Option<&FormState>, state: &FormState, slot: Slot) -> Option<String> {
    // The project path picks the run to compare with, so it is never compared itself
    if slot == Slot::Path {
        return None;
    }

    let old = slot.get(reference?);

    if old == slot.get(state) {
        None
    } else {
        Some(old)
    }
}

// `was 64 · reset` under a field that differs from the run it was filled from
#[component]
pub fn WasMark(#[props(into)] was: String, on_reset: EventHandler<()>) -> Element {
    rsx! {
        span { class: "flex items-center gap-(--space-1) text-(length:--fs-xs) text-(color:--clr-accent)",
            "was {was} ·"
            button {
                r#type: "button",
                class: "cursor-pointer underline-offset-2 hover:underline",
                onclick: move |_| on_reset.call(()),
                "reset"
            }
        }
    }
}

// Text input or text area bound to one slot of the form. Tinted and marked when it differs from
// `reference`.
#[component]
pub fn TextField(
    form: Signal<FormState>,
    reference: Signal<Option<FormState>>,
    slot: Slot,
    #[props(into)] label: String,
    #[props(into, default)] hint: String,
    #[props(into, default)] placeholder: String,
    #[props(into, default)] error: String,
    #[props(default)] numeric: bool,
    // More than zero turns the input into a text area with this many rows
    #[props(default)] rows: u32,
    #[props(default)] disabled: bool,
) -> Element {
    let mut form = form;
    let value = slot.get(&form.read());
    let old_state = reference.read().clone();
    let was = changed_from(old_state.as_ref(), &form.read(), slot);
    let tint = if was.is_some() {
        "bg-(--clr-accent-soft)!"
    } else {
        ""
    };
    let mode = if numeric { "numeric" } else { "text" };
    let area_class = format!("{TEXTAREA_CLASS} {tint}");
    let input_class = format!("{INPUT_CLASS} {tint}");
    let restore = was.clone().unwrap_or_default();
    let was_text = was.as_deref().map(shorten);

    rsx! {
        FormField { label, hint, error,
            if rows > 0 {
                textarea {
                    class: "{area_class}",
                    rows: "{rows}",
                    spellcheck: "false",
                    autocomplete: "off",
                    placeholder: "{placeholder}",
                    value: "{value}",
                    disabled,
                    oninput: move |event| form.with_mut(|state| slot.set(state, event.value())),
                }
            } else {
                input {
                    r#type: "text",
                    inputmode: "{mode}",
                    class: "{input_class}",
                    placeholder: "{placeholder}",
                    value: "{value}",
                    disabled,
                    oninput: move |event| form.with_mut(|state| slot.set(state, event.value())),
                }
            }
            if let Some(was) = was_text {
                WasMark {
                    was,
                    on_reset: move |_| {
                        let old = restore.clone();
                        form.with_mut(|state| slot.set(state, old));
                    },
                }
            }
        }
    }
}

// Number with minus and plus buttons bound to one slot of the form
#[component]
pub fn StepperField(
    form: Signal<FormState>,
    reference: Signal<Option<FormState>>,
    slot: Slot,
    #[props(into)] label: String,
    #[props(into, default)] hint: String,
    #[props(into, default)] error: String,
    #[props(default)] disabled: bool,
) -> Element {
    let mut form = form;
    let value = slot.get(&form.read());
    let old_state = reference.read().clone();
    let was = changed_from(old_state.as_ref(), &form.read(), slot);
    let restore = was.clone().unwrap_or_default();
    let was_text = was.as_deref().map(shorten);
    let aria = label.to_lowercase();

    rsx! {
        div { class: "flex min-w-0 flex-col gap-(--space-1)",
            span { class: "text-(length:--fs-xs) font-(weight:--fw-medium) text-(color:--clr-text-muted)",
                "{label}"
            }
            Stepper {
                label: aria,
                value,
                changed: was_text.is_some(),
                disabled,
                on_change: move |next: String| form.with_mut(|state| slot.set(state, next)),
            }
            if !error.is_empty() {
                span {
                    class: "text-(length:--fs-xs) text-(color:--clr-danger)",
                    role: "alert",
                    "{error}"
                }
            } else if !hint.is_empty() {
                span { class: "text-(length:--fs-xs) text-(color:--clr-text-faint)", "{hint}" }
            }
            if let Some(was) = was_text {
                WasMark {
                    was,
                    on_reset: move |_| {
                        let old = restore.clone();
                        form.with_mut(|state| slot.set(state, old));
                    },
                }
            }
        }
    }
}

// A card with a switch in its header. Off, it folds to the header line alone.
#[component]
pub fn SwitchBlock(
    #[props(into)] title: String,
    #[props(into)] subtitle: String,
    on: bool,
    on_toggle: EventHandler<()>,
    children: Element,
) -> Element {
    let switch_label = format!("Use {title}");

    rsx! {
        Card {
            div { class: "flex items-center justify-between gap-(--space-4) px-(--space-4) py-(--space-3)",
                div { class: "flex min-w-0 flex-col gap-(--space-1)",
                    h2 { class: "text-(length:--fs-base) font-(weight:--fw-semibold) text-(color:--clr-text)",
                        "{title}"
                    }
                    p { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)", "{subtitle}" }
                }
                Switch {
                    on,
                    label: switch_label,
                    on_toggle: move |_| on_toggle.call(()),
                }
            }
            Collapse { open: on,
                div { class: "flex flex-col gap-(--space-4) border-t border-(color:--clr-border) p-(--space-4)",
                    {children}
                }
            }
        }
    }
}

// A smaller block inside a switch block, for the native and the Docker part of a workspace
#[component]
pub fn MiniBlock(
    #[props(into)] title: String,
    #[props(into)] subtitle: String,
    on: bool,
    on_toggle: EventHandler<()>,
    children: Element,
) -> Element {
    let switch_label = format!("Use {title}");

    rsx! {
        div { class: "rounded-(--round-md) border border-(color:--clr-border) bg-(--clr-surface-raised)",
            div { class: "flex items-center justify-between gap-(--space-3) px-(--space-3) py-(--space-2)",
                div { class: "flex min-w-0 flex-col",
                    span { class: "text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text)",
                        "{title}"
                    }
                    span { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)", "{subtitle}" }
                }
                Switch {
                    on,
                    label: switch_label,
                    on_toggle: move |_| on_toggle.call(()),
                }
            }
            Collapse { open: on,
                div { class: "flex flex-col gap-(--space-4) border-t border-(color:--clr-border) p-(--space-3)",
                    {children}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Slot; 27] = [
        Slot::Path,
        Slot::Program,
        Slot::Args,
        Slot::Host,
        Slot::Port,
        Slot::ReadinessPath,
        Slot::ExpectedStatus,
        Slot::ReadinessHeaders,
        Slot::WorkloadPath,
        Slot::WorkloadHeaders,
        Slot::Amount,
        Slot::Concurrency,
        Slot::WorkspaceName,
        Slot::WorkspaceDirectory,
        Slot::EnvFilePath,
        Slot::EnvPasted,
        Slot::ReadyLogLine,
        Slot::ReadyTimeout,
        Slot::LoadCommand,
        Slot::IdleSeconds,
        Slot::NativeBuild,
        Slot::NativeStart,
        Slot::DockerBuild,
        Slot::DockerBuildTimeout,
        Slot::DockerImage,
        Slot::DockerContainer,
        Slot::DockerRun,
    ];

    #[test]
    fn every_slot_reads_back_what_it_wrote() {
        for slot in ALL {
            let mut state = FormState::default();
            let marker = format!("{slot:?}-value");

            slot.set(&mut state, marker.clone());

            assert_eq!(slot.get(&state), marker, "{slot:?}");
        }
    }

    #[test]
    fn a_slot_writes_nowhere_else() {
        for slot in ALL {
            let mut state = FormState::default();

            slot.set(&mut state, "changed".to_string());

            for other in ALL.iter().filter(|other| **other != slot) {
                assert_ne!(
                    other.get(&state),
                    "changed",
                    "{slot:?} wrote into {other:?}"
                );
            }
        }
    }

    #[test]
    fn no_reference_means_no_marker() {
        let state = FormState::default();

        assert_eq!(changed_from(None, &state, Slot::Port), None);
    }

    #[test]
    fn the_project_path_is_never_marked() {
        let mut old = FormState::default();
        let mut now = FormState::default();

        old.path = "/srv/a".to_string();
        now.path = "/srv/b".to_string();

        assert_eq!(changed_from(Some(&old), &now, Slot::Path), None);
    }

    #[test]
    fn a_changed_field_names_the_old_value() {
        let mut old = FormState::default();
        let mut now = FormState::default();

        old.concurrency = "64".to_string();
        now.concurrency = "128".to_string();

        assert_eq!(
            changed_from(Some(&old), &now, Slot::Concurrency),
            Some("64".to_string())
        );
        assert_eq!(changed_from(Some(&old), &now, Slot::Amount), None);
    }

    #[test]
    fn was_text_is_short_and_never_blank() {
        assert_eq!(shorten(""), "empty");
        assert_eq!(shorten("64"), "64");
        assert_eq!(shorten("a\nb"), "a…");
        assert_eq!(shorten(&"x".repeat(40)), format!("{}…", "x".repeat(28)));
    }
}
