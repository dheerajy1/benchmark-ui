use super::models::BenchmarkResult;
use crate::features::connection::{get_json, ApiError};

// GET /api/v1/benchmarks/{run_id}, a malformed id is rejected by the API with 400, an unknown one with 404
pub async fn fetch_run(run_id: &str) -> Result<BenchmarkResult, ApiError> {
    get_json(&format!("/benchmarks/{}", run_id.trim()), &[]).await
}
