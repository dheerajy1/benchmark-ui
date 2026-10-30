// Application shell: sidebar, top bar and the layout every route renders inside

mod not_found;
mod sidebar;
mod top_bar;

use dioxus::prelude::*;

use crate::app::Route;

pub use not_found::NotFound;
use sidebar::Sidebar;
use top_bar::TopBar;

// Layout shared by every route, the router renders the active page in the outlet
#[component]
pub fn AppShell() -> Element {
    let menu_open = use_signal(|| false);

    rsx! {
        div { class: "min-h-screen",
            a {
                href: "#main",
                class: "sr-only focus:not-sr-only focus:fixed focus:top-(--space-4) focus:left-(--space-4) focus:z-50 focus:rounded-(--round-md) focus:bg-(--clr-accent) focus:px-(--space-4) focus:py-(--space-2) focus:text-(color:--clr-accent-contrast)",
                "Skip to content"
            }
            Sidebar { open: menu_open }
            div { class: "flex min-h-screen flex-col lg:pl-(--sidebar-width)",
                TopBar { menu_open }
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
