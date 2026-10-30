use dioxus::prelude::*;

use super::{
    api::submit_live,
    form::{error_for, validate, Field, FieldError, FormState, WorkloadMode},
};
use crate::{
    app::Route,
    components::{
        button_class, ButtonVariant, Card, CardHeader, FormField, Icon, IconKind, PageHeader,
        INPUT_CLASS, TEXTAREA_CLASS,
    },
    features::{
        connection::{ApiError, ApiErrorPanel},
        live::{active_run_id, save_token, SessionTokenStore},
    },
};

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Editing,
    // The API is taking the request, it answers with the run id at once
    Running,
    Failed(ApiError),
}

// Form that starts a live run and then opens its live view
#[component]
pub fn Submission() -> Element {
    let mut form = use_signal(FormState::default);
    let mut errors = use_signal(Vec::<FieldError>::new);
    let mut phase = use_signal(|| Phase::Editing);
    let nav = use_navigator();

    let state = form();
    let current_errors = errors();
    let primary = button_class(ButtonVariant::Primary);
    let secondary = button_class(ButtonVariant::Secondary);
    let running = matches!(phase(), Phase::Running);

    let mut submit = move |_| {
        let request = match validate(&form.peek()) {
            Ok(request) => request,
            Err(found) => {
                errors.set(found);
                return;
            }
        };

        errors.set(Vec::new());
        phase.set(Phase::Running);

        let nav = nav.clone();

        spawn(async move {
            match submit_live(&request).await {
                Ok(accepted) => {
                    // Kept for this tab, so the live page can offer to cancel the run
                    let _ =
                        save_token(&SessionTokenStore, &accepted.run_id, &accepted.cancel_token);

                    let _ = nav.push(Route::LiveRun {
                        run_id: accepted.run_id,
                    });
                }
                Err(error) => phase.set(Phase::Failed(error)),
            }
        });
    };

    let body: Element = match phase() {
        Phase::Failed(error) => {
            // A busy API names the run that holds it, so it can be watched instead
            let active = active_run_id(&error);

            rsx! {
                ApiErrorPanel { error, on_retry: move |_| phase.set(Phase::Editing) }
                if let Some(run_id) = active {
                    div { class: "mt-(--space-3)",
                        Link {
                            to: Route::LiveRun {
                                run_id,
                            },
                            class: "{primary}",
                            "Watch the active run"
                        }
                    }
                }
            }
        }
        Phase::Editing | Phase::Running => rsx! {
            form {
                class: "flex flex-col gap-(--space-4)",
                onsubmit: move |event| {
                    event.prevent_default();
                    submit(());
                },
                Card {
                    CardHeader {
                        title: "Target",
                        subtitle: "The project directory on the machine running benchmark-api.",
                    }
                    div { class: "p-(--space-4)",
                        FormField {
                            label: "Project path",
                            hint: "Absolute path, for example /home/me/projects/my-api",
                            error: error_for(&current_errors, Field::Path),
                            input {
                                r#type: "text",
                                class: "{INPUT_CLASS}",
                                value: "{state.path}",
                                disabled: running,
                                oninput: move |event| form.with_mut(|f| f.path = event.value()),
                            }
                        }
                    }
                }
                Card {
                    CardHeader {
                        title: "Startup",
                        subtitle: "How to start the target and when it counts as ready. Optional.",
                    }
                    div { class: "flex flex-col gap-(--space-4) p-(--space-4)",
                        label { class: "flex cursor-pointer items-center gap-(--space-3) rounded-(--round-md) border border-(color:--clr-border) bg-(--clr-surface-raised) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-text) transition-colors hover:border-(color:--clr-border-strong)",
                            input {
                                r#type: "checkbox",
                                class: "size-4 cursor-pointer accent-(--clr-primary)",
                                checked: state.startup_enabled,
                                disabled: running,
                                onchange: move |event| form.with_mut(|f| f.startup_enabled = event.checked()),
                            }
                            "Configure startup explicitly"
                        }
                        if state.startup_enabled {
                            div { class: "grid grid-cols-1 gap-(--space-4) sm:grid-cols-2",
                                FormField {
                                    label: "Program",
                                    error: error_for(&current_errors, Field::Program),
                                    input {
                                        r#type: "text",
                                        class: "{INPUT_CLASS}",
                                        placeholder: "bun",
                                        value: "{state.program}",
                                        disabled: running,
                                        oninput: move |event| form.with_mut(|f| f.program = event.value()),
                                    }
                                }
                                FormField { label: "Arguments", hint: "Separated by spaces",
                                    input {
                                        r#type: "text",
                                        class: "{INPUT_CLASS}",
                                        placeholder: "run start",
                                        value: "{state.args}",
                                        disabled: running,
                                        oninput: move |event| form.with_mut(|f| f.args = event.value()),
                                    }
                                }
                                FormField {
                                    label: "Readiness host",
                                    error: error_for(&current_errors, Field::Host),
                                    input {
                                        r#type: "text",
                                        class: "{INPUT_CLASS}",
                                        value: "{state.host}",
                                        disabled: running,
                                        oninput: move |event| form.with_mut(|f| f.host = event.value()),
                                    }
                                }
                                FormField {
                                    label: "Readiness port",
                                    error: error_for(&current_errors, Field::Port),
                                    input {
                                        r#type: "text",
                                        inputmode: "numeric",
                                        class: "{INPUT_CLASS}",
                                        placeholder: "5001",
                                        value: "{state.port}",
                                        disabled: running,
                                        oninput: move |event| form.with_mut(|f| f.port = event.value()),
                                    }
                                }
                                FormField {
                                    label: "Readiness path",
                                    hint: "Checked with GET",
                                    error: error_for(&current_errors, Field::ReadinessPath),
                                    input {
                                        r#type: "text",
                                        class: "{INPUT_CLASS}",
                                        value: "{state.readiness_path}",
                                        disabled: running,
                                        oninput: move |event| form.with_mut(|f| f.readiness_path = event.value()),
                                    }
                                }
                                FormField {
                                    label: "Expected status",
                                    error: error_for(&current_errors, Field::ExpectedStatus),
                                    input {
                                        r#type: "text",
                                        inputmode: "numeric",
                                        class: "{INPUT_CLASS}",
                                        value: "{state.expected_status}",
                                        disabled: running,
                                        oninput: move |event| form.with_mut(|f| f.expected_status = event.value()),
                                    }
                                }
                            }
                            FormField {
                                label: "Readiness headers",
                                hint: "Optional. One per line as Name: value. Leave empty to probe with the API's default health credentials. Never stored.",
                                error: error_for(&current_errors, Field::ReadinessHeaders),
                                textarea {
                                    class: "{TEXTAREA_CLASS}",
                                    rows: "3",
                                    spellcheck: "false",
                                    autocomplete: "off",
                                    placeholder: "X-Client-Id: my-client\nX-Client-Secret: my-secret",
                                    value: "{state.readiness_headers}",
                                    disabled: running,
                                    oninput: move |event| form.with_mut(|f| f.readiness_headers = event.value()),
                                }
                            }
                        }
                    }
                }
                Card {
                    CardHeader {
                        title: "Workload",
                        subtitle: "HTTP load sent to the running target. Optional, needed for latency, memory and CPU.",
                    }
                    div { class: "flex flex-col gap-(--space-4) p-(--space-4)",
                        label { class: "flex cursor-pointer items-center gap-(--space-3) rounded-(--round-md) border border-(color:--clr-border) bg-(--clr-surface-raised) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-text) transition-colors hover:border-(color:--clr-border-strong)",
                            input {
                                r#type: "checkbox",
                                class: "size-4 cursor-pointer accent-(--clr-primary)",
                                checked: state.workload_enabled,
                                disabled: running,
                                onchange: move |event| form.with_mut(|f| f.workload_enabled = event.checked()),
                            }
                            "Send an HTTP workload"
                        }
                        if state.workload_enabled {
                            div { class: "grid grid-cols-1 gap-(--space-4) sm:grid-cols-2",
                                FormField {
                                    label: "Request path",
                                    hint: "Sent with GET",
                                    error: error_for(&current_errors, Field::WorkloadPath),
                                    input {
                                        r#type: "text",
                                        class: "{INPUT_CLASS}",
                                        placeholder: "/items",
                                        value: "{state.workload_path}",
                                        disabled: running,
                                        oninput: move |event| form.with_mut(|f| f.workload_path = event.value()),
                                    }
                                }
                                FormField { label: "Run until",
                                    select {
                                        class: "{INPUT_CLASS}",
                                        disabled: running,
                                        onchange: move |event| {
                                            let mode = if event.value() == "duration" {
                                                WorkloadMode::Duration
                                            } else {
                                                WorkloadMode::Requests
                                            };
                                            form.with_mut(|f| f.mode = mode);
                                        },
                                        option {
                                            value: "requests",
                                            selected: state.mode == WorkloadMode::Requests,
                                            "A number of requests"
                                        }
                                        option {
                                            value: "duration",
                                            selected: state.mode == WorkloadMode::Duration,
                                            "A duration"
                                        }
                                    }
                                }
                                FormField {
                                    label: if state.mode == WorkloadMode::Requests { "Requests" } else { "Duration (seconds)" },
                                    hint: if state.mode == WorkloadMode::Requests { "" } else { "Up to 300 seconds" },
                                    error: error_for(&current_errors, Field::Amount),
                                    input {
                                        r#type: "text",
                                        inputmode: "numeric",
                                        class: "{INPUT_CLASS}",
                                        value: "{state.amount}",
                                        disabled: running,
                                        oninput: move |event| form.with_mut(|f| f.amount = event.value()),
                                    }
                                }
                                FormField {
                                    label: "Concurrency",
                                    hint: "Simultaneous requests",
                                    error: error_for(&current_errors, Field::Concurrency),
                                    input {
                                        r#type: "text",
                                        inputmode: "numeric",
                                        class: "{INPUT_CLASS}",
                                        value: "{state.concurrency}",
                                        disabled: running,
                                        oninput: move |event| form.with_mut(|f| f.concurrency = event.value()),
                                    }
                                }
                            }
                            FormField {
                                label: "Request headers",
                                hint: "Optional. One per line as Name: value, for example X-Client-Id: my-client. Sent with every request of the workload and never stored.",
                                error: error_for(&current_errors, Field::WorkloadHeaders),
                                textarea {
                                    class: "{TEXTAREA_CLASS}",
                                    rows: "3",
                                    spellcheck: "false",
                                    autocomplete: "off",
                                    placeholder: "X-Client-Id: my-client\nX-Client-Secret: my-secret",
                                    value: "{state.workload_headers}",
                                    disabled: running,
                                    oninput: move |event| form.with_mut(|f| f.workload_headers = event.value()),
                                }
                            }
                        }
                    }
                }
                div { class: "flex flex-col items-start gap-(--space-3)",
                    button {
                        r#type: "submit",
                        class: "{primary}",
                        disabled: running,
                        Icon { kind: IconKind::Play, class: "size-(--icon-sm)" }
                        if running {
                            "Running benchmark…"
                        } else {
                            "Run benchmark"
                        }
                    }
                    if running {
                        div {
                            class: "flex w-full items-start gap-(--space-3) rounded-(--round-md) border border-(color:--clr-border) bg-(--clr-surface) px-(--space-3) py-(--space-3)",
                            role: "status",
                            span { class: "mt-(--space-1) inline-block size-4 shrink-0 animate-spin rounded-(--round-full) border-2 border-(color:--clr-border-strong) border-t-(color:--clr-accent)" }
                            p { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)",
                                "Starting the run. You will be taken to its live view in a moment."
                            }
                        }
                    }
                    if !current_errors.is_empty() {
                        p {
                            class: "w-full rounded-(--round-md) border border-(color:--clr-danger) bg-(--clr-danger-soft) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-danger)",
                            role: "alert",
                            "Fix the highlighted fields and try again."
                        }
                    }
                }
            }
        },
    };

    rsx! {
        document::Title { "New benchmark · Benchmark" }
        div { class: "mx-auto w-full max-w-(--form-max)",
            PageHeader {
                title: "New benchmark",
                description: "Build, start and measure a project. You follow it live while it runs, and the result is stored in the run history.",
                Link { to: Route::History {}, class: "{secondary}", "All runs" }
            }
            {body}
        }
    }
}
