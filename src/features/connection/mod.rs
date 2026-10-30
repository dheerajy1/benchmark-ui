// HTTP access to benchmark-api and the connection feedback shown in the shell

mod client;
mod error;
mod indicator;
mod panel;
mod status;

pub use client::{get_json, post_json};
pub use error::ApiError;
pub use indicator::ConnectionIndicator;
pub use panel::ApiErrorPanel;
