use serde::{Deserialize, Serialize};

// One entry of GET /api/v1/benchmarks/configurations, newest run first. Fields that are null
// for runs stored before configurations were recorded are Option.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ConfigurationEntry {
    pub run_id: String,
    pub created_at: String,
    pub path: String,
    pub startup: Option<StartupSummary>,
    pub recorded: bool,
    pub configuration: Option<RecordedConfiguration>,
}

// The start command and readiness of a run. Every part can be absent.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StartupSummary {
    pub program: Option<String>,
    pub args: Option<Vec<String>>,
    pub readiness: Option<ReadinessSummary>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ReadinessSummary {
    pub readiness_type: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub method: String,
    pub expected_status: u16,
    // Null when the header names were not recorded
    pub header_names: Option<Vec<String>>,
}

// The configuration of a run that recorded one
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RecordedConfiguration {
    // "custom" or "target"
    pub source: String,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub target_path: String,
    pub startup: Option<StartupSummary>,
    pub workload: Option<WorkloadSummary>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct WorkloadSummary {
    pub method: String,
    pub path: String,
    // "requests" or "duration"
    pub mode: String,
    pub requests: Option<u64>,
    pub duration_seconds: Option<u64>,
    pub concurrency: u64,
    pub header_names: Vec<String>,
}

// GET /api/v1/benchmarks/{run_id}/config
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ConfigurationDetail {
    pub run_id: String,
    pub recorded: bool,
    pub configuration: Option<RecordedConfiguration>,
}
