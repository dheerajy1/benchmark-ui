use dioxus::prelude::*;

use crate::features::{
    configurations::{compare_configurations, fetch_run_configuration, ConfigurationDifference},
    connection::ApiError,
};

// Says when the two compared runs did not use the same configuration. Quiet when they did, and
// quiet when a configuration cannot be loaded, because the comparison itself still stands.
#[component]
pub fn ConfigurationNotice(baseline: String, comparison: String) -> Element {
    let reactive_baseline = baseline.clone();
    let reactive_comparison = comparison.clone();

    let pair = use_resource(use_reactive!(
        |(reactive_baseline, reactive_comparison)| async move {
            let left = fetch_run_configuration(&reactive_baseline).await?;
            let right = fetch_run_configuration(&reactive_comparison).await?;

            Ok::<_, ApiError>((left, right))
        }
    ));

    let body: Element = match pair() {
        None | Some(Err(_)) => rsx! {},
        Some(Ok((left, right))) => match compare_configurations(&left, &right) {
            ConfigurationDifference::Same => rsx! {},
            ConfigurationDifference::NotComparable => rsx! {
                p {
                    role: "status",
                    class: "rounded-(--round-md) border border-(color:--clr-info) bg-(--clr-info-soft) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-info)",
                    "One of these runs has no recorded configuration, so its settings cannot be compared."
                }
            },
            ConfigurationDifference::Differs(fields) => {
                let names = fields.join(", ");

                rsx! {
                    p {
                        role: "status",
                        class: "rounded-(--round-md) border border-(color:--clr-warning) bg-(--clr-warning-soft) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-warning)",
                        "These runs did not use the same configuration. Changed: {names}. Read the results with that in mind."
                    }
                }
            }
        },
    };

    rsx! {
        {body}
    }
}
