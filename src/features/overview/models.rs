use serde::Deserialize;

use crate::features::history::RunSummary;

// Response model of GET /api/v1/benchmarks/overview, see docs/api.md in benchmark-api

// How the runs of the shown project ended
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Totals {
    pub total_runs: u64,
    #[serde(default)]
    pub failed_runs: u64,
    #[serde(default)]
    pub cancelled_runs: u64,
}

// The project shown, a project is one stored target path
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Project {
    pub path: String,
    pub name: String,
    // Runtime code such as `bun`, null while the project has no run
    #[serde(default)]
    pub runtime: Option<String>,
}

// One row of the project switcher
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ProjectEntry {
    pub path: String,
    pub name: String,
    #[serde(default)]
    pub runtime: Option<String>,
    pub total_runs: u64,
    pub last_run_at: String,
}

// One run of the trend, a metric is null when it was not measured, never zero
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TrendPoint {
    pub run_id: String,
    pub created_at: String,
    #[serde(default)]
    pub build_time_ms: Option<u64>,
    #[serde(default)]
    pub startup_time_ms: Option<u64>,
    #[serde(default)]
    pub memory_peak_bytes: Option<u64>,
    #[serde(default)]
    pub cpu_average_percent: Option<f64>,
    #[serde(default)]
    pub requests_per_second: Option<f64>,
    // The requests the workload sent, null under the same rule as requests_per_second
    #[serde(default)]
    pub total_requests: Option<u64>,
    #[serde(default)]
    pub p95_ms: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OverviewData {
    // Null only when no run is stored at all
    #[serde(default)]
    pub project: Option<Project>,
    // Every project that has a run, newest activity first
    #[serde(default)]
    pub projects: Vec<ProjectEntry>,
    pub totals: Totals,
    // The newest done run of the project, null when it has none
    #[serde(default)]
    pub latest: Option<RunSummary>,
    // The done run before `latest`
    #[serde(default)]
    pub previous: Option<RunSummary>,
    // The newest runs, newest first
    #[serde(default)]
    pub trends: Vec<TrendPoint>,
}

// What the page shows for a response
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverviewState {
    // No run is stored at all, there is no switcher
    NoRuns,
    // The project in the address has no run
    UnknownProject,
    // The project has runs but none finished
    NoFinishedRuns,
    Ready,
}

impl OverviewData {
    pub fn state(&self) -> OverviewState {
        if self.project.is_none() || self.projects.is_empty() {
            OverviewState::NoRuns
        } else if self.totals.total_runs == 0 {
            OverviewState::UnknownProject
        } else if self.latest.is_none() {
            OverviewState::NoFinishedRuns
        } else {
            OverviewState::Ready
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_documented_overview() {
        let body = r#"{
            "project": { "path": "/home/me/projects/my-api", "name": "my-api", "runtime": "bun" },
            "projects": [{
                "path": "/home/me/projects/my-api", "name": "my-api", "runtime": "bun",
                "total_runs": 128, "last_run_at": "2026-10-08T05:11:22.123Z"
            }],
            "totals": { "total_runs": 128, "failed_runs": 2, "cancelled_runs": 1 },
            "latest": {
                "run_id": "0192f3a0-0000-7000-8000-000000000000",
                "status": "done",
                "display_name": "my-api",
                "target": { "path": "/home/me/projects/my-api" },
                "discovery": { "runtime": "bun" },
                "created_at": "2026-10-08T05:11:22.123Z",
                "summary": {
                    "build_time_ms": 1842, "startup_time_ms": 1500,
                    "memory_peak_bytes": 73400320, "cpu_average_percent": 12.4,
                    "requests_per_second": 121.45, "p95_ms": 8.7
                }
            },
            "trends": [{
                "run_id": "0192f3a0-0000-7000-8000-000000000000",
                "created_at": "2026-10-08T05:11:22.123Z",
                "build_time_ms": null, "startup_time_ms": 1500,
                "memory_peak_bytes": 73400320, "cpu_average_percent": 12.4,
                "requests_per_second": 121.45, "p95_ms": 8.7
            }]
        }"#;

        let overview: OverviewData = serde_json::from_str(body).expect("parses");

        assert_eq!(overview.totals.total_runs, 128);
        assert!(overview.latest.is_some());
        assert_eq!(overview.totals.failed_runs, 2);
        assert_eq!(overview.projects[0].total_runs, 128);
        assert_eq!(overview.state(), OverviewState::Ready);
        assert_eq!(overview.trends[0].build_time_ms, None);
        assert_eq!(overview.trends[0].startup_time_ms, Some(1500));
    }

    #[test]
    fn parses_an_empty_history() {
        let body = r#"{ "project": null, "projects": [], "totals": { "total_runs": 0 }, "latest": null, "previous": null, "trends": [] }"#;

        let overview: OverviewData = serde_json::from_str(body).expect("parses");

        assert_eq!(overview.totals.total_runs, 0);
        assert!(overview.latest.is_none());
        assert!(overview.trends.is_empty());
        assert_eq!(overview.state(), OverviewState::NoRuns);
    }

    fn parse(body: &str) -> OverviewData {
        serde_json::from_str(body).expect("parses")
    }

    #[test]
    fn a_path_without_runs_is_an_unknown_project() {
        let overview = parse(
            r#"{
                "project": { "path": "/gone", "name": "gone", "runtime": null },
                "projects": [{ "path": "/a", "name": "a", "total_runs": 1, "last_run_at": "2026-10-08T05:11:22Z" }],
                "totals": { "total_runs": 0 }, "latest": null, "trends": []
            }"#,
        );

        assert_eq!(overview.state(), OverviewState::UnknownProject);
    }

    #[test]
    fn a_project_without_a_done_run_has_no_finished_runs() {
        let overview = parse(
            r#"{
                "project": { "path": "/a", "name": "a", "runtime": "rust" },
                "projects": [{ "path": "/a", "name": "a", "runtime": "rust", "total_runs": 2, "last_run_at": "2026-10-08T05:11:22Z" }],
                "totals": { "total_runs": 2, "failed_runs": 1, "cancelled_runs": 1 },
                "latest": null, "trends": []
            }"#,
        );

        assert_eq!(overview.state(), OverviewState::NoFinishedRuns);
        assert_eq!(overview.totals.cancelled_runs, 1);
    }
}
