use serde::Serialize;

// Request body of POST /api/v1/benchmarks, see docs/api.md in benchmark-api

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CommandRequest {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReadinessRequest {
    #[serde(rename = "type")]
    pub readiness_type: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub method: String,
    pub expected_status: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StartupRequest {
    pub command: CommandRequest,
    pub readiness: ReadinessRequest,
}

// Exactly one of `requests` and `duration_seconds` is set
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkloadRequest {
    pub method: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requests: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u64>,
    pub concurrency: u64,
}

// Only `path` is required by the API
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BenchmarkRequest {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup: Option<StartupRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workload: Option<WorkloadRequest>,
}
