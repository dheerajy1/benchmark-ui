use serde::Deserialize;

use crate::features::history::RunSummary;

// Response model of GET /api/v1/benchmarks/overview, see docs/api.md in benchmark-api

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Totals {
    pub total_runs: u64,
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
    #[serde(default)]
    pub p95_ms: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OverviewData {
    pub totals: Totals,
    // Null when no run is stored
    #[serde(default)]
    pub latest: Option<RunSummary>,
    // The newest runs, newest first
    #[serde(default)]
    pub trends: Vec<TrendPoint>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_documented_overview() {
        let body = r#"{
            "totals": { "total_runs": 128 },
            "latest": {
                "run_id": "0192f3a0-0000-7000-8000-000000000000",
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
        assert_eq!(overview.trends[0].build_time_ms, None);
        assert_eq!(overview.trends[0].startup_time_ms, Some(1500));
    }

    #[test]
    fn parses_an_empty_history() {
        let body = r#"{ "totals": { "total_runs": 0 }, "latest": null, "trends": [] }"#;

        let overview: OverviewData = serde_json::from_str(body).expect("parses");

        assert_eq!(overview.totals.total_runs, 0);
        assert!(overview.latest.is_none());
        assert!(overview.trends.is_empty());
    }
}
