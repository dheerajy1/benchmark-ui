use dioxus::prelude::*;

// Selectable pill used for filters and tabs, `count` is shown after the label when present
#[component]
pub fn Pill(
    #[props(into)] label: String,
    #[props(default)] count: Option<u64>,
    #[props(default)] active: bool,
    on_click: EventHandler<()>,
) -> Element {
    let state_class = if active {
        "border-(color:--clr-accent) bg-(--clr-accent-soft) text-(color:--clr-accent)"
    } else {
        "border-(color:--clr-border) bg-(--clr-surface) text-(color:--clr-text-muted) hover:bg-(--clr-surface-hover) hover:text-(color:--clr-text)"
    };
    let count_class = if active {
        "text-(color:--clr-accent)"
    } else {
        "text-(color:--clr-text-faint)"
    };

    rsx! {
        button {
            r#type: "button",
            class: "inline-flex h-8 cursor-pointer items-center gap-(--space-2) rounded-(--round-full) border px-(--space-3) text-(length:--fs-sm) font-(weight:--fw-medium) whitespace-nowrap transition-colors duration-200 focus-visible:outline-none focus-visible:shadow-(--halo-focus) {state_class}",
            "aria-pressed": "{active}",
            onclick: move |_| on_click.call(()),
            "{label}"
            if let Some(count) = count {
                span { class: "tabular-nums {count_class}", "{count}" }
            }
        }
    }
}
