use super::{
    form::{Field, FieldError, FormState, WorkloadMode},
    target_run::TargetForm,
    workspace::{EnvSource, ENV_FILE_PLACEHOLDER},
};
use crate::features::targets::TargetView;

// One condition that must hold before a run can start
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub label: String,
    pub ok: bool,
}

fn check(label: &str, ok: bool) -> Check {
    Check {
        label: label.to_string(),
        ok,
    }
}

fn positive(text: &str) -> bool {
    text.trim().parse::<u64>().is_ok_and(|value| value > 0)
}

fn filled(text: &str) -> bool {
    !text.trim().is_empty()
}

// "5000 requests" or "30 s", an empty amount is shown as an ellipsis
fn amount_text(mode: WorkloadMode, amount: &str) -> String {
    let amount = if filled(amount) { amount.trim() } else { "…" };

    match mode {
        WorkloadMode::Requests => format!("{amount} requests"),
        WorkloadMode::Duration => format!("{amount} s"),
    }
}

// What has to be right for a saved target run
pub fn target_checks(form: Option<&TargetForm>) -> Vec<Check> {
    let Some(form) = form else {
        return vec![check("A saved target is chosen", false)];
    };

    let mut checks = vec![check(
        "A saved target is chosen",
        !form.target_id.is_empty(),
    )];

    if form.has_workload {
        let amount_ok = positive(&form.amount)
            && (form.mode == WorkloadMode::Requests
                || form
                    .amount
                    .trim()
                    .parse::<u64>()
                    .is_ok_and(|seconds| seconds <= 300));

        checks.push(check(
            match form.mode {
                WorkloadMode::Requests => "Request count is above 0",
                WorkloadMode::Duration => "Duration is 1 to 300 seconds",
            },
            amount_ok,
        ));
        checks.push(check("Concurrency is above 0", positive(&form.concurrency)));
    }

    checks
}

// What has to be right for a custom run, from the raw form and the live validation errors
pub fn custom_checks(state: &FormState, errors: &[FieldError]) -> Vec<Check> {
    let failed = |fields: &[Field]| errors.iter().any(|error| fields.contains(&error.field));
    let mut checks = vec![
        check("Project path is set", filled(&state.path)),
        check("Build command is set", filled(&state.build_command)),
    ];

    if filled(&state.cleanup_command) {
        checks.push(check(
            "Cleanup timeout is valid",
            !failed(&[Field::CleanupTimeout]),
        ));
    }

    if state.startup_enabled {
        checks.push(check(
            "Startup settings are valid",
            !failed(&[
                Field::Program,
                Field::Host,
                Field::Port,
                Field::ReadinessPath,
                Field::ExpectedStatus,
                Field::ReadinessHeaders,
            ]),
        ));
    }

    if state.workload_enabled {
        checks.push(check(
            "Workload settings are valid",
            !failed(&[
                Field::WorkloadPath,
                Field::Amount,
                Field::Concurrency,
                Field::WorkloadHeaders,
            ]),
        ));
    }

    let workspace = &state.workspace;
    if workspace.enabled {
        checks.push(check(
            "Workspace name, directory and ready log line are set",
            filled(&workspace.name)
                && filled(&workspace.directory)
                && filled(&workspace.ready_log_line),
        ));
        checks.push(check(
            "Native or Docker is on",
            workspace.native_enabled || workspace.docker_enabled,
        ));

        let env_given = match workspace.env_source {
            EnvSource::None => None,
            EnvSource::File => Some(filled(&workspace.env_file_path)),
            EnvSource::Pasted => Some(filled(&workspace.env_pasted)),
        };

        if let Some(env_filled) = env_given {
            checks.push(check("Env file path or contents are not empty", env_filled));

            if workspace.docker_enabled {
                checks.push(check(
                    "Docker run command contains {ENV_FILE}",
                    workspace.docker_run_command.contains(ENV_FILE_PLACEHOLDER),
                ));
            }
        }
    }

    // Anything the specific checks do not name, such as a timeout that is not a number
    if checks.iter().all(|entry| entry.ok) && !errors.is_empty() {
        checks.push(check("Fix the highlighted fields", false));
    }

    checks
}

// Whether every check holds
pub fn all_ok(checks: &[Check]) -> bool {
    checks.iter().all(|entry| entry.ok)
}

fn line(label: &str, value: String) -> (String, String) {
    (label.to_string(), value)
}

// Facts shown in the panel for a saved target run
pub fn target_lines(
    target: Option<&TargetView>,
    form: Option<&TargetForm>,
) -> Vec<(String, String)> {
    let (Some(target), Some(form)) = (target, form) else {
        return Vec::new();
    };

    let workload = if form.has_workload {
        format!(
            "{} · {} at once",
            amount_text(form.mode, &form.amount),
            if filled(&form.concurrency) {
                form.concurrency.trim()
            } else {
                "…"
            }
        )
    } else {
        "None".to_string()
    };
    let build = target
        .build
        .as_ref()
        .map_or_else(|| "…".to_string(), |build| build.command.clone());
    let workspaces = if target.workspaces.is_empty() {
        "None".to_string()
    } else {
        target
            .workspaces
            .iter()
            .map(|workspace| workspace.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };

    vec![
        line("Target", target.name.clone()),
        line("Path", target.path.clone()),
        line("Build", build),
        line("Workload", workload),
        line("Workspace", workspaces),
    ]
}

// Facts shown in the panel for a custom run
pub fn custom_lines(state: &FormState) -> Vec<(String, String)> {
    let path = if filled(&state.path) {
        state.path.trim().to_string()
    } else {
        "…".to_string()
    };
    let build = if filled(&state.build_command) {
        state.build_command.trim().to_string()
    } else {
        "…".to_string()
    };
    let startup = if state.startup_enabled {
        format!("{} {}", state.program.trim(), state.args.trim())
            .trim()
            .to_string()
    } else {
        "Detected from the project".to_string()
    };
    let workload = if state.workload_enabled {
        format!(
            "GET {} · {} · {} at once",
            if filled(&state.workload_path) {
                state.workload_path.trim()
            } else {
                "…"
            },
            amount_text(state.mode, &state.amount),
            if filled(&state.concurrency) {
                state.concurrency.trim()
            } else {
                "…"
            }
        )
    } else {
        "None".to_string()
    };
    let workspace = &state.workspace;
    let workspace_text = if workspace.enabled {
        let mut modes = Vec::new();

        if workspace.native_enabled {
            modes.push("native");
        }
        if workspace.docker_enabled {
            modes.push("Docker");
        }

        format!(
            "{} · {}",
            if filled(&workspace.name) {
                workspace.name.trim()
            } else {
                "…"
            },
            if modes.is_empty() {
                "no mode".to_string()
            } else {
                modes.join(", ")
            }
        )
    } else {
        "None".to_string()
    };

    let cleanup = if filled(&state.cleanup_command) {
        state.cleanup_command.trim().to_string()
    } else {
        "None".to_string()
    };

    vec![
        line("Path", path),
        line("Build", build),
        line("Cleanup", cleanup),
        line("Startup", startup),
        line("Workload", workload),
        line("Workspace", workspace_text),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target_form(amount: &str, concurrency: &str, mode: WorkloadMode) -> TargetForm {
        TargetForm {
            target_id: "api".to_string(),
            has_workload: true,
            mode,
            amount: amount.to_string(),
            concurrency: concurrency.to_string(),
        }
    }

    #[test]
    fn a_target_without_a_choice_fails_its_only_check() {
        let checks = target_checks(None);

        assert_eq!(checks.len(), 1);
        assert!(!all_ok(&checks));
    }

    #[test]
    fn checks_the_workload_amount_and_concurrency_of_a_target() {
        let good = target_form("100", "4", WorkloadMode::Requests);
        let bad_amount = target_form("0", "4", WorkloadMode::Requests);
        let long = target_form("301", "4", WorkloadMode::Duration);
        let no_concurrency = target_form("100", "", WorkloadMode::Requests);

        assert!(all_ok(&target_checks(Some(&good))));
        assert!(!all_ok(&target_checks(Some(&bad_amount))));
        assert!(!all_ok(&target_checks(Some(&long))));
        assert!(!all_ok(&target_checks(Some(&no_concurrency))));
    }

    #[test]
    fn a_custom_run_needs_a_project_path_and_a_build_command() {
        let mut state = FormState::default();

        assert!(!all_ok(&custom_checks(&state, &[])));

        state.path = "/p".to_string();

        assert!(!all_ok(&custom_checks(&state, &[])));

        state.build_command = "bun run build".to_string();

        assert!(all_ok(&custom_checks(&state, &[])));
    }

    #[test]
    fn a_workspace_needs_a_mode_and_the_env_placeholder() {
        let mut state = FormState {
            path: "/p".to_string(),
            build_command: "build".to_string(),
            ..FormState::default()
        };
        state.workspace.enabled = true;
        state.workspace.name = "api".to_string();
        state.workspace.directory = "sub".to_string();
        state.workspace.ready_log_line = "ready".to_string();

        let without_mode = custom_checks(&state, &[]);
        assert!(without_mode.iter().any(|entry| !entry.ok));

        state.workspace.docker_enabled = true;
        state.workspace.env_source = EnvSource::Pasted;
        state.workspace.env_pasted = "A=1".to_string();
        state.workspace.docker_run_command = "docker run image".to_string();

        let no_placeholder = custom_checks(&state, &[]);
        assert!(no_placeholder
            .iter()
            .any(|entry| !entry.ok && entry.label.contains("ENV_FILE")));

        state.workspace.docker_run_command = "docker run --env-file {ENV_FILE} image".to_string();

        assert!(all_ok(&custom_checks(&state, &[])));
    }

    #[test]
    fn unnamed_errors_add_a_catch_all_check() {
        let state = FormState {
            path: "/p".to_string(),
            build_command: "build".to_string(),
            ..FormState::default()
        };
        let errors = [FieldError {
            field: Field::Workspace,
            message: "x".to_string(),
        }];

        let checks = custom_checks(&state, &errors);

        assert_eq!(checks.last().map(|entry| entry.ok), Some(false));
    }

    #[test]
    fn summarizes_a_custom_run() {
        let state = FormState {
            path: " /p ".to_string(),
            build_command: " cargo xtask build ".to_string(),
            workload_enabled: true,
            workload_path: "/items".to_string(),
            amount: "50".to_string(),
            concurrency: "2".to_string(),
            ..FormState::default()
        };

        let lines = custom_lines(&state);

        assert_eq!(lines[0].1, "/p");
        assert_eq!(lines[1].1, "cargo xtask build");
        assert_eq!(lines[2].1, "None");
        assert_eq!(lines[3].1, "Detected from the project");
        assert_eq!(lines[4].1, "GET /items · 50 requests · 2 at once");
        assert_eq!(lines[5].1, "None");
    }
}
