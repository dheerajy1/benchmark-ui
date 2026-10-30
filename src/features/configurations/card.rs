use dioxus::prelude::*;

use super::{
    api::fetch_run_configuration,
    entry::entry_from_result,
    lines::{configuration_lines, legacy_lines},
};
use crate::{
    app::Route,
    components::{button_class, ButtonVariant, Card, CardHeader, Definition, LoadingPanel},
    features::{connection::ApiErrorPanel, run_detail::BenchmarkResult},
};

// The settings one run executed with, and the button that starts a new run from them. A run
// stored before configurations were recorded shows what its stored result holds.
#[component]
pub fn ConfigurationCard(run_id: String, run: BenchmarkResult) -> Element {
    let reactive_id = run_id.clone();
    let mut detail = use_resource(use_reactive!(|(reactive_id,)| async move {
        fetch_run_configuration(&reactive_id).await
    }));

    let primary = button_class(ButtonVariant::Primary);

    let body: Element = match detail() {
        None => rsx! {
            LoadingPanel { label: "Loading configuration" }
        },
        Some(Err(error)) => rsx! {
            ApiErrorPanel { error, on_retry: move |_| detail.restart() }
        },
        Some(Ok(found)) => match found.configuration {
            Some(config) => definition_grid(configuration_lines(&config)),
            None => definition_grid(legacy_lines(&entry_from_result(&run))),
        },
    };

    rsx! {
        Card {
            CardHeader {
                title: "Configuration",
                subtitle: "The settings this run executed with. Header names only, values are never stored.",
            }
            div { class: "flex flex-col gap-(--space-4) p-(--space-4)",
                {body}
                div {
                    Link {
                        to: Route::SubmissionFrom {
                            run_id: run_id.clone(),
                        },
                        class: "{primary}",
                        "Use these settings"
                    }
                }
            }
        }
    }
}

fn definition_grid(lines: Vec<(String, String)>) -> Element {
    rsx! {
        dl { class: "grid grid-cols-1 gap-(--space-4) md:grid-cols-2",
            for (label, value) in lines {
                Definition { label: label, "{value}" }
            }
        }
    }
}
