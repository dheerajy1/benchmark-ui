use dioxus::prelude::*;

use super::{
    form::FormState,
    workspace::{EnvSource, ENV_FILE_PLACEHOLDER},
};
use crate::components::{Card, CardHeader, FormField, INPUT_CLASS, TEXTAREA_CLASS};

const CHECKBOX_ROW: &str = "flex cursor-pointer items-center gap-(--space-3) rounded-(--round-md) border border-(color:--clr-border) bg-(--clr-surface-raised) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-text) transition-colors hover:border-(color:--clr-border-strong)";
const CHECKBOX: &str = "size-4 cursor-pointer accent-(--clr-primary)";

// The optional workspace section: an extra process, for example the recordlog subscriber,
// that the run measures after the main project, natively and in Docker.
#[component]
pub fn WorkspaceSection(form: Signal<FormState>, error: String, running: bool) -> Element {
    let mut form = form;
    let state = form.read().workspace.clone();

    rsx! {
        Card {
            CardHeader {
                title: "Workspace",
                subtitle: "Optional. Also measures one more process after the project, such as a subscriber, natively and in Docker, one after the other.",
            }
            div { class: "flex flex-col gap-(--space-4) p-(--space-4)",
                label { class: "{CHECKBOX_ROW}",
                    input {
                        r#type: "checkbox",
                        class: "{CHECKBOX}",
                        checked: state.enabled,
                        disabled: running,
                        onchange: move |event| form.with_mut(|f| f.workspace.enabled = event.checked()),
                    }
                    "Measure a workspace"
                }
                if state.enabled {
                    div { class: "grid grid-cols-1 gap-(--space-4) sm:grid-cols-2",
                        FormField {
                            label: "Name",
                            hint: "Shown in results and used to match runs when comparing, for example subscriber",
                            input {
                                r#type: "text",
                                class: "{INPUT_CLASS}",
                                placeholder: "subscriber",
                                value: "{state.name}",
                                disabled: running,
                                oninput: move |event| form.with_mut(|f| f.workspace.name = event.value()),
                            }
                        }
                        FormField {
                            label: "Directory",
                            hint: "Relative to the project path",
                            input {
                                r#type: "text",
                                class: "{INPUT_CLASS}",
                                placeholder: "subscriber",
                                value: "{state.directory}",
                                disabled: running,
                                oninput: move |event| form.with_mut(|f| f.workspace.directory = event.value()),
                            }
                        }
                        FormField {
                            label: "Ready log line",
                            hint: "Text that must appear in the output before the process counts as ready",
                            input {
                                r#type: "text",
                                class: "{INPUT_CLASS}",
                                value: "{state.ready_log_line}",
                                disabled: running,
                                oninput: move |event| form.with_mut(|f| f.workspace.ready_log_line = event.value()),
                            }
                        }
                        FormField {
                            label: "Ready timeout (seconds)",
                            input {
                                r#type: "text",
                                inputmode: "numeric",
                                class: "{INPUT_CLASS}",
                                value: "{state.ready_timeout_seconds}",
                                disabled: running,
                                oninput: move |event| form.with_mut(|f| f.workspace.ready_timeout_seconds = event.value()),
                            }
                        }
                        FormField {
                            label: "Load command",
                            hint: "Optional. Runs while memory is sampled. Leave empty to sample while idle.",
                            input {
                                r#type: "text",
                                class: "{INPUT_CLASS}",
                                value: "{state.load_command}",
                                disabled: running,
                                oninput: move |event| form.with_mut(|f| f.workspace.load_command = event.value()),
                            }
                        }
                        FormField {
                            label: "Idle time (seconds)",
                            hint: "Sampling window when there is no load command",
                            input {
                                r#type: "text",
                                inputmode: "numeric",
                                class: "{INPUT_CLASS}",
                                value: "{state.idle_seconds}",
                                disabled: running,
                                oninput: move |event| form.with_mut(|f| f.workspace.idle_seconds = event.value()),
                            }
                        }
                    }
                    FormField {
                        label: "Environment variables",
                        hint: "Optional. The values are sent to the API for this run only and never stored, only their names are.",
                        select {
                            class: "{INPUT_CLASS}",
                            disabled: running,
                            onchange: move |event| {
                                let source = match event.value().as_str() {
                                    "file" => EnvSource::File,
                                    "pasted" => EnvSource::Pasted,
                                    _ => EnvSource::None,
                                };
                                form.with_mut(|f| f.workspace.env_source = source);
                            },
                            option {
                                value: "none",
                                selected: state.env_source == EnvSource::None,
                                "None"
                            }
                            option {
                                value: "file",
                                selected: state.env_source == EnvSource::File,
                                "An env file path"
                            }
                            option {
                                value: "pasted",
                                selected: state.env_source == EnvSource::Pasted,
                                "Paste the contents of a .env file"
                            }
                        }
                    }
                    if state.env_source == EnvSource::File {
                        FormField {
                            label: "Env file path",
                            hint: "Read on the API's machine, relative to the project path or absolute",
                            input {
                                r#type: "text",
                                class: "{INPUT_CLASS}",
                                placeholder: "subscriber/.env",
                                value: "{state.env_file_path}",
                                disabled: running,
                                oninput: move |event| form.with_mut(|f| f.workspace.env_file_path = event.value()),
                            }
                        }
                    }
                    if state.env_source == EnvSource::Pasted {
                        FormField {
                            label: ".env contents",
                            hint: "One NAME=value per line. Comments, export, quotes and blank lines are fine.",
                            textarea {
                                class: "{TEXTAREA_CLASS}",
                                rows: "6",
                                spellcheck: "false",
                                autocomplete: "off",
                                placeholder: "DATABASE_URL=postgres://...\nAPI_KEY=...",
                                value: "{state.env_pasted}",
                                disabled: running,
                                oninput: move |event| form.with_mut(|f| f.workspace.env_pasted = event.value()),
                            }
                        }
                    }

                    label { class: "{CHECKBOX_ROW}",
                        input {
                            r#type: "checkbox",
                            class: "{CHECKBOX}",
                            checked: state.native_enabled,
                            disabled: running,
                            onchange: move |event| form.with_mut(|f| f.workspace.native_enabled = event.checked()),
                        }
                        "Measure the native process"
                    }
                    if state.native_enabled {
                        div { class: "grid grid-cols-1 gap-(--space-4) sm:grid-cols-2",
                            FormField {
                                label: "Build command",
                                hint: "Optional. Runs once before the start command, for example cargo build --release",
                                input {
                                    r#type: "text",
                                    class: "{INPUT_CLASS}",
                                    value: "{state.native_build_command}",
                                    disabled: running,
                                    oninput: move |event| form.with_mut(|f| f.workspace.native_build_command = event.value()),
                                }
                            }
                            FormField {
                                label: "Start command",
                                hint: "For example bun run dev or ./target/release/subscriber",
                                input {
                                    r#type: "text",
                                    class: "{INPUT_CLASS}",
                                    value: "{state.native_command}",
                                    disabled: running,
                                    oninput: move |event| form.with_mut(|f| f.workspace.native_command = event.value()),
                                }
                            }
                        }
                    }

                    label { class: "{CHECKBOX_ROW}",
                        input {
                            r#type: "checkbox",
                            class: "{CHECKBOX}",
                            checked: state.docker_enabled,
                            disabled: running,
                            onchange: move |event| form.with_mut(|f| f.workspace.docker_enabled = event.checked()),
                        }
                        "Measure the Docker container"
                    }
                    if state.docker_enabled {
                        div { class: "grid grid-cols-1 gap-(--space-4) sm:grid-cols-2",
                            FormField {
                                label: "Build command",
                                hint: "For example docker compose build subscriber",
                                input {
                                    r#type: "text",
                                    class: "{INPUT_CLASS}",
                                    value: "{state.docker_build_command}",
                                    disabled: running,
                                    oninput: move |event| form.with_mut(|f| f.workspace.docker_build_command = event.value()),
                                }
                            }
                            FormField {
                                label: "Build timeout (seconds)",
                                hint: "Long by default so a Rust build is not cut off",
                                input {
                                    r#type: "text",
                                    inputmode: "numeric",
                                    class: "{INPUT_CLASS}",
                                    value: "{state.docker_build_timeout_seconds}",
                                    disabled: running,
                                    oninput: move |event| form.with_mut(|f| f.workspace.docker_build_timeout_seconds = event.value()),
                                }
                            }
                            FormField {
                                label: "Image",
                                hint: "Read for its size after the build",
                                input {
                                    r#type: "text",
                                    class: "{INPUT_CLASS}",
                                    placeholder: "recordlog-subscriber:latest",
                                    value: "{state.docker_image}",
                                    disabled: running,
                                    oninput: move |event| form.with_mut(|f| f.workspace.docker_image = event.value()),
                                }
                            }
                            FormField {
                                label: "Container name",
                                hint: "Used to detect a running copy and to remove the container",
                                input {
                                    r#type: "text",
                                    class: "{INPUT_CLASS}",
                                    placeholder: "recordlog-subscriber",
                                    value: "{state.docker_container_name}",
                                    disabled: running,
                                    oninput: move |event| form.with_mut(|f| f.workspace.docker_container_name = event.value()),
                                }
                            }
                        }
                        FormField {
                            label: "Run command",
                            hint: "Use {ENV_FILE_PLACEHOLDER} where the env file path goes, for example --env-file {ENV_FILE_PLACEHOLDER}. Required when an env is given.",
                            input {
                                r#type: "text",
                                class: "{INPUT_CLASS}",
                                placeholder: "docker run -d --env-file {ENV_FILE_PLACEHOLDER} --name recordlog-subscriber recordlog-subscriber:latest",
                                value: "{state.docker_run_command}",
                                disabled: running,
                                oninput: move |event| form.with_mut(|f| f.workspace.docker_run_command = event.value()),
                            }
                        }
                    }
                    if !error.is_empty() {
                        p {
                            class: "text-(length:--fs-xs) text-(color:--clr-danger)",
                            role: "alert",
                            "{error}"
                        }
                    }
                }
            }
        }
    }
}
