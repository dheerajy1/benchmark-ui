use serde_json::{Map, Value};

use super::{
    form::{FormState, WorkloadMode},
    workspace::{EnvSource, WorkspaceForm},
};

// Fills the custom form from a pasted request body, the same JSON POST /api/v1/benchmarks takes
// (see docs/api.md in benchmark-api). Nothing is sent, the person still checks and runs it.

const TOP_LEVEL: [&str; 4] = ["path", "startup", "workload", "workspaces"];

fn text(object: &Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::to_string)
}

// A number may be written as a number or as a string
fn number(object: &Map<String, Value>, key: &str) -> Option<String> {
    match object.get(key)? {
        Value::Number(number) => Some(number.to_string()),
        Value::String(text) => Some(text.clone()),
        _ => None,
    }
}

fn flag(object: &Map<String, Value>, key: &str) -> bool {
    object.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn child<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<Option<&'a Map<String, Value>>, String> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(inner)) => Ok(Some(inner)),
        Some(_) => Err(format!("\"{key}\" must be an object.")),
    }
}

// The form's header boxes hold one `Name: value` per line
fn header_lines(object: &Map<String, Value>) -> String {
    object
        .get("headers")
        .and_then(Value::as_object)
        .map(|headers| {
            headers
                .iter()
                .filter_map(|(name, value)| value.as_str().map(|value| format!("{name}: {value}")))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

pub fn form_from_json(input: &str) -> Result<FormState, String> {
    let value: Value =
        serde_json::from_str(input).map_err(|error| format!("This is not valid JSON: {error}."))?;

    let Some(root) = value.as_object() else {
        return Err("The request must be a JSON object, written between { and }.".to_string());
    };

    if let Some(unknown) = root.keys().find(|key| !TOP_LEVEL.contains(&key.as_str())) {
        return Err(format!(
            "Unknown field \"{unknown}\". Use path, startup, workload and workspaces."
        ));
    }

    let mut form = FormState::default();

    form.path = text(root, "path").ok_or("\"path\" is required and must be text.")?;

    if let Some(startup) = child(root, "startup")? {
        form.startup_enabled = true;

        if let Some(command) = child(startup, "command")? {
            form.program = text(command, "program").unwrap_or_default();
            form.args = command
                .get("args")
                .and_then(Value::as_array)
                .map(|args| {
                    args.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
        }

        if let Some(readiness) = child(startup, "readiness")? {
            form.host = text(readiness, "host").unwrap_or(form.host);
            form.port = number(readiness, "port").unwrap_or_default();
            form.readiness_path = text(readiness, "path").unwrap_or(form.readiness_path);
            form.expected_status =
                number(readiness, "expected_status").unwrap_or(form.expected_status);
            form.readiness_headers = header_lines(readiness);
        }
    }

    if let Some(workload) = child(root, "workload")? {
        form.workload_enabled = true;
        form.workload_path = text(workload, "path").unwrap_or_default();
        form.workload_headers = header_lines(workload);
        form.concurrency = number(workload, "concurrency").unwrap_or_default();

        if let Some(seconds) = number(workload, "duration_seconds") {
            form.mode = WorkloadMode::Duration;
            form.amount = seconds;
        } else {
            form.mode = WorkloadMode::Requests;
            form.amount = number(workload, "requests").unwrap_or_default();
        }
    }

    match root.get("workspaces") {
        None | Some(Value::Null) => {}
        Some(Value::Array(list)) => match list.as_slice() {
            [] => {}
            [one] => {
                let Some(workspace) = one.as_object() else {
                    return Err("A workspace must be an object.".to_string());
                };

                form.workspace = workspace_form(workspace)?;
            }
            _ => {
                return Err(
                    "The form holds one workspace. Paste a request with a single entry in \"workspaces\"."
                        .to_string(),
                );
            }
        },
        Some(_) => return Err("\"workspaces\" must be a list.".to_string()),
    }

    Ok(form)
}

fn workspace_form(workspace: &Map<String, Value>) -> Result<WorkspaceForm, String> {
    let mut form = WorkspaceForm {
        enabled: true,
        name: text(workspace, "name").unwrap_or_default(),
        directory: text(workspace, "directory").unwrap_or_default(),
        ready_log_line: text(workspace, "ready_log_line").unwrap_or_default(),
        ..WorkspaceForm::default()
    };

    if let Some(timeout) = number(workspace, "ready_timeout_seconds") {
        form.ready_timeout_seconds = timeout;
    }

    if let Some(idle) = number(workspace, "idle_seconds") {
        form.idle_seconds = idle;
    }

    if let Some(load) = child(workspace, "load")? {
        form.load_command = text(load, "command").unwrap_or_default();
    }

    if let Some(env) = child(workspace, "env")? {
        match (text(env, "file_path"), text(env, "pasted")) {
            (Some(_), Some(_)) => {
                return Err("In \"env\" set either file_path or pasted, not both.".to_string());
            }
            (Some(path), None) => {
                form.env_source = EnvSource::File;
                form.env_file_path = path;
            }
            (None, Some(pasted)) => {
                form.env_source = EnvSource::Pasted;
                form.env_pasted = pasted;
            }
            (None, None) => {}
        }
    }

    if let Some(native) = child(workspace, "native")? {
        form.native_enabled = flag(native, "enabled");
        form.native_build_command = text(native, "build_command").unwrap_or_default();
        form.native_command = text(native, "command").unwrap_or_default();
    }

    if let Some(docker) = child(workspace, "docker")? {
        form.docker_enabled = flag(docker, "enabled");
        form.docker_build_command = text(docker, "build_command").unwrap_or_default();
        form.docker_image = text(docker, "image").unwrap_or_default();
        form.docker_container_name = text(docker, "container_name").unwrap_or_default();
        form.docker_run_command = text(docker, "run_command").unwrap_or_default();

        if let Some(timeout) = number(docker, "build_timeout_seconds") {
            form.docker_build_timeout_seconds = timeout;
        }
    }

    Ok(form)
}

#[cfg(test)]
mod tests {
    use super::super::form::validate;
    use super::*;

    const REQUEST: &str = r#"{
        "path": "/home/me/recordlog-api",
        "workspaces": [{
            "name": "subscriber",
            "directory": ".",
            "env": { "file_path": ".env.benchmark" },
            "ready_log_line": "subscribed topic=record-log-status",
            "ready_timeout_seconds": 60,
            "idle_seconds": 30,
            "native": { "enabled": true, "command": "bun src/main.ts" },
            "docker": {
                "enabled": true,
                "build_command": "docker compose build sub",
                "build_timeout_seconds": 1800,
                "image": "sub:latest",
                "container_name": "sub",
                "run_command": "docker run -d --name sub --env-file {ENV_FILE} sub:latest"
            }
        }]
    }"#;

    #[test]
    fn fills_the_form_and_the_result_validates_to_the_same_request() {
        let form = form_from_json(REQUEST).expect("fills");

        assert_eq!(form.path, "/home/me/recordlog-api");
        assert!(form.workspace.enabled);
        assert_eq!(form.workspace.directory, ".");
        assert_eq!(form.workspace.env_source, EnvSource::File);
        assert!(form.workspace.native_enabled && form.workspace.docker_enabled);

        let sent = serde_json::to_value(validate(&form).expect("valid")).unwrap();
        let original: Value = serde_json::from_str(REQUEST).unwrap();

        assert_eq!(
            sent["workspaces"][0]["name"],
            original["workspaces"][0]["name"]
        );
        assert_eq!(
            sent["workspaces"][0]["docker"]["run_command"],
            original["workspaces"][0]["docker"]["run_command"]
        );
    }

    #[test]
    fn fills_startup_and_workload_with_headers() {
        let form = form_from_json(
            r#"{ "path": "/p",
                "startup": { "command": { "program": "bun", "args": ["run", "start"] },
                    "readiness": { "type": "http", "host": "127.0.0.1", "port": 5001,
                        "path": "/health", "method": "GET", "expected_status": 200,
                        "headers": { "X-Client-Id": "a" } } },
                "workload": { "method": "GET", "path": "/items", "duration_seconds": 30,
                    "concurrency": 10, "headers": { "X-A": "1", "X-B": "2" } } }"#,
        )
        .expect("fills");

        assert!(form.startup_enabled);
        assert_eq!(form.args, "run start");
        assert_eq!(form.port, "5001");
        assert_eq!(form.readiness_headers, "X-Client-Id: a");
        assert_eq!(form.mode, WorkloadMode::Duration);
        assert_eq!(form.amount, "30");
        assert_eq!(form.workload_headers, "X-A: 1\nX-B: 2");
        assert!(!form.workspace.enabled);
    }

    #[test]
    fn explains_what_is_wrong() {
        assert!(form_from_json("{ nope")
            .unwrap_err()
            .contains("not valid JSON"));
        assert!(form_from_json("[]").unwrap_err().contains("object"));
        assert!(form_from_json("{}").unwrap_err().contains("path"));
        assert!(form_from_json(r#"{ "path": "/p", "workspace": [] }"#)
            .unwrap_err()
            .contains("Unknown field"));
        assert!(
            form_from_json(r#"{ "path": "/p", "workspaces": [{}, {}] }"#)
                .unwrap_err()
                .contains("one workspace")
        );
        assert!(form_from_json(
            r#"{ "path": "/p", "workspaces": [{ "env": { "file_path": "a", "pasted": "b" } }] }"#
        )
        .unwrap_err()
        .contains("not both"));
    }
}
