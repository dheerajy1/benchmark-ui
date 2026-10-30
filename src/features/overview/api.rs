use super::models::OverviewData;
use crate::features::connection::{get_json, ApiError};

// GET /api/v1/benchmarks/overview, the endpoint takes no query parameters
pub async fn fetch_overview() -> Result<OverviewData, ApiError> {
    get_json("/benchmarks/overview", &[]).await
}
