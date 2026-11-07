use dioxus::prelude::*;

use super::{
    api::{fetch_page, HistoryFilters, PAGE_SIZE},
    columns::{best_values, Column, ColumnPrefs},
    filters::{is_active, FilterRow},
    models::{Counts, RunSummary},
    run_row::RunRow,
    selection,
};
use crate::{
    app::Route,
    components::{
        button_class, ButtonVariant, EmptyState, GhostRows, Icon, IconKind, PageHeader,
        RowsSkeleton,
    },
    features::connection::{ApiError, ApiErrorPanel},
    format::{format_number, path_tail},
};

const HEADER_CELL: &str = "px-(--space-3) py-(--space-2) text-(length:--fs-xs) font-(weight:--fw-medium) whitespace-nowrap text-(color:--clr-text-muted)";

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Loading,
    LoadingMore,
    Ready,
    Failed(ApiError),
}

// Chips that name every active filter, shown when nothing matches
fn active_chips(filters: &HistoryFilters) -> Vec<String> {
    let mut chips = Vec::new();

    if !filters.runtime.is_empty() {
        chips.push(format!("Runtime {}", filters.runtime));
    }
    if !filters.status.is_empty() {
        chips.push(format!("Status {}", filters.status));
    }
    if !filters.target_path.trim().is_empty() {
        chips.push(format!("Project {}", path_tail(filters.target_path.trim())));
    }
    if !filters.search.trim().is_empty() {
        chips.push(format!("Search {}", filters.search.trim()));
    }

    chips
}

// Paginated run history with filters, a Columns menu and a two-run selection for comparisons.
// The filters come from the address (`/runs?target_path=&runtime=&status=&search=`).
#[component]
pub fn History(target_path: String, runtime: String, status: String, search: String) -> Element {
    let mut applied = use_signal(|| HistoryFilters {
        runtime: runtime.clone(),
        target_path: target_path.clone(),
        status: status.clone(),
        search: search.clone(),
    });
    let mut search_draft = use_signal(|| search.clone());
    let mut items = use_signal(Vec::<RunSummary>::new);
    let mut counts = use_signal(Counts::default);
    let mut next_cursor = use_signal(|| None::<String>);
    let mut phase = use_signal(|| Phase::Loading);
    let mut more_failed = use_signal(|| false);
    let mut generation = use_signal(|| 0u64);
    let mut selected = use_signal(Vec::<String>::new);
    let prefs = use_signal(ColumnPrefs::load);

    // Remembers the column choice in this browser
    use_effect(move || prefs.read().save());

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

            more_failed.set(false);
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
                        counts.set(page.counts);
                    } else {
                        items.write().extend(page.items);
                    }
                    next_cursor.set(page.pagination.next_cursor);
                    phase.set(Phase::Ready);
                }
                Err(error) if reset => phase.set(Phase::Failed(error)),
                Err(_) => {
                    more_failed.set(true);
                    phase.set(Phase::Ready);
                }
            }
        });
    };

    // A new address (for example the top bar search) replaces the applied filters
    use_effect(use_reactive!(|(target_path, runtime, status, search)| {
        let from_address = HistoryFilters {
            runtime,
            target_path,
            status,
            search,
        };

        if *applied.peek() != from_address {
            search_draft.set(from_address.search.clone());
            applied.set(from_address);
        }
    }));

    // Reloads from the first page whenever the applied filters change
    use_effect(move || {
        let _ = applied.read();
        load(true);
    });

    let current_phase = phase();
    let run_count = items.read().len();
    let total = counts.read().total;
    let selected_ids = selected();
    let can_compare = selected_ids.len() == selection::MAX_SELECTED;
    let primary = button_class(ButtonVariant::Primary);
    let secondary = button_class(ButtonVariant::Secondary);
    let filters_active = is_active(&applied.read());

    let compare_target = if can_compare {
        Some(Route::Comparisons {
            baseline: selected_ids[0].clone(),
            comparison: selected_ids[1].clone(),
        })
    } else {
        None
    };

    let mut clear_filters = move || {
        search_draft.set(String::new());
        applied.set(HistoryFilters::default());
    };

    let loading_more = matches!(current_phase, Phase::LoadingMore);

    let body: Element = match current_phase.clone() {
        Phase::Loading => rsx! {
            RowsSkeleton {}
        },
        Phase::Failed(error) => rsx! {
            ApiErrorPanel { error, on_retry: move |_| load(true) }
        },
        Phase::Ready | Phase::LoadingMore if run_count == 0 && filters_active => {
            let chips = active_chips(&applied.read());

            rsx! {
                EmptyState {
                    title: "No runs match these filters",
                    description: "Try another runtime or a shorter path.",
                    for chip in chips {
                        span {
                            key: "{chip}",
                            class: "rounded-(--round-full) bg-(--clr-surface-hover) px-(--space-3) py-(--space-1) text-(length:--fs-xs) text-(color:--clr-text-muted)",
                            "{chip}"
                        }
                    }
                    button {
                        r#type: "button",
                        class: "{primary}",
                        onclick: move |_| clear_filters(),
                        "Clear filters"
                    }
                }
            }
        }
        Phase::Ready | Phase::LoadingMore if run_count == 0 => rsx! {
            EmptyState {
                title: "No runs yet",
                description: "Start a benchmark and its result will show up here.",
                ghost: Some(rsx! {
                    GhostRows {}
                }),
                Link { to: Route::new_benchmark(), class: "{primary}", "Start a benchmark" }
            }
        },
        _ => {
            let has_more = next_cursor.read().is_some();
            let shown = prefs.read().shown();
            let runs: Vec<RunSummary> = items.read().clone();
            let best = best_values(&runs, &shown);
            let highest_throughput = runs
                .iter()
                .filter_map(|run| Column::Throughput.measured(run))
                .fold(0.0_f64, f64::max);
            let both_picked = selected_ids.len() == selection::MAX_SELECTED;
            let rows: Vec<(usize, RunSummary, Option<selection::Role>, bool)> = runs
                .into_iter()
                .enumerate()
                .map(|(index, run)| {
                    let role = selection::role_of(&selected_ids, &run.run_id);
                    let dimmed = both_picked && role.is_none();

                    (index, run, role, dimmed)
                })
                .collect();
            let headers: Vec<(Column, String)> = shown
                .iter()
                .map(|column| {
                    let align = if column.is_numeric() {
                        "text-right"
                    } else {
                        "text-left"
                    };

                    (*column, align.to_string())
                })
                .collect();
            let showing = format_number(run_count as f64, 0);
            let of_total = format_number(total as f64, 0);

            rsx! {
                div { class: "rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) p-1.5",
                    div { class: "overflow-x-auto",
                        table { class: "w-full min-w-(--table-min-width) border-separate border-spacing-y-0.5 text-left text-(length:--fs-sm)",
                            thead {
                                tr {
                                    th { class: "w-10 {HEADER_CELL}", span { class: "sr-only", "Select" } }
                                    th { class: "{HEADER_CELL}", "Run" }
                                    for (column , align) in headers {
                                        th { key: "{column.label()}", class: "{HEADER_CELL} {align}",
                                            "{column.label()}"
                                        }
                                    }
                                    th { class: "w-10 {HEADER_CELL}", span { class: "sr-only", "Open" } }
                                }
                            }
                            tbody {
                                for (index , run , role , dimmed) in rows {
                                    RunRow {
                                        key: "{run.run_id}",
                                        run,
                                        columns: shown.clone(),
                                        best: best.clone(),
                                        highest_throughput,
                                        role,
                                        dimmed,
                                        index,
                                        on_toggle: move |id: String| {
                                            selected.with_mut(|current| selection::toggle(current, id));
                                        },
                                    }
                                }
                            }
                        }
                    }
                    div { class: "flex flex-col items-center justify-between gap-(--space-3) border-t border-(color:--clr-border) px-(--space-3) py-(--space-3) sm:flex-row",
                        span { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)",
                            "Showing {showing} of {of_total} runs"
                        }
                        if more_failed() {
                            div { class: "flex items-center gap-(--space-3)",
                                span { class: "text-(length:--fs-sm) text-(color:--clr-danger-text)",
                                    "Couldn't load more runs"
                                }
                                button {
                                    r#type: "button",
                                    class: "{secondary}",
                                    onclick: move |_| load(false),
                                    "Try again"
                                }
                            }
                        } else if has_more {
                            button {
                                r#type: "button",
                                class: "{secondary}",
                                disabled: loading_more,
                                onclick: move |_| load(false),
                                if loading_more {
                                    Icon { kind: IconKind::Refresh, class: "size-(--icon-sm) animate-spin" }
                                    "Loading"
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

    let selected_count = selected_ids.len();
    let selection_text = if selected_count == 1 {
        "1 run selected".to_string()
    } else {
        format!("{selected_count} runs selected")
    };

    rsx! {
        document::Title { "Runs · Benchmark" }
        PageHeader {
            title: "Runs",
            description: "Every stored benchmark run, newest first. Click two rows to compare them.",
            span {}
        }
        div { class: "flex flex-col gap-(--space-4)",
            FilterRow {
                applied,
                search_draft,
                counts: counts(),
                prefs,
                on_clear: move |_| clear_filters(),
            }
            {body}
            if selected_count > 0 {
                div { class: "fade-up sticky bottom-4 z-10 flex items-center justify-between gap-(--space-3) rounded-(--round-lg) bg-(--clr-ink) px-(--space-4) py-(--space-3) text-(color:--clr-ink-contrast) shadow-(--shadow-popover)",
                    span { class: "text-(length:--fs-sm) font-(weight:--fw-medium)", "{selection_text}" }
                    div { class: "flex items-center gap-(--space-2)",
                        button {
                            r#type: "button",
                            class: "cursor-pointer rounded-(--round-md) px-(--space-3) py-(--space-1) text-(length:--fs-sm) transition-colors hover:bg-white/10",
                            onclick: move |_| selected.set(Vec::new()),
                            "Clear"
                        }
                        if let Some(route) = compare_target {
                            Link { to: route, class: "{primary}",
                                Icon { kind: IconKind::Compare, class: "size-(--icon-sm)" }
                                "Compare"
                            }
                        } else {
                            button {
                                r#type: "button",
                                class: "{primary}",
                                disabled: true,
                                "Compare"
                            }
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
    fn names_every_active_filter() {
        let filters = HistoryFilters {
            runtime: "bun".to_string(),
            target_path: "/srv/shop".to_string(),
            status: "failed".to_string(),
            search: " 1a2b ".to_string(),
        };

        assert_eq!(
            active_chips(&filters),
            [
                "Runtime bun",
                "Status failed",
                "Project shop",
                "Search 1a2b"
            ]
        );
        assert!(active_chips(&HistoryFilters::default()).is_empty());
    }
}
