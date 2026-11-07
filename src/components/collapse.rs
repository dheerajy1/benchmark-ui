use dioxus::prelude::*;

// Content that folds and unfolds with a 0.3s height animation
#[component]
pub fn Collapse(open: bool, children: Element) -> Element {
    let rows = if open {
        "grid-rows-[1fr]"
    } else {
        "grid-rows-[0fr]"
    };

    rsx! {
        div {
            class: "grid transition-[grid-template-rows] duration-300 ease-(--ease-out) {rows}",
            "inert": if !open { "true" },
            div { class: "min-h-0 overflow-hidden", {children} }
        }
    }
}
