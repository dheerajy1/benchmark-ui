use crate::features::{
    configurations::ConfigurationEntry,
    targets::{TargetView, TargetsView},
};

use super::{
    form::FormState,
    target_run::{amount_of, mode_of, TargetForm},
};

// What choosing a history row does to the New Benchmark form
#[derive(Debug, Clone, PartialEq)]
pub enum Prefill {
    // Saved target mode, with the workload the run recorded when the target still defines one
    Target {
        form: TargetForm,
    },
    // Custom mode, filled from the run. `notice` says what could not be filled and
    // `header_names` lists the headers the run sent, whose values are never stored.
    Custom {
        form: Box<FormState>,
        notice: Option<String>,
        header_names: Vec<String>,
    },
    // The row cannot be used, `reason` says why
    Unavailable {
        reason: String,
    },
}

// Maps the latest run of a history row onto the form, using the targets currently on the server
pub fn prefill_for(entry: &ConfigurationEntry, targets: &TargetsView) -> Prefill {
    let saved_target = entry
        .configuration
        .as_ref()
        .filter(|configuration| configuration.source == "target");

    if let Some(configuration) = saved_target {
        if let Some(target_id) = configuration.target_id.as_deref() {
            if let Some(target) = targets.targets.iter().find(|target| target.id == target_id) {
                return Prefill::Target {
                    form: target_form_from_run(target, entry),
                };
            }

            // The target was removed from the file, so the run falls back to a custom fill
            let name = configuration
                .target_name
                .clone()
                .unwrap_or_else(|| target_id.to_string());

            if !targets.custom_runs_allowed {
                return Prefill::Unavailable {
                    reason: format!(
                        "The saved target \"{name}\" is no longer in the targets file, and custom runs are disabled on this server."
                    ),
                };
            }

            return custom_fill(
                entry,
                Some(format!(
                    "The saved target \"{name}\" is no longer in the targets file. The run was filled in as a custom run."
                )),
            );
        }
    }

    if !targets.custom_runs_allowed {
        return Prefill::Unavailable {
            reason: "Custom runs are disabled on this server, so this run cannot be filled in."
                .to_string(),
        };
    }

    custom_fill(entry, None)
}

// The target's own workload, replaced by the recorded values when the run has a workload
fn target_form_from_run(target: &TargetView, entry: &ConfigurationEntry) -> TargetForm {
    let mut form = TargetForm::for_target(target);

    let recorded = entry
        .configuration
        .as_ref()
        .and_then(|configuration| configuration.workload.as_ref());

    if let (true, Some(workload)) = (form.has_workload, recorded) {
        form.mode = mode_of(&workload.mode);
        form.amount = amount_of(workload.requests, workload.duration_seconds);
        form.concurrency = workload.concurrency.to_string();
    }

    form
}

fn custom_fill(entry: &ConfigurationEntry, removed_target_notice: Option<String>) -> Prefill {
    let mut form = FormState {
        path: entry.path.clone(),
        ..FormState::default()
    };

    if let Some(startup) = &entry.startup {
        form.startup_enabled = startup.program.is_some() || startup.readiness.is_some();
        form.program = startup.program.clone().unwrap_or_default();
        form.args = startup
            .args
            .as_ref()
            .map(|args| args.join(" "))
            .unwrap_or_default();

        if let Some(readiness) = &startup.readiness {
            form.host = readiness.host.clone();
            form.port = readiness.port.to_string();
            form.readiness_path = readiness.path.clone();
            form.expected_status = readiness.expected_status.to_string();
        }
    }

    let mut notices: Vec<String> = removed_target_notice.into_iter().collect();

    let recorded_workload = entry
        .configuration
        .as_ref()
        .and_then(|configuration| configuration.workload.as_ref());

    match recorded_workload {
        Some(workload) => {
            form.workload_enabled = true;
            form.workload_path = workload.path.clone();
            form.mode = mode_of(&workload.mode);
            form.amount = amount_of(workload.requests, workload.duration_seconds);
            form.concurrency = workload.concurrency.to_string();
        }
        None if !entry.recorded => notices.push(
            "Workload and headers were not recorded for this run, so the workload was left at its defaults."
                .to_string(),
        ),
        None => {}
    }

    let header_names: Vec<String> = crate::features::configurations::header_names_of(entry)
        .into_iter()
        .collect();

    Prefill::Custom {
        form: Box::new(form),
        notice: (!notices.is_empty()).then(|| notices.join(" ")),
        header_names,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn legacy_entry() -> ConfigurationEntry {
        serde_json::from_value(json!({
            "run_id": "old",
            "created_at": "2026-10-20T14:03:11+00:00",
            "path": "/srv/old",
            "startup": {
                "program": "bun",
                "args": ["run", "dev"],
                "readiness": {
                    "readiness_type": "http", "host": "127.0.0.1", "port": 5000,
                    "path": "/health", "method": "GET", "expected_status": 200,
                    "header_names": null
                }
            },
            "recorded": false,
            "configuration": null
        }))
        .expect("valid entry")
    }

    fn no_targets(custom_runs_allowed: bool) -> TargetsView {
        serde_json::from_value(json!({
            "custom_runs_allowed": custom_runs_allowed,
            "targets": []
        }))
        .expect("valid targets")
    }

    #[test]
    fn an_older_run_fills_the_path_and_start_command_and_notes_the_workload() {
        let prefill = prefill_for(&legacy_entry(), &no_targets(true));

        let Prefill::Custom {
            form,
            notice,
            header_names,
        } = prefill
        else {
            panic!("expected a custom fill");
        };

        assert_eq!(form.path, "/srv/old");
        assert!(form.startup_enabled);
        assert_eq!(form.program, "bun");
        assert_eq!(form.args, "run dev");
        assert_eq!(form.port, "5000");
        assert!(!form.workload_enabled);
        assert!(header_names.is_empty());
        assert!(notice.unwrap_or_default().contains("not recorded"));
    }

    #[test]
    fn an_older_run_is_unavailable_when_custom_runs_are_disabled() {
        let prefill = prefill_for(&legacy_entry(), &no_targets(false));

        assert!(matches!(prefill, Prefill::Unavailable { .. }));
    }

    #[test]
    fn a_run_from_a_removed_target_falls_back_to_a_custom_fill_with_a_notice() {
        let entry: ConfigurationEntry = serde_json::from_value(json!({
            "run_id": "gone",
            "created_at": "2026-10-21T14:03:11+00:00",
            "path": "/srv/demo",
            "startup": null,
            "recorded": true,
            "configuration": {
                "source": "target", "target_id": "demo", "target_name": "Demo API",
                "target_path": "/srv/demo", "startup": null, "workload": null
            }
        }))
        .expect("valid entry");

        let Prefill::Custom { notice, .. } = prefill_for(&entry, &no_targets(true)) else {
            panic!("expected a custom fill");
        };

        assert!(notice
            .unwrap_or_default()
            .contains("no longer in the targets file"));
    }
}
