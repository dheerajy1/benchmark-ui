use dioxus::prelude::*;

use super::Tone;

// One headline number with a label and an optional hint line.
// `compact` is the quieter secondary size used for supporting measurements.
#[component]
pub fn StatTile(
    #[props(into)] label: String,
    #[props(into)] value: String,
    #[props(into, default)] hint: String,
    #[props(default)] tone: Tone,
    #[props(default)] compact: bool,
) -> Element {
    let hint_class = tone.text_class();
    // The verdict is also written out so the change never depends on color alone
    let verdict_word = match tone {
        Tone::Success => " · better",
        Tone::Danger => " · worse",
        _ => "",
    };
    let value_class = if compact {
        "text-(length:--fs-xl)"
    } else {
        "text-(length:--fs-2xl)"
    };

    rsx! {
        div { class: "flex min-w-0 flex-col gap-(--space-1) rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) px-(--space-4) py-(--space-3) shadow-(--shadow-card)",
            span { class: "text-(length:--fs-2xs) font-(weight:--fw-medium) uppercase tracking-wider text-(color:--clr-text-faint)",
                "{label}"
            }
            span { class: "truncate font-(weight:--fw-semibold) tabular-nums text-(color:--clr-text) {value_class}",
                "{value}"
            }
            if !hint.is_empty() {
                span { class: "truncate text-(length:--fs-xs) {hint_class}", "{hint}{verdict_word}" }
            }
        }
    }
}
