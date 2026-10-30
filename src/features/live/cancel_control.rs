use dioxus::prelude::*;

use super::{
    cancel::{
        cancel_ui, classify_cancel_error, failure_message, request_cancel, CancelFailure, CancelUi,
    },
    state::LiveState,
    token::{forget_token, SessionTokenStore},
};
use crate::components::{button_class, ButtonVariant};

const DANGER_BUTTON: &str = "inline-flex h-(--control-height) cursor-pointer items-center justify-center whitespace-nowrap rounded-(--round-md) border border-(color:--clr-danger) bg-(--clr-danger-soft) px-(--space-3) text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-danger) transition-colors hover:opacity-80 disabled:cursor-not-allowed disabled:opacity-50";

// Cancel button of a running run, with a confirmation step in place and no dialog.
//
// It only shows what is true: "cancelling" until the run itself ends, the reason when the API
// refuses, and a plain note when this page does not hold the right to cancel.
#[component]
pub fn CancelControl(
    state: Signal<LiveState>,
    run_id: String,
    token: Signal<Option<String>>,
) -> Element {
    let mut token = token;
    let mut confirming = use_signal(|| false);
    let mut sending = use_signal(|| false);
    let mut requested_here = use_signal(|| false);
    let mut problem = use_signal(|| None::<String>);

    let secondary = button_class(ButtonVariant::Secondary);

    let ui = {
        let snapshot = state.read();

        cancel_ui(&snapshot, token.read().is_some(), requested_here())
    };

    let request_id = run_id.clone();

    let confirm = move |_| {
        let Some(secret) = token.read().clone() else {
            return;
        };

        let run_id = request_id.clone();

        sending.set(true);
        problem.set(None);

        spawn(async move {
            match request_cancel(&run_id, &secret).await {
                Ok(_) => {
                    requested_here.set(true);
                    confirming.set(false);
                }
                Err(error) => {
                    let failure = classify_cancel_error(&error);

                    match &failure {
                        // This browser's token is not the run's, so it is dropped
                        CancelFailure::NotAllowed => {
                            forget_token(&SessionTokenStore, &run_id);
                            token.set(None);
                            confirming.set(false);
                        }
                        CancelFailure::TooLate(_) | CancelFailure::Gone => confirming.set(false),
                        CancelFailure::Retry(_) => {}
                    }

                    problem.set(Some(failure_message(&failure)));
                }
            }

            sending.set(false);
        });
    };

    match ui {
        CancelUi::Hidden => rsx! {},
        CancelUi::NeedsToken => rsx! {
            span { class: "text-(length:--fs-xs) text-(color:--clr-text-faint)",
                "Only the tab that started this run can cancel it."
            }
        },
        CancelUi::Cancelling => rsx! {
            span {
                class: "text-(length:--fs-sm) text-(color:--clr-text-muted)",
                role: "status",
                "Cancelling, stopping the target…"
            }
        },
        CancelUi::Available => rsx! {
            div { class: "flex flex-col items-end gap-(--space-1)",
                if confirming() {
                    div {
                        class: "flex flex-wrap items-center justify-end gap-(--space-2)",
                        role: "group",
                        "aria-label": "Confirm cancelling this run",
                        span { class: "text-(length:--fs-sm) text-(color:--clr-text)", "Cancel this run?" }
                        button {
                            r#type: "button",
                            class: "{DANGER_BUTTON}",
                            disabled: sending(),
                            onclick: confirm,
                            if sending() {
                                "Cancelling…"
                            } else {
                                "Yes, cancel"
                            }
                        }
                        button {
                            r#type: "button",
                            class: "{secondary}",
                            disabled: sending(),
                            onclick: move |_| confirming.set(false),
                            "Keep running"
                        }
                    }
                } else {
                    button {
                        r#type: "button",
                        class: "{secondary}",
                        onclick: move |_| {
                            problem.set(None);
                            confirming.set(true);
                        },
                        "Cancel run"
                    }
                }
                if let Some(message) = problem() {
                    span {
                        class: "max-w-prose break-words text-right text-(length:--fs-xs) text-(color:--clr-danger)",
                        role: "alert",
                        "{message}"
                    }
                }
            }
        },
    }
}
