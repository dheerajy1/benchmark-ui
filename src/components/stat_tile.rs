use dioxus::prelude::*;

use super::Tone;

// One headline number with a label and an optional hint line
#[component]
pub fn StatTile(
    #[props(into)] label: String,
    #[props(into)] value: String,
    #[props(into, default)] hint: String,
    #[props(default)] tone: Tone,
) -> Element {
    let hint_class = tone.text_class();

    rsx! {
        div { class: "flex min-w-0 flex-col gap-(--space-1) rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) p-(--space-4) shadow-(--shadow-card)",
            span { class: "text-(length:--fs-xs) font-(weight:--fw-medium) uppercase tracking-wider text-(color:--clr-text-faint)",
                "{label}"
            }
            span { class: "truncate text-(length:--fs-2xl) font-(weight:--fw-semibold) tabular-nums text-(color:--clr-text)",
                "{value}"
            }
            if !hint.is_empty() {
                span { class: "truncate text-(length:--fs-xs) {hint_class}", "{hint}" }
            }
        }
    }
}
