use serde::Serialize;

// The optional workspace section of POST /api/v1/benchmarks, see docs/api.md in benchmark-api.
// A workspace is an extra process, for example the recordlog subscriber, that the run measures
// after the main project, natively and in Docker.

const DEFAULT_READY_TIMEOUT_SECONDS: &str = "60";
const DEFAULT_IDLE_SECONDS: &str = "30";
const DEFAULT_BUILD_TIMEOUT_SECONDS: &str = "1800";

// The placeholder in the Docker run command that the API replaces with the env file path
pub const ENV_FILE_PLACEHOLDER: &str = "{ENV_FILE}";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvSource {
    None,
    File,
    Pasted,
}

// Raw text of every workspace input, parsed and validated only on submit
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceForm {
    pub enabled: bool,
    pub name: String,
    pub directory: String,
    pub env_source: EnvSource,
    pub env_file_path: String,
    // The body of a .env file, never stored by this page
    pub env_pasted: String,
    pub ready_log_line: String,
    pub ready_timeout_seconds: String,
    pub load_command: String,
    pub idle_seconds: String,
    pub native_enabled: bool,
    pub native_build_command: String,
    pub native_command: String,
    pub docker_enabled: bool,
    pub docker_build_command: String,
    pub docker_build_timeout_seconds: String,
    pub docker_image: String,
    pub docker_container_name: String,
    pub docker_run_command: String,
}

impl Default for WorkspaceForm {
    fn default() -> Self {
        Self {
            enabled: false,
            name: String::new(),
            directory: String::new(),
            env_source: EnvSource::None,
            env_file_path: String::new(),
            env_pasted: String::new(),
            ready_log_line: String::new(),
            ready_timeout_seconds: DEFAULT_READY_TIMEOUT_SECONDS.to_string(),
            load_command: String::new(),
            idle_seconds: DEFAULT_IDLE_SECONDS.to_string(),
            native_enabled: false,
            native_build_command: String::new(),
            native_command: String::new(),
            docker_enabled: false,
            docker_build_command: String::new(),
            docker_build_timeout_seconds: DEFAULT_BUILD_TIMEOUT_SECONDS.to_string(),
            docker_image: String::new(),
            docker_container_name: String::new(),
            docker_run_command: String::new(),
        }
    }
}

// Request shapes, field names are the API's
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkspaceRequest {
    pub name: String,
    pub directory: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<WorkspaceEnvRequest>,
    pub ready_log_line: String,
    pub ready_timeout_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load: Option<LoadRequest>,
    pub idle_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native: Option<NativeRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker: Option<DockerRequest>,
}

// Exactly one of the two is set
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkspaceEnvRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pasted: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LoadRequest {
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NativeRequest {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_command: Option<String>,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DockerRequest {
    pub enabled: bool,
    pub build_command: String,
    pub build_timeout_seconds: u64,
    pub image: String,
    pub container_name: String,
    pub run_command: String,
}

// One message for the whole section, it names the field that is wrong
fn number(label: &str, value: &str, max: u64) -> Result<u64, String> {
    value
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|parsed| (1..=max).contains(parsed))
        .ok_or_else(|| format!("{label} must be a whole number from 1 to {max}."))
}

fn required(label: &str, value: &str) -> Result<String, String> {
    let value = value.trim();

    if value.is_empty() {
        Err(format!("Enter {label}."))
    } else {
        Ok(value.to_string())
    }
}

fn optional(value: &str) -> Option<String> {
    let value = value.trim();

    (!value.is_empty()).then(|| value.to_string())
}

// Relative to the project path and never leaving it, the same rule as the API
fn relative_directory(value: &str) -> bool {
    !value.starts_with('/')
        && !value.starts_with('\\')
        && !value.split(['/', '\\']).any(|part| part == "..")
}

// The API checks everything again, this only catches mistakes before a long run starts.
// None when the section is switched off, so the request carries no workspaces at all.
pub fn validate(form: &WorkspaceForm) -> Result<Option<WorkspaceRequest>, String> {
    if !form.enabled {
        return Ok(None);
    }

    let name = required("a workspace name", &form.name)?;
    let directory = required("the workspace directory", &form.directory)?;

    if !relative_directory(&directory) {
        return Err(
            "The workspace directory must be relative to the project path, without '..'."
                .to_string(),
        );
    }

    if !form.native_enabled && !form.docker_enabled {
        return Err("Switch on native or Docker, a workspace needs at least one.".to_string());
    }

    let ready_log_line = required("the ready log line", &form.ready_log_line)?;
    let ready_timeout_seconds = number(
        "The ready timeout",
        &form.ready_timeout_seconds,
        24 * 60 * 60,
    )?;
    let idle_seconds = number("The idle time", &form.idle_seconds, 60 * 60)?;

    let env = match form.env_source {
        EnvSource::None => None,
        EnvSource::File => Some(WorkspaceEnvRequest {
            file_path: Some(required("the env file path", &form.env_file_path)?),
            pasted: None,
        }),
        EnvSource::Pasted => {
            if form.env_pasted.trim().is_empty() {
                return Err("Paste the contents of the .env file.".to_string());
            }

            Some(WorkspaceEnvRequest {
                file_path: None,
                pasted: Some(form.env_pasted.clone()),
            })
        }
    };

    let native = if form.native_enabled {
        Some(NativeRequest {
            enabled: true,
            build_command: optional(&form.native_build_command),
            command: required("the native start command", &form.native_command)?,
        })
    } else {
        None
    };

    let docker = if form.docker_enabled {
        let run_command = required("the Docker run command", &form.docker_run_command)?;
        let has_placeholder = run_command.contains(ENV_FILE_PLACEHOLDER);

        if env.is_some() && !has_placeholder {
            return Err(format!(
                "The Docker run command must contain {ENV_FILE_PLACEHOLDER} when an env is given."
            ));
        }

        if env.is_none() && has_placeholder {
            return Err(format!(
                "The Docker run command contains {ENV_FILE_PLACEHOLDER} but no env is given."
            ));
        }

        Some(DockerRequest {
            enabled: true,
            build_command: required("the Docker build command", &form.docker_build_command)?,
            build_timeout_seconds: number(
                "The Docker build timeout",
                &form.docker_build_timeout_seconds,
                24 * 60 * 60,
            )?,
            image: required("the Docker image name", &form.docker_image)?,
            container_name: required("the Docker container name", &form.docker_container_name)?,
            run_command,
        })
    } else {
        None
    };

    Ok(Some(WorkspaceRequest {
        name,
        directory,
        env,
        ready_log_line,
        ready_timeout_seconds,
        load: optional(&form.load_command).map(|command| LoadRequest { command }),
        idle_seconds,
        native,
        docker,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete() -> WorkspaceForm {
        WorkspaceForm {
            enabled: true,
            name: "subscriber".to_string(),
            directory: "subscriber".to_string(),
            ready_log_line: "subscriber ready".to_string(),
            native_enabled: true,
            native_command: "bun run dev".to_string(),
            ..WorkspaceForm::default()
        }
    }

    fn with_docker() -> WorkspaceForm {
        WorkspaceForm {
            docker_enabled: true,
            docker_build_command: "docker compose build subscriber".to_string(),
            docker_image: "sub:latest".to_string(),
            docker_container_name: "sub".to_string(),
            docker_run_command: "docker run -d --name sub sub:latest".to_string(),
            ..complete()
        }
    }

    #[test]
    fn a_switched_off_section_adds_nothing() {
        assert_eq!(validate(&WorkspaceForm::default()), Ok(None));
    }

    #[test]
    fn builds_the_documented_workspace() {
        let request = validate(&with_docker()).unwrap().unwrap();
        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["name"], "subscriber");
        assert_eq!(json["ready_timeout_seconds"], 60);
        assert_eq!(json["idle_seconds"], 30);
        assert_eq!(json["native"]["command"], "bun run dev");
        assert!(json["native"].get("build_command").is_none());
        assert_eq!(json["docker"]["build_timeout_seconds"], 1800);
        assert!(json.get("env").is_none());
        assert!(json.get("load").is_none());
    }

    #[test]
    fn needs_native_or_docker_and_a_ready_line() {
        let neither = WorkspaceForm {
            native_enabled: false,
            ..complete()
        };
        let no_line = WorkspaceForm {
            ready_log_line: "  ".to_string(),
            ..complete()
        };

        assert!(validate(&neither).is_err());
        assert!(validate(&no_line).is_err());
    }

    #[test]
    fn the_directory_stays_inside_the_project() {
        for bad in ["/etc", "../x", "a/../../b"] {
            let form = WorkspaceForm {
                directory: bad.to_string(),
                ..complete()
            };

            assert!(validate(&form).is_err(), "{bad}");
        }
    }

    #[test]
    fn an_env_needs_the_placeholder_in_the_docker_run_command() {
        let without = WorkspaceForm {
            env_source: EnvSource::Pasted,
            env_pasted: "A=1\n".to_string(),
            ..with_docker()
        };

        assert!(validate(&without).is_err());

        let with = WorkspaceForm {
            docker_run_command: "docker run --env-file {ENV_FILE} sub:latest".to_string(),
            ..without
        };
        let json = serde_json::to_value(validate(&with).unwrap().unwrap()).unwrap();

        assert_eq!(json["env"]["pasted"], "A=1\n");
        assert!(json["env"].get("file_path").is_none());
    }

    #[test]
    fn a_placeholder_without_an_env_is_refused() {
        let form = WorkspaceForm {
            docker_run_command: "docker run --env-file {ENV_FILE} sub:latest".to_string(),
            ..with_docker()
        };

        assert!(validate(&form).is_err());
    }

    #[test]
    fn an_env_file_path_is_sent_as_file_path_only() {
        let form = WorkspaceForm {
            env_source: EnvSource::File,
            env_file_path: "subscriber/.env".to_string(),
            ..complete()
        };
        let json = serde_json::to_value(validate(&form).unwrap().unwrap()).unwrap();

        assert_eq!(json["env"]["file_path"], "subscriber/.env");
        assert!(json["env"].get("pasted").is_none());
    }

    #[test]
    fn numbers_are_checked() {
        let form = WorkspaceForm {
            idle_seconds: "0".to_string(),
            ..complete()
        };

        assert!(validate(&form).is_err());
    }
}
