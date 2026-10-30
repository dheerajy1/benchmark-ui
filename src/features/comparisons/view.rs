use dioxus::prelude::*;

use super::{
    api::fetch_comparison, models::ComparisonData, picker::RunPicker, results::ComparisonResults,
};
use crate::{
    app::Route,
    components::{
        button_class, ButtonVariant, Card, EmptyState, Icon, IconKind, LoadingPanel, PageHeader,
    },
    features::{
        connection::{ApiError, ApiErrorPanel},
        history::fetch_recent_runs,
    },
};

// Compares two stored runs, the selection lives in the URL so a comparison can be shared
#[component]
pub fn Comparisons(baseline: String, comparison: String) -> Element {
    let nav = use_navigator();

    let mut runs = use_resource(move || async move { fetch_recent_runs().await });
    let reactive_baseline = baseline.clone();
    let reactive_comparison = comparison.clone();
    let mut result = use_resource(use_reactive!(
        |(reactive_baseline, reactive_comparison)| async move {
            let same = reactive_baseline
                .trim()
                .eq_ignore_ascii_case(reactive_comparison.trim());

            if reactive_baseline.trim().is_empty() || reactive_comparison.trim().is_empty() || same
            {
                return Ok::<Option<ComparisonData>, ApiError>(None);
            }

            fetch_comparison(&reactive_baseline, &reactive_comparison)
                .await
                .map(Some)
        }
    ));

    let secondary = button_class(ButtonVariant::Secondary);
    let same_selection =
        !baseline.is_empty() && baseline.trim().eq_ignore_ascii_case(comparison.trim());

    let pickers: Element = match runs() {
        None => rsx! {
            LoadingPanel { label: "Loading runs" }
        },
        Some(Err(error)) => rsx! {
            ApiErrorPanel { error, on_retry: move |_| runs.restart() }
        },
        Some(Ok(page)) => {
            let other_for_baseline = comparison.clone();
            let other_for_comparison = baseline.clone();
            let swap_baseline = comparison.clone();
            let swap_comparison = baseline.clone();

            rsx! {
                Card {
                    div { class: "flex flex-col gap-(--space-4) p-(--space-4) md:flex-row md:items-end",
                        div { class: "min-w-0 md:flex-1",
                            RunPicker {
                                label: "Baseline",
                                value: baseline.clone(),
                                runs: page.items.clone(),
                                on_change: move |id: String| {
                                    let _ = nav
                                        .push(Route::Comparisons {
                                            baseline: id,
                                            comparison: other_for_baseline.clone(),
                                        });
                                },
                            }
                        }
                        button {
                            r#type: "button",
                            class: "{secondary}",
                            "aria-label": "Swap baseline and comparison",
                            onclick: move |_| {
                                let _ = nav
                                    .push(Route::Comparisons {
                                        baseline: swap_baseline.clone(),
                                        comparison: swap_comparison.clone(),
                                    });
                            },
                            Icon { kind: IconKind::Swap, class: "size-(--icon-sm)" }
                            "Swap"
                        }
                        div { class: "min-w-0 md:flex-1",
                            RunPicker {
                                label: "Comparison",
                                value: comparison.clone(),
                                runs: page.items.clone(),
                                on_change: move |id: String| {
                                    let _ = nav
                                        .push(Route::Comparisons {
                                            baseline: other_for_comparison.clone(),
                                            comparison: id,
                                        });
                                },
                            }
                        }
                    }
                }
            }
        }
    };

    let body: Element = if same_selection {
        rsx! {
            EmptyState {
                title: "Pick two different runs",
                description: "A run cannot be compared with itself.",
                span { class: "text-(length:--fs-sm) text-(color:--clr-text-faint)", "Change the comparison run above." }
            }
        }
    } else {
        match result() {
            None => rsx! {
                LoadingPanel { label: "Comparing runs" }
            },
            Some(Err(error)) => rsx! {
                ApiErrorPanel { error, on_retry: move |_| result.restart() }
            },
            Some(Ok(None)) => rsx! {
                EmptyState {
                    title: "Choose two runs",
                    description: "The first run is the baseline, the second is compared against it.",
                    span { class: "text-(length:--fs-sm) text-(color:--clr-text-faint)", "Select both above, or pick runs from the history." }
                }
            },
            Some(Ok(Some(data))) => rsx! {
                ComparisonResults { data }
            },
        }
    };

    rsx! {
        document::Title { "Compare · Benchmark" }
        PageHeader {
            title: "Compare runs",
            description: "See what changed between a baseline run and a newer one.",
            Link { to: Route::History {}, class: "{secondary}", "Browse runs" }
        }
        div { class: "flex flex-col gap-(--space-6)",
            {pickers}
            {body}
        }
    }
}
