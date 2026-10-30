use crate::features::targets::TargetView;

use super::{
    form::WorkloadMode,
    models::{TargetRunRequest, TargetWorkloadOverride},
};

// Longest sustained workload the API accepts, mirrors the backend rule
const MAX_DURATION_SECONDS: u64 = 300;

// What the user may change for a saved target: the workload kind, amount and concurrency
#[derive(Debug, Clone, PartialEq)]
pub struct TargetForm {
    pub target_id: String,
    // False when the target defines no workload, then the run has no workload
    pub has_workload: bool,
    pub mode: WorkloadMode,
    pub amount: String,
    pub concurrency: String,
}

impl TargetForm {
    // Starts from the workload the target defines
    pub fn for_target(target: &TargetView) -> Self {
        match &target.workload {
            Some(workload) => Self {
                target_id: target.id.clone(),
                has_workload: true,
                mode: mode_of(&workload.mode),
                amount: amount_of(workload.requests, workload.duration_seconds),
                concurrency: workload.concurrency.to_string(),
            },
            None => Self {
                target_id: target.id.clone(),
                has_workload: false,
                mode: WorkloadMode::Requests,
                amount: String::new(),
                concurrency: String::new(),
            },
        }
    }
}

// "duration" selects a duration, anything else a request count
pub fn mode_of(mode: &str) -> WorkloadMode {
    if mode == "duration" {
        WorkloadMode::Duration
    } else {
        WorkloadMode::Requests
    }
}

// The amount of a workload, whichever of its two modes is set
pub fn amount_of(requests: Option<u64>, duration_seconds: Option<u64>) -> String {
    requests
        .or(duration_seconds)
        .map(|value| value.to_string())
        .unwrap_or_default()
}

// The request for a saved-target run, or the first problem found. Messages name the field and
// never repeat a value.
pub fn validate_target(form: &TargetForm) -> Result<TargetRunRequest, String> {
    if form.target_id.is_empty() {
        return Err("Choose a saved target.".to_string());
    }

    if !form.has_workload {
        return Ok(TargetRunRequest {
            target_id: form.target_id.clone(),
            workload: None,
        });
    }

    let amount = form
        .amount
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0);

    let Some(amount) = amount else {
        return Err(match form.mode {
            WorkloadMode::Requests => "Enter a request count greater than 0.".to_string(),
            WorkloadMode::Duration => "Enter a duration in seconds greater than 0.".to_string(),
        });
    };

    if form.mode == WorkloadMode::Duration && amount > MAX_DURATION_SECONDS {
        return Err("The duration must not exceed 300 seconds.".to_string());
    }

    let concurrency = form
        .concurrency
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "Enter a concurrency greater than 0.".to_string())?;

    let (requests, duration_seconds) = match form.mode {
        WorkloadMode::Requests => (Some(amount), None),
        WorkloadMode::Duration => (None, Some(amount)),
    };

    Ok(TargetRunRequest {
        target_id: form.target_id.clone(),
        workload: Some(TargetWorkloadOverride {
            requests,
            duration_seconds,
            concurrency,
        }),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn duration_target() -> TargetView {
        serde_json::from_value(json!({
            "id": "demo",
            "name": "Demo API",
            "path": "/srv/demo",
            "startup": null,
            "workload": {
                "method": "GET", "path": "/h", "mode": "duration",
                "requests": null, "duration_seconds": 30, "concurrency": 50,
                "header_names": ["X-Client-Secret"]
            }
        }))
        .expect("valid target")
    }

    #[test]
    fn a_target_without_workload_sends_no_workload() {
        let target: TargetView = serde_json::from_value(json!({
            "id": "plain", "name": "Plain", "path": "/srv/plain",
            "startup": null, "workload": null
        }))
        .expect("valid target");

        let request = validate_target(&TargetForm::for_target(&target)).expect("valid");

        assert_eq!(request.target_id, "plain");
        assert!(request.workload.is_none());
    }

    #[test]
    fn the_workload_starts_from_the_target_and_only_the_amount_may_change() {
        let mut form = TargetForm::for_target(&duration_target());

        assert_eq!(form.mode, WorkloadMode::Duration);
        assert_eq!(form.amount, "30");
        assert_eq!(form.concurrency, "50");

        form.amount = "60".to_string();
        form.concurrency = "5".to_string();

        let json = serde_json::to_value(validate_target(&form).expect("valid")).unwrap();

        assert_eq!(
            json,
            json!({ "target_id": "demo", "workload": { "duration_seconds": 60, "concurrency": 5 } })
        );
    }

    #[test]
    fn requests_mode_sends_only_requests() {
        let form = TargetForm {
            mode: WorkloadMode::Requests,
            amount: "10".to_string(),
            ..TargetForm::for_target(&duration_target())
        };

        let json = serde_json::to_value(validate_target(&form).expect("valid")).unwrap();

        assert_eq!(json["workload"]["requests"], 10);
        assert!(json["workload"].get("duration_seconds").is_none());
    }

    #[test]
    fn durations_over_the_limit_are_refused() {
        let form = TargetForm {
            amount: "301".to_string(),
            ..TargetForm::for_target(&duration_target())
        };

        assert_eq!(
            validate_target(&form).unwrap_err(),
            "The duration must not exceed 300 seconds."
        );
    }
}
