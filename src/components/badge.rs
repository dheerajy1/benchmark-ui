use dioxus::prelude::*;

use super::Tone;

// Small pill used for runtimes and statuses
#[component]
pub fn Badge(#[props(into)] label: String, #[props(default)] tone: Tone) -> Element {
    let tone_class = tone.badge_class();

    rsx! {
        span {
            class: "inline-flex items-center gap-(--space-1) rounded-(--round-full) px-(--space-2) py-(--space-1) text-(length:--fs-xs) font-(weight:--fw-medium) whitespace-nowrap {tone_class}",
            "{label}"
        }
    }
}
