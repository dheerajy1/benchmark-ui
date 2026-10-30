use dioxus::prelude::*;

use super::state::{LiveState, Stage, StageState};
use crate::{
    components::{Card, CardHeader},
    format::{format_duration_ms, humanize_code},
};

// Where a step is, one visual state per marker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Pending,
    Running,
    Done,
    Failed,
    // Stopped on request
    Cancelled,
    // Finished without a result of its own, for example no build command
    Neutral,
    Skipped,
    NotRequested,
}

impl Phase {
    fn marker_class(self) -> &'static str {
        match self {
            Self::Pending => "border border-(color:--clr-border-strong)",
            Self::Running => "animate-spin border-2 border-(color:--clr-accent) border-t-transparent",
            Self::Done => "bg-(--clr-success-soft) text-(color:--clr-success)",
            Self::Failed => "bg-(--clr-danger-soft) text-(color:--clr-danger)",
            Self::Cancelled => "border border-(color:--clr-border-strong) text-(color:--clr-text-muted)",
            Self::Neutral | Self::Skipped | Self::NotRequested => {
                "border border-dashed border-(color:--clr-border-strong) text-(color:--clr-text-faint)"
            }
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            Self::Done => "✓",
            Self::Failed => "!",
            Self::Cancelled => "✕",
            Self::Neutral | Self::Skipped | Self::NotRequested => "–",
            Self::Pending | Self::Running => "",
        }
    }

    // Also read by assistive technology, so the state never depends on color alone
    fn word(self) -> &'static str {
        match self {
            Self::Pending => "Waiting",
            Self::Running => "Running",
            Self::Done => "Done",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
            Self::Neutral => "Finished",
            Self::Skipped => "Skipped",
            Self::NotRequested => "Not requested",
        }
    }

    fn label_class(self) -> &'static str {
        match self {
            Self::Pending | Self::NotRequested | Self::Skipped => "text-(color:--clr-text-faint)",
            Self::Failed => "text-(color:--clr-danger)",
            Self::Cancelled => "text-(color:--clr-text-muted)",
            _ => "text-(color:--clr-text)",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Bar {
    percent: u32,
    text: String,
}

#[derive(Debug, Clone, PartialEq)]
struct Row {
    label: &'static str,
    phase: Phase,
    detail: Option<String>,
    // Duration of a finished step, elapsed time of the running one
    time: Option<String>,
    bar: Option<Bar>,
}

fn is_good(status: &str) -> bool {
    matches!(status, "success" | "completed")
}

fn is_bad(status: &str) -> bool {
    matches!(status, "failed" | "timeout" | "unhealthy" | "target_exited")
}

fn join(first: Option<String>, second: Option<String>) -> Option<String> {
    match (first, second) {
        (Some(first), Some(second)) => Some(format!("{first} · {second}")),
        (first, second) => first.or(second),
    }
}

fn row_for(state: &LiveState, stage: Stage, clock_ms: u64) -> Row {
    let detail = state.stage_detail(stage);
    let failed_here = state.failed_stage() == Some(stage);

    let (phase, detail, time, bar) = match state.stage(stage) {
        StageState::Pending => {
            let not_requested = matches!(stage, Stage::Baseline | Stage::Workload)
                && state.workload_requested() == Some(false);

            if not_requested {
                (
                    Phase::NotRequested,
                    Some("No workload was requested".to_string()),
                    None,
                    None,
                )
            } else {
                (Phase::Pending, detail, None, None)
            }
        }
        StageState::Running { .. } => {
            let time = state
                .stage_elapsed_ms(stage, clock_ms)
                .map(|ms| format_duration_ms(ms as f64));

            let bar = if stage == Stage::Workload {
                state.latest_sample().map(|sample| Bar {
                    percent: (sample.progress.ratio() * 100.0).round() as u32,
                    text: sample.progress.text(),
                })
            } else {
                None
            };

            // Asked to stop and still working on it
            let detail = if state.is_cancelling() {
                join(Some("Stopping".to_string()), detail)
            } else {
                detail
            };

            (Phase::Running, detail, time, bar)
        }
        StageState::Done {
            status,
            duration_ms,
        } => {
            let time = duration_ms.map(|ms| format_duration_ms(ms as f64));

            if is_good(status) {
                (Phase::Done, detail, time, None)
            } else if is_bad(status) {
                (
                    Phase::Failed,
                    join(Some(humanize_code(status)), detail),
                    time,
                    None,
                )
            } else {
                (
                    Phase::Neutral,
                    join(Some(humanize_code(status)), detail),
                    time,
                    None,
                )
            }
        }
        StageState::Skipped { reason } => (Phase::Skipped, Some(humanize_code(reason)), None, None),
    };

    let phase = if failed_here {
        Phase::Failed
    } else if state.cancelled_stage() == Some(stage) {
        Phase::Cancelled
    } else {
        phase
    };

    Row {
        label: stage.label(),
        phase,
        detail,
        time,
        bar,
    }
}

#[component]
fn StageRow(row: Row) -> Element {
    let marker = row.phase.marker_class();
    let glyph = row.phase.glyph();
    let word = row.phase.word();
    let label_class = row.phase.label_class();

    rsx! {
        li { class: "flex items-start gap-(--space-3) px-(--space-4) py-(--space-3)",
            span {
                class: "mt-px flex size-6 shrink-0 items-center justify-center rounded-(--round-full) text-(length:--fs-xs) font-(weight:--fw-semibold) {marker}",
                "aria-hidden": "true",
                "{glyph}"
            }
            div { class: "flex min-w-0 flex-1 flex-col gap-(--space-1)",
                div { class: "flex items-baseline justify-between gap-(--space-3)",
                    span { class: "text-(length:--fs-sm) font-(weight:--fw-medium) {label_class}",
                        "{row.label}"
                        span { class: "sr-only", " · {word}" }
                    }
                    if let Some(time) = row.time.clone() {
                        span { class: "shrink-0 text-(length:--fs-xs) tabular-nums text-(color:--clr-text-muted)",
                            "{time}"
                        }
                    }
                }
                if let Some(detail) = row.detail.clone() {
                    span { class: "break-words text-(length:--fs-xs) text-(color:--clr-text-muted)",
                        "{detail}"
                    }
                }
                if let Some(bar) = row.bar.clone() {
                    div { class: "mt-(--space-1) flex flex-col gap-(--space-1)",
                        div {
                            role: "progressbar",
                            "aria-valuemin": "0",
                            "aria-valuemax": "100",
                            "aria-valuenow": "{bar.percent}",
                            "aria-label": "Workload progress",
                            class: "h-1.5 w-full overflow-hidden rounded-(--round-full) bg-(--clr-surface-raised)",
                            div {
                                class: "h-full rounded-(--round-full) bg-(--clr-accent) transition-[width]",
                                style: "width: {bar.percent}%",
                            }
                        }
                        span { class: "text-(length:--fs-xs) tabular-nums text-(color:--clr-text-muted)",
                            "{bar.text}"
                        }
                    }
                }
            }
        }
    }
}

// The steps of a run as a pipeline, every row shows only what the run has reported
#[component]
pub fn Pipeline(state: Signal<LiveState>, clock: Signal<u64>) -> Element {
    let clock_ms = clock();
    let rows: Vec<Row> = {
        let state = state.read();

        Stage::ALL
            .iter()
            .map(|stage| row_for(&state, *stage, clock_ms))
            .collect()
    };

    rsx! {
        Card {
            CardHeader {
                title: "Pipeline",
                subtitle: "Each step reports when it starts and when it finishes.",
            }
            ol { class: "flex flex-col divide-y divide-(color:--clr-border)",
                for (index , row) in rows.into_iter().enumerate() {
                    StageRow { key: "{index}", row }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::features::live::models::StreamEvent;

    fn feed(state: &mut LiveState, seq: u64, t_ms: u64, kind: &str, data: Value) {
        state.apply_event(
            StreamEvent {
                run_id: "r".to_string(),
                seq,
                t_ms,
                kind: kind.to_string(),
                data,
            },
            0,
        );
    }

    fn accepted(state: &mut LiveState, workload: Value) {
        feed(
            state,
            1,
            0,
            "run.accepted",
            json!({"target": {"path": "/p"}, "plan": {"startup": true, "workload": workload}}),
        );
    }

    #[test]
    fn every_step_waits_before_it_is_reported() {
        let state = LiveState::new("r".to_string());

        for stage in Stage::ALL {
            assert_eq!(row_for(&state, stage, 0).phase, Phase::Pending);
        }
    }

    #[test]
    fn a_running_step_shows_its_elapsed_time() {
        let mut state = LiveState::new("r".to_string());

        accepted(&mut state, Value::Null);
        feed(
            &mut state,
            2,
            1_000,
            "stage.started",
            json!({"stage": "build"}),
        );

        let row = row_for(&state, Stage::Build, 3_500);

        assert_eq!(row.phase, Phase::Running);
        // The last event, at server time 1 s, arrived at local 0 ms and 3.5 s of local time
        // have passed since, so the step started 3.5 s ago
        assert_eq!(row.time.as_deref(), Some("3.50 s"));
    }

    #[test]
    fn a_finished_step_shows_its_duration_and_a_failed_one_says_so() {
        let mut state = LiveState::new("r".to_string());

        accepted(&mut state, Value::Null);
        feed(
            &mut state,
            2,
            10,
            "stage.completed",
            json!({"stage": "build", "status": "success", "duration_ms": 2300}),
        );
        feed(
            &mut state,
            3,
            20,
            "stage.completed",
            json!({"stage": "startup", "status": "timeout", "duration_ms": 30000}),
        );
        feed(
            &mut state,
            4,
            30,
            "stage.completed",
            json!({"stage": "discovery", "status": "no_build_command", "duration_ms": null}),
        );

        let build = row_for(&state, Stage::Build, 0);
        let startup = row_for(&state, Stage::Startup, 0);
        let discovery = row_for(&state, Stage::Discovery, 0);

        assert_eq!(build.phase, Phase::Done);
        assert_eq!(build.time.as_deref(), Some("2.30 s"));
        assert_eq!(startup.phase, Phase::Failed);
        assert_eq!(startup.detail.as_deref(), Some("Timeout"));
        assert_eq!(discovery.phase, Phase::Neutral);
        assert_eq!(discovery.detail.as_deref(), Some("No build command"));
    }

    #[test]
    fn without_a_workload_the_baseline_and_workload_are_not_requested() {
        let mut state = LiveState::new("r".to_string());

        accepted(&mut state, Value::Null);

        assert_eq!(
            row_for(&state, Stage::Baseline, 0).phase,
            Phase::NotRequested
        );
        assert_eq!(
            row_for(&state, Stage::Workload, 0).phase,
            Phase::NotRequested
        );
        assert_eq!(row_for(&state, Stage::Build, 0).phase, Phase::Pending);
    }

    #[test]
    fn a_skipped_step_gives_its_reason() {
        let mut state = LiveState::new("r".to_string());

        accepted(&mut state, Value::Null);
        feed(
            &mut state,
            2,
            5,
            "stage.skipped",
            json!({"stage": "workload", "reason": "startup_not_successful"}),
        );

        let row = row_for(&state, Stage::Workload, 0);

        assert_eq!(row.phase, Phase::Skipped);
        assert_eq!(row.detail.as_deref(), Some("Startup not successful"));
    }

    #[test]
    fn the_running_workload_shows_real_progress_only_once_a_sample_exists() {
        let mut state = LiveState::new("r".to_string());

        accepted(
            &mut state,
            json!({"mode": "requests", "requests": 1000, "concurrency": 2}),
        );
        feed(
            &mut state,
            2,
            100,
            "stage.started",
            json!({"stage": "workload"}),
        );

        assert!(row_for(&state, Stage::Workload, 0).bar.is_none());

        feed(
            &mut state,
            3,
            600,
            "sample",
            json!({
                "workload_elapsed_ms": 500, "memory": null, "cpu": null,
                "http": { "completed": 250, "successful": 250, "failed": 0,
                          "requests_per_second": 500.0,
                          "progress": { "mode": "requests", "done": 250, "total": 1000 } }
            }),
        );

        let bar = row_for(&state, Stage::Workload, 0).bar.unwrap();

        assert_eq!(bar.percent, 25);
        assert_eq!(bar.text, "250 of 1,000 requests");
    }

    #[test]
    fn the_stage_a_run_failed_in_is_marked_failed() {
        let mut state = LiveState::new("r".to_string());

        accepted(&mut state, Value::Null);
        feed(&mut state, 2, 5, "stage.started", json!({"stage": "build"}));
        feed(
            &mut state,
            3,
            9,
            "run.failed",
            json!({"code": "X", "message": "boom", "stage": "build", "persisted": false}),
        );

        assert_eq!(row_for(&state, Stage::Build, 0).phase, Phase::Failed);
    }

    #[test]
    fn a_running_step_says_it_is_stopping_once_cancellation_was_requested() {
        let mut state = LiveState::new("r".to_string());

        accepted(&mut state, Value::Null);
        feed(
            &mut state,
            2,
            100,
            "stage.started",
            json!({"stage": "build"}),
        );

        assert_eq!(row_for(&state, Stage::Build, 0).detail, None);

        feed(
            &mut state,
            3,
            200,
            "run.cancelling",
            json!({"stage": "build"}),
        );

        let row = row_for(&state, Stage::Build, 0);

        assert_eq!(row.phase, Phase::Running);
        assert_eq!(row.detail.as_deref(), Some("Stopping"));
    }

    #[test]
    fn the_step_a_run_was_cancelled_in_is_marked_cancelled() {
        let mut state = LiveState::new("r".to_string());

        accepted(&mut state, Value::Null);
        feed(
            &mut state,
            2,
            100,
            "stage.started",
            json!({"stage": "startup"}),
        );
        feed(
            &mut state,
            3,
            200,
            "run.cancelling",
            json!({"stage": "startup"}),
        );
        feed(
            &mut state,
            4,
            300,
            "run.cancelled",
            json!({"stage": "startup", "persisted": false}),
        );

        assert_eq!(row_for(&state, Stage::Startup, 0).phase, Phase::Cancelled);
        assert_eq!(row_for(&state, Stage::Build, 0).phase, Phase::Pending);
    }
}
