use dioxus::prelude::*;

// Row under the top bar with a short description and the primary actions on the right.
// The title is read by assistive technology only, the top bar already shows it visually.
#[component]
pub fn PageHeader(
    #[props(into)] title: String,
    #[props(into)] description: String,
    children: Element,
) -> Element {
    rsx! {
        header { class: "flex flex-col gap-(--space-3) pb-(--space-5) sm:flex-row sm:items-center sm:justify-between",
            h1 { class: "sr-only", "{title}" }
            p { class: "min-w-0 text-(length:--fs-base) text-(color:--clr-text-muted)",
                "{description}"
            }
            div { class: "flex flex-wrap items-center gap-(--space-2)", {children} }
        }
    }
}
