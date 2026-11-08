use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use super::state::{LiveState, Terminal};

const PLAIN_ICON: Asset = asset!("/assets/favicon.ico");
const DONE_ICON: Asset = asset!("/assets/favicon-dot.png");

// The badge shows when a run finished while its tab was in the background
fn badge_wanted(watched_running: bool, completed: bool, hidden: bool) -> bool {
    watched_running && completed && hidden
}

fn page_hidden() -> bool {
    web_sys::window()
        .and_then(|window| window.document())
        .is_some_and(|document| document.hidden())
}

// Points the icon link of the page at another file
fn set_icon(href: &str) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };

    if let Ok(Some(link)) = document.query_selector("link[rel~='icon']") {
        let _ = link.set_attribute("href", href);
    }
}

// Puts a green dot on the tab icon when the run completes while the tab is in the background, and
// takes it off when the person comes back to the tab. A run that was already finished when the
// page opened gets no dot.
#[component]
pub fn TabBadge(state: Signal<LiveState>) -> Element {
    let mut watched_running = use_signal(|| false);
    let mut shown = use_signal(|| false);

    use_effect(move || {
        let (finished, completed) = {
            let snapshot = state.read();

            (
                snapshot.is_terminal(),
                matches!(snapshot.terminal(), Some(Terminal::Completed(_))),
            )
        };

        if !finished {
            if !*watched_running.peek() {
                watched_running.set(true);
            }

            return;
        }

        if badge_wanted(*watched_running.peek(), completed, page_hidden()) && !*shown.peek() {
            shown.set(true);
        }
    });

    use_effect(move || {
        if shown() {
            set_icon(&DONE_ICON.to_string());

            spawn(async move {
                loop {
                    TimeoutFuture::new(500).await;

                    if !page_hidden() {
                        shown.set(false);
                        break;
                    }
                }
            });
        } else {
            set_icon(&PLAIN_ICON.to_string());
        }
    });

    use_drop(|| set_icon(&PLAIN_ICON.to_string()));

    rsx! {}
}

#[cfg(test)]
mod tests {
    use super::badge_wanted;

    #[test]
    fn a_dot_needs_a_run_seen_running_that_completed_in_the_background() {
        assert!(badge_wanted(true, true, true));
        assert!(!badge_wanted(false, true, true));
        assert!(!badge_wanted(true, false, true));
        assert!(!badge_wanted(true, true, false));
    }
}
