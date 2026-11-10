use dioxus::prelude::*;

use super::{
    build_suggestions::{BuildChips, BuildSuggestion},
    fields::{MiniBlock, Slot, StepperField, SwitchBlock, TextField, WasMark},
    form::{error_for, Field, FieldError, FormState, WorkloadMode},
    workspace::{EnvSource, ENV_FILE_PLACEHOLDER},
};
use crate::components::{Card, CardHeader, SegmentSwitch};

const GRID: &str = "grid grid-cols-1 gap-(--space-4) sm:grid-cols-2";
const GROUP_TITLE: &str =
    "text-(length:--fs-xs) font-(weight:--fw-semibold) uppercase tracking-wide text-(color:--clr-text-faint)";

fn mode_index(mode: WorkloadMode) -> usize {
    match mode {
        WorkloadMode::Duration => 0,
        WorkloadMode::Requests => 1,
    }
}

fn mode_from_index(index: usize) -> WorkloadMode {
    if index == 0 {
        WorkloadMode::Duration
    } else {
        WorkloadMode::Requests
    }
}

fn mode_name(mode: WorkloadMode) -> &'static str {
    match mode {
        WorkloadMode::Duration => "Duration",
        WorkloadMode::Requests => "Requests",
    }
}

fn env_index(source: EnvSource) -> usize {
    match source {
        EnvSource::None => 0,
        EnvSource::File => 1,
        EnvSource::Pasted => 2,
    }
}

fn env_from_index(index: usize) -> EnvSource {
    match index {
        1 => EnvSource::File,
        2 => EnvSource::Pasted,
        _ => EnvSource::None,
    }
}

fn env_name(source: EnvSource) -> &'static str {
    match source {
        EnvSource::None => "None",
        EnvSource::File => "Env file",
        EnvSource::Pasted => "Paste",
    }
}

// The form of a custom run. Startup, Workload and Workspace are switches that fold away when
// they are off. A field that differs from `reference` is tinted and shows `was ...`.
#[component]
pub fn CustomForm(
    form: Signal<FormState>,
    reference: Signal<Option<FormState>>,
    errors: Vec<FieldError>,
    // Builds of past successful runs of the typed path, and whether the history is still loading
    suggestions: Vec<BuildSuggestion>,
    suggestions_loading: bool,
    running: bool,
) -> Element {
    let mut form = form;
    let state = form.read().clone();
    let old = reference.read().clone();

    let old_mode = old
        .as_ref()
        .map(|old| old.mode)
        .filter(|mode| *mode != state.mode);
    let old_env = old
        .as_ref()
        .map(|old| old.workspace.env_source)
        .filter(|source| *source != state.workspace.env_source);

    let requests = state.mode == WorkloadMode::Requests;
    let amount_label = if requests {
        "Requests"
    } else {
        "Duration (seconds)"
    };
    let amount_hint = if requests { "" } else { "Up to 300 seconds" };
    let workspace_error = error_for(&errors, Field::Workspace);

    rsx! {
        Card {
            CardHeader {
                title: "Target",
                subtitle: "The project directory on the machine running benchmark-api.",
            }
            div { class: "p-(--space-4)",
                TextField {
                    form,
                    reference,
                    slot: Slot::Path,
                    label: "Project path",
                    hint: "Absolute path, for example /home/me/projects/my-api",
                    error: error_for(&errors, Field::Path),
                    disabled: running,
                }
            }
        }

        Card {
            CardHeader {
                title: "Build",
                subtitle: "The command that builds the project. The artifact size is measured from its output once it finishes.",
            }
            div { class: "flex flex-col gap-(--space-3) p-(--space-4)",
                TextField {
                    form,
                    reference,
                    slot: Slot::BuildCommand,
                    label: "Build command",
                    hint: "Required. Run in the project path, for example bun run build or cargo xtask build --release",
                    placeholder: "bun run build",
                    error: error_for(&errors, Field::BuildCommand),
                    disabled: running,
                }
                BuildChips {
                    form,
                    suggestions,
                    loading: suggestions_loading,
                    disabled: running,
                }
                TextField {
                    form,
                    reference,
                    slot: Slot::BuildOutputDir,
                    label: "Build output folder",
                    hint: "Optional, inside the project. Left empty, dist is measured, or target for a Rust project.",
                    placeholder: "dist, or target/release",
                    error: error_for(&errors, Field::BuildOutputDir),
                    disabled: running,
                }
                div { class: "{GRID}",
                    TextField {
                        form,
                        reference,
                        slot: Slot::CleanupCommand,
                        label: "Cleanup command",
                        hint: "Optional. Runs in the project path at the very end of the run, for example docker system prune -af. If it fails, the run is still done.",
                        placeholder: "docker system prune -af",
                        disabled: running,
                    }
                    TextField {
                        form,
                        reference,
                        slot: Slot::CleanupTimeout,
                        label: "Cleanup timeout (seconds)",
                        hint: "Optional, 1 to 3600. Left empty, 120 seconds.",
                        placeholder: "120",
                        numeric: true,
                        error: error_for(&errors, Field::CleanupTimeout),
                        disabled: running,
                    }
                }
            }
        }

        SwitchBlock {
            title: "Startup",
            subtitle: "How to start the target and when it counts as ready. Optional.",
            on: state.startup_enabled,
            on_toggle: move |_| form.with_mut(|f| f.startup_enabled = !f.startup_enabled),
            div { class: "{GRID}",
                TextField {
                    form,
                    reference,
                    slot: Slot::Program,
                    label: "Program",
                    placeholder: "bun",
                    error: error_for(&errors, Field::Program),
                    disabled: running,
                }
                TextField {
                    form,
                    reference,
                    slot: Slot::Args,
                    label: "Arguments",
                    hint: "Separated by spaces",
                    placeholder: "run start",
                    disabled: running,
                }
            }
            p { class: "{GROUP_TITLE}", "Ready when" }
            div { class: "{GRID}",
                TextField {
                    form,
                    reference,
                    slot: Slot::Host,
                    label: "Host",
                    error: error_for(&errors, Field::Host),
                    disabled: running,
                }
                TextField {
                    form,
                    reference,
                    slot: Slot::Port,
                    label: "Port",
                    placeholder: "5001",
                    numeric: true,
                    error: error_for(&errors, Field::Port),
                    disabled: running,
                }
                TextField {
                    form,
                    reference,
                    slot: Slot::ReadinessPath,
                    label: "Path",
                    hint: "Checked with GET",
                    error: error_for(&errors, Field::ReadinessPath),
                    disabled: running,
                }
                TextField {
                    form,
                    reference,
                    slot: Slot::ExpectedStatus,
                    label: "Expected status",
                    numeric: true,
                    error: error_for(&errors, Field::ExpectedStatus),
                    disabled: running,
                }
            }
            TextField {
                form,
                reference,
                slot: Slot::ReadinessHeaders,
                label: "Readiness headers",
                hint: "Optional. One per line as Name: value. Leave empty to probe with the API's default health credentials. Never stored.",
                placeholder: "X-Client-Id: my-client\nX-Client-Secret: my-secret",
                rows: 3,
                error: error_for(&errors, Field::ReadinessHeaders),
                disabled: running,
            }
        }

        SwitchBlock {
            title: "Workload",
            subtitle: "HTTP load sent to the running target. Optional, needed for latency, memory and CPU.",
            on: state.workload_enabled,
            on_toggle: move |_| form.with_mut(|f| f.workload_enabled = !f.workload_enabled),
            div { class: "{GRID}",
                TextField {
                    form,
                    reference,
                    slot: Slot::WorkloadPath,
                    label: "Request path",
                    hint: "Sent with GET",
                    placeholder: "/items",
                    error: error_for(&errors, Field::WorkloadPath),
                    disabled: running,
                }
                div { class: "flex min-w-0 flex-col gap-(--space-1)",
                    span { class: "text-(length:--fs-xs) font-(weight:--fw-medium) text-(color:--clr-text-muted)",
                        "Run until"
                    }
                    SegmentSwitch {
                        label: "Run until",
                        options: vec![("Duration", running), ("Requests", running)],
                        selected: mode_index(state.mode),
                        width_rem: 6.5,
                        on_pick: move |index: usize| form.with_mut(|f| f.mode = mode_from_index(index)),
                    }
                    if let Some(was) = old_mode {
                        WasMark {
                            was: mode_name(was),
                            on_reset: move |_| form.with_mut(|f| f.mode = was),
                        }
                    }
                }
                TextField {
                    form,
                    reference,
                    slot: Slot::Amount,
                    label: amount_label,
                    hint: amount_hint,
                    numeric: true,
                    error: error_for(&errors, Field::Amount),
                    disabled: running,
                }
                StepperField {
                    form,
                    reference,
                    slot: Slot::Concurrency,
                    label: "Concurrency",
                    hint: "Simultaneous requests",
                    error: error_for(&errors, Field::Concurrency),
                    disabled: running,
                }
            }
            TextField {
                form,
                reference,
                slot: Slot::WorkloadHeaders,
                label: "Request headers",
                hint: "Optional. One per line as Name: value, for example X-Client-Id: my-client. Sent with every request of the workload and never stored.",
                placeholder: "X-Client-Id: my-client\nX-Client-Secret: my-secret",
                rows: 3,
                error: error_for(&errors, Field::WorkloadHeaders),
                disabled: running,
            }
        }

        SwitchBlock {
            title: "Workspace",
            subtitle: "Optional. Also measures one more process after the project, such as a subscriber, natively and in Docker, one after the other.",
            on: state.workspace.enabled,
            on_toggle: move |_| form.with_mut(|f| f.workspace.enabled = !f.workspace.enabled),
            div { class: "{GRID}",
                TextField {
                    form,
                    reference,
                    slot: Slot::WorkspaceName,
                    label: "Name",
                    hint: "Shown in results and used to match runs when comparing",
                    placeholder: "subscriber",
                    disabled: running,
                }
                TextField {
                    form,
                    reference,
                    slot: Slot::WorkspaceDirectory,
                    label: "Directory",
                    hint: "Relative to the project path",
                    placeholder: "subscriber",
                    disabled: running,
                }
                TextField {
                    form,
                    reference,
                    slot: Slot::ReadyLogLine,
                    label: "Ready log line",
                    hint: "Text that must appear in the output before the process counts as ready",
                    disabled: running,
                }
                TextField {
                    form,
                    reference,
                    slot: Slot::ReadyTimeout,
                    label: "Ready timeout (seconds)",
                    numeric: true,
                    disabled: running,
                }
                TextField {
                    form,
                    reference,
                    slot: Slot::IdleSeconds,
                    label: "Idle time (seconds)",
                    hint: "Sampling window when there is no load command",
                    numeric: true,
                    disabled: running,
                }
                TextField {
                    form,
                    reference,
                    slot: Slot::LoadCommand,
                    label: "Load command",
                    hint: "Optional. Runs while memory is sampled. Leave empty to sample while idle.",
                    disabled: running,
                }
            }
            div { class: "flex flex-col gap-(--space-1)",
                span { class: "text-(length:--fs-xs) font-(weight:--fw-medium) text-(color:--clr-text-muted)",
                    "Environment variables"
                }
                SegmentSwitch {
                    label: "Environment variables",
                    options: vec![("None", running), ("Env file", running), ("Paste", running)],
                    selected: env_index(state.workspace.env_source),
                    width_rem: 6.5,
                    on_pick: move |index: usize| {
                        form.with_mut(|f| f.workspace.env_source = env_from_index(index));
                    },
                }
                span { class: "text-(length:--fs-xs) text-(color:--clr-text-faint)",
                    "Optional. The values are sent to the API for this run only and never stored, only their names are."
                }
                if let Some(was) = old_env {
                    WasMark {
                        was: env_name(was),
                        on_reset: move |_| form.with_mut(|f| f.workspace.env_source = was),
                    }
                }
            }
            if state.workspace.env_source == EnvSource::File {
                TextField {
                    form,
                    reference,
                    slot: Slot::EnvFilePath,
                    label: "Env file path",
                    hint: "Read on the API's machine, relative to the project path or absolute",
                    placeholder: "subscriber/.env",
                    disabled: running,
                }
            }
            if state.workspace.env_source == EnvSource::Pasted {
                TextField {
                    form,
                    reference,
                    slot: Slot::EnvPasted,
                    label: ".env contents",
                    hint: "One NAME=value per line. Comments, export, quotes and blank lines are fine.",
                    placeholder: "DATABASE_URL=postgres://...\nAPI_KEY=...",
                    rows: 6,
                    disabled: running,
                }
            }

            MiniBlock {
                title: "Native process",
                subtitle: "Build and start it directly on this machine.",
                on: state.workspace.native_enabled,
                on_toggle: move |_| form.with_mut(|f| f.workspace.native_enabled = !f.workspace.native_enabled),
                div { class: "{GRID}",
                    TextField {
                        form,
                        reference,
                        slot: Slot::NativeBuild,
                        label: "Build command",
                        hint: "Optional. Runs once before the start command, for example cargo build --release",
                        disabled: running,
                    }
                    TextField {
                        form,
                        reference,
                        slot: Slot::NativeStart,
                        label: "Start command",
                        hint: "For example bun run dev or ./target/release/subscriber",
                        disabled: running,
                    }
                }
            }

            MiniBlock {
                title: "Docker container",
                subtitle: "Build an image and run it as a container.",
                on: state.workspace.docker_enabled,
                on_toggle: move |_| form.with_mut(|f| f.workspace.docker_enabled = !f.workspace.docker_enabled),
                div { class: "{GRID}",
                    TextField {
                        form,
                        reference,
                        slot: Slot::DockerBuild,
                        label: "Build command",
                        hint: "For example docker compose build subscriber",
                        disabled: running,
                    }
                    TextField {
                        form,
                        reference,
                        slot: Slot::DockerBuildTimeout,
                        label: "Build timeout (seconds)",
                        hint: "Long by default so a Rust build is not cut off",
                        numeric: true,
                        disabled: running,
                    }
                    TextField {
                        form,
                        reference,
                        slot: Slot::DockerImage,
                        label: "Image",
                        hint: "Read for its size after the build",
                        placeholder: "recordlog-subscriber:latest",
                        disabled: running,
                    }
                    TextField {
                        form,
                        reference,
                        slot: Slot::DockerContainer,
                        label: "Container name",
                        hint: "Used to detect a running copy and to remove the container",
                        placeholder: "recordlog-subscriber",
                        disabled: running,
                    }
                }
                TextField {
                    form,
                    reference,
                    slot: Slot::DockerRun,
                    label: "Run command",
                    hint: "Use {ENV_FILE_PLACEHOLDER} where the env file path goes, for example --env-file {ENV_FILE_PLACEHOLDER}. Required when an env is given.",
                    placeholder: "docker run -d --env-file {ENV_FILE_PLACEHOLDER} --name recordlog-subscriber recordlog-subscriber:latest",
                    disabled: running,
                }
                div { class: "{GRID}",
                    TextField {
                        form,
                        reference,
                        slot: Slot::DockerCleanup,
                        label: "Cleanup command",
                        hint: "Optional. Runs in the project path at the very end of the run, for example docker rmi recordlog-subscriber:latest",
                        placeholder: "docker rmi recordlog-subscriber:latest",
                        disabled: running,
                    }
                    TextField {
                        form,
                        reference,
                        slot: Slot::DockerCleanupTimeout,
                        label: "Cleanup timeout (seconds)",
                        hint: "Optional, 1 to 3600. Left empty, 120 seconds.",
                        placeholder: "120",
                        numeric: true,
                        disabled: running,
                    }
                }
            }

            if !workspace_error.is_empty() {
                p {
                    class: "text-(length:--fs-xs) text-(color:--clr-danger)",
                    role: "alert",
                    "{workspace_error}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_and_env_indexes_round_trip() {
        for mode in [WorkloadMode::Duration, WorkloadMode::Requests] {
            assert_eq!(mode_from_index(mode_index(mode)), mode);
        }

        for source in [EnvSource::None, EnvSource::File, EnvSource::Pasted] {
            assert_eq!(env_from_index(env_index(source)), source);
        }
    }

    #[test]
    fn names_match_the_switch_segments() {
        assert_eq!(mode_name(WorkloadMode::Duration), "Duration");
        assert_eq!(env_name(EnvSource::File), "Env file");
    }
}
