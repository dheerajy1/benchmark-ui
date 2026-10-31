use serde::Deserialize;

// GET /api/v1/targets. Header values are never returned, only their names.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetsView {
    pub custom_runs_allowed: bool,
    pub targets: Vec<TargetView>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetView {
    pub id: String,
    pub name: String,
    pub path: String,
    pub startup: Option<TargetStartup>,
    pub workload: Option<TargetWorkload>,
    // Workspaces the target also measures, absent for a target without one
    #[serde(default)]
    pub workspaces: Vec<TargetWorkspace>,
}

// What the page shows of a workspace. Environment values never arrive, only names.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetWorkspace {
    pub name: String,
    pub directory: String,
    #[serde(default)]
    pub env_variable_names: Vec<String>,
    #[serde(default)]
    pub native: Option<serde_json::Value>,
    #[serde(default)]
    pub docker: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetStartup {
    pub command: TargetCommand,
    pub readiness: TargetReadiness,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetCommand {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetReadiness {
    pub host: String,
    pub port: u16,
    pub path: String,
    pub method: String,
    pub expected_status: u16,
    // Null when the server's health credentials are used
    pub header_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetWorkload {
    pub method: String,
    pub path: String,
    // "requests" or "duration"
    pub mode: String,
    pub requests: Option<u64>,
    pub duration_seconds: Option<u64>,
    pub concurrency: u64,
    pub header_names: Vec<String>,
}
