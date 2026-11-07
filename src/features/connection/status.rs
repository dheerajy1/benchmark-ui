use dioxus::prelude::*;

// Last observed reachability of benchmark-api
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    // No request has completed yet
    Unknown,
    // The last request received an HTTP response, whatever its status
    Online,
    // The last request failed before any response arrived
    Offline,
}

// Updated by the HTTP client after every request, read by the top bar
pub static CONNECTION: GlobalSignal<ConnectionState> = Signal::global(|| ConnectionState::Unknown);

// The `x-request-id` of the newest answer, copied with the diagnostics
pub static LAST_REQUEST_ID: GlobalSignal<Option<String>> = Signal::global(|| None);

pub fn mark(state: ConnectionState) {
    if *CONNECTION.peek() != state {
        *CONNECTION.write() = state;
    }
}

pub fn remember_request_id(request_id: Option<&str>) {
    if let Some(request_id) = request_id {
        if LAST_REQUEST_ID.peek().as_deref() != Some(request_id) {
            *LAST_REQUEST_ID.write() = Some(request_id.to_string());
        }
    }
}
