use super::models::BenchmarkRequest;
use crate::features::{
    connection::{post_json, ApiError},
    live::LiveRunAccepted,
};

// POST /api/v1/live-runs, answers at once with the run id while the benchmark runs on
pub async fn submit_live(request: &BenchmarkRequest) -> Result<LiveRunAccepted, ApiError> {
    post_json("/live-runs", request).await
}
