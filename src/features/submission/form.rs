use super::models::{
    BenchmarkRequest, CommandRequest, ReadinessRequest, StartupRequest, WorkloadRequest,
};

// Longest sustained workload the API accepts, mirrors the backend rule
const MAX_DURATION_SECONDS: u64 = 300;

// Only loopback hosts are accepted for readiness probes
const LOOPBACK_HOSTS: [&str; 3] = ["127.0.0.1", "localhost", "::1"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadMode {
    Requests,
    Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Path,
    Program,
    Host,
    Port,
    ReadinessPath,
    ExpectedStatus,
    WorkloadPath,
    Amount,
    Concurrency,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    pub field: Field,
    pub message: String,
}

// Raw text of every input, parsed and validated only on submit
#[derive(Debug, Clone, PartialEq)]
pub struct FormState {
    pub path: String,
    pub startup_enabled: bool,
    pub program: String,
    pub args: String,
    pub host: String,
    pub port: String,
    pub readiness_path: String,
    pub expected_status: String,
    pub workload_enabled: bool,
    pub workload_path: String,
    pub mode: WorkloadMode,
    pub amount: String,
    pub concurrency: String,
}

impl Default for FormState {
    fn default() -> Self {
        Self {
            path: String::new(),
            startup_enabled: false,
            program: String::new(),
            args: String::new(),
            host: "127.0.0.1".to_string(),
            port: String::new(),
            readiness_path: "/health".to_string(),
            expected_status: "200".to_string(),
            workload_enabled: false,
            workload_path: String::new(),
            mode: WorkloadMode::Requests,
            amount: String::new(),
            concurrency: String::new(),
        }
    }
}

pub fn error_for(errors: &[FieldError], field: Field) -> String {
    errors
        .iter()
        .find(|error| error.field == field)
        .map(|error| error.message.clone())
        .unwrap_or_default()
}

fn parse_number<T: std::str::FromStr>(value: &str) -> Option<T> {
    value.trim().parse().ok()
}

fn valid_url_path(value: &str) -> bool {
    value.starts_with('/')
        && !value
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
}

// Applies the same rules as the backend so mistakes are caught before a long benchmark starts
pub fn validate(state: &FormState) -> Result<BenchmarkRequest, Vec<FieldError>> {
    let mut errors: Vec<FieldError> = Vec::new();
    let mut fail = |field: Field, message: &str| {
        errors.push(FieldError {
            field,
            message: message.to_string(),
        });
    };

    let path = state.path.trim().to_string();
    if path.is_empty() {
        fail(Field::Path, "Enter the path of the project to benchmark.");
    }

    let mut startup = None;
    if state.startup_enabled {
        let program = state.program.trim().to_string();
        if program.is_empty() {
            fail(Field::Program, "Enter the program that starts the target.");
        }

        let host = state.host.trim().to_string();
        if !LOOPBACK_HOSTS
            .iter()
            .any(|allowed| host.eq_ignore_ascii_case(allowed))
        {
            fail(
                Field::Host,
                "Use a loopback host: 127.0.0.1, localhost or ::1.",
            );
        }

        let port = parse_number::<u16>(&state.port).filter(|port| *port > 0);
        if port.is_none() {
            fail(Field::Port, "Enter a port between 1 and 65535.");
        }

        let readiness_path = state.readiness_path.trim().to_string();
        if !valid_url_path(&readiness_path) {
            fail(
                Field::ReadinessPath,
                "Start with '/' and use no whitespace, for example /health.",
            );
        }

        let expected_status = parse_number::<u16>(&state.expected_status)
            .filter(|status| (100..=599).contains(status));
        if expected_status.is_none() {
            fail(
                Field::ExpectedStatus,
                "Enter an HTTP status between 100 and 599.",
            );
        }

        if let (Some(port), Some(expected_status)) = (port, expected_status) {
            startup = Some(StartupRequest {
                command: CommandRequest {
                    program,
                    args: state.args.split_whitespace().map(str::to_string).collect(),
                },
                readiness: ReadinessRequest {
                    readiness_type: "http".to_string(),
                    host,
                    port,
                    path: readiness_path,
                    method: "GET".to_string(),
                    expected_status,
                },
            });
        }
    }

    let mut workload = None;
    if state.workload_enabled {
        let workload_path = state.workload_path.trim().to_string();
        if !valid_url_path(&workload_path) {
            fail(
                Field::WorkloadPath,
                "Start with '/' and use no whitespace, for example /items.",
            );
        }

        let amount = parse_number::<u64>(&state.amount).filter(|amount| *amount > 0);
        match (amount, state.mode) {
            (None, WorkloadMode::Requests) => {
                fail(Field::Amount, "Enter a request count greater than 0.")
            }
            (None, WorkloadMode::Duration) => {
                fail(Field::Amount, "Enter a duration in seconds greater than 0.")
            }
            (Some(seconds), WorkloadMode::Duration) if seconds > MAX_DURATION_SECONDS => {
                fail(Field::Amount, "The duration must not exceed 300 seconds.")
            }
            _ => {}
        }

        let concurrency = parse_number::<u64>(&state.concurrency).filter(|value| *value > 0);
        if concurrency.is_none() {
            fail(Field::Concurrency, "Enter a concurrency greater than 0.");
        }

        if let (Some(amount), Some(concurrency)) = (amount, concurrency) {
            let within_limit =
                state.mode == WorkloadMode::Requests || amount <= MAX_DURATION_SECONDS;

            if within_limit && valid_url_path(&workload_path) {
                workload = Some(WorkloadRequest {
                    method: "GET".to_string(),
                    path: workload_path,
                    requests: (state.mode == WorkloadMode::Requests).then_some(amount),
                    duration_seconds: (state.mode == WorkloadMode::Duration).then_some(amount),
                    concurrency,
                });
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(BenchmarkRequest {
        path,
        startup,
        workload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_state() -> FormState {
        FormState {
            path: " /home/me/api ".to_string(),
            startup_enabled: true,
            program: "bun".to_string(),
            args: "run  start".to_string(),
            port: "5001".to_string(),
            workload_enabled: true,
            workload_path: "/items".to_string(),
            amount: "1000".to_string(),
            concurrency: "50".to_string(),
            ..FormState::default()
        }
    }

    #[test]
    fn only_the_path_is_required() {
        let state = FormState {
            path: "/home/me/api".to_string(),
            ..FormState::default()
        };

        let request = validate(&state).expect("valid");

        assert_eq!(request.path, "/home/me/api");
        assert!(request.startup.is_none());
        assert!(request.workload.is_none());
    }

    #[test]
    fn builds_the_documented_request() {
        let request = validate(&complete_state()).expect("valid");
        let json = serde_json::to_value(&request).expect("serializes");

        assert_eq!(json["path"], "/home/me/api");
        assert_eq!(json["startup"]["command"]["program"], "bun");
        assert_eq!(json["startup"]["command"]["args"][1], "start");
        assert_eq!(json["startup"]["readiness"]["type"], "http");
        assert_eq!(json["startup"]["readiness"]["port"], 5001);
        assert_eq!(json["workload"]["requests"], 1000);
        assert!(json["workload"].get("duration_seconds").is_none());
    }

    #[test]
    fn duration_mode_sends_only_duration() {
        let state = FormState {
            mode: WorkloadMode::Duration,
            amount: "30".to_string(),
            ..complete_state()
        };

        let json = serde_json::to_value(validate(&state).expect("valid")).expect("serializes");

        assert_eq!(json["workload"]["duration_seconds"], 30);
        assert!(json["workload"].get("requests").is_none());
    }

    #[test]
    fn rejects_an_empty_path() {
        let errors = validate(&FormState::default()).expect_err("invalid");

        assert_eq!(error_for(&errors, Field::Path).is_empty(), false);
    }

    #[test]
    fn mirrors_backend_rules() {
        let state = FormState {
            host: "example.com".to_string(),
            port: "0".to_string(),
            readiness_path: "health".to_string(),
            expected_status: "99".to_string(),
            workload_path: "items".to_string(),
            amount: "0".to_string(),
            concurrency: "0".to_string(),
            ..complete_state()
        };

        let errors = validate(&state).expect_err("invalid");

        for field in [
            Field::Host,
            Field::Port,
            Field::ReadinessPath,
            Field::ExpectedStatus,
            Field::WorkloadPath,
            Field::Amount,
            Field::Concurrency,
        ] {
            assert!(!error_for(&errors, field).is_empty(), "{field:?}");
        }
    }

    #[test]
    fn rejects_durations_over_the_limit() {
        let state = FormState {
            mode: WorkloadMode::Duration,
            amount: "301".to_string(),
            ..complete_state()
        };

        let errors = validate(&state).expect_err("invalid");

        assert!(!error_for(&errors, Field::Amount).is_empty());
    }
}
