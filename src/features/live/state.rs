use serde_json::Value;

use super::models::{Opened, ReplayData, StreamEvent};
use crate::{
    components::Tone,
    features::{charts::SeriesPoint, run_detail::BenchmarkResult},
    format::{format_bytes, format_number, humanize_code},
};

// Samples kept in memory, the API keeps the same number
pub const MAX_SAMPLES: usize = 1_200;

// Samples drawn in a live chart, the most recent minute at the 500 ms interval
pub const CHART_WINDOW: usize = 120;

const MAX_LOG_LINES: usize = 200;

// The steps of a run in the order they happen, the keys are the API's stage names
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Discovery,
    ArtifactSize,
    Build,
    Startup,
    Baseline,
    Workload,
    Workspaces,
    Saving,
}

impl Stage {
    pub const ALL: [Stage; 8] = [
        Stage::Discovery,
        Stage::ArtifactSize,
        Stage::Build,
        Stage::Startup,
        Stage::Baseline,
        Stage::Workload,
        Stage::Workspaces,
        Stage::Saving,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::ArtifactSize => "artifact_size",
            Self::Build => "build",
            Self::Startup => "startup",
            Self::Baseline => "baseline",
            Self::Workload => "workload",
            Self::Workspaces => "workspaces",
            Self::Saving => "saving",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Discovery => "Detect runtime",
            Self::ArtifactSize => "Measure artifact size",
            Self::Build => "Build",
            Self::Startup => "Start and wait until ready",
            Self::Baseline => "Baseline",
            Self::Workload => "Workload",
            Self::Workspaces => "Measure workspace",
            Self::Saving => "Save result",
        }
    }

    pub(super) fn from_key(key: &str) -> Option<Stage> {
        Stage::ALL.into_iter().find(|stage| stage.key() == key)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StageState {
    Pending,
    Running {
        started_t_ms: u64,
    },
    Done {
        status: String,
        duration_ms: Option<u64>,
    },
    Skipped {
        reason: String,
    },
}

// How far a workload is, only what the backend can really know
#[derive(Debug, Clone, PartialEq)]
pub enum Progress {
    Requests { done: u64, total: u64 },
    Duration { elapsed_ms: u64, total_ms: u64 },
}

impl Progress {
    // Between 0 and 1
    pub fn ratio(&self) -> f64 {
        let (done, total) = match self {
            Self::Requests { done, total } => (*done, *total),
            Self::Duration {
                elapsed_ms,
                total_ms,
            } => (*elapsed_ms, *total_ms),
        };

        if total == 0 {
            0.0
        } else {
            (done as f64 / total as f64).min(1.0)
        }
    }

    pub fn text(&self) -> String {
        match self {
            Self::Requests { done, total } => format!(
                "{} of {} requests",
                format_number(*done as f64, 0),
                format_number(*total as f64, 0)
            ),
            Self::Duration {
                elapsed_ms,
                total_ms,
            } => format!(
                "{:.1} s of {:.0} s",
                (*elapsed_ms).min(*total_ms) as f64 / 1000.0,
                *total_ms as f64 / 1000.0
            ),
        }
    }
}

// One reading of a workspace mode while it is sampled, a value is None until measured
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspacePoint {
    pub elapsed_ms: u64,
    pub rss_bytes: Option<u64>,
    pub peak_rss_bytes: Option<u64>,
    pub cpu_interval: Option<f64>,
    pub cpu_average: Option<f64>,
    pub cpu_peak: Option<f64>,
}

// The workspace mode the run is on, or was on last: where it is and what it measured so far
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceLive {
    pub workspace: String,
    // "native" or "docker"
    pub mode: String,
    // "started", "sampling" or "finished"
    pub phase: String,
    // Only known once the mode finished
    pub status: Option<String>,
    pub points: Vec<WorkspacePoint>,
}

pub fn mode_title(mode: &str) -> &'static str {
    match mode {
        "docker" => "Docker",
        _ => "Native",
    }
}

fn phase_text(phase: &str) -> &'static str {
    match phase {
        "started" => "building and starting",
        "sampling" => "sampling memory and CPU",
        _ => "finished",
    }
}

// The readings of one workspace mode for a chart, the most recent minute
pub fn workspace_series(
    live: &WorkspaceLive,
    pick: fn(&WorkspacePoint) -> Option<f64>,
) -> Vec<SeriesPoint> {
    let start = live.points.len().saturating_sub(CHART_WINDOW);

    live.points[start..]
        .iter()
        .map(|point| SeriesPoint {
            axis_label: format!("{} s", point.elapsed_ms / 1000),
            detail_label: format!(
                "{:.1} s into the sampling",
                point.elapsed_ms as f64 / 1000.0
            ),
            value: pick(point),
        })
        .collect()
}

// One live sample of the workload, a value is None until it has been measured
#[derive(Debug, Clone, PartialEq)]
pub struct SamplePoint {
    pub elapsed_ms: u64,
    pub rss_bytes: Option<u64>,
    pub peak_rss_bytes: Option<u64>,
    pub cpu_interval: Option<f64>,
    pub cpu_average: Option<f64>,
    pub cpu_peak: Option<f64>,
    pub completed: u64,
    pub failed: u64,
    pub requests_per_second: f64,
    pub progress: Progress,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Terminal {
    // The stored result, None when it could not be read
    Completed(Option<Box<BenchmarkResult>>),
    Failed {
        code: Option<String>,
        message: String,
        stage: Option<String>,
    },
    // Stopped on request, the work and its cleanup are over and nothing was stored
    Cancelled {
        stage: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub path: String,
    // None when no workload was requested
    pub workload: Option<String>,
    // The saved target's name, None for a custom run
    pub target_name: Option<String>,
    // How many workspaces the run also measures, 0 when none
    pub workspaces: u64,
}

// One line of the event panel
#[derive(Debug, Clone, PartialEq)]
pub struct LogLine {
    pub t_ms: u64,
    pub text: String,
    pub tone: Tone,
}

// What applying one event did
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    Accepted,
    // Already seen, ignored
    Duplicate,
    // Events are missing, reconnect and ask for them after the last seq
    Gap,
    // Belongs to another run and is never shown here
    OtherRun,
}

// Everything the live page shows, built only from the events of one run.
//
// Ordering follows the API's rule: a seq at or below the last one is a duplicate, and a
// gap is only legitimate while the last seq is below the highest announced eviction of
// samples. Anything else is a gap and the caller reconnects.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveState {
    run_id: String,
    plan: Option<Plan>,
    stages: Vec<StageState>,
    last_seq: u64,
    dropped_through_seq: u64,
    samples: Vec<SamplePoint>,
    workspace_live: Option<WorkspaceLive>,
    // The modes that finished before the current one, with their readings
    workspace_done: Vec<WorkspaceLive>,
    log: Vec<LogLine>,
    terminal: Option<Terminal>,
    recovered: bool,
    // The run said it can be cancelled, and whether cancellation was requested
    can_cancel: bool,
    cancelling: bool,
    discovery_detail: Option<String>,
    artifact_detail: Option<String>,
    baseline_detail: Option<String>,
    // The server time of the latest event and the local clock when it arrived
    anchor_t_ms: u64,
    anchor_clock_ms: u64,
}

impl LiveState {
    pub fn new(run_id: String) -> Self {
        Self {
            run_id,
            plan: None,
            stages: vec![StageState::Pending; Stage::ALL.len()],
            last_seq: 0,
            dropped_through_seq: 0,
            samples: Vec::new(),
            workspace_live: None,
            workspace_done: Vec::new(),
            log: Vec::new(),
            terminal: None,
            recovered: false,
            can_cancel: false,
            cancelling: false,
            discovery_detail: None,
            artifact_detail: None,
            baseline_detail: None,
            anchor_t_ms: 0,
            anchor_clock_ms: 0,
        }
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn plan(&self) -> Option<&Plan> {
        self.plan.as_ref()
    }

    pub fn last_seq(&self) -> u64 {
        self.last_seq
    }

    pub fn terminal(&self) -> Option<&Terminal> {
        self.terminal.as_ref()
    }

    pub fn is_terminal(&self) -> bool {
        self.terminal.is_some()
    }

    // The result came from storage, the live record was gone
    pub fn is_recovered(&self) -> bool {
        self.recovered
    }

    pub fn samples(&self) -> &[SamplePoint] {
        &self.samples
    }

    // Every workspace mode the run has reported, in the order they ran
    pub fn workspace_modes(&self) -> Vec<WorkspaceLive> {
        self.workspace_done
            .iter()
            .chain(self.workspace_live.iter())
            .cloned()
            .collect()
    }

    pub fn latest_sample(&self) -> Option<&SamplePoint> {
        self.samples.last()
    }

    pub fn log(&self) -> &[LogLine] {
        &self.log
    }

    pub fn stage(&self, stage: Stage) -> &StageState {
        &self.stages[stage as usize]
    }

    // The stage a failed run stopped in
    pub fn failed_stage(&self) -> Option<Stage> {
        match &self.terminal {
            Some(Terminal::Failed {
                stage: Some(key), ..
            }) => Stage::from_key(key),
            _ => None,
        }
    }

    // Whether the run offers cancellation, known from its first event
    pub fn can_cancel(&self) -> bool {
        self.can_cancel
    }

    // Cancellation was requested and the run has not ended yet
    pub fn is_cancelling(&self) -> bool {
        self.cancelling && !self.is_terminal()
    }

    // The stage a cancelled run stopped in
    pub fn cancelled_stage(&self) -> Option<Stage> {
        match &self.terminal {
            Some(Terminal::Cancelled {
                stage: Some(key), ..
            }) => Stage::from_key(key),
            _ => None,
        }
    }

    pub fn has_events(&self) -> bool {
        self.last_seq > 0
    }

    // Text shown under a stage, only what the run has really reported
    pub fn stage_detail(&self, stage: Stage) -> Option<String> {
        match stage {
            Stage::Discovery => self.discovery_detail.clone(),
            Stage::ArtifactSize => self.artifact_detail.clone(),
            Stage::Baseline => self.baseline_detail.clone(),
            Stage::Workload => self.plan.as_ref().and_then(|plan| plan.workload.clone()),
            Stage::Workspaces => {
                matches!(self.stage(stage), StageState::Running { .. }).then(|| {
                    match &self.workspace_live {
                        Some(live) => format!(
                            "{} · {} · {}",
                            live.workspace,
                            mode_title(&live.mode),
                            phase_text(&live.phase)
                        ),
                        None => "Preparing the first workspace".to_string(),
                    }
                })
            }
            _ => None,
        }
    }

    // Whether the request includes workspaces, known from the first event
    pub fn workspaces_requested(&self) -> Option<bool> {
        self.plan.as_ref().map(|plan| plan.workspaces > 0)
    }

    // Whether the request includes a workload, known from the first event
    pub fn workload_requested(&self) -> Option<bool> {
        self.plan.as_ref().map(|plan| plan.workload.is_some())
    }

    // Time since the run was accepted. Server times anchor it, the local clock only fills the
    // time between events, so a skewed device clock cannot show a wrong duration.
    pub fn run_elapsed_ms(&self, clock_ms: u64) -> u64 {
        if self.is_terminal() {
            return self.anchor_t_ms;
        }

        self.anchor_t_ms + clock_ms.saturating_sub(self.anchor_clock_ms)
    }

    pub fn stage_elapsed_ms(&self, stage: Stage, clock_ms: u64) -> Option<u64> {
        match self.stage(stage) {
            StageState::Running { started_t_ms } => {
                Some(self.run_elapsed_ms(clock_ms).saturating_sub(*started_t_ms))
            }
            _ => None,
        }
    }

    pub fn apply_opened(&mut self, opened: &Opened) -> bool {
        if opened.run_id != self.run_id {
            return false;
        }

        self.dropped_through_seq = self.dropped_through_seq.max(opened.dropped_through_seq);

        true
    }

    pub fn apply_dropped(&mut self, through_seq: u64) {
        self.dropped_through_seq = self.dropped_through_seq.max(through_seq);
    }

    // The events of a polled replay, in order
    pub fn apply_replay(&mut self, replay: ReplayData, clock_ms: u64) {
        self.apply_dropped(replay.dropped_through_seq);

        for event in replay.events {
            if self.apply_event(event, clock_ms) == Applied::Gap {
                break;
            }
        }
    }

    // A run that is stored but whose live record is gone
    pub fn finish_from_stored(&mut self, result: BenchmarkResult) {
        self.terminal = Some(Terminal::Completed(Some(Box::new(result))));
        self.recovered = true;
    }

    pub fn apply_event(&mut self, event: StreamEvent, clock_ms: u64) -> Applied {
        if event.run_id != self.run_id {
            return Applied::OtherRun;
        }

        if event.seq <= self.last_seq {
            return Applied::Duplicate;
        }

        if event.seq != self.last_seq + 1 && self.last_seq >= self.dropped_through_seq {
            return Applied::Gap;
        }

        self.last_seq = event.seq;
        self.anchor_t_ms = event.t_ms;
        self.anchor_clock_ms = clock_ms;

        let data = &event.data;

        match event.kind.as_str() {
            "run.accepted" => {
                self.plan = parse_plan(data);
                self.can_cancel = data
                    .pointer("/capabilities/cancel")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
            }
            "run.cancelling" => self.cancelling = true,
            "run.cancelled" => {
                self.terminal = Some(Terminal::Cancelled {
                    stage: text(data, "stage"),
                });
            }
            "stage.started" => {
                if let Some(stage) = stage_of(data) {
                    self.stages[stage as usize] = StageState::Running {
                        started_t_ms: event.t_ms,
                    };
                }
            }
            "stage.completed" => {
                if let (Some(stage), Some(status)) = (stage_of(data), text(data, "status")) {
                    self.stages[stage as usize] = StageState::Done {
                        status,
                        duration_ms: unsigned(data, "duration_ms"),
                    };
                }
            }
            "stage.skipped" => {
                if let (Some(stage), Some(reason)) = (stage_of(data), text(data, "reason")) {
                    self.stages[stage as usize] = StageState::Skipped { reason };
                }
            }
            "workspace.phase" => {
                if let (Some(workspace), Some(mode), Some(phase)) = (
                    text(data, "workspace"),
                    text(data, "mode"),
                    text(data, "phase"),
                ) {
                    // A new mode starts with an empty chart, a finished one keeps its readings
                    let points = match self.workspace_live.take() {
                        Some(live)
                            if phase != "started"
                                && live.mode == mode
                                && live.workspace == workspace =>
                        {
                            live.points
                        }
                        Some(live) => {
                            self.workspace_done.push(live);

                            Vec::new()
                        }
                        None => Vec::new(),
                    };

                    self.workspace_live = Some(WorkspaceLive {
                        workspace,
                        mode,
                        phase,
                        status: text(data, "status"),
                        points,
                    });
                }
            }
            "workspace.sample" => {
                if let (Some(workspace), Some(mode)) = (text(data, "workspace"), text(data, "mode"))
                {
                    let memory = data.get("memory").filter(|value| !value.is_null());
                    let cpu = data.get("cpu").filter(|value| !value.is_null());

                    let point = WorkspacePoint {
                        elapsed_ms: unsigned(data, "elapsed_ms").unwrap_or(0),
                        rss_bytes: memory.and_then(|memory| unsigned(memory, "rss_bytes")),
                        peak_rss_bytes: memory
                            .and_then(|memory| unsigned(memory, "peak_rss_bytes")),
                        cpu_interval: cpu.and_then(|cpu| float(cpu, "interval_percent")),
                        cpu_average: cpu.and_then(|cpu| float(cpu, "average_percent")),
                        cpu_peak: cpu.and_then(|cpu| float(cpu, "peak_percent")),
                    };

                    // Samples may arrive without their phase event when it was missed
                    let live = self.workspace_live.get_or_insert_with(|| WorkspaceLive {
                        workspace: workspace.clone(),
                        mode: mode.clone(),
                        phase: "sampling".to_string(),
                        status: None,
                        points: Vec::new(),
                    });

                    if live.mode == mode && live.workspace == workspace {
                        live.points.push(point);

                        if live.points.len() > MAX_SAMPLES {
                            live.points.remove(0);
                        }
                    }
                }
            }
            "measurement.available" => self.apply_measurement(data),
            "baseline.measured" => {
                self.baseline_detail = unsigned(data, "memory_rss_bytes")
                    .map(|bytes| format!("Idle memory {}", format_bytes(bytes as f64)));
            }
            "sample" => {
                if let Some(sample) = parse_sample(data) {
                    self.samples.push(sample);

                    if self.samples.len() > MAX_SAMPLES {
                        self.samples.remove(0);
                    }
                }
            }
            "run.completed" => {
                let result = data
                    .get("result")
                    .and_then(|value| serde_json::from_value::<BenchmarkResult>(value.clone()).ok())
                    .map(Box::new);

                self.terminal = Some(Terminal::Completed(result));
            }
            "run.failed" => {
                self.terminal = Some(Terminal::Failed {
                    code: text(data, "code"),
                    message: text(data, "message").unwrap_or_else(|| "The run failed.".to_string()),
                    stage: text(data, "stage"),
                });
            }
            // A kind this version does not know is skipped, its seq still counts
            _ => {}
        }

        if let Some((line, tone)) = describe(&event.kind, data) {
            self.log.push(LogLine {
                t_ms: event.t_ms,
                text: line,
                tone,
            });

            if self.log.len() > MAX_LOG_LINES {
                self.log.remove(0);
            }
        }

        Applied::Accepted
    }

    fn apply_measurement(&mut self, data: &Value) {
        let Some(value) = data.get("value") else {
            return;
        };

        match data.get("metric").and_then(Value::as_str) {
            Some("discovery") => {
                self.discovery_detail = Some(match text(value, "runtime") {
                    Some(runtime) => format!("Runtime {runtime}"),
                    None => "No runtime detected".to_string(),
                });
            }
            Some("artifact_size") => {
                let part = |key: &str| value.get(key).and_then(|category| text(category, "size"));

                if let (Some(source), Some(build)) = (part("source"), part("build")) {
                    self.artifact_detail = Some(format!("Source {source} · build output {build}"));
                }
            }
            _ => {}
        }
    }

    // The most recent samples of a metric for a live chart, None leaves a gap
    pub fn chart_series(&self, pick: fn(&SamplePoint) -> Option<f64>) -> Vec<SeriesPoint> {
        let start = self.samples.len().saturating_sub(CHART_WINDOW);

        self.samples[start..]
            .iter()
            .map(|sample| SeriesPoint {
                axis_label: format!("{} s", sample.elapsed_ms / 1000),
                detail_label: format!(
                    "{:.1} s into the workload",
                    sample.elapsed_ms as f64 / 1000.0
                ),
                value: pick(sample),
            })
            .collect()
    }

    // Whether the charts show only the most recent part of the samples
    pub fn chart_is_windowed(&self) -> bool {
        self.samples.len() > CHART_WINDOW
    }
}

fn stage_of(data: &Value) -> Option<Stage> {
    data.get("stage")
        .and_then(Value::as_str)
        .and_then(Stage::from_key)
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn unsigned(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn float(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn parse_plan(data: &Value) -> Option<Plan> {
    let path = data.get("target").and_then(|target| text(target, "path"))?;

    let workload = data
        .get("plan")
        .and_then(|plan| plan.get("workload"))
        .filter(|workload| !workload.is_null())
        .map(|workload| {
            let concurrency = unsigned(workload, "concurrency").unwrap_or(1);

            let amount = match workload.get("mode").and_then(Value::as_str) {
                Some("requests") => format!(
                    "{} requests",
                    format_number(unsigned(workload, "requests").unwrap_or(0) as f64, 0)
                ),
                Some("duration") => {
                    format!("{} s", unsigned(workload, "duration_seconds").unwrap_or(0))
                }
                _ => "Workload".to_string(),
            };

            format!("{amount} · {concurrency} concurrent")
        });

    let target_name = data.get("target").and_then(|target| text(target, "name"));

    Some(Plan {
        path,
        workload,
        target_name,
        workspaces: data
            .get("plan")
            .and_then(|plan| unsigned(plan, "workspaces"))
            .unwrap_or(0),
    })
}

fn parse_sample(data: &Value) -> Option<SamplePoint> {
    let http = data.get("http")?;
    let progress = http.get("progress")?;

    let progress = match progress.get("mode").and_then(Value::as_str)? {
        "requests" => Progress::Requests {
            done: unsigned(progress, "done")?,
            total: unsigned(progress, "total")?,
        },
        "duration" => Progress::Duration {
            elapsed_ms: unsigned(progress, "elapsed_ms")?,
            total_ms: unsigned(progress, "total_ms")?,
        },
        _ => return None,
    };

    let memory = data.get("memory").filter(|value| !value.is_null());
    let cpu = data.get("cpu").filter(|value| !value.is_null());

    Some(SamplePoint {
        elapsed_ms: unsigned(data, "workload_elapsed_ms")?,
        rss_bytes: memory.and_then(|memory| unsigned(memory, "rss_bytes")),
        peak_rss_bytes: memory.and_then(|memory| unsigned(memory, "peak_rss_bytes")),
        cpu_interval: cpu.and_then(|cpu| float(cpu, "interval_percent")),
        cpu_average: cpu.and_then(|cpu| float(cpu, "average_percent")),
        cpu_peak: cpu.and_then(|cpu| float(cpu, "peak_percent")),
        completed: unsigned(http, "completed")?,
        failed: unsigned(http, "failed")?,
        requests_per_second: float(http, "requests_per_second")?,
        progress,
    })
}

fn status_tone(status: &str) -> Tone {
    match status {
        "success" | "completed" => Tone::Success,
        "failed" | "timeout" | "unhealthy" | "target_exited" => Tone::Danger,
        _ => Tone::Neutral,
    }
}

// The line an event adds to the event panel, samples and raw measurements add none
fn describe(kind: &str, data: &Value) -> Option<(String, Tone)> {
    let label = || stage_of(data).map(Stage::label);

    match kind {
        "run.accepted" => Some(("Run accepted".to_string(), Tone::Neutral)),
        "stage.started" => Some((format!("{} started", label()?), Tone::Neutral)),
        "stage.completed" => {
            let status = text(data, "status")?;

            Some((
                format!("{} finished · {}", label()?, humanize_code(&status)),
                status_tone(&status),
            ))
        }
        "stage.skipped" => Some((
            format!(
                "{} skipped · {}",
                label()?,
                humanize_code(&text(data, "reason")?)
            ),
            Tone::Neutral,
        )),
        "baseline.measured" => Some(("Baseline measured".to_string(), Tone::Neutral)),
        "workspace.phase" => {
            let workspace = text(data, "workspace")?;
            let mode = mode_title(&text(data, "mode")?);
            let phase = text(data, "phase")?;

            match (phase.as_str(), text(data, "status")) {
                ("finished", Some(status)) => Some((
                    format!("{workspace} · {mode} finished · {}", humanize_code(&status)),
                    status_tone(&status),
                )),
                (phase, _) => Some((
                    format!("{workspace} · {mode} {}", phase_text(phase)),
                    Tone::Neutral,
                )),
            }
        }
        "run.cancelling" => Some((
            match text(data, "stage").as_deref().and_then(Stage::from_key) {
                Some(stage) => format!("Cancellation requested, stopping {}", stage.label()),
                None => "Cancellation requested".to_string(),
            },
            Tone::Neutral,
        )),
        "run.cancelled" => Some((
            "Run cancelled, nothing was stored".to_string(),
            Tone::Neutral,
        )),
        "run.completed" => Some(("Run completed and stored".to_string(), Tone::Success)),
        "run.failed" => Some((
            format!(
                "Run failed · {}",
                text(data, "message").unwrap_or_else(|| "unknown error".to_string())
            ),
            Tone::Danger,
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const RUN: &str = "run-1";

    fn event(seq: u64, t_ms: u64, kind: &str, data: Value) -> StreamEvent {
        StreamEvent {
            run_id: RUN.to_string(),
            seq,
            t_ms,
            kind: kind.to_string(),
            data,
        }
    }

    fn state() -> LiveState {
        LiveState::new(RUN.to_string())
    }

    fn sample_data(elapsed_ms: u64, completed: u64) -> Value {
        json!({
            "workload_elapsed_ms": elapsed_ms,
            "memory": { "rss_bytes": 80_000_000, "peak_rss_bytes": 82_000_000 },
            "cpu": { "interval_percent": 41.2, "average_percent": 38.9, "peak_percent": 55.0 },
            "http": {
                "completed": completed, "successful": completed, "failed": 0,
                "requests_per_second": 206.0,
                "progress": { "mode": "requests", "done": completed, "total": 1000 }
            }
        })
    }

    fn accepted(workload: Value) -> StreamEvent {
        event(
            1,
            0,
            "run.accepted",
            json!({
                "target": { "path": "/home/me/projects/api" },
                "plan": { "startup": true, "workload": workload },
                "capabilities": { "cancel": false }
            }),
        )
    }

    // The workspace mode the run is on now, the last one reported
    fn current(state: &LiveState) -> WorkspaceLive {
        state.workspace_modes().pop().unwrap()
    }

    fn workspace_sample(seq: u64, mode: &str, elapsed_ms: u64) -> StreamEvent {
        event(
            seq,
            elapsed_ms,
            "workspace.sample",
            json!({
                "workspace": "subscriber", "mode": mode, "elapsed_ms": elapsed_ms,
                "memory": { "rss_bytes": 86_000_000, "peak_rss_bytes": 87_000_000 },
                "cpu": { "interval_percent": 1.5, "average_percent": 0.4, "peak_percent": 9.8 }
            }),
        )
    }

    #[test]
    fn workspace_readings_follow_the_current_mode_and_a_new_mode_starts_empty() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);
        state.apply_event(
            event(
                2,
                10,
                "workspace.phase",
                json!({"workspace": "subscriber", "mode": "native", "phase": "started", "status": null}),
            ),
            0,
        );
        state.apply_event(workspace_sample(3, "native", 500), 0);
        state.apply_event(workspace_sample(4, "native", 1000), 0);

        let live = current(&state);

        assert_eq!(live.mode, "native");
        assert_eq!(live.points.len(), 2);
        assert_eq!(live.points[1].rss_bytes, Some(86_000_000));
        assert_eq!(
            workspace_series(&current(&state), |point| point.cpu_interval).len(),
            2
        );

        state.apply_event(
            event(
                5,
                1100,
                "workspace.phase",
                json!({"workspace": "subscriber", "mode": "native", "phase": "finished", "status": "success"}),
            ),
            0,
        );

        // A finished mode keeps its readings on screen
        assert_eq!(current(&state).points.len(), 2);
        assert_eq!(current(&state).status.as_deref(), Some("success"));

        state.apply_event(
            event(
                6,
                1200,
                "workspace.phase",
                json!({"workspace": "subscriber", "mode": "docker", "phase": "started", "status": null}),
            ),
            0,
        );

        assert_eq!(current(&state).mode, "docker");
        assert!(current(&state).points.is_empty());
        // The native readings are kept for the charts of the finished run
        assert_eq!(state.workspace_modes().len(), 2);
        assert_eq!(state.workspace_modes()[0].points.len(), 2);
        assert_eq!(
            state.log().last().unwrap().text,
            "subscriber · Docker building and starting"
        );
    }

    #[test]
    fn events_apply_in_order_and_move_the_last_seq() {
        let mut state = state();

        assert_eq!(
            state.apply_event(accepted(Value::Null), 0),
            Applied::Accepted
        );
        assert_eq!(
            state.apply_event(
                event(2, 5, "stage.started", json!({"stage": "discovery"})),
                10
            ),
            Applied::Accepted
        );
        assert_eq!(state.last_seq(), 2);
        assert!(state.has_events());
    }

    #[test]
    fn a_duplicate_is_ignored() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);

        assert_eq!(
            state.apply_event(accepted(Value::Null), 0),
            Applied::Duplicate
        );
        assert_eq!(state.last_seq(), 1);
    }

    #[test]
    fn a_gap_is_reported_and_changes_nothing() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);

        let outcome = state.apply_event(event(5, 9, "stage.started", json!({"stage": "build"})), 0);

        assert_eq!(outcome, Applied::Gap);
        assert_eq!(state.last_seq(), 1);
        assert_eq!(*state.stage(Stage::Build), StageState::Pending);
    }

    #[test]
    fn a_gap_inside_the_announced_eviction_is_accepted() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);
        state.apply_dropped(9);

        assert_eq!(
            state.apply_event(event(10, 50, "sample", sample_data(500, 5)), 0),
            Applied::Accepted
        );
        assert_eq!(
            state.apply_event(event(11, 60, "sample", sample_data(1000, 9)), 0),
            Applied::Accepted
        );
        assert_eq!(
            state.apply_event(event(13, 70, "sample", sample_data(1500, 12)), 0),
            Applied::Gap
        );
    }

    #[test]
    fn the_opening_frame_announces_evicted_samples() {
        let mut state = state();

        assert!(state.apply_opened(&Opened {
            run_id: RUN.to_string(),
            dropped_through_seq: 20
        }));

        assert_eq!(
            state.apply_event(event(30, 1, "stage.started", json!({"stage": "build"})), 0),
            Applied::Accepted
        );
    }

    #[test]
    fn an_opening_frame_of_another_run_is_refused() {
        let mut state = state();

        assert!(!state.apply_opened(&Opened {
            run_id: "other".to_string(),
            dropped_through_seq: 0
        }));
    }

    #[test]
    fn an_event_of_another_run_is_never_applied() {
        let mut state = state();
        let mut foreign = accepted(Value::Null);

        foreign.run_id = "other".to_string();

        assert_eq!(state.apply_event(foreign, 0), Applied::OtherRun);
        assert!(!state.has_events());
        assert!(state.plan().is_none());
    }

    #[test]
    fn the_plan_describes_the_workload_only_when_one_was_requested() {
        let mut state = state();

        state.apply_event(
            accepted(json!({ "mode": "duration", "duration_seconds": 30, "concurrency": 50 })),
            0,
        );

        let plan = state.plan().unwrap();

        assert_eq!(plan.path, "/home/me/projects/api");
        assert_eq!(plan.workload.as_deref(), Some("30 s · 50 concurrent"));
        assert_eq!(state.workload_requested(), Some(true));

        let mut without = self::state();

        without.apply_event(accepted(Value::Null), 0);

        assert_eq!(without.plan().unwrap().workload, None);
        assert_eq!(without.workload_requested(), Some(false));
        assert_eq!(without.stage_detail(Stage::Workload), None);
    }

    #[test]
    fn a_request_workload_is_described_with_a_grouped_count() {
        let mut state = state();

        state.apply_event(
            accepted(json!({ "mode": "requests", "requests": 20000, "concurrency": 8 })),
            0,
        );

        assert_eq!(
            state.stage_detail(Stage::Workload).as_deref(),
            Some("20,000 requests · 8 concurrent")
        );
    }

    #[test]
    fn stages_move_through_their_states() {
        let mut state = state();

        assert_eq!(*state.stage(Stage::Build), StageState::Pending);

        state.apply_event(accepted(Value::Null), 0);
        state.apply_event(event(2, 100, "stage.started", json!({"stage": "build"})), 0);

        assert_eq!(
            *state.stage(Stage::Build),
            StageState::Running { started_t_ms: 100 }
        );

        state.apply_event(
            event(
                3,
                2_400,
                "stage.completed",
                json!({"stage": "build", "status": "success", "duration_ms": 2300}),
            ),
            0,
        );

        assert_eq!(
            *state.stage(Stage::Build),
            StageState::Done {
                status: "success".to_string(),
                duration_ms: Some(2300)
            }
        );

        state.apply_event(
            event(
                4,
                2_500,
                "stage.skipped",
                json!({"stage": "workload", "reason": "no_workload"}),
            ),
            0,
        );

        assert_eq!(
            *state.stage(Stage::Workload),
            StageState::Skipped {
                reason: "no_workload".to_string()
            }
        );
    }

    #[test]
    fn a_stage_without_a_duration_has_none() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);
        state.apply_event(
            event(
                2,
                10,
                "stage.completed",
                json!({"stage": "discovery", "status": "success", "duration_ms": null}),
            ),
            0,
        );

        assert_eq!(
            *state.stage(Stage::Discovery),
            StageState::Done {
                status: "success".to_string(),
                duration_ms: None
            }
        );
    }

    #[test]
    fn measurements_add_details_to_their_stages() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);
        state.apply_event(
            event(
                2,
                10,
                "measurement.available",
                json!({"metric": "discovery", "value": {"runtime": "bun", "build": "bun run build"}}),
            ),
            0,
        );
        state.apply_event(
            event(
                3,
                20,
                "measurement.available",
                json!({"metric": "artifact_size", "value": {
                    "source": {"bytes": 1, "size": "1.20 MiB", "file_count": 3},
                    "build": {"bytes": 2, "size": "3.40 MiB", "file_count": 4},
                    "excluded": {"bytes": 0, "size": "0 B", "file_count": 0}}}),
            ),
            0,
        );
        state.apply_event(
            event(
                4,
                30,
                "baseline.measured",
                json!({"memory_rss_bytes": 73_400_320u64, "cpu_percent": 0.0}),
            ),
            0,
        );

        assert_eq!(
            state.stage_detail(Stage::Discovery).as_deref(),
            Some("Runtime bun")
        );
        assert_eq!(
            state.stage_detail(Stage::ArtifactSize).as_deref(),
            Some("Source 1.20 MiB · build output 3.40 MiB")
        );
        assert_eq!(
            state.stage_detail(Stage::Baseline).as_deref(),
            Some("Idle memory 70.0 MiB")
        );
    }

    #[test]
    fn no_runtime_is_said_plainly() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);
        state.apply_event(
            event(
                2,
                10,
                "measurement.available",
                json!({"metric": "discovery", "value": {"runtime": null, "build": null}}),
            ),
            0,
        );

        assert_eq!(
            state.stage_detail(Stage::Discovery).as_deref(),
            Some("No runtime detected")
        );
    }

    #[test]
    fn samples_are_parsed_with_their_missing_values_kept_missing() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);
        state.apply_event(
            event(
                2,
                600,
                "sample",
                json!({
                    "workload_elapsed_ms": 500,
                    "memory": null,
                    "cpu": { "interval_percent": null, "average_percent": 12.5, "peak_percent": null },
                    "http": {
                        "completed": 10, "successful": 9, "failed": 1, "requests_per_second": 20.0,
                        "progress": { "mode": "duration", "elapsed_ms": 500, "total_ms": 30000 }
                    }
                }),
            ),
            0,
        );

        let sample = state.latest_sample().unwrap();

        assert_eq!(sample.rss_bytes, None);
        assert_eq!(sample.cpu_interval, None);
        assert_eq!(sample.cpu_average, Some(12.5));
        assert_eq!(sample.cpu_peak, None);
        assert_eq!(sample.failed, 1);
        assert_eq!(
            sample.progress,
            Progress::Duration {
                elapsed_ms: 500,
                total_ms: 30000
            }
        );
    }

    #[test]
    fn a_sample_that_breaks_the_contract_is_skipped_but_its_seq_counts() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);

        assert_eq!(
            state.apply_event(event(2, 600, "sample", json!({"nonsense": true})), 0),
            Applied::Accepted
        );
        assert!(state.latest_sample().is_none());
        assert_eq!(state.last_seq(), 2);
    }

    #[test]
    fn the_samples_kept_are_bounded() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);

        for index in 0..(MAX_SAMPLES as u64 + 50) {
            state.apply_event(
                event(index + 2, index, "sample", sample_data(index * 500, index)),
                0,
            );
        }

        assert_eq!(state.samples().len(), MAX_SAMPLES);
        assert_eq!(state.samples()[0].elapsed_ms, 50 * 500);
    }

    #[test]
    fn charts_show_the_most_recent_window_and_keep_gaps() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);

        for index in 0..(CHART_WINDOW as u64 + 10) {
            state.apply_event(
                event(index + 2, index, "sample", sample_data(index * 500, index)),
                0,
            );
        }

        let series = state.chart_series(|sample| sample.cpu_interval);

        assert_eq!(series.len(), CHART_WINDOW);
        assert_eq!(series[0].axis_label, "5 s");
        assert_eq!(series[0].value, Some(41.2));
        assert!(state.chart_is_windowed());

        let mut empty_cpu = self::state();

        empty_cpu.apply_event(accepted(Value::Null), 0);
        empty_cpu.apply_event(
            event(
                2,
                1,
                "sample",
                json!({
                    "workload_elapsed_ms": 500, "memory": null, "cpu": null,
                    "http": { "completed": 0, "successful": 0, "failed": 0,
                              "requests_per_second": 0.0,
                              "progress": { "mode": "requests", "done": 0, "total": 10 } }
                }),
            ),
            0,
        );

        assert_eq!(
            empty_cpu.chart_series(|sample| sample.cpu_interval)[0].value,
            None
        );
        assert!(!empty_cpu.chart_is_windowed());
    }

    #[test]
    fn progress_is_real_and_never_above_one() {
        let requests = Progress::Requests {
            done: 412,
            total: 1000,
        };

        assert!((requests.ratio() - 0.412).abs() < 1e-9);
        assert_eq!(requests.text(), "412 of 1,000 requests");

        let late = Progress::Duration {
            elapsed_ms: 31_000,
            total_ms: 30_000,
        };

        assert_eq!(late.ratio(), 1.0);
        assert_eq!(late.text(), "30.0 s of 30 s");

        assert_eq!(Progress::Requests { done: 1, total: 0 }.ratio(), 0.0);
    }

    #[test]
    fn a_completed_run_keeps_its_stored_result() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);
        state.apply_event(
            event(
                2,
                9_000,
                "run.completed",
                json!({
                    "persisted": true,
                    "result": {
                        "run_id": RUN,
                        "target": { "path": "/p" },
                        "discovery": { "runtime": "bun", "build": null },
                        "metrics": {
                            "artifact_size": {
                                "source": { "bytes": 1, "size": "1 B", "file_count": 1 },
                                "build": { "bytes": 0, "size": "0 B", "file_count": 0 },
                                "excluded": { "bytes": 0, "size": "0 B", "file_count": 0 }
                            },
                            "build_time": { "status": "no_build_command" },
                            "startup_time": { "status": "no_start_command" }
                        }
                    }
                }),
            ),
            0,
        );

        match state.terminal() {
            Some(Terminal::Completed(Some(result))) => assert_eq!(result.run_id, RUN),
            other => panic!("expected a completed run with a result, got {other:?}"),
        }

        assert!(state.is_terminal());
        assert!(!state.is_recovered());
    }

    #[test]
    fn a_completed_run_whose_result_cannot_be_read_is_still_completed() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);
        state.apply_event(
            event(
                2,
                1,
                "run.completed",
                json!({"persisted": true, "result": 7}),
            ),
            0,
        );

        assert_eq!(state.terminal(), Some(&Terminal::Completed(None)));
    }

    #[test]
    fn a_failed_run_names_its_stage() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);
        state.apply_event(
            event(
                2,
                9,
                "run.failed",
                json!({"code": "INTERNAL_SERVER_ERROR", "message": "boom", "stage": "build", "persisted": false}),
            ),
            0,
        );

        assert_eq!(
            state.terminal(),
            Some(&Terminal::Failed {
                code: Some("INTERNAL_SERVER_ERROR".to_string()),
                message: "boom".to_string(),
                stage: Some("build".to_string())
            })
        );
        assert_eq!(state.failed_stage(), Some(Stage::Build));
    }

    #[test]
    fn a_failure_without_a_stage_marks_no_stage() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);
        state.apply_event(
            event(
                2,
                9,
                "run.failed",
                json!({"message": "boom", "stage": null}),
            ),
            0,
        );

        assert_eq!(state.failed_stage(), None);
    }

    #[test]
    fn elapsed_time_uses_server_times_and_only_the_clock_between_events() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 5_000);
        state.apply_event(
            event(2, 1_000, "stage.started", json!({"stage": "build"})),
            6_000,
        );

        // Two seconds of local time after the last event
        assert_eq!(state.run_elapsed_ms(8_000), 3_000);
        assert_eq!(state.stage_elapsed_ms(Stage::Build, 8_000), Some(2_000));
        assert_eq!(state.stage_elapsed_ms(Stage::Startup, 8_000), None);

        state.apply_event(
            event(3, 4_000, "run.failed", json!({"message": "x"})),
            9_000,
        );

        // A finished run stops counting
        assert_eq!(state.run_elapsed_ms(60_000), 4_000);
    }

    #[test]
    fn the_event_log_lists_steps_and_not_samples() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);
        state.apply_event(event(2, 1, "stage.started", json!({"stage": "build"})), 0);
        state.apply_event(event(3, 2, "sample", sample_data(500, 1)), 0);
        state.apply_event(
            event(
                4,
                3,
                "stage.completed",
                json!({"stage": "build", "status": "failed", "duration_ms": 5}),
            ),
            0,
        );
        state.apply_event(
            event(
                5,
                4,
                "stage.skipped",
                json!({"stage": "workload", "reason": "startup_not_successful"}),
            ),
            0,
        );

        let lines: Vec<&str> = state.log().iter().map(|line| line.text.as_str()).collect();

        assert_eq!(
            lines,
            vec![
                "Run accepted",
                "Build started",
                "Build finished · Failed",
                "Workload skipped · Startup not successful",
            ]
        );
        assert_eq!(state.log()[2].tone, Tone::Danger);
    }

    #[test]
    fn an_unknown_event_kind_only_advances_the_seq() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);

        assert_eq!(
            state.apply_event(event(2, 1, "something.new", json!({})), 0),
            Applied::Accepted
        );
        assert_eq!(state.last_seq(), 2);
        assert_eq!(state.log().len(), 1);
    }

    #[test]
    fn a_polled_replay_applies_its_events_and_its_eviction_announcement() {
        let mut state = state();

        state.apply_replay(
            ReplayData {
                dropped_through_seq: 9,
                events: vec![
                    accepted(Value::Null),
                    event(10, 50, "sample", sample_data(500, 5)),
                ],
            },
            0,
        );

        assert_eq!(state.last_seq(), 10);
        assert_eq!(state.samples().len(), 1);
    }

    #[test]
    fn a_stored_run_recovers_a_run_whose_live_record_is_gone() {
        let mut state = state();

        let result: BenchmarkResult = serde_json::from_value(json!({
            "run_id": RUN,
            "target": { "path": "/p" },
            "metrics": {
                "artifact_size": {
                    "source": { "bytes": 1, "size": "1 B", "file_count": 1 },
                    "build": { "bytes": 0, "size": "0 B", "file_count": 0 },
                    "excluded": { "bytes": 0, "size": "0 B", "file_count": 0 }
                },
                "build_time": { "status": "success" },
                "startup_time": { "status": "success" }
            }
        }))
        .unwrap();

        state.finish_from_stored(result);

        assert!(state.is_terminal());
        assert!(state.is_recovered());
    }

    #[test]
    fn a_run_says_in_its_first_event_whether_it_can_be_cancelled() {
        let mut yes = state();

        yes.apply_event(
            event(
                1,
                0,
                "run.accepted",
                json!({"target": {"path": "/p"}, "capabilities": {"cancel": true}}),
            ),
            0,
        );

        let mut no = self::state();

        no.apply_event(
            event(
                1,
                0,
                "run.accepted",
                json!({"target": {"path": "/p"}, "capabilities": {"cancel": false}}),
            ),
            0,
        );

        let mut unknown = self::state();

        unknown.apply_event(
            event(1, 0, "run.accepted", json!({"target": {"path": "/p"}})),
            0,
        );

        assert!(yes.can_cancel());
        assert!(!no.can_cancel());
        assert!(!unknown.can_cancel());
        assert!(!self::state().can_cancel());
    }

    #[test]
    fn a_cancel_request_marks_the_run_as_cancelling_until_it_ends() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);

        assert!(!state.is_cancelling());

        state.apply_event(event(2, 5, "stage.started", json!({"stage": "build"})), 0);
        state.apply_event(event(3, 6, "run.cancelling", json!({"stage": "build"})), 0);

        assert!(state.is_cancelling());
        assert!(!state.is_terminal());

        state.apply_event(
            event(
                4,
                7,
                "run.cancelled",
                json!({"stage": "build", "persisted": false}),
            ),
            0,
        );

        assert!(!state.is_cancelling());
        assert!(state.is_terminal());
    }

    #[test]
    fn a_cancelled_run_names_its_stage_and_is_not_a_failure() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);
        state.apply_event(
            event(
                2,
                9,
                "run.cancelled",
                json!({"stage": "workload", "persisted": false}),
            ),
            0,
        );

        assert_eq!(
            state.terminal(),
            Some(&Terminal::Cancelled {
                stage: Some("workload".to_string())
            })
        );
        assert_eq!(state.cancelled_stage(), Some(Stage::Workload));
        assert_eq!(state.failed_stage(), None);
    }

    #[test]
    fn a_run_cancelled_before_any_step_has_no_stage() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);
        state.apply_event(
            event(
                2,
                9,
                "run.cancelled",
                json!({"stage": null, "persisted": false}),
            ),
            0,
        );

        assert_eq!(state.cancelled_stage(), None);
        assert!(state.is_terminal());
    }

    #[test]
    fn a_cancelled_run_stops_counting_time() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 1_000);
        state.apply_event(
            event(2, 4_000, "run.cancelled", json!({"stage": "build"})),
            2_000,
        );

        assert_eq!(state.run_elapsed_ms(90_000), 4_000);
    }

    #[test]
    fn the_event_log_tells_the_cancellation_in_plain_words() {
        let mut state = state();

        state.apply_event(accepted(Value::Null), 0);
        state.apply_event(event(2, 1, "run.cancelling", json!({"stage": "build"})), 0);
        state.apply_event(event(3, 2, "run.cancelled", json!({"stage": "build"})), 0);

        let lines: Vec<&str> = state.log().iter().map(|line| line.text.as_str()).collect();

        assert_eq!(
            lines,
            vec![
                "Run accepted",
                "Cancellation requested, stopping Build",
                "Run cancelled, nothing was stored",
            ]
        );
    }
}
