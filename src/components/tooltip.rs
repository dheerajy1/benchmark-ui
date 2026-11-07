use dioxus::prelude::*;

// Dark tooltip under its trigger, shown on hover and keyboard focus with no script.
// `hint` is an optional shortcut such as Ctrl+B.
#[component]
pub fn Tooltip(
    #[props(into)] text: String,
    #[props(into, default)] hint: String,
    #[props(into, default)] class: String,
    // Starts at the left edge of the trigger instead of centering, for triggers near an edge
    #[props(default)] start: bool,
    // Hides the tooltip, for example while a floating panel covers its place
    #[props(default)] suppressed: bool,
    children: Element,
) -> Element {
    let position_class = if start {
        "left-0"
    } else {
        "left-1/2 -translate-x-1/2"
    };

    let visibility_class = if suppressed { "hidden" } else { "" };

    rsx! {
        span { class: "group/tip relative inline-flex {class}",
            {children}
            span {
                role: "tooltip",
                class: "pointer-events-none absolute top-full {position_class} z-50 {visibility_class} mt-(--space-2) flex items-center gap-(--space-2) rounded-(--round-md) bg-(--clr-ink) px-(--space-3) py-(--space-1) text-(length:--fs-xs) whitespace-nowrap text-(color:--clr-ink-contrast) opacity-0 transition-opacity duration-150 group-hover/tip:opacity-100 group-focus-within/tip:opacity-100",
                "{text}"
                if !hint.is_empty() {
                    span { class: "rounded-(--round-sm) bg-white/15 px-(--space-1) text-(length:--fs-2xs)",
                        "{hint}"
                    }
                }
            }
        }
    }
}
