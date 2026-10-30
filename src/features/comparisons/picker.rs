use dioxus::prelude::*;

use crate::{
    components::INPUT_CLASS,
    features::{charts::ChartTone, history::RunSummary},
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

// Select listing the newest runs, keeps an id that is not in the list selectable.
// The dot in front of the label uses the same color as that run in the charts.
#[component]
pub fn RunPicker(
    #[props(into)] label: String,
    value: String,
    runs: Vec<RunSummary>,
    tone: ChartTone,
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
    let swatch = tone.swatch_class();

    rsx! {
        label { class: "flex min-w-0 flex-col gap-(--space-1)",
            span { class: "flex items-center gap-(--space-2) text-(length:--fs-xs) font-(weight:--fw-medium) text-(color:--clr-text-muted)",
                span { class: "inline-block size-(--dot-size) rounded-(--round-full) {swatch}" }
                "{label}"
            }
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
