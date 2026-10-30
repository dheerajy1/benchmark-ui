use serde::{Deserialize, Serialize};

use super::state::LiveState;
use crate::features::connection::{post_json, ApiError};

// Body of POST /api/v1/live-runs/{run_id}/cancel
#[derive(Serialize)]
struct CancelBody<'a> {
    cancel_token: &'a str,
}

// `data` of the 202 of a cancel request
#[derive(Deserialize)]
struct CancelAccepted {
    already_requested: bool,
}

// Asks the API to stop a run. Answers whether the request was a duplicate. The run is not
// cancelled yet when this answers, the page learns it from the run's own events.
pub async fn request_cancel(run_id: &str, token: &str) -> Result<bool, ApiError> {
    let accepted: CancelAccepted = post_json(
        &format!("/live-runs/{}/cancel", run_id.trim()),
        &CancelBody {
            cancel_token: token,
        },
    )
    .await?;

    Ok(accepted.already_requested)
}

// What the cancel control shows
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelUi {
    // Nothing to cancel or nothing this page can do about it
    Hidden,
    // Cancelling is possible but this page does not hold the right to do it
    NeedsToken,
    Available,
    // Cancellation was requested and the run is stopping
    Cancelling,
}

// `requested_here` is true once this page's own request was taken, before the run's event arrives
pub fn cancel_ui(state: &LiveState, has_token: bool, requested_here: bool) -> CancelUi {
    if state.is_terminal() || !state.can_cancel() {
        CancelUi::Hidden
    } else if state.is_cancelling() || requested_here {
        CancelUi::Cancelling
    } else if !has_token {
        CancelUi::NeedsToken
    } else {
        CancelUi::Available
    }
}

// Why a cancel request did not go through
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelFailure {
    // The run can no longer be cancelled, the API says why
    TooLate(String),
    // The token is missing or not the run's
    NotAllowed,
    // The API no longer has the run
    Gone,
    // The request did not get through, trying again is fine
    Retry(String),
}

pub fn classify_cancel_error(error: &ApiError) -> CancelFailure {
    match error {
        ApiError::Http {
            status: 409,
            detail,
            message,
            ..
        } => CancelFailure::TooLate(detail.clone().unwrap_or_else(|| message.clone())),
        ApiError::Http { status: 403, .. } => CancelFailure::NotAllowed,
        ApiError::Http { status: 404, .. } => CancelFailure::Gone,
        other => CancelFailure::Retry(other.description()),
    }
}

pub fn failure_message(failure: &CancelFailure) -> String {
    match failure {
        CancelFailure::TooLate(reason) => reason.clone(),
        CancelFailure::NotAllowed => {
            "This browser cannot cancel this run, it was started from somewhere else.".to_string()
        }
        CancelFailure::Gone => "The run is no longer available.".to_string(),
        CancelFailure::Retry(reason) => {
            format!("The cancel request did not get through. {reason}")
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::features::live::models::StreamEvent;

    fn state_with(capabilities_cancel: Option<bool>) -> LiveState {
        let mut state = LiveState::new("r".to_string());

        let mut data =
            json!({ "target": { "path": "/p" }, "plan": { "startup": true, "workload": null } });

        if let Some(cancel) = capabilities_cancel {
            data["capabilities"] = json!({ "cancel": cancel });
        }

        state.apply_event(event(1, "run.accepted", data), 0);

        state
    }

    fn event(seq: u64, kind: &str, data: Value) -> StreamEvent {
        StreamEvent {
            run_id: "r".to_string(),
            seq,
            t_ms: seq,
            kind: kind.to_string(),
            data,
        }
    }

    #[test]
    fn a_running_run_with_a_token_can_be_cancelled() {
        assert_eq!(
            cancel_ui(&state_with(Some(true)), true, false),
            CancelUi::Available
        );
    }

    #[test]
    fn without_the_token_the_page_says_so_instead_of_offering_a_button() {
        assert_eq!(
            cancel_ui(&state_with(Some(true)), false, false),
            CancelUi::NeedsToken
        );
    }

    #[test]
    fn nothing_is_offered_before_the_run_is_known_or_when_it_cannot_be_cancelled() {
        assert_eq!(
            cancel_ui(&LiveState::new("r".to_string()), true, false),
            CancelUi::Hidden
        );
        assert_eq!(
            cancel_ui(&state_with(Some(false)), true, false),
            CancelUi::Hidden
        );
        assert_eq!(cancel_ui(&state_with(None), true, false), CancelUi::Hidden);
    }

    #[test]
    fn a_request_taken_here_or_announced_by_the_run_means_cancelling() {
        assert_eq!(
            cancel_ui(&state_with(Some(true)), true, true),
            CancelUi::Cancelling
        );

        let mut state = state_with(Some(true));

        state.apply_event(event(2, "run.cancelling", json!({ "stage": "build" })), 0);

        // Another viewer asked, and this page shows it too, with or without a token
        assert_eq!(cancel_ui(&state, true, false), CancelUi::Cancelling);
        assert_eq!(cancel_ui(&state, false, false), CancelUi::Cancelling);
    }

    #[test]
    fn nothing_is_offered_once_the_run_has_ended() {
        for terminal in ["run.completed", "run.failed", "run.cancelled"] {
            let mut state = state_with(Some(true));

            state.apply_event(event(2, terminal, json!({ "message": "x" })), 0);

            assert_eq!(
                cancel_ui(&state, true, false),
                CancelUi::Hidden,
                "{terminal}"
            );
            assert_eq!(
                cancel_ui(&state, true, true),
                CancelUi::Hidden,
                "{terminal}"
            );
        }
    }

    fn http(status: u16, detail: Option<&str>) -> ApiError {
        ApiError::Http {
            status,
            code: None,
            message: "catalogue message".to_string(),
            detail: detail.map(str::to_string),
        }
    }

    #[test]
    fn a_409_says_why_it_is_too_late() {
        assert_eq!(
            classify_cancel_error(&http(409, Some("The run already finished (completed)."))),
            CancelFailure::TooLate("The run already finished (completed).".to_string())
        );
        assert_eq!(
            classify_cancel_error(&http(409, None)),
            CancelFailure::TooLate("catalogue message".to_string())
        );
    }

    #[test]
    fn a_403_and_a_404_are_final_and_other_errors_can_be_retried() {
        assert_eq!(
            classify_cancel_error(&http(403, None)),
            CancelFailure::NotAllowed
        );
        assert_eq!(classify_cancel_error(&http(404, None)), CancelFailure::Gone);
        assert!(matches!(
            classify_cancel_error(&http(500, Some("boom"))),
            CancelFailure::Retry(_)
        ));
    }

    #[test]
    fn the_messages_are_plain_and_never_contain_a_token() {
        for failure in [
            CancelFailure::TooLate("The run is already saving its result.".to_string()),
            CancelFailure::NotAllowed,
            CancelFailure::Gone,
            CancelFailure::Retry("Network error.".to_string()),
        ] {
            let message = failure_message(&failure);

            assert!(!message.is_empty());
            assert!(!message.contains("token"), "{message}");
        }

        assert_eq!(
            failure_message(&CancelFailure::Gone),
            "The run is no longer available."
        );
    }

    #[test]
    fn the_request_body_carries_only_the_token() {
        let body = serde_json::to_value(CancelBody {
            cancel_token: "3f9a0c1b7d2e48a6b5c4d3e2f1a09b8c",
        })
        .unwrap();

        assert_eq!(
            body,
            json!({ "cancel_token": "3f9a0c1b7d2e48a6b5c4d3e2f1a09b8c" })
        );
    }

    #[test]
    fn the_answer_is_read_for_its_duplicate_flag() {
        let accepted: CancelAccepted =
            serde_json::from_value(json!({ "state": "cancelling", "already_requested": true }))
                .unwrap();

        assert!(accepted.already_requested);
    }
}
