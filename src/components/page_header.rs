use dioxus::prelude::*;

// Page title block, `children` holds the primary actions on the right
#[component]
pub fn PageHeader(
    #[props(into)] title: String,
    #[props(into)] description: String,
    children: Element,
) -> Element {
    rsx! {
        header { class: "flex flex-col gap-(--space-4) pb-(--space-6) sm:flex-row sm:items-end sm:justify-between",
            div { class: "min-w-0",
                h1 { class: "text-(length:--fs-2xl) font-(weight:--fw-bold) tracking-tight text-(color:--clr-text)",
                    "{title}"
                }
                p { class: "mt-(--space-1) max-w-prose text-(length:--fs-base) text-(color:--clr-text-muted)",
                    "{description}"
                }
            }
            div { class: "flex flex-wrap items-center gap-(--space-2)", {children} }
        }
    }
}
