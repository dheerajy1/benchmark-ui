use super::models::BenchmarkRequest;
use crate::features::{
    connection::{post_json, ApiError},
    run_detail::BenchmarkResult,
};

// POST /api/v1/benchmarks, blocks until the benchmark finished and is stored
pub async fn submit_benchmark(request: &BenchmarkRequest) -> Result<BenchmarkResult, ApiError> {
    post_json("/benchmarks", request).await
}
