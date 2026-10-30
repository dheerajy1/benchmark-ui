use dioxus::prelude::*;

use crate::{
    app::Route,
    components::{Icon, IconKind},
};

// One navigation link, `active` marks the page the user is on
#[component]
fn NavItem(
    to: Route,
    #[props(into)] label: String,
    icon: IconKind,
    active: bool,
    on_navigate: EventHandler<()>,
) -> Element {
    let state_class = if active {
        "bg-(--clr-accent-soft) text-(color:--clr-accent)"
    } else {
        "text-(color:--clr-text-muted) hover:bg-(--clr-surface-hover) hover:text-(color:--clr-text)"
    };

    rsx! {
        Link {
            to,
            class: "flex items-center gap-(--space-3) rounded-(--round-md) px-(--space-3) py-(--space-2) text-(length:--fs-base) font-(weight:--fw-medium) transition-colors {state_class}",
            onclick: move |_| on_navigate.call(()),
            "aria-current": if active { "page" },
            Icon { kind: icon }
            "{label}"
        }
    }
}

// Fixed navigation on large screens, a slide-over drawer on small ones
#[component]
pub fn Sidebar(open: Signal<bool>) -> Element {
    let route = use_route::<Route>();
    let mut open = open;

    let overview_active = matches!(route, Route::Overview {});
    let runs_active = matches!(route, Route::History {} | Route::RunDetail { .. });
    let compare_active = matches!(route, Route::Comparisons { .. });
    let new_active = matches!(route, Route::Submission {});

    let drawer_class = if open() {
        "translate-x-0"
    } else {
        "-translate-x-full"
    };

    rsx! {
        if open() {
            button {
                r#type: "button",
                class: "fixed inset-0 z-30 bg-(--clr-overlay) lg:hidden",
                "aria-label": "Close navigation",
                onclick: move |_| open.set(false),
            }
        }
        aside {
            class: "fixed inset-y-0 left-0 z-40 flex w-(--sidebar-width) flex-col border-r border-(color:--clr-border) bg-(--clr-surface) transition-transform lg:translate-x-0 {drawer_class}",
            "aria-label": "Primary",
            div { class: "flex h-(--topbar-height) items-center gap-(--space-3) border-b border-(color:--clr-border) px-(--space-5)",
                span { class: "flex size-(--icon-lg) items-center justify-center rounded-(--round-sm) bg-(--clr-accent) text-(color:--clr-accent-contrast)",
                    Icon { kind: IconKind::Dashboard, class: "size-(--icon-sm)" }
                }
                span { class: "text-(length:--fs-lg) font-(weight:--fw-bold) tracking-tight", "Benchmark" }
            }
            nav { class: "flex flex-1 flex-col gap-(--space-1) p-(--space-3)",
                NavItem {
                    to: Route::Overview {},
                    label: "Overview",
                    icon: IconKind::Dashboard,
                    active: overview_active,
                    on_navigate: move |_| open.set(false),
                }
                NavItem {
                    to: Route::History {},
                    label: "Runs",
                    icon: IconKind::History,
                    active: runs_active,
                    on_navigate: move |_| open.set(false),
                }
                NavItem {
                    to: Route::Comparisons {
                        baseline: String::new(),
                        comparison: String::new(),
                    },
                    label: "Compare",
                    icon: IconKind::Compare,
                    active: compare_active,
                    on_navigate: move |_| open.set(false),
                }
                NavItem {
                    to: Route::Submission {},
                    label: "New benchmark",
                    icon: IconKind::Play,
                    active: new_active,
                    on_navigate: move |_| open.set(false),
                }
            }
            p { class: "border-t border-(color:--clr-border) px-(--space-5) py-(--space-4) text-(length:--fs-xs) text-(color:--clr-text-faint)",
                "Benchmark dashboard"
            }
        }
    }
}
