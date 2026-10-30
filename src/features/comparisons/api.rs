use super::models::ComparisonData;
use crate::features::connection::{get_json, ApiError};

// GET /api/v1/comparisons/summary?runIds=<baseline>,<comparison>, the first id is always the baseline
pub async fn fetch_comparison(
    baseline: &str,
    comparison: &str,
) -> Result<ComparisonData, ApiError> {
    let run_ids = format!("{},{}", baseline.trim(), comparison.trim());

    get_json("/comparisons/summary", &[("runIds", run_ids)]).await
}
