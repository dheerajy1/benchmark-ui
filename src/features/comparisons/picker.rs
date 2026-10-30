use dioxus::prelude::*;

use crate::{
    components::{FormField, INPUT_CLASS},
    features::history::RunSummary,
    format::{date_part, path_tail, short_id, MISSING},
};

fn option_label(run: &RunSummary) -> String {
    format!(
        "{} · {} · {} · {}",
        short_id(&run.run_id),
        run.discovery.runtime.as_deref().unwrap_or(MISSING),
        path_tail(&run.target.path),
        date_part(&run.created_at)
    )
}

// Select listing the newest runs, keeps an id that is not in the list selectable
#[component]
pub fn RunPicker(
    #[props(into)] label: String,
    value: String,
    runs: Vec<RunSummary>,
    on_change: EventHandler<String>,
) -> Element {
    let listed = runs.iter().any(|run| run.run_id == value);
    let unlisted = if !value.is_empty() && !listed {
        Some(format!("{} · not in the newest runs", short_id(&value)))
    } else {
        None
    };
    let options: Vec<(String, String, bool)> = runs
        .iter()
        .map(|run| (run.run_id.clone(), option_label(run), run.run_id == value))
        .collect();
    let value_for_unlisted = value.clone();

    rsx! {
        FormField { label,
            select {
                class: "{INPUT_CLASS}",
                onchange: move |event| on_change.call(event.value()),
                option { value: "", selected: value.is_empty(), "Choose a run" }
                if let Some(text) = unlisted {
                    option { value: "{value_for_unlisted}", selected: true, "{text}" }
                }
                for (id , text , is_selected) in options {
                    option { value: "{id}", selected: is_selected, "{text}" }
                }
            }
        }
    }
}
