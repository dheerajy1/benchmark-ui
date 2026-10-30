use super::models::ReplayData;
use crate::features::{
    connection::{get_json, ApiError},
    run_detail::BenchmarkResult,
};

// GET /api/v1/live-runs/{run_id}?after=, the events after a sequence number
pub async fn fetch_replay(run_id: &str, after: u64) -> Result<ReplayData, ApiError> {
    get_json(
        &format!("/live-runs/{}", run_id.trim()),
        &[("after", after.to_string())],
    )
    .await
}

// GET /api/v1/benchmarks/{run_id}, answers for a run that is stored even when its live
// record has expired
pub async fn fetch_stored_run(run_id: &str) -> Result<BenchmarkResult, ApiError> {
    get_json(&format!("/benchmarks/{}", run_id.trim()), &[]).await
}

// A 409 of POST /api/v1/live-runs names the active live run in its detail, a legacy run has
// no id and gets a plain sentence
pub fn active_run_id(error: &ApiError) -> Option<String> {
    match error {
        ApiError::Http {
            status: 409,
            detail: Some(detail),
            ..
        } if looks_like_run_id(detail) => Some(detail.clone()),
        _ => None,
    }
}

fn looks_like_run_id(text: &str) -> bool {
    text.len() == 36
        && text.char_indices().all(|(index, character)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                character == '-'
            } else {
                character.is_ascii_hexdigit()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conflict(detail: Option<&str>) -> ApiError {
        ApiError::Http {
            status: 409,
            code: Some("CONFLICT".to_string()),
            message: "Another benchmark is already running.".to_string(),
            detail: detail.map(str::to_string),
        }
    }

    #[test]
    fn reads_the_active_run_id_from_a_conflict() {
        let id = "0192f3a0-7b1e-7c2d-9a4b-5e6f708192a3";

        assert_eq!(active_run_id(&conflict(Some(id))), Some(id.to_string()));
    }

    #[test]
    fn a_plain_conflict_has_no_active_run_id() {
        assert_eq!(
            active_run_id(&conflict(Some("A benchmark is already running."))),
            None
        );
        assert_eq!(active_run_id(&conflict(None)), None);
    }

    #[test]
    fn only_a_conflict_carries_an_active_run_id() {
        let error = ApiError::Http {
            status: 400,
            code: None,
            message: "x".to_string(),
            detail: Some("0192f3a0-7b1e-7c2d-9a4b-5e6f708192a3".to_string()),
        };

        assert_eq!(active_run_id(&error), None);
    }
}
