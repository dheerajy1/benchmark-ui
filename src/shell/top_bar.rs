use dioxus::prelude::*;

use crate::{
    app::Route,
    components::{button_class, ButtonVariant, Icon, IconKind},
    features::connection::ConnectionIndicator,
};

fn page_title(route: &Route) -> &'static str {
    match route {
        Route::Overview {} => "Overview",
        Route::History {} => "Runs",
        Route::RunDetail { .. } => "Run details",
        Route::Comparisons { .. } => "Compare",
        Route::Submission {} => "New benchmark",
        Route::NotFound { .. } => "Not found",
    }
}

// Sticky bar with the menu button on small screens, the page title and the API connection status
#[component]
pub fn TopBar(menu_open: Signal<bool>) -> Element {
    let route = use_route::<Route>();
    let mut menu_open = menu_open;
    let title = page_title(&route);
    let ghost = button_class(ButtonVariant::Ghost);

    rsx! {
        header { class: "sticky top-0 z-20 flex h-(--topbar-height) items-center justify-between gap-(--space-3) border-b border-(color:--clr-border) bg-(--clr-canvas) px-(--space-4) sm:px-(--space-6) lg:px-(--space-8)",
            div { class: "flex min-w-0 items-center gap-(--space-3)",
                button {
                    r#type: "button",
                    class: "{ghost} lg:hidden",
                    "aria-label": "Open navigation",
                    "aria-expanded": "{menu_open()}",
                    onclick: move |_| menu_open.set(true),
                    Icon { kind: IconKind::Menu }
                }
                span { class: "truncate text-(length:--fs-base) font-(weight:--fw-medium) text-(color:--clr-text-muted)",
                    "{title}"
                }
            }
            ConnectionIndicator {}
        }
    }
}
