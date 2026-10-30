use dioxus::prelude::*;

use super::{button_class, ButtonVariant, Icon, IconKind};

// Placeholder shown while a request is in flight
#[component]
pub fn LoadingPanel(#[props(into)] label: String) -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-(--space-3) rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) p-(--space-5)",
            role: "status",
            "aria-live": "polite",
            span { class: "sr-only", "{label}" }
            div { class: "h-(--space-4) w-1/3 animate-pulse rounded-(--round-sm) bg-(--clr-surface-raised)" }
            div { class: "h-(--space-8) w-full animate-pulse rounded-(--round-sm) bg-(--clr-surface-raised)" }
            div { class: "h-(--space-8) w-5/6 animate-pulse rounded-(--round-sm) bg-(--clr-surface-raised)" }
            div { class: "h-(--space-8) w-2/3 animate-pulse rounded-(--round-sm) bg-(--clr-surface-raised)" }
        }
    }
}

// Shown when a request succeeded but there is nothing to display, `children` holds the next step
#[component]
pub fn EmptyState(
    #[props(into)] title: String,
    #[props(into)] description: String,
    children: Element,
) -> Element {
    rsx! {
        div { class: "flex flex-col items-center gap-(--space-3) rounded-(--round-lg) border border-dashed border-(color:--clr-border-strong) bg-(--clr-surface) px-(--space-6) py-(--space-12) text-center",
            h2 { class: "text-(length:--fs-lg) font-(weight:--fw-semibold) text-(color:--clr-text)",
                "{title}"
            }
            p { class: "max-w-md text-(length:--fs-base) text-(color:--clr-text-muted)",
                "{description}"
            }
            div { class: "mt-(--space-2) flex flex-wrap items-center justify-center gap-(--space-2)",
                {children}
            }
        }
    }
}

// Shown when a request failed, `detail` is optional technical information
#[component]
pub fn ErrorState(
    #[props(into)] title: String,
    #[props(into)] message: String,
    #[props(into, default)] detail: String,
    on_retry: EventHandler<()>,
) -> Element {
    let retry_class = button_class(ButtonVariant::Secondary);

    rsx! {
        div {
            class: "flex flex-col gap-(--space-3) rounded-(--round-lg) border border-(color:--clr-danger) bg-(--clr-danger-soft) p-(--space-5)",
            role: "alert",
            div { class: "flex items-start gap-(--space-3)",
                span { class: "mt-(--space-1) text-(color:--clr-danger)",
                    Icon { kind: IconKind::Alert }
                }
                div { class: "min-w-0 flex-1",
                    h2 { class: "text-(length:--fs-lg) font-(weight:--fw-semibold) text-(color:--clr-text)",
                        "{title}"
                    }
                    p { class: "mt-(--space-1) text-(length:--fs-base) text-(color:--clr-text-muted)",
                        "{message}"
                    }
                    if !detail.is_empty() {
                        p { class: "mt-(--space-2) break-words font-(family-name:--font-code) text-(length:--fs-xs) text-(color:--clr-text-faint)",
                            "{detail}"
                        }
                    }
                }
            }
            div {
                button {
                    r#type: "button",
                    class: "{retry_class}",
                    onclick: move |_| on_retry.call(()),
                    Icon { kind: IconKind::Refresh, class: "size-(--icon-sm)" }
                    "Try again"
                }
            }
        }
    }
}
