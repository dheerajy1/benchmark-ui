use dioxus::prelude::*;

// Shared class string for text inputs and selects
pub const INPUT_CLASS: &str = "h-(--control-height) w-full rounded-(--round-md) border border-(color:--clr-border-strong) bg-(--clr-canvas) px-(--space-3) text-(length:--fs-sm) text-(color:--clr-text) transition-colors hover:border-(color:--clr-text-faint) focus:border-(color:--clr-accent) placeholder:text-(color:--clr-text-faint) disabled:cursor-not-allowed disabled:opacity-50";

// Label, control, hint and error message wired together for assistive technology
#[component]
pub fn FormField(
    #[props(into)] label: String,
    #[props(into, default)] hint: String,
    #[props(into, default)] error: String,
    children: Element,
) -> Element {
    // A failed field turns its control border red, the message below carries the meaning
    let invalid_class = if error.is_empty() {
        ""
    } else {
        "[&_input]:border-(color:--clr-danger) [&_select]:border-(color:--clr-danger)"
    };

    rsx! {
        label { class: "flex min-w-0 flex-col gap-(--space-1) {invalid_class}",
            span { class: "text-(length:--fs-xs) font-(weight:--fw-medium) text-(color:--clr-text-muted)",
                "{label}"
            }
            {children}
            if !error.is_empty() {
                span {
                    class: "text-(length:--fs-xs) text-(color:--clr-danger)",
                    role: "alert",
                    "{error}"
                }
            } else if !hint.is_empty() {
                span { class: "text-(length:--fs-xs) text-(color:--clr-text-faint)", "{hint}" }
            }
        }
    }
}
