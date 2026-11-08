// HTTP access to benchmark-api and the connection feedback shown in the shell

mod client;
mod environment;
mod error;
mod health;
mod indicator;
mod panel;
mod status;

pub use client::{base_url, delete_json, get_json, post_json, ws_endpoint};
pub use error::ApiError;
#[cfg(test)]
pub use error::RequestInfo;
pub use health::{probe, HealthSample, HEALTH};
pub use indicator::ApiPill;
pub use panel::ApiErrorPanel;
pub use status::{ConnectionState, CONNECTION, LAST_REQUEST_ID};
