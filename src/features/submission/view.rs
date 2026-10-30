use dioxus::prelude::*;

use super::{
    api::submit_benchmark,
    form::{error_for, validate, Field, FieldError, FormState, WorkloadMode},
};
use crate::{
    app::Route,
    components::{
        button_class, Badge, ButtonVariant, Card, CardHeader, FormField, Icon, IconKind,
        PageHeader, StatTile, Tone, INPUT_CLASS,
    },
    features::{
        connection::{ApiError, ApiErrorPanel},
        run_detail::BenchmarkResult,
    },
    format::{format_latency_ms, format_per_second, short_id, MISSING},
};

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Editing,
    Running,
    Done(Box<BenchmarkResult>),
    Failed(ApiError),
}

// Summary of a finished run with a link to its stored details
#[component]
fn Finished(result: BenchmarkResult, on_reset: EventHandler<()>) -> Element {
    let primary = button_class(ButtonVariant::Primary);
    let secondary = button_class(ButtonVariant::Secondary);
    let short = short_id(&result.run_id);
    let latency = result.metrics.http_latency.as_ref();
    let throughput = latency.map_or_else(
        || MISSING.to_string(),
        |l| format_per_second(l.requests_per_second),
    );
    let p95 = latency
        .and_then(|l| l.p95_ms)
        .map_or_else(|| MISSING.to_string(), format_latency_ms);

    rsx! {
        Card {
            CardHeader {
                title: "Benchmark completed",
                subtitle: "The result is stored and appears in the run history.",
            }
            div { class: "flex flex-col gap-(--space-4) p-(--space-4)",
                div { class: "flex items-center gap-(--space-2) text-(length:--fs-xs) text-(color:--clr-text-muted)",
                    Badge { label: "Completed", tone: Tone::Success }
                    span { class: "font-(family-name:--font-code)", "Run {short}" }
                }
                div { class: "grid grid-cols-2 gap-(--space-3)",
                    StatTile { label: "Throughput", value: throughput }
                    StatTile { label: "p95 latency", value: p95 }
                }
                div { class: "flex flex-wrap gap-(--space-2)",
                    Link {
                        to: Route::RunDetail {
                            run_id: result.run_id.clone(),
                        },
                        class: "{primary}",
                        "View run {short}"
                    }
                    button {
                        r#type: "button",
                        class: "{secondary}",
                        onclick: move |_| on_reset.call(()),
                        "Run another"
                    }
                }
            }
        }
    }
}

// Form that submits POST /api/v1/benchmarks and waits for the stored result
#[component]
pub fn Submission() -> Element {
    let mut form = use_signal(FormState::default);
    let mut errors = use_signal(Vec::<FieldError>::new);
    let mut phase = use_signal(|| Phase::Editing);

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

        spawn(async move {
            match submit_benchmark(&request).await {
                Ok(result) => phase.set(Phase::Done(Box::new(result))),
                Err(error) => phase.set(Phase::Failed(error)),
            }
        });
    };

    let body: Element = match phase() {
        Phase::Done(result) => rsx! {
            Finished {
                result: *result,
                on_reset: move |_| {
                    form.set(FormState::default());
                    phase.set(Phase::Editing);
                },
            }
        },
        Phase::Failed(error) => rsx! {
            ApiErrorPanel { error, on_retry: move |_| phase.set(Phase::Editing) }
        },
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
                                "The API is building, starting and measuring the target. This can take several minutes, keep this tab open."
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
                description: "Build, start and measure a project. The result is stored in the run history.",
                Link { to: Route::History {}, class: "{secondary}", "All runs" }
            }
            {body}
        }
    }
}
