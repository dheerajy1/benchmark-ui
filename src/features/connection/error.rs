use std::fmt;

use crate::components::ErrorKind;

// Where an error came from: the request line and the id the API gave the request
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RequestInfo {
    // For example `GET /api/v1/benchmarks?limit=20`
    pub line: String,
    // The `x-request-id` of the answer, the same id is in the API log
    pub request_id: Option<String>,
}

// Everything that can go wrong while talking to benchmark-api
#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    // API_BASE_URL was not set when the bundle was built
    NotConfigured,
    // The frontend configuration is unusable, for example a malformed API_BASE_URL
    Config(String),
    // The request never produced an HTTP response, the API is down or the browser blocked it
    Network {
        base_url: String,
        detail: String,
        request: RequestInfo,
    },
    // The API answered with an error status and, normally, the documented error body
    Http {
        status: u16,
        code: Option<String>,
        message: String,
        detail: Option<String>,
        request: RequestInfo,
    },
    // The API answered with a body that does not match the documented contract
    Malformed {
        detail: String,
        request: RequestInfo,
    },
}

impl ApiError {
    pub fn title(&self) -> String {
        match self {
            Self::NotConfigured => "API address is not set".to_string(),
            Self::Config(_) => "Frontend configuration error".to_string(),
            Self::Network { .. } => "API unreachable".to_string(),
            Self::Http { status: 404, .. } => "Not found".to_string(),
            Self::Http { status, .. } if (400..500).contains(status) => {
                "Request rejected".to_string()
            }
            Self::Http { .. } => "The API hit an error".to_string(),
            Self::Malformed { .. } => "Unexpected API response".to_string(),
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::NotConfigured => "Set API_BASE_URL and restart.".to_string(),
            Self::Config(message) => message.clone(),
            Self::Network { base_url, .. } => format!(
                "Could not reach benchmark-api at {base_url}. Check that it is running and that it allows requests from this origin (CORS)."
            ),
            Self::Http {
                message, detail, ..
            } => match detail {
                Some(detail) if !detail.is_empty() && detail != message => {
                    format!("{message} {detail}")
                }
                _ => message.clone(),
            },
            Self::Malformed { detail, .. } => format!(
                "The API returned a response this dashboard could not read. {detail}"
            ),
        }
    }

    // Color of the page error: red = we could not get the data, amber = temporary or
    // recoverable, grey = it does not exist
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Network { .. } => ErrorKind::Amber,
            Self::Http { status: 404, .. } => ErrorKind::Grey,
            _ => ErrorKind::Red,
        }
    }

    fn request(&self) -> Option<&RequestInfo> {
        match self {
            Self::Network { request, .. }
            | Self::Http { request, .. }
            | Self::Malformed { request, .. } => Some(request),
            Self::NotConfigured | Self::Config(_) => None,
        }
    }

    pub fn request_id(&self) -> Option<&str> {
        self.request()
            .and_then(|request| request.request_id.as_deref())
    }

    // Text of the folded `Technical details` and of `Copy details`: request line, status or
    // network error, request id and time. `at` is the time the error was shown.
    pub fn details_text(&self, at: &str) -> String {
        let mut lines: Vec<String> = Vec::new();

        if let Some(request) = self.request().filter(|request| !request.line.is_empty()) {
            lines.push(request.line.clone());
        }

        lines.push(match self {
            Self::NotConfigured => "API_BASE_URL is not set".to_string(),
            Self::Config(message) => format!("Configuration error: {message}"),
            Self::Network { detail, .. } => format!("Network error: {detail}"),
            Self::Http {
                status,
                code,
                message,
                ..
            } => match code {
                Some(code) => format!("HTTP {status} {code}: {message}"),
                None => format!("HTTP {status}: {message}"),
            },
            Self::Malformed { detail, .. } => format!("Unexpected response: {detail}"),
        });

        if let Some(request_id) = self.request_id() {
            lines.push(format!("Request id: {request_id}"));
        }

        lines.push(format!("Time: {at}"));

        lines.join("\n")
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::Http { status: 404, .. })
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> RequestInfo {
        RequestInfo {
            line: "GET /api/v1/benchmarks?limit=20".to_string(),
            request_id: Some("a3f09c1e".to_string()),
        }
    }

    fn http(status: u16) -> ApiError {
        ApiError::Http {
            status,
            code: Some("INTERNAL_SERVER_ERROR".to_string()),
            message: "Something broke.".to_string(),
            detail: None,
            request: request(),
        }
    }

    #[test]
    fn network_error_mentions_cors() {
        let error = ApiError::Network {
            base_url: "http://127.0.0.1:5001".to_string(),
            detail: "Failed to fetch".to_string(),
            request: RequestInfo::default(),
        };

        assert!(error.description().contains("CORS"));
        assert_eq!(error.title(), "API unreachable");
        assert_eq!(error.kind(), ErrorKind::Amber);
    }

    #[test]
    fn http_error_combines_message_and_detail() {
        let error = ApiError::Http {
            status: 400,
            code: Some("VALIDATION_ERROR".to_string()),
            message: "Invalid request.".to_string(),
            detail: Some("limit must be between 1 and 100.".to_string()),
            request: request(),
        };

        assert_eq!(
            error.description(),
            "Invalid request. limit must be between 1 and 100."
        );
        assert_eq!(error.title(), "Request rejected");
        assert_eq!(error.kind(), ErrorKind::Red);
    }

    #[test]
    fn server_errors_and_missing_runs_have_their_own_look() {
        assert_eq!(http(500).title(), "The API hit an error");
        assert_eq!(http(500).kind(), ErrorKind::Red);
        assert_eq!(http(404).title(), "Not found");
        assert_eq!(http(404).kind(), ErrorKind::Grey);
        assert!(http(404).is_not_found());
    }

    #[test]
    fn a_missing_address_names_the_fix() {
        let error = ApiError::NotConfigured;

        assert_eq!(error.description(), "Set API_BASE_URL and restart.");
        assert_eq!(error.request_id(), None);
    }

    #[test]
    fn details_list_request_status_id_and_time() {
        assert_eq!(
            http(500).details_text("2026-11-05T09:33:00.000Z"),
            "GET /api/v1/benchmarks?limit=20\nHTTP 500 INTERNAL_SERVER_ERROR: Something broke.\nRequest id: a3f09c1e\nTime: 2026-11-05T09:33:00.000Z"
        );
    }

    #[test]
    fn details_of_a_network_error_have_no_request_id() {
        let error = ApiError::Network {
            base_url: "http://x".to_string(),
            detail: "Failed to fetch".to_string(),
            request: RequestInfo {
                line: "GET /api/v1/health".to_string(),
                request_id: None,
            },
        };

        assert_eq!(
            error.details_text("t"),
            "GET /api/v1/health\nNetwork error: Failed to fetch\nTime: t"
        );
    }
}
