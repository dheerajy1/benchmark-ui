use dioxus::prelude::*;

use super::{Icon, IconKind};

// Row of equal segments with a soft thumb that slides to the selected one. A disabled segment
// stays visible but cannot be picked.
#[component]
pub fn SegmentSwitch(
    #[props(into)] label: String,
    options: Vec<(&'static str, bool)>,
    selected: usize,
    #[props(default = 7.0)] width_rem: f64,
    // Segment shown with a lock, `lock_hint` is its tooltip
    #[props(default)] locked: Option<usize>,
    #[props(into, default)] lock_hint: String,
    on_pick: EventHandler<usize>,
) -> Element {
    let offset = selected as f64 * width_rem;

    let segments: Vec<(usize, &'static str, bool, &'static str, String)> = options
        .into_iter()
        .enumerate()
        .map(|(index, (name, disabled))| {
            let class = if index == selected {
                "font-(weight:--fw-medium) text-(color:--clr-accent)"
            } else if disabled {
                "cursor-not-allowed text-(color:--clr-text-faint)"
            } else {
                "cursor-pointer text-(color:--clr-text-muted) hover:text-(color:--clr-text)"
            };

            let tip = if locked == Some(index) {
                lock_hint.clone()
            } else {
                String::new()
            };

            (index, name, disabled, class, tip)
        })
        .collect();

    rsx! {
        div {
            class: "relative inline-flex shrink-0 rounded-(--round-full) border border-(color:--clr-border) bg-(--clr-surface) p-1",
            role: "group",
            "aria-label": "{label}",
            span {
                class: "pointer-events-none absolute top-1 bottom-1 left-1 rounded-(--round-full) bg-(--clr-accent-soft) transition-transform duration-350 ease-(--ease-out)",
                style: "width: {width_rem}rem; transform: translateX({offset}rem)",
            }
            for (index , name , disabled , class , tip) in segments {
                button {
                    key: "{name}",
                    r#type: "button",
                    class: "relative z-10 rounded-(--round-full) py-(--space-1) text-center text-(length:--fs-sm) transition-colors {class}",
                    style: "width: {width_rem}rem",
                    "aria-pressed": "{index == selected}",
                    title: "{tip}",
                    disabled,
                    onclick: move |_| on_pick.call(index),
                    if locked == Some(index) {
                        Icon { kind: IconKind::Lock, class: "mr-(--space-1) mb-px inline size-3.5" }
                    }
                    "{name}"
                }
            }
        }
    }
}
