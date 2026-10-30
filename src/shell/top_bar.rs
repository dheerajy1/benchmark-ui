use dioxus::prelude::*;

use crate::{
    app::Route,
    components::{button_class, ButtonVariant, Icon, IconKind},
    features::connection::ConnectionIndicator,
    format::short_id,
};

// Section the page belongs to as a link, shown before the current page when the page is nested
fn parent(route: &Route) -> Option<(&'static str, Route)> {
    match route {
        Route::RunDetail { .. } => Some(("Runs", Route::History {})),
        _ => None,
    }
}

// Current page name, the only visible title of a page
fn current_title(route: &Route) -> String {
    match route {
        Route::Overview {} => "Overview".to_string(),
        Route::History {} => "Runs".to_string(),
        Route::RunDetail { run_id } => format!("Run {}", short_id(run_id)),
        Route::Comparisons { .. } => "Compare".to_string(),
        Route::Submission {} => "New benchmark".to_string(),
        Route::NotFound { .. } => "Not found".to_string(),
    }
}

// Sticky bar with the menu button on small screens, the breadcrumb and the API connection status
#[component]
pub fn TopBar(menu_open: Signal<bool>) -> Element {
    let route = use_route::<Route>();
    let mut menu_open = menu_open;
    let title = current_title(&route);
    let parent_link = parent(&route);
    let ghost = button_class(ButtonVariant::Ghost);

    rsx! {
        header { class: "sticky top-0 z-20 flex h-(--topbar-height) items-center justify-between gap-(--space-3) border-b border-(color:--clr-border) bg-(--clr-canvas) px-(--space-4) sm:px-(--space-6) lg:px-(--space-8)",
            div { class: "flex min-w-0 items-center gap-(--space-2)",
                button {
                    r#type: "button",
                    class: "{ghost} -ml-2 px-(--space-2) lg:hidden",
                    "aria-label": "Open navigation",
                    "aria-expanded": "{menu_open()}",
                    onclick: move |_| menu_open.set(true),
                    Icon { kind: IconKind::Menu }
                }
                nav {
                    class: "flex min-w-0 items-center gap-(--space-2) text-(length:--fs-sm)",
                    "aria-label": "Breadcrumb",
                    if let Some((label , target)) = parent_link {
                        Link {
                            to: target,
                            class: "text-(color:--clr-text-muted) transition-colors hover:text-(color:--clr-text)",
                            "{label}"
                        }
                        span { class: "text-(color:--clr-text-faint)", "aria-hidden": "true", "/" }
                    }
                    span {
                        class: "truncate font-(weight:--fw-medium) text-(color:--clr-text)",
                        "aria-current": "page",
                        "{title}"
                    }
                }
            }
            ConnectionIndicator {}
        }
    }
}
