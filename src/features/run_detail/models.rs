use serde::Deserialize;

// Response model of GET /api/v1/benchmarks/{run_id} and of the POST that creates a run

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Target {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct Discovery {
    pub runtime: Option<String>,
    pub build: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ArtifactCategory {
    pub bytes: u64,
    pub size: String,
    pub file_count: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ArtifactSize {
    pub source: ArtifactCategory,
    pub build: ArtifactCategory,
    pub excluded: ArtifactCategory,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Command {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Readiness {
    #[serde(rename = "type")]
    pub readiness_type: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub method: String,
    pub expected_status: u16,
}

// Build and startup share a shape, `ready_at` and `readiness` only exist for startup
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StepResult {
    pub status: String,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub command: Option<Command>,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub readiness: Option<Readiness>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub finished_at: Option<String>,
    #[serde(default)]
    pub ready_at: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MemoryReading {
    pub rss_bytes: u64,
    pub rss_size: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Memory {
    pub baseline: MemoryReading,
    pub peak: MemoryReading,
    pub delta: MemoryReading,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Cpu {
    pub baseline_percent: f64,
    pub average_percent: f64,
    pub peak_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HttpLatency {
    // "completed" or "target_exited"
    pub status: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub duration_ms: u64,
    pub requests_per_second: f64,
    #[serde(default)]
    pub min_ms: Option<f64>,
    #[serde(default)]
    pub mean_ms: Option<f64>,
    #[serde(default)]
    pub p50_ms: Option<f64>,
    #[serde(default)]
    pub p95_ms: Option<f64>,
    #[serde(default)]
    pub p99_ms: Option<f64>,
    #[serde(default)]
    pub max_ms: Option<f64>,
}

// One mode (native or docker) of one workspace, see metrics.workspaces in docs/api.md of
// benchmark-api. A mode that could not be measured keeps its status and null numbers.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WorkspaceResult {
    pub name: String,
    // "native" or "docker"
    pub mode: String,
    pub status: String,
    #[serde(default)]
    pub message: Option<String>,
    // Names only, the API never returns a value
    #[serde(default)]
    pub env_variable_names: Vec<String>,
    #[serde(default)]
    pub build: Option<WorkspaceBuild>,
    #[serde(default)]
    pub image: Option<ImageSize>,
    #[serde(default)]
    pub startup: Option<WorkspaceStartup>,
    #[serde(default)]
    pub memory: Option<Memory>,
    #[serde(default)]
    pub cpu: Option<Cpu>,
    #[serde(default)]
    pub load: Option<WorkspaceLoad>,
    #[serde(default)]
    pub idle_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WorkspaceBuild {
    pub status: String,
    pub duration_ms: u64,
    #[serde(default)]
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ImageSize {
    pub bytes: u64,
    pub size: String,
    // Unpacked size on disk, absent when Docker did not report it
    #[serde(default)]
    pub disk_usage: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WorkspaceStartup {
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WorkspaceLoad {
    pub status: String,
    #[serde(default)]
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Metrics {
    pub artifact_size: ArtifactSize,
    pub build_time: StepResult,
    pub startup_time: StepResult,
    #[serde(default)]
    pub memory: Option<Memory>,
    #[serde(default)]
    pub cpu: Option<Cpu>,
    #[serde(default)]
    pub http_latency: Option<HttpLatency>,
    // Absent when the run had no workspace
    #[serde(default)]
    pub workspaces: Vec<WorkspaceResult>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BenchmarkResult {
    pub run_id: String,
    pub target: Target,
    #[serde(default)]
    pub discovery: Discovery,
    pub metrics: Metrics,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_full_result() {
        let body = r#"{
            "run_id": "0192f3a0-0000-7000-8000-000000000000",
            "target": { "path": "/p" },
            "discovery": { "runtime": "bun", "build": null },
            "metrics": {
                "artifact_size": {
                    "source": { "bytes": 10, "size": "10 B", "file_count": 1 },
                    "build": { "bytes": 0, "size": "0 B", "file_count": 0 },
                    "excluded": { "bytes": 5, "size": "5 B", "file_count": 2 }
                },
                "build_time": { "status": "no_build_command", "duration_ms": null, "command": null,
                    "working_directory": null, "started_at": null, "finished_at": null, "exit_code": null },
                "startup_time": { "status": "success", "duration_ms": 1500,
                    "command": { "program": "bun", "args": ["run", "start"] },
                    "working_directory": "/p",
                    "readiness": { "type": "http", "host": "127.0.0.1", "port": 5001,
                        "path": "/health", "method": "GET", "expected_status": 200 },
                    "started_at": "2026-10-08T05:11:20.000Z", "ready_at": "2026-10-08T05:11:21.500Z",
                    "exit_code": null },
                "memory": {
                    "baseline": { "rss_bytes": 1, "rss_size": "1 B" },
                    "peak": { "rss_bytes": 3, "rss_size": "3 B" },
                    "delta": { "rss_bytes": 2, "rss_size": "2 B" }
                },
                "cpu": { "baseline_percent": 1.0, "average_percent": 2.0, "peak_percent": 3.0 },
                "http_latency": { "status": "completed", "total_requests": 10, "successful_requests": 10,
                    "failed_requests": 0, "duration_ms": 100, "requests_per_second": 100.0,
                    "min_ms": 1.0, "mean_ms": 2.0, "p50_ms": 2.0, "p95_ms": 3.0, "p99_ms": 4.0, "max_ms": 5.0 }
            }
        }"#;

        let result: BenchmarkResult = serde_json::from_str(body).expect("parses");

        assert_eq!(result.discovery.runtime.as_deref(), Some("bun"));
        assert_eq!(result.metrics.build_time.duration_ms, None);
        assert_eq!(result.metrics.startup_time.duration_ms, Some(1500));
        assert!(result.metrics.http_latency.is_some());
    }

    #[test]
    fn parses_a_run_without_workload() {
        let body = r#"{
            "run_id": "a", "target": { "path": "/p" }, "discovery": { "runtime": null, "build": null },
            "metrics": {
                "artifact_size": {
                    "source": { "bytes": 0, "size": "0 B", "file_count": 0 },
                    "build": { "bytes": 0, "size": "0 B", "file_count": 0 },
                    "excluded": { "bytes": 0, "size": "0 B", "file_count": 0 }
                },
                "build_time": { "status": "no_build_command" },
                "startup_time": { "status": "no_start_command" },
                "memory": null, "cpu": null, "http_latency": null
            }
        }"#;

        let result: BenchmarkResult = serde_json::from_str(body).expect("parses");

        assert!(result.metrics.memory.is_none());
        assert!(result.metrics.cpu.is_none());
        assert!(result.metrics.http_latency.is_none());
    }

    #[test]
    fn parses_workspace_results_and_their_absence() {
        let body = r#"{
            "run_id": "a", "target": { "path": "/p" },
            "metrics": {
                "artifact_size": {
                    "source": { "bytes": 0, "size": "0 B", "file_count": 0 },
                    "build": { "bytes": 0, "size": "0 B", "file_count": 0 },
                    "excluded": { "bytes": 0, "size": "0 B", "file_count": 0 }
                },
                "build_time": { "status": "no_build_command" },
                "startup_time": { "status": "no_start_command" },
                "workspaces": [
                    { "name": "subscriber", "mode": "docker", "status": "success",
                      "env_variable_names": ["A"],
                      "build": { "status": "success", "duration_ms": 5, "exit_code": 0 },
                      "image": { "bytes": 10, "size": "10 B" },
                      "startup": { "duration_ms": 7 },
                      "memory": {
                        "baseline": { "rss_bytes": 1, "rss_size": "1 B" },
                        "peak": { "rss_bytes": 3, "rss_size": "3 B" },
                        "delta": { "rss_bytes": 2, "rss_size": "2 B" } },
                      "cpu": { "baseline_percent": 1.0, "average_percent": 2.0, "peak_percent": 3.0 },
                      "load": { "status": "success", "exit_code": 0, "duration_ms": 9 },
                      "idle_seconds": null },
                    { "name": "subscriber", "mode": "native", "status": "timeout",
                      "message": "The ready line was not seen within 60 seconds" }
                ]
            }
        }"#;

        let result: BenchmarkResult = serde_json::from_str(body).expect("parses");

        assert_eq!(result.metrics.workspaces.len(), 2);
        assert_eq!(
            result.metrics.workspaces[0].image.as_ref().unwrap().bytes,
            10
        );
        assert!(result.metrics.workspaces[1].memory.is_none());

        let plain = body.replace("\"workspaces\"", "\"ignored\"");
        let result: BenchmarkResult = serde_json::from_str(&plain).expect("parses");

        assert!(result.metrics.workspaces.is_empty());
    }
}
