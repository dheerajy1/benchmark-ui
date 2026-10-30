use std::collections::BTreeMap;

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
    // Left out when empty, so the API probes with its own default health credentials
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
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
    // Sent with every request of the workload, left out when empty
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
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

// Request body of POST /api/v1/live-runs for a saved target. The target supplies the path,
// the start command, the readiness and the headers. Only the workload can be changed.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TargetRunRequest {
    pub target_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workload: Option<TargetWorkloadOverride>,
}

// Exactly one of `requests` and `duration_seconds` is set
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TargetWorkloadOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requests: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u64>,
    pub concurrency: u64,
}
