use dioxus::prelude::*;
use serde::Deserialize;

use crate::{
    features::connection::{get_json, ApiError},
    format::format_number,
};

// Newest progress of the workload of the active run
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum WorkloadProgress {
    Requests { done: u64, total: u64 },
    Duration { elapsed_ms: u64, total_ms: u64 },
}

// `data` of GET /api/v1/live-runs/active, null when nothing runs
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ActiveRun {
    pub run_id: String,
    // Null for the instant the run is not registered yet
    #[serde(default)]
    pub elapsed_ms: Option<u64>,
    // Null until the workload has sampled
    #[serde(default)]
    pub workload_progress: Option<WorkloadProgress>,
}

// The run that executes now, updated by `refresh_active` and read by the top bar and the sidebar
pub static ACTIVE_RUN: GlobalSignal<Option<ActiveRun>> = Signal::global(|| None);

// Asks the API for the active run. A failed request keeps the last known value, the
// connection state is already marked by the HTTP client.
pub async fn refresh_active() {
    let result: Result<Option<ActiveRun>, ApiError> = get_json("/live-runs/active", &[]).await;

    if let Ok(run) = result {
        if *ACTIVE_RUN.peek() != run {
            *ACTIVE_RUN.write() = run;
        }
    }
}

// Text of the `Run in progress` chip, for example `12 s of 30 s`
pub fn progress_text(run: &ActiveRun) -> String {
    match &run.workload_progress {
        Some(WorkloadProgress::Duration {
            elapsed_ms,
            total_ms,
        }) => format!("{} s of {} s", elapsed_ms / 1_000, total_ms / 1_000),
        Some(WorkloadProgress::Requests { done, total }) => format!(
            "{} of {} requests",
            format_number(*done as f64, 0),
            format_number(*total as f64, 0)
        ),
        None => run
            .elapsed_ms
            .map_or_else(String::new, |elapsed| format!("{} s", elapsed / 1_000)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(body: &str) -> ActiveRun {
        serde_json::from_str(body).expect("parses")
    }

    #[test]
    fn shows_duration_progress_in_seconds() {
        let active = run(r#"{"run_id":"a","stage":"workload","elapsed_ms":18400,
                "workload_progress":{"mode":"duration","elapsed_ms":12000,"total_ms":30000}}"#);

        assert_eq!(progress_text(&active), "12 s of 30 s");
    }

    #[test]
    fn shows_request_progress() {
        let active = run(r#"{"run_id":"a","elapsed_ms":5000,
                "workload_progress":{"mode":"requests","done":420,"total":10000}}"#);

        assert_eq!(progress_text(&active), "420 of 10,000 requests");
    }

    #[test]
    fn falls_back_to_elapsed_time_before_the_workload() {
        let active =
            run(r#"{"run_id":"a","stage":"build","elapsed_ms":7400,"workload_progress":null}"#);

        assert_eq!(progress_text(&active), "7 s");
        assert_eq!(
            progress_text(&run(r#"{"run_id":"a","elapsed_ms":null}"#)),
            ""
        );
    }
}
