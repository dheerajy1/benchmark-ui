use dioxus::prelude::*;

// Small on and off switch. `label` is read out as is by assistive technology.
#[component]
pub fn Switch(
    on: bool,
    #[props(default)] locked: bool,
    #[props(into)] label: String,
    on_toggle: EventHandler<()>,
) -> Element {
    let track = if on {
        "bg-(--clr-accent)"
    } else {
        "bg-(--clr-border-strong)"
    };
    let knob = if on { "translate-x-4" } else { "translate-x-0" };
    let cursor = if locked {
        "cursor-not-allowed opacity-60"
    } else {
        "cursor-pointer"
    };

    rsx! {
        button {
            r#type: "button",
            role: "switch",
            class: "relative h-5 w-9 shrink-0 rounded-(--round-full) transition-colors duration-200 {track} {cursor}",
            "aria-checked": "{on}",
            "aria-label": "{label}",
            disabled: locked,
            onclick: move |_| on_toggle.call(()),
            span { class: "absolute top-0.5 left-0.5 size-4 rounded-(--round-full) bg-white shadow-sm transition-transform duration-200 {knob}" }
        }
    }
}
