use dioxus::prelude::*;

use super::{
    api::{fetch_page, HistoryFilters, PAGE_SIZE, RUNTIMES},
    models::RunSummary,
    run_row::RunRow,
    selection,
};
use crate::{
    app::Route,
    components::{
        button_class, ButtonVariant, Card, CardHeader, EmptyState, FormField, Icon, IconKind,
        LoadingPanel, PageHeader, INPUT_CLASS,
    },
    features::connection::{ApiError, ApiErrorPanel},
};

const HEADER_CELL: &str = "px-(--space-3) py-(--space-2) text-(length:--fs-xs) font-(weight:--fw-medium) uppercase tracking-wider text-(color:--clr-text-faint)";

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Loading,
    LoadingMore,
    Ready,
    Failed(ApiError),
}

// Paginated run history with filters and a two-run selection for comparisons
#[component]
pub fn History() -> Element {
    let mut applied = use_signal(HistoryFilters::default);
    let mut runtime_draft = use_signal(String::new);
    let mut path_draft = use_signal(String::new);
    let mut items = use_signal(Vec::<RunSummary>::new);
    let mut next_cursor = use_signal(|| None::<String>);
    let mut phase = use_signal(|| Phase::Loading);
    let mut generation = use_signal(|| 0u64);
    let mut selected = use_signal(Vec::<String>::new);

    // Loads the first page, or the next one when `reset` is false
    let load = move |reset: bool| {
        spawn(async move {
            // A newer reset invalidates every response still in flight
            let ticket = if reset {
                let next = *generation.peek() + 1;
                generation.set(next);
                next
            } else {
                *generation.peek()
            };

            let filters = applied.peek().clone();
            let before = if reset {
                None
            } else {
                next_cursor.peek().clone()
            };

            phase.set(if reset {
                Phase::Loading
            } else {
                Phase::LoadingMore
            });

            let result = fetch_page(&filters, before.as_deref(), PAGE_SIZE).await;

            if *generation.peek() != ticket {
                return;
            }

            match result {
                Ok(page) => {
                    if reset {
                        items.set(page.items);
                    } else {
                        items.write().extend(page.items);
                    }
                    next_cursor.set(page.pagination.next_cursor);
                    phase.set(Phase::Ready);
                }
                Err(error) => phase.set(Phase::Failed(error)),
            }
        });
    };

    // Reloads from the first page whenever the applied filters change
    use_effect(move || {
        let _ = applied.read();
        load(true);
    });

    let current_phase = phase();
    let run_count = items.read().len();
    let selected_ids = selected();
    let can_compare = selected_ids.len() == selection::MAX_SELECTED;
    let primary = button_class(ButtonVariant::Primary);
    let secondary = button_class(ButtonVariant::Secondary);

    let compare_target = if can_compare {
        Some(Route::Comparisons {
            baseline: selected_ids[0].clone(),
            comparison: selected_ids[1].clone(),
        })
    } else {
        None
    };

    let loading_more = matches!(current_phase, Phase::LoadingMore);

    let body: Element = match current_phase.clone() {
        Phase::Loading => rsx! {
            LoadingPanel { label: "Loading run history" }
        },
        Phase::Failed(error) => rsx! {
            ApiErrorPanel { error, on_retry: move |_| load(true) }
        },
        Phase::Ready | Phase::LoadingMore if run_count == 0 => rsx! {
            EmptyState {
                title: "No runs match",
                description: "Nothing has been recorded for these filters yet. Clear the filters or run a new benchmark.",
                Link { to: Route::Submission {}, class: "{primary}", "New benchmark" }
            }
        },
        _ => {
            let has_more = next_cursor.read().is_some();
            let rows: Vec<(String, RunSummary, bool)> = items
                .read()
                .iter()
                .map(|run| {
                    (
                        run.run_id.clone(),
                        run.clone(),
                        selected_ids.contains(&run.run_id),
                    )
                })
                .collect();

            rsx! {
                Card {
                    CardHeader {
                        title: "Runs",
                        subtitle: "Newest first. Select two runs to compare them.",
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full min-w-(--table-min-width) border-collapse text-left text-(length:--fs-sm)",
                            thead {
                                tr {
                                    th { class: "{HEADER_CELL}", span { class: "sr-only", "Select" } }
                                    th { class: "{HEADER_CELL}", "Run" }
                                    th { class: "{HEADER_CELL}", "Target" }
                                    th { class: "{HEADER_CELL}", "Runtime" }
                                    th { class: "{HEADER_CELL} text-right", "Build" }
                                    th { class: "{HEADER_CELL} text-right", "Startup" }
                                    th { class: "{HEADER_CELL} text-right", "Peak memory" }
                                    th { class: "{HEADER_CELL} text-right", "CPU avg" }
                                    th { class: "{HEADER_CELL} text-right", "Throughput" }
                                    th { class: "{HEADER_CELL} text-right", "p95" }
                                }
                            }
                            tbody {
                                for (row_key , run , is_selected) in rows {
                                    RunRow {
                                        key: "{row_key}",
                                        run,
                                        selected: is_selected,
                                        on_toggle: move |id: String| {
                                            selected.with_mut(|current| selection::toggle(current, id));
                                        },
                                    }
                                }
                            }
                        }
                    }
                    div { class: "flex flex-col items-center justify-between gap-(--space-3) border-t border-(color:--clr-border) px-(--space-5) py-(--space-4) sm:flex-row",
                        span { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)",
                            "Showing {run_count} runs"
                        }
                        if has_more {
                            button {
                                r#type: "button",
                                class: "{secondary}",
                                disabled: loading_more,
                                onclick: move |_| load(false),
                                if loading_more {
                                    "Loading…"
                                } else {
                                    "Load more"
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    rsx! {
        document::Title { "Runs · Benchmark" }
        PageHeader {
            title: "Run history",
            description: "Every stored benchmark run with its headline measurements.",
            Link { to: Route::Submission {}, class: "{primary}",
                Icon { kind: IconKind::Play, class: "size-(--icon-sm)" }
                "New benchmark"
            }
        }
        div { class: "flex flex-col gap-(--space-6)",
            Card {
                form {
                    class: "flex flex-col gap-(--space-3) p-(--space-4) md:flex-row md:items-end",
                    onsubmit: move |event| {
                        event.prevent_default();
                        applied
                            .set(HistoryFilters {
                                runtime: runtime_draft(),
                                target_path: path_draft(),
                            });
                    },
                    div { class: "md:w-56",
                        FormField { label: "Runtime",
                            select {
                                class: "{INPUT_CLASS}",
                                onchange: move |event| runtime_draft.set(event.value()),
                                option { value: "", selected: runtime_draft().is_empty(), "All runtimes" }
                                for (code , name) in RUNTIMES {
                                    option { value: "{code}", selected: runtime_draft() == code, "{name}" }
                                }
                            }
                        }
                    }
                    div { class: "min-w-0 md:flex-1",
                        FormField {
                            label: "Target path",
                            hint: "Exact match on the stored path",
                            input {
                                r#type: "text",
                                class: "{INPUT_CLASS}",
                                placeholder: "/home/me/projects/my-api",
                                value: "{path_draft}",
                                oninput: move |event| path_draft.set(event.value()),
                            }
                        }
                    }
                    div { class: "flex gap-(--space-2)",
                        button { r#type: "submit", class: "{primary}", "Apply" }
                        button {
                            r#type: "button",
                            class: "{secondary}",
                            onclick: move |_| {
                                runtime_draft.set(String::new());
                                path_draft.set(String::new());
                                applied.set(HistoryFilters::default());
                            },
                            "Reset"
                        }
                    }
                }
            }
            div { class: "flex flex-col gap-(--space-3) rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) px-(--space-4) py-(--space-3) sm:flex-row sm:items-center sm:justify-between",
                span { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)",
                    if selected_ids.is_empty() {
                        "Select two runs to compare. The first one you pick is the baseline."
                    } else if can_compare {
                        "Two runs selected. Ready to compare."
                    } else {
                        "One run selected. Pick one more."
                    }
                }
                div { class: "flex gap-(--space-2)",
                    if let Some(route) = compare_target {
                        Link { to: route, class: "{primary}",
                            Icon { kind: IconKind::Compare, class: "size-(--icon-sm)" }
                            "Compare selected"
                        }
                    } else {
                        button {
                            r#type: "button",
                            class: "{primary}",
                            disabled: true,
                            "Compare selected"
                        }
                    }
                    if !selected_ids.is_empty() {
                        button {
                            r#type: "button",
                            class: "{secondary}",
                            onclick: move |_| selected.set(Vec::new()),
                            "Clear"
                        }
                    }
                }
            }
            {body}
        }
    }
}
