use dioxus::prelude::*;

use crate::{
    app::Route,
    components::{button_class, ButtonVariant, EmptyState},
};

// Shown for any URL that no route matches
#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let path = segments.join("/");
    let primary = button_class(ButtonVariant::Primary);

    rsx! {
        EmptyState {
            title: "Page not found",
            description: "Nothing lives at /{path}.",
            Link { to: Route::Overview {}, class: "{primary}", "Back to overview" }
        }
    }
}
