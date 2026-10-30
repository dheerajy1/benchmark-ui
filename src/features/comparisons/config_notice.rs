use dioxus::prelude::*;

use crate::{
    features::configurations::{
        compare_entries, fetch_entry, ConfigurationComparison, FieldChange,
    },
    format::short_id,
};

// Says whether the two compared runs used the same configuration, and which settings differ.
// Every selected run is looked up directly, so a run outside the history list is still compared.
// When a run cannot be loaded, the notice says so instead of disappearing.
#[component]
pub fn ConfigurationNotice(baseline: String, comparison: String) -> Element {
    let reactive_baseline = baseline.clone();
    let reactive_comparison = comparison.clone();

    let outcome = use_resource(use_reactive!(
        |(reactive_baseline, reactive_comparison)| async move {
            let left = fetch_entry(&reactive_baseline)
                .await
                .map_err(|_| short_id(&reactive_baseline))?;
            let right = fetch_entry(&reactive_comparison)
                .await
                .map_err(|_| short_id(&reactive_comparison))?;

            Ok::<_, String>(compare_entries(&left, &right))
        }
    ));

    let body: Element = match outcome() {
        None => rsx! {},
        Some(Ok(result)) => comparison_view(result),
        Some(Err(run)) => rsx! {
            p {
                role: "status",
                class: "rounded-(--round-md) border border-(color:--clr-info) bg-(--clr-info-soft) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-info)",
                "The configuration of run {run} could not be loaded, so the settings cannot be compared."
            }
        },
    };

    rsx! {
        {body}
    }
}

fn comparison_view(result: ConfigurationComparison) -> Element {
    match result {
        ConfigurationComparison::Same => rsx! {
            p { class: "text-(length:--fs-xs) text-(color:--clr-text-faint)", "Same configuration" }
        },
        ConfigurationComparison::Changed(changes) => warning_box(changes),
        ConfigurationComparison::Partial {
            unrecorded,
            changes,
        } => {
            let names: Vec<String> = unrecorded.iter().map(|id| short_id(id)).collect();
            let noun = if names.len() == 1 { "run" } else { "runs" };
            let names = names.join(" and ");

            rsx! {
                p {
                    role: "status",
                    class: "rounded-(--round-md) border border-(color:--clr-info) bg-(--clr-info-soft) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-info)",
                    "Workload settings were not recorded for {noun} {names}, so only the start command and readiness could be compared."
                }
                if !changes.is_empty() {
                    {warning_box(changes)}
                }
            }
        }
    }
}

fn warning_box(changes: Vec<FieldChange>) -> Element {
    rsx! {
        div {
            role: "status",
            class: "flex flex-col gap-(--space-3) rounded-(--round-md) border border-(color:--clr-warning) bg-(--clr-warning-soft) p-(--space-3) text-(length:--fs-sm) text-(color:--clr-warning)",
            p { "These runs used different configurations." }
            div { class: "overflow-x-auto",
                table { class: "min-w-full text-left text-(length:--fs-sm) text-(color:--clr-text)",
                    thead {
                        tr {
                            th { class: "px-(--space-3) py-(--space-2) font-(weight:--fw-medium)", "Setting" }
                            th { class: "px-(--space-3) py-(--space-2) font-(weight:--fw-medium)", "Baseline" }
                            th { class: "px-(--space-3) py-(--space-2) font-(weight:--fw-medium)", "Comparison" }
                        }
                    }
                    tbody {
                        for change in changes {
                            tr { class: "border-t border-(color:--clr-border)",
                                td { class: "px-(--space-3) py-(--space-2) text-(color:--clr-text-muted)", "{change.label}" }
                                td { class: "px-(--space-3) py-(--space-2) break-all", "{change.baseline}" }
                                td { class: "px-(--space-3) py-(--space-2) break-all", "{change.comparison}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
