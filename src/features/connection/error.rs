use std::fmt;

// Everything that can go wrong while talking to benchmark-api
#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    // The frontend configuration is unusable, for example a malformed API_BASE_URL
    Config(String),
    // The request never produced an HTTP response, the API is down or the browser blocked it
    Network {
        base_url: String,
        detail: String,
    },
    // The API answered with an error status and, normally, the documented error body
    Http {
        status: u16,
        code: Option<String>,
        message: String,
        detail: Option<String>,
    },
    // The API answered with a body that does not match the documented contract
    Malformed(String),
}

impl ApiError {
    pub fn title(&self) -> String {
        match self {
            Self::Config(_) => "Frontend configuration error".to_string(),
            Self::Network { .. } => "API unreachable".to_string(),
            Self::Http { status: 404, .. } => "Not found".to_string(),
            Self::Http { status, .. } if (400..500).contains(status) => {
                "Request rejected".to_string()
            }
            Self::Http { .. } => "API error".to_string(),
            Self::Malformed(_) => "Unexpected API response".to_string(),
        }
    }

    pub fn description(&self) -> String {
        match self {
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
            Self::Malformed(detail) => format!(
                "The API returned a response this dashboard could not read. {detail}"
            ),
        }
    }

    // Technical detail for the collapsed section of the error panel
    pub fn technical_detail(&self) -> Option<String> {
        match self {
            Self::Network { detail, .. } => Some(detail.clone()),
            Self::Http { status, code, .. } => Some(match code {
                Some(code) => format!("HTTP {status} {code}"),
                None => format!("HTTP {status}"),
            }),
            Self::Malformed(detail) => Some(detail.clone()),
            Self::Config(_) => None,
        }
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

    #[test]
    fn network_error_mentions_cors() {
        let error = ApiError::Network {
            base_url: "http://127.0.0.1:5001".to_string(),
            detail: "Failed to fetch".to_string(),
        };

        assert!(error.description().contains("CORS"));
        assert_eq!(error.title(), "API unreachable");
    }

    #[test]
    fn http_error_combines_message_and_detail() {
        let error = ApiError::Http {
            status: 400,
            code: Some("VALIDATION_ERROR".to_string()),
            message: "Invalid request.".to_string(),
            detail: Some("limit must be between 1 and 100.".to_string()),
        };

        assert_eq!(
            error.description(),
            "Invalid request. limit must be between 1 and 100."
        );
        assert_eq!(error.title(), "Request rejected");
        assert_eq!(
            error.technical_detail().as_deref(),
            Some("HTTP 400 VALIDATION_ERROR")
        );
    }

    #[test]
    fn detects_not_found() {
        let error = ApiError::Http {
            status: 404,
            code: None,
            message: "missing".to_string(),
            detail: None,
        };

        assert!(error.is_not_found());
    }
}
