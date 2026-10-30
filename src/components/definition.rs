use dioxus::prelude::*;

// Label and value pair for detail grids, use inside a `dl`
#[component]
pub fn Definition(#[props(into)] label: String, children: Element) -> Element {
    rsx! {
        div { class: "flex min-w-0 flex-col gap-(--space-1)",
            dt { class: "text-(length:--fs-xs) font-(weight:--fw-medium) uppercase tracking-wider text-(color:--clr-text-faint)",
                "{label}"
            }
            dd { class: "min-w-0 break-words text-(length:--fs-base) text-(color:--clr-text)",
                {children}
            }
        }
    }
}
