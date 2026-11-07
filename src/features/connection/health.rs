use dioxus::prelude::*;
use serde::Deserialize;

use super::{client::get_json, error::ApiError};

// `data` of GET /api/v1/health
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Health {
    version: String,
}

// Result of the newest health probe
#[derive(Debug, Clone, PartialEq)]
pub struct HealthSample {
    // Round trip of the probe in milliseconds, measured here
    pub latency_ms: u64,
    // API version, `None` when the API answered 503 because its database is unreachable
    pub version: Option<String>,
    pub database_ok: bool,
}

// Updated by `probe`, read by the API pill and the settings menu
pub static HEALTH: GlobalSignal<Option<HealthSample>> = Signal::global(|| None);

// Calls GET /health once and stores the latency. A network failure clears the sample, the
// connection state is already marked by the HTTP client.
pub async fn probe() {
    let started = js_sys::Date::now();
    let result = get_json::<Health>("/health", &[]).await;
    let latency_ms = (js_sys::Date::now() - started).max(0.0).round() as u64;

    let sample = match result {
        Ok(health) => Some(HealthSample {
            latency_ms,
            version: Some(health.version),
            database_ok: true,
        }),
        Err(ApiError::Http { status: 503, .. }) => Some(HealthSample {
            latency_ms,
            version: None,
            database_ok: false,
        }),
        Err(_) => None,
    };

    *HEALTH.write() = sample;
}
