use dioxus::prelude::*;

use super::error::ApiError;
use crate::components::ErrorState;

// Error state wired to an `ApiError`
#[component]
pub fn ApiErrorPanel(error: ApiError, on_retry: EventHandler<()>) -> Element {
    rsx! {
        ErrorState {
            title: error.title(),
            message: error.description(),
            detail: error.technical_detail().unwrap_or_default(),
            on_retry,
        }
    }
}
