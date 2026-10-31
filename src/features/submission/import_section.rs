use dioxus::prelude::*;

use super::{
    form::{FieldError, FormState},
    import::form_from_json,
};
use crate::components::{button_class, ButtonVariant, Card, CardHeader, FormField, TEXTAREA_CLASS};

// Paste a request as JSON and the form below fills itself. Nothing is sent until Run benchmark.
#[component]
pub fn ImportSection(
    form: Signal<FormState>,
    errors: Signal<Vec<FieldError>>,
    running: bool,
) -> Element {
    let mut form = form;
    let mut errors = errors;
    let mut text = use_signal(String::new);
    let mut message = use_signal(|| None::<(bool, String)>);

    let secondary = button_class(ButtonVariant::Secondary);

    rsx! {
        Card {
            CardHeader {
                title: "Paste a request",
                subtitle: "Optional. Paste the JSON of a request and the form below fills itself. Nothing runs until you press Run benchmark.",
            }
            div { class: "flex flex-col gap-(--space-3) p-(--space-4)",
                FormField {
                    label: "Request JSON",
                    hint: "The same body the API takes: path, startup, workload and one workspace. It is not kept, the box is cleared once the form is filled.",
                    textarea {
                        class: "{TEXTAREA_CLASS}",
                        rows: "8",
                        spellcheck: "false",
                        autocomplete: "off",
                        placeholder: "{{ \"path\": \"/home/me/project\", \"workspaces\": [ ... ] }}",
                        value: "{text}",
                        disabled: running,
                        oninput: move |event| text.set(event.value()),
                    }
                }
                div { class: "flex flex-wrap items-center gap-(--space-3)",
                    button {
                        r#type: "button",
                        class: "{secondary}",
                        disabled: running || text.read().trim().is_empty(),
                        onclick: move |_| {
                            let pasted = text.peek().clone();
                            match form_from_json(&pasted) {
                                Ok(filled) => {
                                    form.set(filled);
                                    errors.set(Vec::new());
                                    text.set(String::new());
                                    message
                                        .set(
                                            Some((
                                                true,
                                                "The form is filled. Check it below, then press Run benchmark."
                                                    .to_string(),
                                            )),
                                        );
                                }
                                Err(problem) => message.set(Some((false, problem))),
                            }
                        },
                        "Fill the form"
                    }
                    if let Some((ok, shown)) = message() {
                        p {
                            class: if ok { "text-(length:--fs-sm) text-(color:--clr-success)" } else { "text-(length:--fs-sm) text-(color:--clr-danger)" },
                            role: "alert",
                            "{shown}"
                        }
                    }
                }
            }
        }
    }
}
