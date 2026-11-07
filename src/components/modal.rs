use dioxus::prelude::*;

use super::{Icon, IconKind};

// Centered dialog over a blurred backdrop. The X, a click on the backdrop and Esc all call
// `on_close`.
#[component]
pub fn Modal(
    #[props(into)] title: String,
    on_close: EventHandler<()>,
    #[props(into, default = "max-w-xl".to_string())] width: String,
    children: Element,
) -> Element {
    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center p-(--space-4)",
            div {
                class: "backdrop-in absolute inset-0 bg-(--clr-ink)/30 backdrop-blur-sm",
                onclick: move |_| on_close.call(()),
            }
            div {
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "{title}",
                tabindex: "-1",
                class: "pop-in relative flex max-h-full w-full flex-col rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) shadow-(--shadow-popover) focus:outline-none {width}",
                onmounted: move |event| async move {
                    let _ = event.set_focus(true).await;
                },
                onkeydown: move |event| {
                    if event.key() == Key::Escape {
                        on_close.call(());
                    }
                },
                header { class: "flex items-center justify-between gap-(--space-3) border-b border-(color:--clr-border) px-(--space-4) py-(--space-3)",
                    h2 { class: "text-(length:--fs-base) font-(weight:--fw-semibold) text-(color:--clr-text)",
                        "{title}"
                    }
                    button {
                        r#type: "button",
                        class: "inline-flex size-8 cursor-pointer items-center justify-center rounded-(--round-md) text-(color:--clr-text-muted) transition-colors hover:bg-(--clr-surface-hover) hover:text-(color:--clr-text)",
                        "aria-label": "Close",
                        onclick: move |_| on_close.call(()),
                        Icon { kind: IconKind::Close, class: "size-(--icon-md)" }
                    }
                }
                div { class: "overflow-y-auto p-(--space-4)", {children} }
            }
        }
    }
}
