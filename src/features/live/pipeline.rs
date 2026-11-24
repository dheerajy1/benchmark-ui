use dioxus::prelude::*;

use super::state::{cleanup_label, mode_title, phase_text, LiveState, Stage, StageState, Terminal};
use crate::{
    components::{Card, Skeleton},
    features::submission::{last_for, StageAverages},
    format::{format_bytes, format_duration_ms, humanize_code},
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
    // The one thing that moves: the running marker turns, a marker that just finished pops in
    fn motion_class(self) -> &'static str {
        match self {
            Self::Running => "mk-spin",
            Self::Done | Self::Failed => "pop-in",
            _ => "",
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

    // A step that has an outcome reads dark, the one that runs and those still waiting read grey
    fn label_class(self) -> &'static str {
        match self {
            Self::Done | Self::Failed | Self::Neutral => "text-(color:--clr-text)",
            Self::Running | Self::Pending | Self::Cancelled => "text-(color:--clr-text-muted)",
            Self::Skipped | Self::NotRequested => "text-(color:--clr-text-faint)",
        }
    }

    // The line under a step is red when the step failed, the failure is said there and not in
    // the title
    fn detail_class(self) -> &'static str {
        match self {
            Self::Failed => "text-(color:--clr-danger-text)",
            _ => "text-(color:--clr-text-muted)",
        }
    }
}

// The round marker of a step. Drawn as SVG on a 20 by 20 grid, so every state stays crisp at the
// size of a step and at the smaller size of a workspace mode. The colors and the draw-in of the
// check and the cross live in tailwind.css under `mk-`.
#[component]
fn Marker(phase: Phase, size: u32) -> Element {
    let motion = phase.motion_class();

    let shape: Element = match phase {
        Phase::Pending => rsx! {
            circle { class: "mk-ring", cx: "10", cy: "10", r: "9" }
        },
        Phase::Running => rsx! {
            circle { class: "mk-ring", cx: "10", cy: "10", r: "9" }
            path { class: "mk-arc", d: "M10 1a9 9 0 0 1 9 9" }
        },
        Phase::Done => rsx! {
            circle { class: "mk-ok", cx: "10", cy: "10", r: "10" }
            path { class: "mk-pen mk-tick", d: "M5.8 10.4l2.9 2.9 5.5-6" }
        },
        Phase::Failed => rsx! {
            circle { class: "mk-bad", cx: "10", cy: "10", r: "10" }
            path { class: "mk-pen mk-tick", d: "M6.6 6.6l6.8 6.8M13.4 6.6l-6.8 6.8" }
        },
        Phase::Cancelled => rsx! {
            circle { class: "mk-ring", cx: "10", cy: "10", r: "9" }
            rect {
                class: "mk-stop",
                x: "7",
                y: "7",
                width: "6",
                height: "6",
                rx: "1.5",
            }
        },
        Phase::Neutral | Phase::Skipped | Phase::NotRequested => rsx! {
            circle { class: "mk-ring mk-dashed", cx: "10", cy: "10", r: "9" }
            path { class: "mk-dash", d: "M7 10h6" }
        },
    };

    rsx! {
        svg {
            class: "shrink-0 {motion}",
            width: "{size}",
            height: "{size}",
            view_box: "0 0 20 20",
            fill: "none",
            "aria-hidden": "true",
            {shape}
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Bar {
    percent: u32,
    text: String,
}

// How a finished step compares with the same step of the last run
#[derive(Debug, Clone, PartialEq)]
struct Delta {
    text: String,
    class: &'static str,
}

// One workspace mode under the Measure workspace step
#[derive(Debug, Clone, PartialEq)]
struct SubRow {
    label: &'static str,
    phase: Phase,
    detail: Option<String>,
    // The line under the label is still to come, it holds its place as a skeleton
    waiting: bool,
}

// The values of a step the run has not reported yet. Each one holds its place as a skeleton, so
// nothing moves when the value arrives. A name that is already known is shown as text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Waiting {
    // The duration of a step that has not started
    time: bool,
    // The line under a title that a measurement fills in, such as the runtime
    detail: bool,
    // How the step compares with the last run, known to come once the step ends
    change: bool,
    // The progress bar of the workload, until its first sample
    bar: bool,
}

// Whether the run is still going. Once it ended or was asked to stop, a value that was not
// reported is not coming and its skeleton goes.
fn is_live(state: &LiveState) -> bool {
    !state.is_terminal() && !state.is_cancelling()
}

// The steps whose line under the title is a measurement made while the step runs. The other
// steps have no such line, or know it from the start like the workload does.
fn reports_detail(stage: Stage) -> bool {
    matches!(
        stage,
        Stage::Discovery | Stage::ArtifactSize | Stage::Baseline
    )
}

// One piece of the line a measuring step writes once it has its values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Piece {
    // A word the line always has, it is shown as text while the value is awaited
    Word(&'static str),
    // A value, shown as a skeleton of about its width
    Value(&'static str),
}

// The shape of the line under a step that measures something, so only the values wait and the
// words that are already known stay as text
fn detail_shape(stage: Stage) -> &'static [Piece] {
    match stage {
        Stage::Discovery => &[Piece::Word("Runtime"), Piece::Value("w-12")],
        Stage::ArtifactSize => &[
            Piece::Word("Source"),
            Piece::Value("w-14"),
            Piece::Word("· build output"),
            Piece::Value("w-16"),
        ],
        Stage::Baseline => &[Piece::Word("Idle memory"), Piece::Value("w-14")],
        _ => &[Piece::Value("w-44")],
    }
}

fn waiting_for(stage: Stage, phase: Phase, live: bool, has_detail: bool, has_bar: bool) -> Waiting {
    let ahead = live && matches!(phase, Phase::Pending | Phase::Running);

    Waiting {
        // A running step shows its elapsed time, so only a step that has not started waits
        time: live && phase == Phase::Pending,
        detail: ahead && !has_detail && reports_detail(stage),
        // Set once the last run is known, see `waits_for_change`
        change: false,
        // The place of the bar is held from the start, a step that has not started has none yet
        bar: stage == Stage::Workload
            && live
            && (phase == Phase::Pending || (phase == Phase::Running && !has_bar)),
    }
}

// A step that has a time in the last run ends with a comparison under its own time
fn waits_for_change(phase: Phase, live: bool, has_last: bool) -> bool {
    live && has_last && matches!(phase, Phase::Pending | Phase::Running)
}

// A workspace goes through its two modes, in this order
const WORKSPACE_MODES: [&str; 2] = ["native", "docker"];

// The titles of the rows the Measure workspace step is expected to report, every workspace brings
// its two modes. The Workspaces section waits for the same two modes.
fn workspace_labels(workspaces: u64) -> Vec<&'static str> {
    (0..workspaces)
        .flat_map(|_| WORKSPACE_MODES)
        .map(mode_title)
        .collect()
}

// The rows a step is expected to report and has not yet. They sit in place as waiting rows, so the
// step does not grow with every report. A row the step ended without reporting is marked not
// requested, one the run never got to says it was not run.
fn reserved_rows(missing: Vec<&'static str>, step: Phase, live: bool) -> Vec<SubRow> {
    let coming = live && matches!(step, Phase::Pending | Phase::Running);
    let ended = matches!(step, Phase::Done | Phase::Neutral);

    missing
        .into_iter()
        .map(|label| {
            if coming {
                SubRow {
                    label,
                    phase: Phase::Pending,
                    detail: None,
                    waiting: true,
                }
            } else if ended {
                SubRow {
                    label,
                    phase: Phase::NotRequested,
                    detail: Some("Not requested".to_string()),
                    waiting: false,
                }
            } else {
                SubRow {
                    label,
                    phase: Phase::Pending,
                    detail: Some("Not run".to_string()),
                    waiting: false,
                }
            }
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
struct Row {
    stage: Stage,
    label: &'static str,
    phase: Phase,
    detail: Option<String>,
    // Duration of a finished step, elapsed time of the running one
    time: Option<String>,
    bar: Option<Bar>,
    // The duration of a step that finished well
    finished_ms: Option<u64>,
    change: Option<Delta>,
    // The workspace modes the run has reported, only on the Measure workspace step
    subs: Vec<SubRow>,
    waiting: Waiting,
}

// The modes of a run in the order they ran. A mode that never finished takes the state of the
// step, so a stopped run does not keep a spinner.
fn workspace_subs(state: &LiveState, step: Phase) -> Vec<SubRow> {
    let mut subs: Vec<SubRow> = state
        .workspace_modes()
        .iter()
        .map(|live| {
            let workspace = Some(live.workspace.clone());
            let peak = live
                .points
                .iter()
                .filter_map(|point| point.peak_rss_bytes)
                .max()
                .map(|bytes| format!("peak {}", format_bytes(bytes as f64)));

            let (phase, detail) = match (live.phase.as_str(), live.status.as_deref()) {
                ("finished", Some(status)) if is_bad(status) => {
                    (Phase::Failed, join(workspace, Some(humanize_code(status))))
                }
                ("finished", Some(status)) if !is_good(status) => {
                    (Phase::Neutral, join(workspace, Some(humanize_code(status))))
                }
                ("finished", _) => (Phase::Done, join(workspace, peak)),
                (phase, _) => {
                    let shown = match step {
                        Phase::Running => Phase::Running,
                        Phase::Failed => Phase::Failed,
                        Phase::Cancelled => Phase::Cancelled,
                        _ => Phase::Neutral,
                    };

                    (shown, join(workspace, Some(phase_text(phase).to_string())))
                }
            };

            SubRow {
                label: mode_title(&live.mode),
                phase,
                detail,
                waiting: false,
            }
        })
        .collect();

    // The modes still to come keep their place, the rows after the reported ones
    if state.workspaces_requested() == Some(true) {
        let workspaces = state.plan().map_or(0, |plan| plan.workspaces);
        let missing = workspace_labels(workspaces)
            .into_iter()
            .skip(subs.len())
            .collect();

        subs.extend(reserved_rows(missing, step, is_live(state)));
    }

    subs
}

// The cleanup commands in the order they ran. One that never finished takes the state of the
// step, so a stopped run does not keep a spinner.
fn cleanup_subs(state: &LiveState, step: Phase) -> Vec<SubRow> {
    let mut subs: Vec<SubRow> = state
        .cleanup_steps()
        .iter()
        .map(|live| {
            let workspace = live.workspace.clone();

            let (phase, detail) = match (live.phase.as_str(), live.status.as_deref()) {
                ("finished", Some(status)) if is_bad(status) => (
                    Phase::Failed,
                    join(
                        workspace,
                        live.message.clone().or_else(|| Some(humanize_code(status))),
                    ),
                ),
                ("finished", Some(status)) if !is_good(status) => {
                    (Phase::Neutral, join(workspace, Some(humanize_code(status))))
                }
                ("finished", _) => (Phase::Done, workspace),
                _ => {
                    let shown = match step {
                        Phase::Running => Phase::Running,
                        Phase::Failed => Phase::Failed,
                        Phase::Cancelled => Phase::Cancelled,
                        _ => Phase::Neutral,
                    };

                    (shown, join(workspace, Some("running".to_string())))
                }
            };

            SubRow {
                label: cleanup_label(&live.step),
                phase,
                detail,
                waiting: false,
            }
        })
        .collect();

    // The build cleanup is expected with any cleanup, the Docker one when workspaces are measured.
    // One that does not come ends as not requested.
    if state.cleanup_requested() == Some(true) {
        let workspaces = state.plan().map_or(0, |plan| plan.workspaces);
        let expected: &[&str] = if workspaces > 0 {
            &["build", "docker"]
        } else {
            &["build"]
        };
        let missing = expected
            .iter()
            .filter(|key| {
                !state
                    .cleanup_steps()
                    .iter()
                    .any(|known| known.step == **key)
            })
            .map(|key| cleanup_label(key))
            .collect();

        subs.extend(reserved_rows(missing, step, is_live(state)));
    }

    subs
}

// `+3.20 s vs last`, `-120 ms vs last`, or `same as last` when the difference is small. A
// longer time is worse, so it is red and a shorter one is green.
fn change_vs_last(duration_ms: u64, last_ms: u64) -> Delta {
    let difference = duration_ms.abs_diff(last_ms);

    if difference < (last_ms / 20).max(50) {
        return Delta {
            text: "same as last".to_string(),
            class: "text-(color:--clr-text-faint)",
        };
    }

    if duration_ms > last_ms {
        Delta {
            text: format!("+{} vs last", format_duration_ms(difference as f64)),
            class: "text-(color:--clr-danger-text)",
        }
    } else {
        Delta {
            text: format!("-{} vs last", format_duration_ms(difference as f64)),
            class: "text-(color:--clr-success-text)",
        }
    }
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
            let no_workload = matches!(stage, Stage::Baseline | Stage::Workload)
                && state.workload_requested() == Some(false);
            let no_workspace =
                stage == Stage::Workspaces && state.workspaces_requested() == Some(false);
            let no_cleanup = stage == Stage::Cleanup && state.cleanup_requested() == Some(false);

            if no_workload || no_workspace || no_cleanup {
                (
                    Phase::NotRequested,
                    Some(
                        if no_workspace {
                            "No workspace was requested"
                        } else if no_cleanup {
                            "No cleanup was requested"
                        } else {
                            "No workload was requested"
                        }
                        .to_string(),
                    ),
                    None,
                    None,
                )
            } else {
                // A run that stopped early says so on every step it never reached
                let detail = match state.terminal() {
                    Some(Terminal::Failed { persisted, .. })
                    | Some(Terminal::Cancelled { persisted, .. }) => Some(
                        if stage == Stage::Saving && *persisted {
                            "Partial result saved"
                        } else {
                            "Not run"
                        }
                        .to_string(),
                    ),
                    _ => detail,
                };

                (Phase::Pending, detail, None, None)
            }
        }
        StageState::Running { .. } => {
            let time = state
                .stage_elapsed_ms(stage, clock_ms)
                .map(|ms| format_duration_ms(ms as f64));

            // Asked to stop and still working on it
            let detail = if state.is_cancelling() {
                join(Some("Stopping".to_string()), detail)
            } else {
                detail
            };

            (Phase::Running, detail, time, None)
        }
        StageState::Done {
            status,
            duration_ms,
        } => {
            let time = duration_ms.map(|ms| format_duration_ms(ms as f64));

            if is_good(status) {
                (Phase::Done, detail, time, None)
            } else if is_bad(status) && stage != Stage::Cleanup {
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

    // The bar of the workload stays once the step ends, so the card does not shrink when it
    // finishes. Before the first sample the place is held, see `Waiting`.
    let bar = if stage == Stage::Workload
        && matches!(
            phase,
            Phase::Running | Phase::Done | Phase::Failed | Phase::Cancelled
        ) {
        state.latest_sample().map(|sample| Bar {
            percent: (sample.progress.ratio() * 100.0).round() as u32,
            text: sample.progress.text(),
        })
    } else {
        bar
    };

    let finished_ms = match state.stage(stage) {
        StageState::Done { duration_ms, .. } if phase == Phase::Done => *duration_ms,
        _ => None,
    };

    let subs = match stage {
        Stage::Workspaces => workspace_subs(state, phase),
        Stage::Cleanup => cleanup_subs(state, phase),
        _ => Vec::new(),
    };

    // The modes below say where the step is, so the step does not repeat it
    let detail = if phase == Phase::Running && !subs.is_empty() {
        state.is_cancelling().then(|| "Stopping".to_string())
    } else {
        detail
    };

    let waiting = waiting_for(
        stage,
        phase,
        is_live(state),
        detail.is_some(),
        bar.is_some(),
    );

    Row {
        stage,
        label: stage.label(),
        phase,
        detail,
        time,
        bar,
        finished_ms,
        change: None,
        subs,
        waiting,
    }
}

// The line under a step is green once the step is done, grey while the way is still open
fn connector_class(phase: Phase) -> &'static str {
    if phase == Phase::Done {
        "bg-(--clr-success)"
    } else {
        "bg-(--clr-border-strong)"
    }
}

#[component]
fn StageRow(row: Row, last: bool) -> Element {
    let word = row.phase.word();
    let label_class = row.phase.label_class();
    let detail_class = row.phase.detail_class();
    let line_class = connector_class(row.phase);
    // Spare height of the card is shared by the steps, the last one stays at the bottom
    let grow = if last { "" } else { "grow" };
    // The last step has no step below it, so the line it keeps for its own detail is the room
    // under it and it needs no padding there
    let pad = if last {
        "pt-(--space-3)"
    } else {
        "py-(--space-3)"
    };
    // Only the step that runs has a skeleton that moves, those still to come stay still
    let motion = if row.phase == Phase::Running {
        "sk-live"
    } else {
        "sk-still"
    };

    rsx! {
        li { class: "relative flex items-start gap-(--space-3) px-(--space-4) {pad} {grow}",
            if !last {
                span {
                    class: "absolute w-0.5 {line_class}",
                    style: "left: calc(var(--space-4) + 0.625rem - 1px); top: calc(var(--space-3) + 1.25rem + 2px); bottom: calc(var(--space-3) * -1 + 2px);",
                    "aria-hidden": "true",
                }
            }
            Marker { phase: row.phase, size: 20 }
            div { class: "flex min-w-0 flex-1 flex-col gap-(--space-1)",
                // First line: the name of the step and its time
                div { class: "flex items-baseline justify-between gap-(--space-3)",
                    span { class: "text-(length:--fs-sm) font-(weight:--fw-medium) {label_class}",
                        "{row.label}"
                        span { class: "sr-only", " · {word}" }
                    }
                    if let Some(time) = row.time.clone() {
                        span { class: "value-in shrink-0 text-(length:--fs-xs) tabular-nums text-(color:--clr-text-muted)",
                            "{time}"
                        }
                    } else if row.waiting.time {
                        div { class: "line-xs shrink-0", Skeleton { class: "h-3 w-10 {motion}" } }
                    }
                }
                // Second line: what the step measured on the left and how it compares with the
                // last run on the right. Every step has this line, empty or not, so the steps
                // follow each other at the same distance whatever they report.
                div { class: "line-xs items-baseline",
                    if let Some(detail) = row.detail.clone() {
                        span { class: "value-in min-w-0 break-words text-(length:--fs-xs) {detail_class}",
                            "{detail}"
                        }
                    } else if row.waiting.detail {
                        WaitingDetail {
                            stage: row.stage,
                            motion,
                            text_class: detail_class,
                        }
                    }
                    if let Some(change) = row.change.clone() {
                        span { class: "value-in ml-auto shrink-0 pl-(--space-3) text-(length:--fs-2xs) tabular-nums {change.class}",
                            "{change.text}"
                        }
                    } else if row.waiting.change {
                        div { class: "line-2xs ml-auto shrink-0 pl-(--space-3)",
                            Skeleton { class: "h-2.5 w-16 {motion}" }
                        }
                    }
                }
                if !row.subs.is_empty() {
                    ul { class: "mt-(--space-1) flex flex-col gap-(--space-2)",
                        for (index , sub) in row.subs.iter().enumerate() {
                            {
                                let word = sub.phase.word();
                                let label_class = sub.phase.label_class();
                                let detail_class = sub.phase.detail_class();

                                rsx! {
                                    li { key: "{index}", class: "flex items-start gap-(--space-2)",
                                        Marker { phase: sub.phase, size: 16 }
                                        div { class: "flex min-w-0 flex-col",
                                            span { class: "text-(length:--fs-xs) font-(weight:--fw-medium) {label_class}",
                                                "{sub.label}"
                                                span { class: "sr-only", " · {word}" }
                                            }
                                            // The line is always there, so a row that reports no
                                            // detail does not change height when it finishes
                                            div { class: "line-2xs",
                                                if let Some(detail) = sub.detail.clone() {
                                                    span { class: "value-in min-w-0 break-words text-(length:--fs-2xs) {detail_class}",
                                                        "{detail}"
                                                    }
                                                } else if sub.waiting {
                                                    Skeleton { class: "h-2.5 w-32 {motion}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
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
                        span { class: "value-in text-(length:--fs-xs) tabular-nums text-(color:--clr-text-muted)",
                            "{bar.text}"
                        }
                    }
                } else if row.waiting.bar {
                    // The empty track and the line below it, until the first sample arrives
                    div { class: "mt-(--space-1) flex flex-col gap-(--space-1)",
                        div { class: "h-1.5 w-full overflow-hidden rounded-(--round-full) bg-(--clr-surface-raised)" }
                        div { class: "line-xs", Skeleton { class: "h-3 w-36 {motion}" } }
                    }
                }
            }
        }
    }
}

// The line under a step while its values are awaited. The words the step always writes stay as
// text and only the values are skeletons. The line has the room of the real one.
#[component]
fn WaitingDetail(stage: Stage, motion: &'static str, text_class: &'static str) -> Element {
    let pieces = detail_shape(stage)
        .iter()
        .enumerate()
        .map(|(index, piece)| {
            let gap = if index == 0 { "" } else { "ml-(--space-1)" };

            (index, *piece, gap)
        });

    rsx! {
        div {
            class: "line-xs text-(length:--fs-xs) {text_class}",
            "aria-hidden": "true",
            for (index , piece , gap) in pieces {
                if let Piece::Value(width) = piece {
                    Skeleton { key: "{index}", class: "h-3 {width} {gap} {motion}" }
                } else if let Piece::Word(word) = piece {
                    span { key: "{index}", class: "{gap}", "{word}" }
                }
            }
        }
    }
}

// The steps of a run as a pipeline, every row shows only what the run has reported
#[component]
pub fn Pipeline(
    state: Signal<LiveState>,
    clock: Signal<u64>,
    averages: Option<StageAverages>,
) -> Element {
    let clock_ms = clock();
    let rows: Vec<Row> = {
        let state = state.read();

        Stage::ALL
            .iter()
            .map(|stage| {
                let mut row = row_for(&state, *stage, clock_ms);
                let last = averages.as_ref().and_then(|known| last_for(*stage, known));

                row.change = row
                    .finished_ms
                    .zip(last)
                    .map(|(duration, last)| change_vs_last(duration, last));
                row.waiting.change = waits_for_change(row.phase, is_live(&state), last.is_some());

                row
            })
            .collect()
    };

    let (count_text, percent, stopped) = progress_summary(&rows);
    let count_class = if stopped {
        "text-(color:--clr-danger-text)"
    } else {
        "text-(color:--clr-text-muted)"
    };
    let fill_class = if stopped {
        "bg-(--clr-danger)"
    } else {
        "bg-(--clr-text)"
    };
    let last_index = rows.len().saturating_sub(1);

    rsx! {
        Card { class: "flex flex-col",
            div { class: "flex items-center gap-(--space-3) px-(--space-4) pt-(--space-4) pb-(--space-2)",
                h3 { class: "text-(length:--fs-base) font-(weight:--fw-semibold) text-(color:--clr-text)",
                    "Pipeline"
                }
                div { class: "ml-auto flex items-center gap-(--space-3)",
                    span { class: "text-(length:--fs-xs) tabular-nums {count_class}", "{count_text}" }
                    div {
                        role: "progressbar",
                        "aria-valuemin": "0",
                        "aria-valuemax": "100",
                        "aria-valuenow": "{percent}",
                        "aria-label": "Steps finished",
                        class: "h-1 w-24 overflow-hidden rounded-(--round-full) bg-(--clr-surface-raised)",
                        div {
                            class: "h-full rounded-(--round-full) transition-[width] {fill_class}",
                            style: "width: {percent}%",
                        }
                    }
                }
            }
            ol { class: "flex flex-1 flex-col",
                for (index , row) in rows.into_iter().enumerate() {
                    StageRow {
                        key: "{index}",
                        row,
                        last: index == last_index,
                    }
                }
            }
        }
    }
}

// `5 of 8` while the run goes, `stopped at 4 of 8` when it failed or was cancelled, the share of
// steps finished, and whether the run stopped. Steps that were not requested are not counted.
fn progress_summary(rows: &[Row]) -> (String, u32, bool) {
    let counted: Vec<&Row> = rows
        .iter()
        .filter(|row| row.phase != Phase::NotRequested)
        .collect();
    let total = counted.len();
    let finished = counted
        .iter()
        .filter(|row| matches!(row.phase, Phase::Done | Phase::Neutral | Phase::Skipped))
        .count();
    let stopped_at = counted
        .iter()
        .position(|row| matches!(row.phase, Phase::Failed | Phase::Cancelled));

    let percent = (finished * 100)
        .checked_div(total)
        .map_or(0, |share| share as u32);

    match stopped_at {
        Some(index) => (
            format!("stopped at {} of {total}", index + 1),
            percent,
            true,
        ),
        None => (format!("{finished} of {total}"), percent, false),
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
    fn the_cleanup_step_is_only_requested_when_the_plan_says_so() {
        let mut state = LiveState::new("r".to_string());

        accepted(&mut state, Value::Null);

        let row = row_for(&state, Stage::Cleanup, 0);

        assert_eq!(row.phase, Phase::NotRequested);
        assert_eq!(row.detail.as_deref(), Some("No cleanup was requested"));

        let mut state = LiveState::new("r".to_string());

        feed(
            &mut state,
            1,
            0,
            "run.accepted",
            json!({"target": {"path": "/p"}, "plan": {"startup": true, "workload": null, "cleanup": true}}),
        );

        assert_eq!(row_for(&state, Stage::Cleanup, 0).phase, Phase::Pending);
    }

    fn cleanup_step(
        state: &mut LiveState,
        seq: u64,
        step: &str,
        phase: &str,
        status: Value,
        message: Value,
    ) {
        feed(
            state,
            seq,
            0,
            "cleanup.step",
            json!({
                "step": step,
                "workspace": if step == "docker" { json!("subscriber") } else { Value::Null },
                "phase": phase,
                "status": status,
                "message": message,
            }),
        );
    }

    #[test]
    fn the_cleanup_step_lists_each_command_and_a_failed_one_does_not_stop_the_run() {
        let mut state = LiveState::new("r".to_string());

        feed(
            &mut state,
            1,
            0,
            "run.accepted",
            json!({"target": {"path": "/p"}, "plan": {"startup": true, "workload": null, "cleanup": true}}),
        );
        feed(
            &mut state,
            2,
            0,
            "stage.started",
            json!({"stage": "cleanup"}),
        );
        cleanup_step(&mut state, 3, "build", "started", Value::Null, Value::Null);
        cleanup_step(
            &mut state,
            4,
            "build",
            "finished",
            json!("success"),
            Value::Null,
        );
        cleanup_step(&mut state, 5, "docker", "started", Value::Null, Value::Null);
        cleanup_step(
            &mut state,
            6,
            "docker",
            "finished",
            json!("failed"),
            json!("The cleanup command failed with exit code 1."),
        );
        feed(
            &mut state,
            7,
            0,
            "stage.completed",
            json!({"stage": "cleanup", "status": "failed", "duration_ms": 900}),
        );

        let row = row_for(&state, Stage::Cleanup, 0);

        assert_eq!(row.phase, Phase::Neutral);
        assert_eq!(row.subs.len(), 2);
        assert_eq!(row.subs[0].label, "Build cleanup");
        assert_eq!(row.subs[0].phase, Phase::Done);
        assert_eq!(row.subs[1].label, "Docker cleanup");
        assert_eq!(row.subs[1].phase, Phase::Failed);
        assert_eq!(
            row.subs[1].detail.as_deref(),
            Some("subscriber · The cleanup command failed with exit code 1.")
        );
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
        // The bar holds its place until then
        assert!(row_for(&state, Stage::Workload, 0).waiting.bar);

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
        assert!(!row_for(&state, Stage::Workload, 0).waiting.bar);
    }

    #[test]
    fn the_workload_bar_is_held_before_the_step_and_kept_after_it() {
        let mut state = LiveState::new("r".to_string());

        accepted(
            &mut state,
            json!({"mode": "requests", "requests": 1000, "concurrency": 2}),
        );

        // Not started: the place of the bar is held
        let row = row_for(&state, Stage::Workload, 0);

        assert!(row.bar.is_none());
        assert!(row.waiting.bar);

        feed(
            &mut state,
            2,
            100,
            "stage.started",
            json!({"stage": "workload"}),
        );
        feed(
            &mut state,
            3,
            600,
            "sample",
            json!({
                "workload_elapsed_ms": 500, "memory": null, "cpu": null,
                "http": { "completed": 1000, "successful": 1000, "failed": 0,
                          "requests_per_second": 500.0,
                          "progress": { "mode": "requests", "done": 1000, "total": 1000 } }
            }),
        );
        feed(
            &mut state,
            4,
            700,
            "stage.completed",
            json!({"stage": "workload", "status": "success", "duration_ms": 600}),
        );

        // Finished: the bar stays, so the card does not shrink
        let row = row_for(&state, Stage::Workload, 0);

        assert_eq!(row.phase, Phase::Done);
        assert_eq!(row.bar.map(|bar| bar.percent), Some(100));
        assert!(!row.waiting.bar);

        // A run that ended before the workload does not wait for it
        let mut state = LiveState::new("r".to_string());

        accepted(&mut state, json!({"mode": "requests", "requests": 10}));
        feed(
            &mut state,
            2,
            5,
            "run.failed",
            json!({"code": "X", "message": "boom", "stage": "build", "persisted": false}),
        );

        assert!(!row_for(&state, Stage::Workload, 0).waiting.bar);
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

    #[test]
    fn a_step_is_compared_with_the_last_run() {
        assert_eq!(change_vs_last(5_000, 5_100).text, "same as last");
        assert_eq!(change_vs_last(8_200, 5_000).text, "+3.20 s vs last");
        assert_eq!(change_vs_last(4_880, 5_000).text, "same as last");
        assert_eq!(change_vs_last(3_800, 5_000).text, "-1.20 s vs last");
        assert!(change_vs_last(8_200, 5_000).class.contains("danger"));
        assert!(change_vs_last(3_800, 5_000).class.contains("success"));
    }

    fn row_in(phase: Phase) -> Row {
        Row {
            stage: Stage::Discovery,
            label: "Step",
            phase,
            detail: None,
            time: None,
            bar: None,
            finished_ms: None,
            change: None,
            subs: Vec::new(),
            waiting: Waiting::default(),
        }
    }

    #[test]
    fn the_header_counts_finished_steps_and_leaves_out_those_not_requested() {
        let rows = vec![
            row_in(Phase::Done),
            row_in(Phase::Neutral),
            row_in(Phase::Running),
            row_in(Phase::NotRequested),
            row_in(Phase::Pending),
        ];

        assert_eq!(progress_summary(&rows), ("2 of 4".to_string(), 50, false));
    }

    #[test]
    fn the_header_says_where_a_run_stopped() {
        let rows = vec![
            row_in(Phase::Done),
            row_in(Phase::Done),
            row_in(Phase::Failed),
            row_in(Phase::Pending),
        ];

        assert_eq!(
            progress_summary(&rows),
            ("stopped at 3 of 4".to_string(), 50, true)
        );
        assert_eq!(progress_summary(&[]), ("0 of 0".to_string(), 0, false));
    }

    #[test]
    fn the_line_under_a_step_is_green_only_when_the_step_is_done() {
        assert!(connector_class(Phase::Done).contains("success"));
        assert!(connector_class(Phase::Running).contains("border"));
        assert!(connector_class(Phase::Failed).contains("border"));
    }

    #[test]
    fn only_the_running_marker_turns_and_a_finished_one_pops_in() {
        assert_eq!(Phase::Running.motion_class(), "mk-spin");
        assert_eq!(Phase::Done.motion_class(), "pop-in");
        assert_eq!(Phase::Failed.motion_class(), "pop-in");

        for phase in [
            Phase::Pending,
            Phase::Cancelled,
            Phase::Neutral,
            Phase::Skipped,
            Phase::NotRequested,
        ] {
            assert_eq!(phase.motion_class(), "");
        }
    }

    #[test]
    fn a_step_with_an_outcome_reads_dark_and_one_still_to_come_reads_grey() {
        assert!(Phase::Done.label_class().contains("--clr-text)"));
        assert!(Phase::Failed.label_class().contains("--clr-text)"));
        assert!(Phase::Running.label_class().contains("text-muted"));
        assert!(Phase::Pending.label_class().contains("text-muted"));
        assert!(Phase::NotRequested.label_class().contains("text-faint"));
    }

    #[test]
    fn only_a_failed_step_has_a_red_detail_line() {
        assert!(Phase::Failed.detail_class().contains("danger"));

        for phase in [Phase::Done, Phase::Running, Phase::Pending, Phase::Neutral] {
            assert!(!phase.detail_class().contains("danger"));
        }
    }

    #[test]
    fn steps_a_stopped_run_never_reached_say_so() {
        let mut state = LiveState::new("r".to_string());

        accepted(&mut state, Value::Null);
        feed(
            &mut state,
            2,
            5,
            "run.failed",
            json!({"code": "X", "message": "boom", "stage": "build", "persisted": true}),
        );

        assert_eq!(
            row_for(&state, Stage::Startup, 0).detail.as_deref(),
            Some("Not run")
        );
        assert_eq!(
            row_for(&state, Stage::Saving, 0).detail.as_deref(),
            Some("Partial result saved")
        );
    }

    fn workspace_phase(state: &mut LiveState, seq: u64, mode: &str, phase: &str, status: Value) {
        feed(
            state,
            seq,
            seq * 100,
            "workspace.phase",
            json!({"workspace": "subscriber", "mode": mode, "phase": phase, "status": status}),
        );
    }

    fn accepted_with(state: &mut LiveState, workspaces: u64, cleanup: bool) {
        feed(
            state,
            1,
            0,
            "run.accepted",
            json!({
                "target": {"path": "/p"},
                "plan": {"startup": true, "workload": null, "workspaces": workspaces, "cleanup": cleanup}
            }),
        );
    }

    fn labels(subs: &[SubRow]) -> Vec<&'static str> {
        subs.iter().map(|sub| sub.label).collect()
    }

    #[test]
    fn the_workspace_step_keeps_a_row_for_each_mode_before_it_reports() {
        let mut state = LiveState::new("r".to_string());

        accepted_with(&mut state, 1, false);
        feed(
            &mut state,
            2,
            100,
            "stage.started",
            json!({"stage": "workspaces"}),
        );

        let row = row_for(&state, Stage::Workspaces, 0);

        assert_eq!(labels(&row.subs), ["Native", "Docker"]);
        assert!(row
            .subs
            .iter()
            .all(|sub| sub.phase == Phase::Pending && sub.waiting && sub.detail.is_none()));

        // The number of rows stays the same while the modes report, only their state changes
        workspace_phase(&mut state, 3, "native", "started", Value::Null);

        let row = row_for(&state, Stage::Workspaces, 0);

        assert_eq!(labels(&row.subs), ["Native", "Docker"]);
        assert_eq!(row.subs[0].phase, Phase::Running);
        assert!(!row.subs[0].waiting);
        assert_eq!(row.subs[1].phase, Phase::Pending);
        assert!(row.subs[1].waiting);

        workspace_phase(&mut state, 4, "native", "finished", json!("success"));
        workspace_phase(&mut state, 5, "docker", "sampling", Value::Null);

        let row = row_for(&state, Stage::Workspaces, 0);

        assert_eq!(labels(&row.subs), ["Native", "Docker"]);
        assert_eq!(row.subs[0].phase, Phase::Done);
        assert_eq!(row.subs[1].phase, Phase::Running);
    }

    #[test]
    fn a_workspace_brings_its_two_modes_and_none_are_reserved_without_a_workspace() {
        let mut state = LiveState::new("r".to_string());

        accepted_with(&mut state, 2, false);

        assert_eq!(
            labels(&row_for(&state, Stage::Workspaces, 0).subs),
            ["Native", "Docker", "Native", "Docker"]
        );

        let mut state = LiveState::new("r".to_string());

        accepted_with(&mut state, 0, false);

        assert!(row_for(&state, Stage::Workspaces, 0).subs.is_empty());
    }

    #[test]
    fn a_mode_the_step_never_reported_ends_as_not_requested() {
        let mut state = LiveState::new("r".to_string());

        accepted_with(&mut state, 1, false);
        feed(
            &mut state,
            2,
            100,
            "stage.started",
            json!({"stage": "workspaces"}),
        );
        workspace_phase(&mut state, 3, "native", "started", Value::Null);
        workspace_phase(&mut state, 4, "native", "finished", json!("success"));
        feed(
            &mut state,
            5,
            500,
            "stage.completed",
            json!({"stage": "workspaces", "status": "success", "duration_ms": 400}),
        );

        let row = row_for(&state, Stage::Workspaces, 0);

        assert_eq!(row.phase, Phase::Done);
        assert_eq!(labels(&row.subs), ["Native", "Docker"]);
        assert_eq!(row.subs[1].phase, Phase::NotRequested);
        assert_eq!(row.subs[1].detail.as_deref(), Some("Not requested"));
        assert!(!row.subs[1].waiting);
    }

    #[test]
    fn rows_a_stopped_run_never_reached_say_so_and_stop_waiting() {
        let mut state = LiveState::new("r".to_string());

        accepted_with(&mut state, 1, true);
        feed(
            &mut state,
            2,
            5,
            "run.failed",
            json!({"code": "X", "message": "boom", "stage": "build", "persisted": false}),
        );

        for stage in [Stage::Workspaces, Stage::Cleanup] {
            let row = row_for(&state, stage, 0);

            assert_eq!(row.subs.len(), 2);
            assert!(row.subs.iter().all(|sub| {
                sub.phase == Phase::Pending
                    && !sub.waiting
                    && sub.detail.as_deref() == Some("Not run")
            }));
        }
    }

    #[test]
    fn the_cleanup_step_keeps_a_row_for_each_command_it_is_expected_to_run() {
        let mut state = LiveState::new("r".to_string());

        accepted_with(&mut state, 1, true);
        feed(
            &mut state,
            2,
            0,
            "stage.started",
            json!({"stage": "cleanup"}),
        );

        let row = row_for(&state, Stage::Cleanup, 0);

        assert_eq!(labels(&row.subs), ["Build cleanup", "Docker cleanup"]);
        assert!(row.subs.iter().all(|sub| sub.waiting));

        cleanup_step(&mut state, 3, "build", "started", Value::Null, Value::Null);

        let row = row_for(&state, Stage::Cleanup, 0);

        assert_eq!(labels(&row.subs), ["Build cleanup", "Docker cleanup"]);
        assert_eq!(row.subs[0].phase, Phase::Running);
        assert!(!row.subs[0].waiting);
        assert!(row.subs[1].waiting);

        // Without a workspace there is no Docker cleanup to expect
        let mut state = LiveState::new("r".to_string());

        accepted_with(&mut state, 0, true);

        assert_eq!(
            labels(&row_for(&state, Stage::Cleanup, 0).subs),
            ["Build cleanup"]
        );
    }

    #[test]
    fn values_a_live_run_has_not_reported_wait_as_skeletons_and_known_text_does_not() {
        let mut state = LiveState::new("r".to_string());

        accepted(&mut state, Value::Null);

        // Not started: the time is awaited, the runtime line too
        let discovery = row_for(&state, Stage::Discovery, 0);

        assert!(discovery.waiting.time);
        assert!(discovery.waiting.detail);

        // The build has no line of its own, only a time
        let build = row_for(&state, Stage::Build, 0);

        assert!(build.waiting.time);
        assert!(!build.waiting.detail);

        // A step that was not requested never reports, so it does not wait
        let baseline = row_for(&state, Stage::Baseline, 0);

        assert_eq!(baseline.phase, Phase::NotRequested);
        assert_eq!(baseline.waiting, Waiting::default());

        // The runtime is known once measured, so it shows as text
        feed(
            &mut state,
            2,
            10,
            "measurement.available",
            json!({"metric": "discovery", "value": {"runtime": "bun"}}),
        );

        let discovery = row_for(&state, Stage::Discovery, 0);

        assert_eq!(discovery.detail.as_deref(), Some("Runtime bun"));
        assert!(!discovery.waiting.detail);

        // A running step shows its own elapsed time, not a skeleton
        feed(
            &mut state,
            3,
            20,
            "stage.started",
            json!({"stage": "discovery"}),
        );

        assert!(!row_for(&state, Stage::Discovery, 0).waiting.time);

        // Once the run ended, what never came is not awaited
        feed(
            &mut state,
            4,
            30,
            "run.failed",
            json!({"code": "X", "message": "boom", "stage": "build", "persisted": false}),
        );

        assert!(!row_for(&state, Stage::Build, 0).waiting.time);
        assert!(!row_for(&state, Stage::ArtifactSize, 0).waiting.detail);
    }

    #[test]
    fn a_step_waits_for_its_comparison_only_when_the_last_run_has_a_time() {
        assert!(waits_for_change(Phase::Pending, true, true));
        assert!(waits_for_change(Phase::Running, true, true));
        assert!(!waits_for_change(Phase::Pending, true, false));
        assert!(!waits_for_change(Phase::Done, true, true));
        assert!(!waits_for_change(Phase::Running, false, true));
    }

    // The words of a shape must be the ones the step writes, in the same order, or the real text
    // would not look like the line it replaces
    fn words_come_in_order(stage: Stage, text: &str) -> bool {
        let mut rest = text;

        detail_shape(stage).iter().all(|piece| match piece {
            Piece::Word(word) => match rest.find(word) {
                Some(at) => {
                    rest = &rest[at + word.len()..];
                    true
                }
                None => false,
            },
            Piece::Value(_) => true,
        })
    }

    #[test]
    fn the_waiting_line_keeps_the_words_a_step_writes_and_only_its_values_wait() {
        // The lines the state writes for these steps, see its tests
        assert!(words_come_in_order(Stage::Discovery, "Runtime bun"));
        assert!(words_come_in_order(
            Stage::ArtifactSize,
            "Source 1.20 MiB · build output 3.40 MiB"
        ));
        assert!(words_come_in_order(Stage::Baseline, "Idle memory 70.0 MiB"));
        assert!(!words_come_in_order(Stage::Baseline, "Runtime bun"));

        // Every step that waits for a line has a value to wait for, and starts with its words
        for stage in Stage::ALL.iter().filter(|stage| reports_detail(**stage)) {
            let shape = detail_shape(*stage);

            assert!(matches!(shape.first(), Some(Piece::Word(_))), "{stage:?}");
            assert!(shape.iter().any(|piece| matches!(piece, Piece::Value(_))));
        }
    }

    #[test]
    fn the_workspace_step_lists_the_modes_the_run_reported() {
        let mut state = LiveState::new("r".to_string());

        accepted(&mut state, Value::Null);
        feed(
            &mut state,
            2,
            100,
            "stage.started",
            json!({"stage": "workspaces"}),
        );

        assert!(row_for(&state, Stage::Workspaces, 0).subs.is_empty());

        workspace_phase(&mut state, 3, "native", "started", Value::Null);
        workspace_phase(&mut state, 4, "native", "finished", json!("success"));
        workspace_phase(&mut state, 5, "docker", "sampling", Value::Null);

        let row = row_for(&state, Stage::Workspaces, 0);

        assert_eq!(row.subs.len(), 2);
        assert_eq!(row.subs[0].label, "Native");
        assert_eq!(row.subs[0].phase, Phase::Done);
        assert_eq!(row.subs[1].label, "Docker");
        assert_eq!(row.subs[1].phase, Phase::Running);
        // The modes say where the step is, so the step has no line of its own
        assert_eq!(row.detail, None);
    }

    #[test]
    fn a_mode_that_was_running_when_the_run_failed_is_marked_failed() {
        let mut state = LiveState::new("r".to_string());

        accepted(&mut state, Value::Null);
        feed(
            &mut state,
            2,
            100,
            "stage.started",
            json!({"stage": "workspaces"}),
        );
        workspace_phase(&mut state, 3, "native", "started", Value::Null);
        feed(
            &mut state,
            4,
            400,
            "run.failed",
            json!({"stage": "workspaces", "message": "boom", "persisted": false}),
        );

        let row = row_for(&state, Stage::Workspaces, 0);

        assert_eq!(row.phase, Phase::Failed);
        assert_eq!(row.subs[0].phase, Phase::Failed);
    }
}
