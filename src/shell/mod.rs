// Application shell: sidebar, top bar and the layout every route renders inside

mod not_found;
mod pinned;
mod recents;
mod run_menu;
mod settings_menu;
mod sidebar;
mod state;
mod top_bar;

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::{
    app::Route,
    components::ToastProvider,
    features::{
        connection::probe,
        live::{refresh_active, ACTIVE_RUN},
    },
};

pub use not_found::NotFound;
use sidebar::Sidebar;
use state::use_shell_provider;
use top_bar::TopBar;

// The active run is asked for every 3 seconds, the API health every fifth time
const ACTIVE_POLL_MS: u32 = 3_000;
const HEALTH_EVERY: u32 = 5;

// Layout shared by every route, the router renders the active page in the outlet
#[component]
pub fn AppShell() -> Element {
    let shell = use_shell_provider();

    // Keeps the active run chip and the API pill current on every page
    use_future(move || async move {
        let mut until_health = 0u32;

        loop {
            refresh_active().await;

            if until_health == 0 {
                probe().await;
                until_health = HEALTH_EVERY;
            }

            until_health -= 1;
            TimeoutFuture::new(ACTIVE_POLL_MS).await;
        }
    });

    // When the active run ends it appears in the Recents list
    let mut last_active = use_signal(|| None::<String>);
    use_effect(move || {
        let current = ACTIVE_RUN.read().as_ref().map(|run| run.run_id.clone());
        let previous = last_active.peek().clone();

        if previous.is_some() && current.is_none() {
            shell.bump_recents();
        }
        if previous != current {
            last_active.set(current);
        }
    });

    let shift_class = if shell.is_pinned() {
        "lg:pl-(--sidebar-width)"
    } else {
        ""
    };

    rsx! {
        ToastProvider {
            // Ctrl+B pins the sidebar and Ctrl+K focuses the search. The root takes focus, so the
            // keys work anywhere on the page.
            div {
                class: "min-h-screen outline-none",
                tabindex: "-1",
                onmounted: move |event| async move {
                    let _ = event.set_focus(true).await;
                },
                onkeydown: move |event| {
                    let modifiers = event.modifiers();
                    if !(modifiers.contains(Modifiers::CONTROL) || modifiers.contains(Modifiers::META)) {
                        return;
                    }
                    if let Key::Character(key) = event.key() {
                        if key.eq_ignore_ascii_case("b") {
                            event.prevent_default();
                            shell.toggle_pin();
                        } else if key.eq_ignore_ascii_case("k") {
                            event.prevent_default();
                            shell.focus_search();
                        }
                    }
                },
                a {
                    href: "#main",
                    class: "sr-only focus:not-sr-only focus:fixed focus:top-(--space-4) focus:left-(--space-4) focus:z-50 focus:rounded-(--round-md) focus:bg-(--clr-accent) focus:px-(--space-4) focus:py-(--space-2) focus:text-(color:--clr-accent-contrast)",
                    "Skip to content"
                }
                Sidebar {}
                div { class: "flex min-h-screen flex-col transition-[padding] duration-[320ms] ease-(--ease-out) {shift_class}",
                    TopBar {}
                    main {
                        id: "main",
                        "tabindex": "-1",
                        class: "mx-auto w-full max-w-(--content-max) flex-1 px-(--space-4) py-(--space-5) sm:px-(--space-6) lg:px-(--space-8)",
                        Outlet::<Route> {}
                    }
                }
            }
        }
    }
}
