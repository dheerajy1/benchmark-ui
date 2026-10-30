use dioxus::prelude::*;

use super::{api::fetch_run_configuration, lines::configuration_lines};
use crate::{
    app::Route,
    components::{button_class, ButtonVariant, Card, CardHeader, Definition, LoadingPanel},
    features::connection::ApiErrorPanel,
};

// The settings one run executed with, and the button that starts a new run from them
#[component]
pub fn ConfigurationCard(run_id: String) -> Element {
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
            Some(config) => {
                let lines = configuration_lines(&config);

                rsx! {
                    dl { class: "grid grid-cols-1 gap-(--space-4) md:grid-cols-2",
                        for (label, value) in lines.iter() {
                            Definition { label: label.clone(), "{value}" }
                        }
                    }
                }
            }
            None => rsx! {
                p { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)",
                    "No configuration was recorded for this run, because it was stored before configurations were recorded. The path and start command are still shown in the history, and the button fills them in."
                }
            },
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
