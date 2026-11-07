use gloo_net::http::{Request, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{
    error::{ApiError, RequestInfo},
    status::{mark, remember_request_id, ConnectionState},
};

// Every documented endpoint lives below this prefix
const API_PREFIX: &str = "/api/v1";

// Base URL baked in at build time from API_BASE_URL, never contains secrets. Empty when it
// was not set, the UI then shows a full-page "set API_BASE_URL" error instead of guessing.
pub fn base_url() -> String {
    option_env!("API_BASE_URL")
        .unwrap_or_default()
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
    request_id: Option<String>,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

fn endpoint(path: &str) -> Result<String, ApiError> {
    let base = base_url();

    if base.is_empty() {
        return Err(ApiError::NotConfigured);
    }

    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Err(ApiError::Config(format!(
            "API_BASE_URL must start with http:// or https://, got \"{base}\"."
        )));
    }

    Ok(format!("{base}{API_PREFIX}{path}"))
}

// WebSocket URL of an API path, https becomes wss and http becomes ws
pub fn ws_endpoint(path: &str) -> Result<String, ApiError> {
    let url = endpoint(path)?;

    to_websocket_url(&url).ok_or_else(|| {
        ApiError::Config(format!(
            "API_BASE_URL must start with http:// or https://, got \"{}\".",
            base_url()
        ))
    })
}

fn to_websocket_url(url: &str) -> Option<String> {
    if let Some(rest) = url.strip_prefix("https://") {
        Some(format!("wss://{rest}"))
    } else {
        url.strip_prefix("http://")
            .map(|rest| format!("ws://{rest}"))
    }
}

// `GET /api/v1/benchmarks?limit=20`, the first line of the technical details
fn request_line(method: &str, path: &str, query: &[(&str, String)]) -> String {
    let mut line = format!("{method} {API_PREFIX}{path}");

    for (index, (key, value)) in query.iter().enumerate() {
        line.push(if index == 0 { '?' } else { '&' });
        line.push_str(&format!("{key}={value}"));
    }

    line
}

fn network_error(error: gloo_net::Error, line: String) -> ApiError {
    mark(ConnectionState::Offline);

    ApiError::Network {
        base_url: base_url(),
        detail: error.to_string(),
        request: RequestInfo {
            line,
            request_id: None,
        },
    }
}

fn malformed(detail: String, line: &str) -> ApiError {
    ApiError::Malformed {
        detail,
        request: RequestInfo {
            line: line.to_string(),
            request_id: None,
        },
    }
}

// Sends a GET request and returns the `data` member of the success envelope
pub async fn get_json<T: DeserializeOwned>(
    path: &str,
    query: &[(&str, String)],
) -> Result<T, ApiError> {
    let url = endpoint(path)?;
    let line = request_line("GET", path, query);

    let mut builder = Request::get(&url);
    if !query.is_empty() {
        builder = builder.query(query.iter().map(|(key, value)| (*key, value.as_str())));
    }

    let response = builder
        .send()
        .await
        .map_err(|error| network_error(error, line.clone()))?;

    read_response(response, line).await
}

// Sends a JSON POST request and returns the `data` member of the success envelope
pub async fn post_json<B: Serialize, T: DeserializeOwned>(
    path: &str,
    body: &B,
) -> Result<T, ApiError> {
    let url = endpoint(path)?;
    let line = request_line("POST", path, &[]);

    let payload = serde_json::to_string(body)
        .map_err(|error| malformed(format!("Could not encode the request: {error}"), &line))?;

    let request = Request::post(&url)
        .header("Content-Type", "application/json")
        .body(payload)
        .map_err(|error| malformed(format!("Could not build the request: {error}"), &line))?;

    let response = request
        .send()
        .await
        .map_err(|error| network_error(error, line.clone()))?;

    read_response(response, line).await
}

async fn read_response<T: DeserializeOwned>(
    response: Response,
    line: String,
) -> Result<T, ApiError> {
    // Any HTTP response proves the API is reachable, even an error status
    mark(ConnectionState::Online);

    let status = response.status();
    let ok = response.ok();

    // Readable by the page only when the API exposes the header through CORS
    let request_id = response.headers().get("x-request-id");
    remember_request_id(request_id.as_deref());

    let request = RequestInfo { line, request_id };

    let body = response.text().await.map_err(|error| ApiError::Malformed {
        detail: format!("The body could not be read: {error}"),
        request: request.clone(),
    })?;

    let result = decode_body(status, ok, &body, &request);

    // An id that only the error body carries is remembered here, not in the pure decoder
    if let Err(error) = &result {
        remember_request_id(error.request_id());
    }

    result
}

// Pure decoding step, kept separate so it can be tested without a browser
pub fn decode_body<T: DeserializeOwned>(
    status: u16,
    ok: bool,
    body: &str,
    request: &RequestInfo,
) -> Result<T, ApiError> {
    if ok {
        let envelope: SuccessEnvelope<T> =
            serde_json::from_str(body).map_err(|error| ApiError::Malformed {
                detail: format!("HTTP {status}: {error}."),
                request: request.clone(),
            })?;

        return Ok(envelope.data);
    }

    match serde_json::from_str::<ErrorEnvelope>(body) {
        Ok(envelope) => {
            // The header wins, the body copy covers APIs that do not expose the header
            let mut request = request.clone();
            if request.request_id.is_none() {
                request.request_id = envelope.request_id;
            }

            Err(ApiError::Http {
                status,
                code: envelope.code,
                message: envelope
                    .message
                    .unwrap_or_else(|| format!("The API answered with HTTP {status}.")),
                detail: envelope.error,
                request,
            })
        }
        Err(_) => Err(ApiError::Http {
            status,
            code: None,
            message: format!("The API answered with HTTP {status}."),
            detail: None,
            request: request.clone(),
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

    fn request() -> RequestInfo {
        RequestInfo {
            line: "GET /api/v1/benchmarks/x".to_string(),
            request_id: Some("abc123".to_string()),
        }
    }

    #[test]
    fn builds_the_request_line_with_its_query() {
        assert_eq!(
            request_line(
                "GET",
                "/benchmarks",
                &[("limit", "20".to_string()), ("offset", "40".to_string())]
            ),
            "GET /api/v1/benchmarks?limit=20&offset=40"
        );
        assert_eq!(
            request_line("POST", "/live-runs", &[]),
            "POST /api/v1/live-runs"
        );
    }

    #[test]
    fn takes_the_request_id_from_the_error_body_when_the_header_is_hidden() {
        let body =
            r#"{"success":false,"httpCode":500,"code":"X","message":"m","requestId":"from-body"}"#;
        let hidden = RequestInfo {
            line: "GET /api/v1/x".to_string(),
            request_id: None,
        };

        let error = decode_body::<Payload>(500, false, body, &hidden).expect_err("is an error");

        assert_eq!(error.request_id(), Some("from-body"));
    }

    #[test]
    fn switches_the_scheme_for_a_websocket() {
        assert_eq!(
            to_websocket_url("https://abc.trycloudflare.com/api/v1/live-runs/x/ws"),
            Some("wss://abc.trycloudflare.com/api/v1/live-runs/x/ws".to_string())
        );
        assert_eq!(
            to_websocket_url("http://127.0.0.1:5001/api/v1/live-runs/x/ws"),
            Some("ws://127.0.0.1:5001/api/v1/live-runs/x/ws".to_string())
        );
        assert_eq!(to_websocket_url("ftp://example.com"), None);
    }

    #[test]
    fn decodes_success_envelope() {
        let body = r#"{"success":true,"httpCode":200,"message":"ok","data":{"value":7}}"#;

        let payload: Payload = decode_body(200, true, body, &request()).expect("decodes");

        assert_eq!(payload, Payload { value: 7 });
    }

    #[test]
    fn decodes_documented_error_envelope() {
        let body = r#"{"success":false,"httpCode":404,"code":"NOT_FOUND","message":"Run not found.","error":"no such run"}"#;

        let error = decode_body::<Payload>(404, false, body, &request()).expect_err("is an error");

        assert_eq!(
            error,
            ApiError::Http {
                status: 404,
                code: Some("NOT_FOUND".to_string()),
                message: "Run not found.".to_string(),
                detail: Some("no such run".to_string()),
                request: request(),
            }
        );
    }

    #[test]
    fn reports_non_json_error_bodies() {
        let error =
            decode_body::<Payload>(502, false, "bad gateway", &request()).expect_err("is an error");

        assert!(matches!(error, ApiError::Http { status: 502, .. }));
    }

    #[test]
    fn rejects_success_bodies_that_break_the_contract() {
        let error = decode_body::<Payload>(200, true, r#"{"data":{"value":"x"}}"#, &request())
            .expect_err("is an error");

        assert!(matches!(error, ApiError::Malformed { .. }));
    }

    #[test]
    fn base_url_has_no_trailing_slash() {
        assert!(!base_url().ends_with('/'));
    }
}
