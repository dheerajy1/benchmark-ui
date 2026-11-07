use dioxus::prelude::*;

use super::{
    health::HEALTH,
    status::{ConnectionState, CONNECTION},
};

// Top bar pill with the API state. On hover it turns soft green and slides out the latency.
#[component]
pub fn ApiPill() -> Element {
    let state = *CONNECTION.read();
    let sample = HEALTH.read().clone();

    let database_down = sample.as_ref().is_some_and(|sample| !sample.database_ok);
    let latency = sample
        .as_ref()
        .map(|sample| format!("{} ms", sample.latency_ms));

    let (label, dot_class, hover_class) = match state {
        ConnectionState::Unknown => (
            "API",
            "bg-(--clr-text-faint) animate-pulse",
            "hover:bg-(--clr-surface-hover)",
        ),
        ConnectionState::Online if database_down => (
            "API · database down",
            "bg-(--clr-warning)",
            "hover:bg-(--clr-warning-soft)",
        ),
        ConnectionState::Online => (
            "API",
            "bg-(--clr-success) pulse-dot text-(color:--clr-success)",
            "hover:bg-(--clr-success-pill)",
        ),
        ConnectionState::Offline => (
            "API offline",
            "bg-(--clr-danger)",
            "hover:bg-(--clr-danger-soft)",
        ),
    };

    rsx! {
        div {
            class: "group/api flex h-8 min-w-0 items-center rounded-(--round-full) bg-(--clr-surface-hover) px-(--space-3) transition-colors duration-200 {hover_class}",
            role: "status",
            "aria-label": "{label}",
            span { class: "inline-block size-(--dot-size) shrink-0 rounded-(--round-full) {dot_class}" }
            span { class: "ml-(--space-2) shrink-0 text-(length:--fs-xs) font-(weight:--fw-medium) text-(color:--clr-text)",
                "{label}"
            }
            if let Some(latency) = latency {
                span { class: "max-w-0 overflow-hidden text-(length:--fs-xs) whitespace-nowrap tabular-nums text-(color:--clr-success-text) opacity-0 transition-all duration-400 ease-(--ease-out) group-hover/api:ml-(--space-2) group-hover/api:max-w-20 group-hover/api:opacity-100",
                    "{latency}"
                }
            }
        }
    }
}
