use super::models::TargetsView;
use crate::features::connection::{get_json, ApiError};

// GET /api/v1/targets
pub async fn fetch_targets() -> Result<TargetsView, ApiError> {
    get_json("/targets", &[]).await
}
