use std::collections::HashMap;

use serde::Deserialize;

use super::api::RUNTIMES;

// Response models of GET /api/v1/benchmarks, see docs/api.md in benchmark-api

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RunTarget {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RunDiscovery {
    // Null when no runtime was detected
    #[serde(default)]
    pub runtime: Option<String>,
}

// One workspace and mode of a run, only the sizes the Runs table can show
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct WorkspaceSummary {
    // "native" or "docker"
    pub mode: String,
    pub image_bytes: Option<u64>,
    pub disk_usage_bytes: Option<u64>,
}

// Every metric is null when the run did not produce it
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct RunMetrics {
    pub build_output_bytes: Option<u64>,
    pub workspaces: Vec<WorkspaceSummary>,
    pub build_time_ms: Option<u64>,
    pub startup_time_ms: Option<u64>,
    pub memory_peak_bytes: Option<u64>,
    pub cpu_average_percent: Option<f64>,
    pub requests_per_second: Option<f64>,
    // The requests the workload sent
    pub total_requests: Option<u64>,
    pub p95_ms: Option<f64>,
}

// One row of the history list, also the shape of `latest` in the overview
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RunSummary {
    pub run_id: String,
    // How the run ended: done, failed or cancelled
    pub status: String,
    // The saved target name, otherwise the last folder of the target path
    pub display_name: String,
    pub target: RunTarget,
    pub discovery: RunDiscovery,
    pub created_at: String,
    pub summary: RunMetrics,
}

impl RunSummary {
    // Runtime code such as `bun`, `None` when the run had no detected runtime
    pub fn runtime_code(&self) -> Option<&str> {
        self.discovery.runtime.as_deref()
    }

    // Readable run name, for example `shop-api, Rust`
    pub fn title(&self) -> String {
        match self.runtime_code() {
            Some(code) => format!("{}, {}", self.display_name, runtime_label(code)),
            None => self.display_name.clone(),
        }
    }
}

// Display name of a runtime lookup code, unknown codes are shown as sent
pub fn runtime_label(code: &str) -> String {
    RUNTIMES
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(code))
        .map_or_else(|| code.to_string(), |(_, label)| (*label).to_string())
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Pagination {
    pub limit: u32,
    pub has_more: bool,
    // Opaque cursor, sent back unchanged as `before`, null on the last page
    #[serde(default)]
    pub next_cursor: Option<String>,
}

// Run counts for the filter pills and the footer. Each facet ignores its own filter, so a pill
// shows what picking it would give.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct Counts {
    pub total: u64,
    pub by_runtime: HashMap<String, u64>,
    pub by_status: HashMap<String, u64>,
}

impl Counts {
    pub fn runtime(&self, code: &str) -> u64 {
        self.by_runtime.get(code).copied().unwrap_or(0)
    }

    pub fn status(&self, status: &str) -> u64 {
        self.by_status.get(status).copied().unwrap_or(0)
    }

    // Runs when no runtime is chosen
    pub fn any_runtime(&self) -> u64 {
        self.by_runtime.values().sum()
    }

    // Runs when no status is chosen
    pub fn any_status(&self) -> u64 {
        self.by_status.values().sum()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HistoryPage {
    pub items: Vec<RunSummary>,
    #[serde(default)]
    pub counts: Counts,
    pub pagination: Pagination,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_documented_history_page() {
        let body = r#"{
            "items": [{
                "run_id": "0192f3a0-0000-7000-8000-000000000000",
                "status": "done",
                "display_name": "my-api",
                "target": { "path": "/home/me/projects/my-api" },
                "discovery": { "runtime": "bun" },
                "created_at": "2026-10-08T05:11:22.123Z",
                "summary": {
                    "build_time_ms": 1842,
                    "startup_time_ms": 1500,
                    "memory_peak_bytes": 73400320,
                    "cpu_average_percent": 12.4,
                    "requests_per_second": 121.45,
                    "p95_ms": 8.7
                }
            }],
            "pagination": { "limit": 20, "has_more": true, "next_cursor": "MTc5" }
        }"#;

        let page: HistoryPage = serde_json::from_str(body).expect("parses");

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].summary.build_time_ms, Some(1842));
        assert_eq!(page.pagination.next_cursor.as_deref(), Some("MTc5"));
    }

    #[test]
    fn null_metrics_stay_missing() {
        let body = r#"{
            "items": [{
                "run_id": "a",
                "status": "failed",
                "display_name": "p",
                "target": { "path": "/p" },
                "discovery": { "runtime": null },
                "created_at": "2026-10-08T05:11:22.123Z",
                "summary": {
                    "build_time_ms": null,
                    "startup_time_ms": null,
                    "memory_peak_bytes": null,
                    "cpu_average_percent": null,
                    "requests_per_second": null,
                    "p95_ms": null
                }
            }],
            "pagination": { "limit": 20, "has_more": false, "next_cursor": null }
        }"#;

        let page: HistoryPage = serde_json::from_str(body).expect("parses");

        assert_eq!(page.items[0].discovery.runtime, None);
        assert_eq!(page.items[0].summary, RunMetrics::default());
        assert_eq!(page.pagination.next_cursor, None);
    }

    #[test]
    fn names_a_run_after_its_target_and_runtime() {
        let body = r#"{
            "run_id": "a", "status": "done", "display_name": "shop-api",
            "target": { "path": "/home/me/shop-api" },
            "discovery": { "runtime": "rust" },
            "created_at": "2026-10-08T05:11:22.123Z",
            "summary": {}
        }"#;

        let mut run: RunSummary = serde_json::from_str(body).expect("parses");

        assert_eq!(run.title(), "shop-api, Rust");

        run.discovery.runtime = None;

        assert_eq!(run.title(), "shop-api");
        assert_eq!(runtime_label("node"), "Node.js");
        assert_eq!(runtime_label("zig"), "zig");
    }

    #[test]
    fn reads_the_counts_of_the_filter_pills() {
        let body = r#"{
            "items": [],
            "counts": {
                "total": 12,
                "by_runtime": { "bun": 7, "rust": 4, "unknown": 1 },
                "by_status": { "done": 9, "failed": 2, "cancelled": 1 }
            },
            "pagination": { "limit": 20, "has_more": false, "next_cursor": null }
        }"#;

        let page: HistoryPage = serde_json::from_str(body).expect("parses");

        assert_eq!(page.counts.total, 12);
        assert_eq!(page.counts.runtime("bun"), 7);
        assert_eq!(page.counts.runtime("go"), 0);
        assert_eq!(page.counts.any_runtime(), 12);
        assert_eq!(page.counts.status("failed"), 2);
        assert_eq!(page.counts.any_status(), 12);
    }

    #[test]
    fn reads_sizes_and_workspaces_of_the_summary() {
        let body = r#"{
            "run_id": "a", "status": "done", "display_name": "p",
            "target": { "path": "/p" }, "discovery": { "runtime": "bun" },
            "created_at": "2026-10-08T05:11:22.123Z",
            "summary": {
                "build_output_bytes": 2400000,
                "workspaces": [{
                    "name": "subscriber", "mode": "docker", "status": "success",
                    "image_bytes": 98304000, "disk_usage_bytes": 276000000, "build_duration_ms": 412000
                }]
            }
        }"#;

        let run: RunSummary = serde_json::from_str(body).expect("parses");

        assert_eq!(run.summary.build_output_bytes, Some(2_400_000));
        assert_eq!(run.summary.workspaces[0].mode, "docker");
        assert_eq!(run.summary.workspaces[0].image_bytes, Some(98_304_000));
    }
}
