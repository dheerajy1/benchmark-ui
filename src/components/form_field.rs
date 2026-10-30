use dioxus::prelude::*;

// Shared class string for text inputs and selects
pub const INPUT_CLASS: &str = "h-(--control-height) w-full rounded-(--round-md) border border-(color:--clr-border-strong) bg-(--clr-canvas) px-(--space-3) text-(length:--fs-sm) text-(color:--clr-text) placeholder:text-(color:--clr-text-faint) disabled:cursor-not-allowed disabled:opacity-50";

// Label, control, hint and error message wired together for assistive technology
#[component]
pub fn FormField(
    #[props(into)] label: String,
    #[props(into, default)] hint: String,
    #[props(into, default)] error: String,
    children: Element,
) -> Element {
    rsx! {
        label { class: "flex min-w-0 flex-col gap-(--space-1)",
            span { class: "text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text)",
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
