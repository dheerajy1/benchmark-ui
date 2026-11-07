use dioxus::prelude::*;

use crate::{
    app::Route,
    components::{button_class, ButtonVariant, ErrorKind, ErrorState},
};

// Shown for any URL that no route matches
#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let path = segments.join("/");
    let primary = button_class(ButtonVariant::Primary);

    rsx! {
        ErrorState {
            title: "Page not found",
            message: "Nothing lives at /{path}.",
            kind: ErrorKind::Grey,
            retryable: false,
            actions: Some(rsx! {
                Link { to: Route::overview(), class: "{primary}", "Go to Overview" }
            }),
            on_retry: move |_| {},
        }
    }
}
