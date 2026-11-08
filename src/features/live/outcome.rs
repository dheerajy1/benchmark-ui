use dioxus::prelude::*;

use super::state::{LiveState, Stage, Terminal};
use crate::{
    app::Route,
    components::{button_class, ButtonVariant, Card, CardHeader, StatTile},
    features::run_detail::BenchmarkResult,
    format::{
        format_bytes, format_duration_ms, format_latency_ms, format_number, format_per_second,
        format_percent, short_id, MISSING,
    },
};

const BANNER_DANGER: &str =
    "border-(color:--clr-danger) bg-(--clr-danger-soft) text-(color:--clr-danger)";

// `build` becomes `build`, a key this version does not know is shown as it came
fn stage_name(key: &str) -> String {
    Stage::from_key(key).map_or_else(|| key.to_string(), |stage| stage.label().to_lowercase())
}

// What a stored run says about its own end
fn saved_text(persisted: bool) -> &'static str {
    if persisted {
        "The partial result was saved."
    } else {
        "The result was not stored."
    }
}

// `Stopped during the build step.`, nothing when the run stopped before any step
pub fn stopped_text(stage: Option<&str>) -> Option<String> {
    stage.map(|key| format!("Stopped during the {} step.", stage_name(key)))
}

// The line under a cancelled run
pub fn cancelled_subtitle(persisted: bool) -> String {
    format!(
        "The run was stopped on request and its target was stopped too. {}",
        saved_text(persisted)
    )
}

// The details under the message of a failed run
pub fn failed_detail(stage: Option<&str>, persisted: bool, code: Option<&str>) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(stopped) = stopped_text(stage) {
        parts.push(stopped);
    }

    parts.push(saved_text(persisted).to_string());

    if let Some(code) = code {
        parts.push(code.to_string());
    }

    parts.join(" ")
}

// Headline numbers of a stored result, the same as the run detail page
#[component]
fn FinalTiles(result: BenchmarkResult) -> Element {
    let metrics = &result.metrics;

    let throughput = metrics.http_latency.as_ref().map_or_else(
        || MISSING.to_string(),
        |latency| format_per_second(latency.requests_per_second),
    );
    let sent = metrics
        .http_latency
        .as_ref()
        .map(|latency| format!("{} sent", format_number(latency.total_requests as f64, 0)))
        .unwrap_or_default();
    let p95 = metrics
        .http_latency
        .as_ref()
        .and_then(|latency| latency.p95_ms)
        .map_or_else(|| MISSING.to_string(), format_latency_ms);
    let memory = metrics.memory.as_ref().map_or_else(
        || MISSING.to_string(),
        |memory| format_bytes(memory.peak.rss_bytes as f64),
    );
    let cpu = metrics.cpu.as_ref().map_or_else(
        || MISSING.to_string(),
        |cpu| format_percent(cpu.average_percent),
    );
    let startup = metrics
        .startup_time
        .duration_ms
        .map_or_else(|| MISSING.to_string(), |ms| format_duration_ms(ms as f64));
    let build = metrics
        .build_time
        .duration_ms
        .map_or_else(|| MISSING.to_string(), |ms| format_duration_ms(ms as f64));

    rsx! {
        div { class: "grid grid-cols-2 gap-(--space-3) md:grid-cols-3 xl:grid-cols-6",
            StatTile {
                label: "Throughput",
                value: throughput,
                hint: sent,
            }
            StatTile { label: "p95 latency", value: p95 }
            StatTile { label: "Peak memory", value: memory }
            StatTile { label: "CPU average", value: cpu }
            StatTile { label: "Startup", value: startup }
            StatTile { label: "Build", value: build }
        }
    }
}

// The end of the run, as a banner on this page. There is no automatic redirect, the final
// numbers stay on screen until the user moves on.
#[component]
pub fn Outcome(state: Signal<LiveState>, last_run_id: Option<String>) -> Element {
    let secondary = button_class(ButtonVariant::Secondary);
    let primary = button_class(ButtonVariant::Primary);

    let (terminal, run_id, recovered) = {
        let state = state.read();

        (
            state.terminal().cloned(),
            state.run_id().to_string(),
            state.is_recovered(),
        )
    };
    let short = short_id(&run_id);

    match terminal {
        None => rsx! {},
        Some(Terminal::Cancelled { stage, persisted }) => {
            let stopped = stopped_text(stage.as_deref());
            // The main action is the stored run when there is one
            let new_class = if persisted {
                secondary.clone()
            } else {
                primary.clone()
            };

            rsx! {
                Card {
                    CardHeader {
                        title: "Benchmark cancelled",
                        subtitle: cancelled_subtitle(persisted),
                    }
                    div { class: "flex flex-col gap-(--space-3) p-(--space-4)",
                        if let Some(stopped) = stopped {
                            p { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)", "{stopped}" }
                        }
                        div { class: "flex flex-wrap items-center gap-(--space-2)",
                            if persisted {
                                Link {
                                    to: Route::RunDetail {
                                        run_id: run_id.clone(),
                                    },
                                    class: "{primary}",
                                    "Open run {short}"
                                }
                            }
                            Link { to: Route::new_benchmark(), class: "{new_class}",
                                "New benchmark"
                            }
                        }
                    }
                }
            }
        }
        Some(Terminal::Completed(result)) => {
            let subtitle = if recovered {
                "The live record had expired, this is the stored result."
            } else {
                "The result is stored and appears in the run history."
            };

            rsx! {
                Card {
                    CardHeader { title: "Benchmark completed", subtitle }
                    div { class: "flex flex-col gap-(--space-4) p-(--space-4)",
                        if let Some(result) = result {
                            FinalTiles { result: *result }
                        }
                        div { class: "flex flex-wrap items-center gap-(--space-2)",
                            Link {
                                to: Route::RunDetail {
                                    run_id: run_id.clone(),
                                },
                                class: "{primary}",
                                "Open run {short}"
                            }
                            if let Some(last) = last_run_id {
                                Link {
                                    to: Route::Comparisons {
                                        baseline: last,
                                        comparison: run_id.clone(),
                                    },
                                    class: "{secondary}",
                                    "Compare with last run"
                                }
                            }
                            Link { to: Route::new_benchmark(), class: "{secondary}", "Run another benchmark" }
                        }
                    }
                }
            }
        }
        Some(Terminal::Failed {
            code,
            message,
            stage,
            persisted,
        }) => {
            let detail = failed_detail(stage.as_deref(), persisted, code.as_deref());

            rsx! {
                div {
                    class: "flex flex-col gap-(--space-3) rounded-(--round-lg) border px-(--space-4) py-(--space-3) {BANNER_DANGER}",
                    role: "alert",
                    div { class: "flex flex-col gap-(--space-1)",
                        span { class: "text-(length:--fs-base) font-(weight:--fw-semibold)", "The benchmark failed" }
                        span { class: "text-(length:--fs-sm) break-words", "{message}" }
                        span { class: "text-(length:--fs-xs) break-words opacity-80", "{detail}" }
                    }
                    div { class: "flex flex-wrap items-center gap-(--space-2)",
                        if persisted {
                            Link {
                                to: Route::SubmissionFrom {
                                    run_id: run_id.clone(),
                                },
                                class: "{primary}",
                                "Edit settings and run again"
                            }
                            Link {
                                to: Route::RunDetail {
                                    run_id: run_id.clone(),
                                },
                                class: "{secondary}",
                                "Open run {short}"
                            }
                        } else {
                            Link { to: Route::new_benchmark(), class: "{primary}", "New benchmark" }
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
    fn names_the_step_a_run_stopped_in() {
        assert_eq!(
            stopped_text(Some("build")).as_deref(),
            Some("Stopped during the build step.")
        );
        assert_eq!(stopped_text(None), None);
        assert_eq!(
            stopped_text(Some("mystery")).as_deref(),
            Some("Stopped during the mystery step.")
        );
    }

    #[test]
    fn a_cancelled_run_says_whether_its_partial_result_was_saved() {
        assert!(cancelled_subtitle(true).ends_with("The partial result was saved."));
        assert!(cancelled_subtitle(false).ends_with("The result was not stored."));
    }

    #[test]
    fn a_failed_run_lists_its_step_its_storage_and_its_code() {
        assert_eq!(
            failed_detail(Some("build"), true, Some("BUILD_FAILED")),
            "Stopped during the build step. The partial result was saved. BUILD_FAILED"
        );
        assert_eq!(
            failed_detail(None, false, None),
            "The result was not stored."
        );
    }
}
