use serde::Deserialize;

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

// Every metric is null when the run did not produce it
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct RunMetrics {
    pub build_time_ms: Option<u64>,
    pub startup_time_ms: Option<u64>,
    pub memory_peak_bytes: Option<u64>,
    pub cpu_average_percent: Option<f64>,
    pub requests_per_second: Option<f64>,
    pub p95_ms: Option<f64>,
}

// One row of the history list, also the shape of `latest` in the overview
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RunSummary {
    pub run_id: String,
    pub target: RunTarget,
    pub discovery: RunDiscovery,
    pub created_at: String,
    pub summary: RunMetrics,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Pagination {
    pub limit: u32,
    pub has_more: bool,
    // Opaque cursor, sent back unchanged as `before`, null on the last page
    #[serde(default)]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HistoryPage {
    pub items: Vec<RunSummary>,
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
}
