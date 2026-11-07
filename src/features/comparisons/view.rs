use dioxus::prelude::*;

use super::{
    api::fetch_comparison,
    config_notice::ConfigurationNotice,
    hero::Hero,
    kpi::KpiStrip,
    models::ComparisonData,
    results::ComparisonResults,
    selectors::RunSelectors,
    skeleton::{CompareSkeleton, PickersSkeleton},
};
use crate::{
    app::Route,
    components::{button_class, ButtonVariant, EmptyState, GhostSelectors, PageHeader},
    features::{
        connection::{ApiError, ApiErrorPanel},
        history::fetch_recent_runs,
    },
};

// Compares two stored runs, the selection lives in the URL so a comparison can be shared
#[component]
pub fn Comparisons(baseline: String, comparison: String) -> Element {
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
    let primary = button_class(ButtonVariant::Primary);
    let same_selection =
        !baseline.is_empty() && baseline.trim().eq_ignore_ascii_case(comparison.trim());

    let stored = match runs() {
        Some(Ok(ref page)) => Some(page.items.len()),
        _ => None,
    };

    let pickers: Element = match runs() {
        None => rsx! {
            PickersSkeleton {}
        },
        Some(Err(error)) => rsx! {
            ApiErrorPanel { error, on_retry: move |_| runs.restart() }
        },
        Some(Ok(page)) => rsx! {
            RunSelectors {
                runs: page.items,
                baseline: baseline.clone(),
                comparison: comparison.clone(),
            }
        },
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
                CompareSkeleton {}
            },
            Some(Err(error)) => rsx! {
                ApiErrorPanel { error, on_retry: move |_| result.restart() }
            },
            Some(Ok(None)) => match stored {
                Some(count) if count < 2 => rsx! {
                    EmptyState {
                        title: "One run is not enough",
                        description: "Comparing needs two stored runs. Run another benchmark first.",
                        ghost: Some(rsx! {
                            GhostSelectors { left: "One stored run", right: "Nothing to compare with" }
                        }),
                        Link { to: Route::new_benchmark(), class: "{primary}", "New benchmark" }
                    }
                },
                _ => rsx! {
                    EmptyState {
                        title: "Pick two runs to compare",
                        description: "The first run is the baseline, the second is compared against it.",
                        ghost: Some(rsx! {
                            GhostSelectors { left: "Baseline run", right: "Comparison run" }
                        }),
                        Link { to: Route::runs(), class: "{primary}", "Go to Runs" }
                    }
                },
            },
            Some(Ok(Some(data))) => {
                let baseline_id = data.baseline.run_id.clone();
                let comparison_id = data.comparison.run_id.clone();

                rsx! {
                    ConfigurationNotice { baseline: baseline_id, comparison: comparison_id }
                    Hero { data: data.clone() }
                    KpiStrip { data: data.clone() }
                    ComparisonResults { data }
                }
            }
        }
    };

    rsx! {
        document::Title { "Compare · Benchmark" }
        PageHeader {
            title: "Compare runs",
            description: "See what changed between a baseline run and a newer one.",
            Link { to: Route::runs(), class: "{secondary}", "Browse runs" }
        }
        div { class: "flex flex-col gap-(--space-4)",
            {pickers}
            {body}
        }
    }
}
