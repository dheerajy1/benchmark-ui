use gloo_net::http::{Request, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{
    error::ApiError,
    status::{mark, ConnectionState},
};

// Used when API_BASE_URL was not provided at build time, matches the backend's documented APP_PORT
const DEFAULT_API_BASE_URL: &str = "http://127.0.0.1:5001";

// Every documented endpoint lives below this prefix
const API_PREFIX: &str = "/api/v1";

// Base URL baked in at build time from API_BASE_URL, never contains secrets
pub fn base_url() -> String {
    option_env!("API_BASE_URL")
        .unwrap_or(DEFAULT_API_BASE_URL)
        .trim()
        .trim_end_matches('/')
        .to_string()
}

// { success: true, httpCode, message, data }
#[derive(Deserialize)]
struct SuccessEnvelope<T> {
    data: T,
}

// { success: false, httpCode, code, message, error }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorEnvelope {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

fn endpoint(path: &str) -> Result<String, ApiError> {
    let base = base_url();

    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Err(ApiError::Config(format!(
            "API_BASE_URL must start with http:// or https://, got \"{base}\"."
        )));
    }

    Ok(format!("{base}{API_PREFIX}{path}"))
}

fn network_error(error: gloo_net::Error) -> ApiError {
    mark(ConnectionState::Offline);

    ApiError::Network {
        base_url: base_url(),
        detail: error.to_string(),
    }
}

// Sends a GET request and returns the `data` member of the success envelope
pub async fn get_json<T: DeserializeOwned>(
    path: &str,
    query: &[(&str, String)],
) -> Result<T, ApiError> {
    let url = endpoint(path)?;

    let mut builder = Request::get(&url);
    if !query.is_empty() {
        builder = builder.query(query.iter().map(|(key, value)| (*key, value.as_str())));
    }

    let response = builder.send().await.map_err(network_error)?;

    read_response(response).await
}

// Sends a JSON POST request and returns the `data` member of the success envelope
pub async fn post_json<B: Serialize, T: DeserializeOwned>(
    path: &str,
    body: &B,
) -> Result<T, ApiError> {
    let url = endpoint(path)?;

    let payload = serde_json::to_string(body)
        .map_err(|error| ApiError::Malformed(format!("Could not encode the request: {error}")))?;

    let request = Request::post(&url)
        .header("Content-Type", "application/json")
        .body(payload)
        .map_err(|error| ApiError::Malformed(format!("Could not build the request: {error}")))?;

    let response = request.send().await.map_err(network_error)?;

    read_response(response).await
}

async fn read_response<T: DeserializeOwned>(response: Response) -> Result<T, ApiError> {
    // Any HTTP response proves the API is reachable, even an error status
    mark(ConnectionState::Online);

    let status = response.status();
    let ok = response.ok();

    let body = response
        .text()
        .await
        .map_err(|error| ApiError::Malformed(format!("The body could not be read: {error}")))?;

    decode_body(status, ok, &body)
}

// Pure decoding step, kept separate so it can be tested without a browser
pub fn decode_body<T: DeserializeOwned>(status: u16, ok: bool, body: &str) -> Result<T, ApiError> {
    if ok {
        let envelope: SuccessEnvelope<T> = serde_json::from_str(body)
            .map_err(|error| ApiError::Malformed(format!("HTTP {status}: {error}.")))?;

        return Ok(envelope.data);
    }

    match serde_json::from_str::<ErrorEnvelope>(body) {
        Ok(envelope) => Err(ApiError::Http {
            status,
            code: envelope.code,
            message: envelope
                .message
                .unwrap_or_else(|| format!("The API answered with HTTP {status}.")),
            detail: envelope.error,
        }),
        Err(_) => Err(ApiError::Http {
            status,
            code: None,
            message: format!("The API answered with HTTP {status}."),
            detail: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Payload {
        value: u32,
    }

    #[test]
    fn decodes_success_envelope() {
        let body = r#"{"success":true,"httpCode":200,"message":"ok","data":{"value":7}}"#;

        let payload: Payload = decode_body(200, true, body).expect("decodes");

        assert_eq!(payload, Payload { value: 7 });
    }

    #[test]
    fn decodes_documented_error_envelope() {
        let body = r#"{"success":false,"httpCode":404,"code":"NOT_FOUND","message":"Run not found.","error":"no such run"}"#;

        let error = decode_body::<Payload>(404, false, body).expect_err("is an error");

        assert_eq!(
            error,
            ApiError::Http {
                status: 404,
                code: Some("NOT_FOUND".to_string()),
                message: "Run not found.".to_string(),
                detail: Some("no such run".to_string()),
            }
        );
    }

    #[test]
    fn reports_non_json_error_bodies() {
        let error = decode_body::<Payload>(502, false, "bad gateway").expect_err("is an error");

        assert!(matches!(error, ApiError::Http { status: 502, .. }));
    }

    #[test]
    fn rejects_success_bodies_that_break_the_contract() {
        let error = decode_body::<Payload>(200, true, r#"{"data":{"value":"x"}}"#)
            .expect_err("is an error");

        assert!(matches!(error, ApiError::Malformed(_)));
    }

    #[test]
    fn default_base_url_has_no_trailing_slash() {
        assert!(!base_url().ends_with('/'));
    }
}
