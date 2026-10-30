use super::models::{ConfigurationDetail, ConfigurationEntry};
use crate::features::connection::{get_json, ApiError};

// Every run in one call, newest first. The backend accepts 1 to 200.
const FETCH_LIMIT: u32 = 200;

// GET /api/v1/benchmarks/configurations?limit=
pub async fn fetch_configurations() -> Result<Vec<ConfigurationEntry>, ApiError> {
    get_json(
        "/benchmarks/configurations",
        &[("limit", FETCH_LIMIT.to_string())],
    )
    .await
}

// GET /api/v1/benchmarks/{run_id}/config, the settings one run executed with
pub async fn fetch_run_configuration(run_id: &str) -> Result<ConfigurationDetail, ApiError> {
    get_json(&format!("/benchmarks/{}/config", run_id.trim()), &[]).await
}
