use dioxus::prelude::*;

use super::INPUT_CLASS;

// Number input with a minus and a plus button. The value stays text so a half typed number is
// never rewritten, the buttons count from the last number that was typed.
#[component]
pub fn Stepper(
    value: String,
    #[props(default)] changed: bool,
    #[props(default)] disabled: bool,
    #[props(into)] label: String,
    on_change: EventHandler<String>,
) -> Element {
    let current = value.trim().parse::<u64>().unwrap_or(0);
    let tint = if changed {
        "bg-(--clr-accent-soft)!"
    } else {
        ""
    };
    let button = "inline-flex size-(--control-height) shrink-0 cursor-pointer items-center justify-center rounded-(--round-md) border border-(color:--clr-border-strong) bg-(--clr-surface) text-(length:--fs-base) text-(color:--clr-text-muted) transition-colors hover:bg-(--clr-surface-hover) hover:text-(color:--clr-text) disabled:cursor-not-allowed disabled:opacity-50";

    rsx! {
        div { class: "flex items-center gap-(--space-2)",
            button {
                r#type: "button",
                class: "{button}",
                disabled: disabled || current <= 1,
                "aria-label": "Decrease {label}",
                onclick: move |_| on_change.call(current.saturating_sub(1).max(1).to_string()),
                "−"
            }
            input {
                r#type: "text",
                inputmode: "numeric",
                class: "{INPUT_CLASS} text-center tabular-nums {tint}",
                value: "{value}",
                disabled,
                "aria-label": "{label}",
                oninput: move |event| on_change.call(event.value()),
            }
            button {
                r#type: "button",
                class: "{button}",
                disabled,
                "aria-label": "Increase {label}",
                onclick: move |_| on_change.call((current + 1).to_string()),
                "+"
            }
        }
    }
}
