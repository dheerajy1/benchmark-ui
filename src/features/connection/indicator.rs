use dioxus::prelude::*;

use super::{
    client::base_url,
    status::{ConnectionState, CONNECTION},
};

// Top bar pill that reports whether benchmark-api answered the last request
#[component]
pub fn ConnectionIndicator() -> Element {
    let state = *CONNECTION.read();
    let url = base_url();

    let (label, dot_class, text_class) = match state {
        ConnectionState::Unknown => (
            "Checking API",
            "bg-(--clr-text-faint) animate-pulse",
            "text-(color:--clr-text-muted)",
        ),
        ConnectionState::Online => (
            "API connected",
            "bg-(--clr-success)",
            "text-(color:--clr-text)",
        ),
        ConnectionState::Offline => (
            "API unreachable",
            "bg-(--clr-danger)",
            "text-(color:--clr-danger)",
        ),
    };

    rsx! {
        div {
            class: "flex h-6 min-w-0 items-center gap-(--space-2) rounded-(--round-full) border border-(color:--clr-border) bg-(--clr-surface) px-(--space-3)",
            role: "status",
            title: "{url}",
            span { class: "inline-block size-(--dot-size) shrink-0 rounded-(--round-full) {dot_class}" }
            span { class: "shrink-0 text-(length:--fs-xs) font-(weight:--fw-medium) {text_class}",
                "{label}"
            }
            span { class: "hidden max-w-48 truncate border-l border-(color:--clr-border) pl-(--space-2) font-(family-name:--font-code) text-(length:--fs-2xs) text-(color:--clr-text-faint) md:inline xl:max-w-80",
                "{url}"
            }
        }
    }
}
