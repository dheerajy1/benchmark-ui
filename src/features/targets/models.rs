use serde::Deserialize;

// GET /api/v1/targets. Header values are never returned, only their names.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetsView {
    pub custom_runs_allowed: bool,
    pub targets: Vec<TargetView>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetView {
    pub id: String,
    pub name: String,
    pub path: String,
    // The build the target defines, absent when the server did not send one
    #[serde(default)]
    pub build: Option<TargetBuild>,
    // The cleanup command that runs when a run of the target is over, null or absent for none
    #[serde(default)]
    pub cleanup: Option<TargetCleanup>,
    pub startup: Option<TargetStartup>,
    pub workload: Option<TargetWorkload>,
    // Workspaces the target also measures, absent for a target without one
    #[serde(default)]
    pub workspaces: Vec<TargetWorkspace>,
}

// The build command of a saved target and its output folder, null when the default is measured
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetBuild {
    pub command: String,
    pub output_dir: Option<String>,
}

// The cleanup of a saved target, the timeout is null when the default applies
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetCleanup {
    pub command: String,
    pub timeout_seconds: Option<u64>,
}

impl TargetView {
    // Whether a run of this target cleans up after itself: its own cleanup, or a Docker cleanup
    // of one of its workspaces
    pub fn has_cleanup(&self) -> bool {
        self.cleanup
            .as_ref()
            .is_some_and(|cleanup| !cleanup.command.trim().is_empty())
            || self.workspaces.iter().any(|workspace| {
                workspace
                    .docker
                    .as_ref()
                    .and_then(|docker| docker.get("cleanup"))
                    .and_then(|cleanup| cleanup.get("command"))
                    .and_then(|command| command.as_str())
                    .is_some_and(|command| !command.trim().is_empty())
            })
    }
}

// What the page shows of a workspace. Environment values never arrive, only names.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetWorkspace {
    pub name: String,
    pub directory: String,
    #[serde(default)]
    pub env_variable_names: Vec<String>,
    #[serde(default)]
    pub native: Option<serde_json::Value>,
    #[serde(default)]
    pub docker: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetStartup {
    pub command: TargetCommand,
    pub readiness: TargetReadiness,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetCommand {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetReadiness {
    pub host: String,
    pub port: u16,
    pub path: String,
    pub method: String,
    pub expected_status: u16,
    // Null when the server's health credentials are used
    pub header_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TargetWorkload {
    pub method: String,
    pub path: String,
    // "requests" or "duration"
    pub mode: String,
    pub requests: Option<u64>,
    pub duration_seconds: Option<u64>,
    pub concurrency: u64,
    pub header_names: Vec<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn target(cleanup: serde_json::Value, docker: serde_json::Value) -> TargetView {
        serde_json::from_value(json!({
            "id": "api", "name": "API", "path": "/p",
            "cleanup": cleanup,
            "startup": null, "workload": null,
            "workspaces": [{ "name": "app", "directory": "app", "docker": docker }]
        }))
        .expect("parses")
    }

    #[test]
    fn a_target_cleans_up_with_its_own_command_or_a_docker_one() {
        let own = json!({ "command": "docker image prune -f", "timeout_seconds": null });
        let docker = json!({ "cleanup": { "command": "docker rmi app", "timeout_seconds": 30 } });

        assert!(target(own, json!(null)).has_cleanup());
        assert!(target(json!(null), docker).has_cleanup());
        assert!(!target(json!(null), json!(null)).has_cleanup());
        assert!(!target(json!({ "command": "  " }), json!({ "cleanup": null })).has_cleanup());
    }
}
