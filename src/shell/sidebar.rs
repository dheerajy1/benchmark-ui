use dioxus::prelude::*;

use super::{
    pinned::PinnedSection, recents::RecentsSection, settings_menu::SettingsMenu, state::use_shell,
};
use crate::{
    app::Route,
    components::{Icon, IconKind, Popover, PopoverAlign},
    features::connection::{ConnectionState, CONNECTION},
};

const NAV_CLASS: &str = "group/nav relative flex h-10 w-full cursor-pointer items-center gap-(--space-3) rounded-(--round-md) px-(--space-3) text-left text-(length:--fs-base) font-(weight:--fw-medium) transition-colors";

fn nav_state_class(active: bool) -> &'static str {
    if active {
        "bg-(--clr-accent-soft) text-(color:--clr-accent)"
    } else {
        "text-(color:--clr-text-muted) hover:bg-(--clr-hover-wash) hover:text-(color:--clr-text)"
    }
}

// One navigation link, `active` marks the page the user is on
#[component]
fn NavItem(to: Route, #[props(into)] label: String, icon: IconKind, active: bool) -> Element {
    let shell = use_shell();
    let state_class = nav_state_class(active);

    rsx! {
        Link {
            to,
            class: "{NAV_CLASS} {state_class}",
            onclick: move |_| shell.close_peek(),
            "aria-current": if active { "page" },
            if active {
                span { class: "absolute top-1/2 left-0 h-5 w-[3px] -translate-y-1/2 rounded-r-full bg-(--clr-accent)" }
            }
            Icon {
                kind: icon,
                class: "size-5 transition-transform duration-200 ease-(--ease-out) group-hover/nav:scale-110",
            }
            "{label}"
        }
    }
}

// Sidebar: a floating card that peeks while the mouse is on the menu button, or a panel docked
// to the left edge when pinned
#[component]
pub fn Sidebar() -> Element {
    let shell = use_shell();
    let route = use_route::<Route>();
    let mut settings_open = use_signal(|| false);

    // The floating sidebar stays open while the settings menu is open
    use_effect(move || shell.set_hold_open(settings_open()));

    let overview_active = matches!(route, Route::Overview { .. });
    let runs_active = matches!(route, Route::History { .. } | Route::RunDetail { .. });
    let compare_active = matches!(route, Route::Comparisons { .. });
    let new_active = matches!(
        route,
        Route::Submission { .. } | Route::SubmissionFrom { .. } | Route::LiveRun { .. }
    );

    let frame_class = if shell.is_pinned() {
        "inset-y-0 left-0 border-r"
    } else if shell.is_peeking() {
        "top-[calc(var(--topbar-height)+0.5rem)] bottom-2 left-2 rounded-[14px] border shadow-(--shadow-sidebar)"
    } else {
        "pointer-events-none invisible top-[calc(var(--topbar-height)+0.5rem)] bottom-2 -left-72 rounded-[14px] border opacity-0"
    };

    let motion = if shell.animates() {
        "transition-[left,opacity,visibility] duration-300 ease-(--ease-out)"
    } else {
        ""
    };

    let (dot_class, status_text) = match *CONNECTION.read() {
        ConnectionState::Unknown => ("bg-(--clr-text-faint) animate-pulse", "Checking API"),
        ConnectionState::Online => (
            "bg-(--clr-success) pulse-dot text-(color:--clr-success)",
            "API connected",
        ),
        ConnectionState::Offline => ("bg-(--clr-danger)", "API unreachable"),
    };

    rsx! {
        aside {
            class: "fixed z-40 flex w-(--sidebar-width) flex-col border-(color:--clr-border) bg-(--clr-surface) {motion} {frame_class}",
            "aria-label": "Primary",
            onmouseenter: move |_| shell.peek_in(),
            onmouseleave: move |_| shell.peek_out(),
            div { class: "flex h-(--topbar-height) shrink-0 items-center gap-(--space-3) px-(--space-4)",
                span { class: "flex size-7 items-center justify-center rounded-(--round-md) bg-(--clr-accent-soft) text-(color:--clr-accent)",
                    Icon { kind: IconKind::Gauge, class: "size-4" }
                }
                span { class: "text-(length:--fs-md) font-(weight:--fw-semibold) tracking-tight text-(color:--clr-text)",
                    "Benchmark"
                }
            }
            nav {
                class: "flex shrink-0 flex-col gap-(--space-1) px-(--space-2) pb-(--space-2)",
                "aria-label": "Main",
                NavItem {
                    to: Route::new_benchmark(),
                    label: "New benchmark",
                    icon: IconKind::Plus,
                    active: new_active,
                }
                button {
                    r#type: "button",
                    class: "{NAV_CLASS} {nav_state_class(false)}",
                    onclick: move |_| {
                        shell.close_peek();
                        shell.focus_search();
                    },
                    Icon {
                        kind: IconKind::Search,
                        class: "size-5 transition-transform duration-200 ease-(--ease-out) group-hover/nav:scale-110",
                    }
                    "Search"
                }
                NavItem {
                    to: Route::overview(),
                    label: "Overview",
                    icon: IconKind::Dashboard,
                    active: overview_active,
                }
                NavItem {
                    to: Route::runs(),
                    label: "Runs",
                    icon: IconKind::History,
                    active: runs_active,
                }
                NavItem {
                    to: Route::Comparisons {
                        baseline: String::new(),
                        comparison: String::new(),
                    },
                    label: "Compare",
                    icon: IconKind::Compare,
                    active: compare_active,
                }
            }
            div { class: "flex min-h-0 flex-1 flex-col gap-(--space-2) border-t border-(color:--clr-border) px-(--space-2) pt-(--space-2)",
                PinnedSection {}
                RecentsSection {}
            }
            div { class: "relative flex h-12 shrink-0 items-center gap-(--space-2) border-t border-(color:--clr-border) px-(--space-4)",
                span { class: "size-(--dot-size) shrink-0 rounded-(--round-full) {dot_class}" }
                span { class: "min-w-0 flex-1 truncate text-(length:--fs-xs) text-(color:--clr-text-muted)",
                    "{status_text}"
                }
                button {
                    r#type: "button",
                    class: "inline-flex size-8 cursor-pointer items-center justify-center rounded-(--round-md) text-(color:--clr-text-muted) transition-colors hover:bg-(--clr-hover-wash) hover:text-(color:--clr-text)",
                    "aria-label": "Settings",
                    "aria-expanded": "{settings_open()}",
                    onclick: move |_| settings_open.set(!settings_open()),
                    Icon { kind: IconKind::Settings }
                }
                if settings_open() {
                    Popover {
                        align: PopoverAlign::Right,
                        above: true,
                        rise: true,
                        width: "w-64",
                        on_close: move |_| settings_open.set(false),
                        SettingsMenu { on_close: move |_| settings_open.set(false) }
                    }
                }
            }
        }
    }
}
