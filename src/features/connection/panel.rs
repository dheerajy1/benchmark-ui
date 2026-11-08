use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use super::error::ApiError;
use crate::components::ErrorState;

// A network error is tried again by itself after this many seconds
const AUTO_RETRY_SECONDS: u32 = 10;

fn now_iso() -> String {
    String::from(js_sys::Date::new_0().to_iso_string())
}

// Error state wired to an `ApiError`: its color, folded technical details and copy button.
// An unreachable API is retried every few seconds with a visible countdown.
#[component]
pub fn ApiErrorPanel(
    error: ApiError,
    // Replaces the title of the error, empty keeps it
    #[props(into, default)] title: String,
    on_retry: EventHandler<()>,
) -> Element {
    let auto_retry = matches!(error, ApiError::Network { .. });
    let retryable = !matches!(error, ApiError::NotConfigured);
    let mut seconds = use_signal(|| AUTO_RETRY_SECONDS);
    let shown_at = use_hook(now_iso);

    use_future(move || async move {
        if !auto_retry {
            return;
        }

        loop {
            TimeoutFuture::new(1_000).await;

            let left = *seconds.peek();
            if left <= 1 {
                seconds.set(AUTO_RETRY_SECONDS);
                on_retry.call(());
            } else {
                seconds.set(left - 1);
            }
        }
    });

    let note = if auto_retry {
        format!("Trying again in {} s", seconds())
    } else {
        String::new()
    };

    rsx! {
        ErrorState {
            title: if title.is_empty() { error.title() } else { title },
            message: error.description(),
            detail: error.details_text(&shown_at),
            copy_extra: super::environment::environment_text(),
            kind: error.kind(),
            retryable,
            note,
            on_retry,
        }
    }
}
