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

pub fn mark(state: ConnectionState) {
    if *CONNECTION.peek() != state {
        *CONNECTION.write() = state;
    }
}
