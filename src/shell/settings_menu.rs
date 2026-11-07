use dioxus::prelude::*;

use crate::{
    components::{copy_text, use_toasts, Icon, IconKind},
    features::connection::{base_url, HealthSample, HEALTH, LAST_REQUEST_ID},
};

const ROW: &str = "flex h-9 w-full cursor-pointer items-center gap-(--space-2) rounded-(--round-md) px-(--space-2) text-left text-(length:--fs-sm) text-(color:--clr-text) transition-colors hover:bg-(--clr-hover-wash)";

// Text copied by `Copy diagnostics`
fn diagnostics(
    app_version: &str,
    health: Option<&HealthSample>,
    address: &str,
    last_request_id: Option<&str>,
) -> String {
    let address = if address.is_empty() {
        "not set"
    } else {
        address
    };
    let last_request_id = last_request_id.unwrap_or("none");
    let api_version = health
        .and_then(|sample| sample.version.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let latency = health.map_or_else(
        || "unknown".to_string(),
        |sample| format!("{} ms", sample.latency_ms),
    );

    format!(
        "benchmark-ui {app_version}\nAPI version {api_version}\nAPI address {address}\nLatency {latency}\nLast request id {last_request_id}"
    )
}

// Content of the settings menu: the API address and the diagnostics. `on_close` runs after a copy.
#[component]
pub fn SettingsMenu(on_close: EventHandler<()>) -> Element {
    let toasts = use_toasts();
    let address = base_url();
    let address_for_copy = address.clone();
    let has_address = !address.is_empty();
    let shown_address = if has_address {
        address.clone()
    } else {
        "Not set".to_string()
    };

    rsx! {
        div { class: "p-(--space-2)",
            p { class: "px-(--space-2) pt-(--space-1) text-(length:--fs-xs) text-(color:--clr-text-muted)",
                "API address"
            }
            div { class: "flex items-center gap-(--space-2) px-(--space-2) pb-(--space-2)",
                code { class: "min-w-0 flex-1 truncate font-(family-name:--font-code) text-(length:--fs-xs) text-(color:--clr-text)",
                    title: "{shown_address}",
                    "{shown_address}"
                }
                if has_address {
                button {
                    r#type: "button",
                    class: "inline-flex size-8 shrink-0 cursor-pointer items-center justify-center rounded-(--round-md) text-(color:--clr-text-muted) transition-colors hover:bg-(--clr-hover-wash) hover:text-(color:--clr-text)",
                    "aria-label": "Copy API address",
                    onclick: move |_| {
                        copy_text(&address_for_copy);
                        toasts.show("API address copied");
                        on_close.call(());
                    },
                    Icon { kind: IconKind::Copy }
                }
                }
            }
            div { class: "my-(--space-1) border-t border-(color:--clr-border)" }
            button {
                r#type: "button",
                class: "{ROW}",
                onclick: move |_| {
                    let health = HEALTH.peek().clone();
                    let text = diagnostics(
                        env!("CARGO_PKG_VERSION"),
                        health.as_ref(),
                        &base_url(),
                        LAST_REQUEST_ID.peek().as_deref(),
                    );
                    copy_text(&text);
                    toasts.show("Diagnostics copied");
                    on_close.call(());
                },
                Icon { kind: IconKind::Copy }
                "Copy diagnostics"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_what_is_known_and_marks_the_rest_unknown() {
        let sample = HealthSample {
            latency_ms: 12,
            version: Some("0.1.0".to_string()),
            database_ok: true,
        };

        assert_eq!(
            diagnostics("0.2.0", Some(&sample), "http://127.0.0.1:5001", Some("a3f09c1e")),
            "benchmark-ui 0.2.0\nAPI version 0.1.0\nAPI address http://127.0.0.1:5001\nLatency 12 ms\nLast request id a3f09c1e"
        );
        assert_eq!(
            diagnostics("0.2.0", None, "", None),
            "benchmark-ui 0.2.0\nAPI version unknown\nAPI address not set\nLatency unknown\nLast request id none"
        );
    }
}
