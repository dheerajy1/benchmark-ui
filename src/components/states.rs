use dioxus::prelude::*;

use super::{button_class, copy_text, use_toasts, ButtonVariant, Icon, IconKind, Skeleton};

// Color and icon of a page error: red = the data could not be loaded, amber = temporary and
// recoverable, grey = it does not exist
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Red,
    Amber,
    Grey,
}

impl ErrorKind {
    fn tile_class(self) -> &'static str {
        match self {
            Self::Red => "bg-(--clr-danger-soft) text-(color:--clr-danger)",
            Self::Amber => {
                "bg-(--clr-warning-soft) text-(color:--clr-warning-text) shake-sometimes"
            }
            Self::Grey => "bg-(--clr-surface-raised) text-(color:--clr-text-muted)",
        }
    }

    fn icon(self) -> IconKind {
        match self {
            Self::Red => IconKind::Alert,
            Self::Amber => IconKind::Plug,
            Self::Grey => IconKind::Search,
        }
    }
}

// Placeholder shown while a request is in flight
#[component]
pub fn LoadingPanel(#[props(into)] label: String) -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-(--space-3) rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) p-(--space-5)",
            role: "status",
            "aria-live": "polite",
            span { class: "sr-only", "{label}" }
            Skeleton { class: "h-4 w-1/3" }
            Skeleton { class: "h-8 w-full" }
            Skeleton { class: "h-8 w-5/6" }
            Skeleton { class: "h-8 w-2/3" }
        }
    }
}

// Shown when a request succeeded but there is nothing to display. `children` holds at most two
// buttons, the blue one is always the next real step.
#[component]
pub fn EmptyState(
    #[props(into)] title: String,
    #[props(into)] description: String,
    #[props(default)] ghost: Option<Element>,
    children: Element,
) -> Element {
    rsx! {
        div { class: "flex flex-col items-center gap-(--space-3) rounded-(--round-lg) border border-dashed border-(color:--clr-border-strong) bg-(--clr-surface) px-(--space-6) py-(--space-12) text-center",
            if let Some(ghost) = ghost {
                div { class: "mb-(--space-2) flex w-full justify-center", {ghost} }
            }
            h2 { class: "text-(length:--fs-base) font-(weight:--fw-semibold) text-(color:--clr-text)",
                "{title}"
            }
            p { class: "max-w-md text-(length:--fs-sm) text-(color:--clr-text-muted)", "{description}" }
            div { class: "mt-(--space-2) flex flex-wrap items-center justify-center gap-(--space-2)",
                {children}
            }
        }
    }
}

// Page-level error that fills the content area, the shell around it stays usable.
// `detail` is the folded technical text (request line, status, request id, time).
#[component]
pub fn ErrorState(
    #[props(into)] title: String,
    #[props(into)] message: String,
    #[props(into, default)] detail: String,
    #[props(default)] retrying: bool,
    #[props(default = ErrorKind::Red)] kind: ErrorKind,
    // False hides `Try again`, for errors that only a rebuild can fix
    #[props(default = true)] retryable: bool,
    // Small line under the message, for example the auto retry countdown
    #[props(into, default)] note: String,
    // Extra buttons after the standard ones, for example links to other pages
    #[props(default)] actions: Option<Element>,
    on_retry: EventHandler<()>,
) -> Element {
    let retry_class = button_class(ButtonVariant::Primary);
    let copy_class = button_class(ButtonVariant::Secondary);
    let toasts = use_toasts();
    let tile_class = kind.tile_class();
    let detail_for_copy = detail.clone();

    rsx! {
        div {
            class: "flex flex-col items-center gap-(--space-3) rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) px-(--space-6) py-(--space-12) text-center",
            role: "alert",
            span { class: "flex size-[46px] items-center justify-center rounded-(--round-lg) {tile_class}",
                Icon { kind: kind.icon(), class: "size-(--icon-md)" }
            }
            h2 { class: "text-(length:--fs-md) font-(weight:--fw-semibold) text-(color:--clr-text)",
                "{title}"
            }
            p { class: "max-w-md text-(length:--fs-sm) text-(color:--clr-text-muted)", "{message}" }
            if !note.is_empty() {
                p { class: "text-(length:--fs-xs) text-(color:--clr-text-faint)", "{note}" }
            }
            div { class: "mt-(--space-2) flex flex-wrap items-center justify-center gap-(--space-2)",
                if retryable {
                    button {
                        r#type: "button",
                        class: "group {retry_class}",
                        disabled: retrying,
                        onclick: move |_| on_retry.call(()),
                        if retrying {
                            Icon { kind: IconKind::Refresh, class: "size-(--icon-sm) animate-spin" }
                        } else {
                            Icon {
                                kind: IconKind::Refresh,
                                class: "size-(--icon-sm) transition-transform duration-500 ease-(--ease-in-out) group-hover:rotate-180",
                            }
                        }
                        "Try again"
                    }
                }
                if !detail.is_empty() {
                    button {
                        r#type: "button",
                        class: "{copy_class}",
                        onclick: move |_| {
                            copy_text(&detail_for_copy);
                            toasts.show("Details copied");
                        },
                        Icon { kind: IconKind::Copy, class: "size-(--icon-sm)" }
                        "Copy details"
                    }
                }
                if let Some(actions) = actions {
                    {actions}
                }
            }
            if !detail.is_empty() {
                details { class: "mt-(--space-2) w-full max-w-md text-left",
                    summary { class: "cursor-pointer text-center text-(length:--fs-xs) text-(color:--clr-text-faint) hover:text-(color:--clr-text-muted)",
                        "Technical details"
                    }
                    p { class: "mt-(--space-2) rounded-(--round-md) bg-(--clr-surface-raised) p-(--space-3) font-(family-name:--font-code) text-(length:--fs-xs) break-words whitespace-pre-wrap text-(color:--clr-text-muted)",
                        "{detail}"
                    }
                }
            }
        }
    }
}

// One card that failed while the rest of the page is fine: a pale red tile in its place
#[component]
pub fn CardError(#[props(into)] title: String, on_retry: EventHandler<()>) -> Element {
    let secondary = button_class(ButtonVariant::Secondary);

    rsx! {
        div {
            class: "flex h-(--chart-height) w-full flex-col items-center justify-center gap-(--space-1) rounded-(--round-md) bg-(--clr-danger-soft) px-(--space-4) text-center",
            role: "alert",
            p { class: "text-(length:--fs-sm) font-(weight:--fw-semibold) text-(color:--clr-danger)",
                "{title}"
            }
            p { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)", "The others are fine." }
            button {
                r#type: "button",
                class: "mt-(--space-2) {secondary}",
                onclick: move |_| on_retry.call(()),
                "Try again"
            }
        }
    }
}
