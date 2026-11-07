use dioxus::prelude::*;

// Bordered surface that groups related content
#[component]
pub fn Card(#[props(into, default)] class: String, children: Element) -> Element {
    rsx! {
        section {
            class: "min-w-0 rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) {class}",
            {children}
        }
    }
}

// Title row of a card, `subtitle` is hidden when empty. `trailing` sits at the right end.
#[component]
pub fn CardHeader(
    #[props(into)] title: String,
    #[props(into, default)] subtitle: String,
    #[props(default)] trailing: Option<Element>,
) -> Element {
    rsx! {
        div { class: "flex items-center justify-between gap-(--space-3) border-b border-(color:--clr-border) px-(--space-4) py-(--space-3)",
            div { class: "flex min-w-0 flex-col gap-(--space-1)",
                h2 { class: "text-(length:--fs-base) font-(weight:--fw-semibold) text-(color:--clr-text)",
                    "{title}"
                }
                if !subtitle.is_empty() {
                    p { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)", "{subtitle}" }
                }
            }
            if let Some(trailing) = trailing {
                {trailing}
            }
        }
    }
}
