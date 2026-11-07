use dioxus::prelude::*;

// Pulsing grey block, give it the exact size of the content it stands in for
#[component]
pub fn Skeleton(#[props(into, default)] class: String) -> Element {
    rsx! {
        div { class: "skeleton rounded-(--round-sm) {class}", "aria-hidden": "true" }
    }
}
