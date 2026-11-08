use dioxus::prelude::*;

use super::{
    form::{FieldError, FormState},
    import::{form_from_json, found_parts, SAMPLE_REQUEST},
};
use crate::components::{button_class, ButtonVariant, Modal, TEXTAREA_CLASS};

// Set by a button on another page, the New benchmark page opens the dialog once and clears it
pub static PASTE_REQUEST: GlobalSignal<bool> = Signal::global(|| false);

// What the dialog says about the pasted text
#[derive(Debug, Clone, PartialEq, Eq)]
enum Status {
    Empty,
    Valid(Vec<&'static str>),
    Invalid(String),
}

fn status_of(text: &str) -> Status {
    if text.trim().is_empty() {
        return Status::Empty;
    }

    match form_from_json(text) {
        Ok(_) => Status::Valid(found_parts(text)),
        Err(problem) => Status::Invalid(problem),
    }
}

// Paste the JSON of a request and the form fills itself. Nothing runs until Run benchmark.
#[component]
pub fn PasteModal(
    form: Signal<FormState>,
    errors: Signal<Vec<FieldError>>,
    on_filled: EventHandler<()>,
    on_close: EventHandler<()>,
) -> Element {
    let mut form = form;
    let mut errors = errors;
    let mut text = use_signal(String::new);

    let status = status_of(&text.read());
    let can_fill = matches!(status, Status::Valid(_));
    let primary = button_class(ButtonVariant::Primary);
    let secondary = button_class(ButtonVariant::Secondary);

    let (message, message_class) = match &status {
        Status::Empty => (
            "Paste the JSON body of a request, the same one POST /api/v1/benchmarks takes."
                .to_string(),
            "text-(color:--clr-text-muted)",
        ),
        Status::Valid(found) => (
            format!("Valid. Found: {}.", found.join(", ")),
            "text-(color:--clr-success-text)",
        ),
        Status::Invalid(problem) => (problem.clone(), "text-(color:--clr-danger-text)"),
    };

    rsx! {
        Modal { title: "Paste JSON", on_close,
            div { class: "flex flex-col gap-(--space-4)",
                textarea {
                    class: "{TEXTAREA_CLASS}",
                    rows: "12",
                    spellcheck: "false",
                    autocomplete: "off",
                    placeholder: "{{ \"path\": \"/home/me/project\", \"workload\": {{ ... }} }}",
                    value: "{text}",
                    oninput: move |event| text.set(event.value()),
                }
                p { class: "text-(length:--fs-sm) {message_class}", role: "status", "{message}" }
                p { class: "text-(length:--fs-xs) text-(color:--clr-text-faint)",
                    "Nothing is sent or kept. The form fills, you check it, then press Run benchmark."
                }
                div { class: "flex flex-wrap items-center justify-between gap-(--space-3)",
                    button {
                        r#type: "button",
                        class: "cursor-pointer text-(length:--fs-sm) text-(color:--clr-accent) underline-offset-2 hover:underline",
                        onclick: move |_| text.set(SAMPLE_REQUEST.to_string()),
                        "Try a sample"
                    }
                    div { class: "flex items-center gap-(--space-2)",
                        button {
                            r#type: "button",
                            class: "{secondary}",
                            onclick: move |_| on_close.call(()),
                            "Cancel"
                        }
                        button {
                            r#type: "button",
                            class: "{primary}",
                            disabled: !can_fill,
                            onclick: move |_| {
                                if let Ok(filled) = form_from_json(&text.peek()) {
                                    form.set(filled);
                                    errors.set(Vec::new());
                                    on_filled.call(());
                                    on_close.call(());
                                }
                            },
                            "Fill form"
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_has_its_own_status() {
        assert_eq!(status_of("  \n"), Status::Empty);
    }

    #[test]
    fn valid_text_lists_what_it_found() {
        assert_eq!(
            status_of(r#"{"path":"/p","workload":{"path":"/a","requests":1,"concurrency":1}}"#),
            Status::Valid(vec!["path", "workload"])
        );
    }

    #[test]
    fn invalid_text_carries_the_reason() {
        assert!(matches!(status_of("{ nope"), Status::Invalid(reason) if reason.contains("JSON")));
        assert!(matches!(status_of("{}"), Status::Invalid(_)));
    }
}
