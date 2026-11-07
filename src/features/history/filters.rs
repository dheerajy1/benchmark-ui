use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use super::{
    api::{HistoryFilters, RUNTIMES},
    columns::ColumnPrefs,
    columns_menu::ColumnsMenu,
    models::Counts,
};
use crate::{
    components::{Icon, IconKind},
    format::path_tail,
};

// How long typing must pause before the search is sent
const SEARCH_DELAY_MS: u32 = 350;

// Whether any filter narrows the list
pub fn is_active(filters: &HistoryFilters) -> bool {
    !filters.runtime.is_empty()
        || !filters.status.is_empty()
        || !filters.target_path.trim().is_empty()
        || !filters.search.trim().is_empty()
}

// Pill with a label and the number of runs it would show
#[component]
fn Pill(
    #[props(into)] label: String,
    count: u64,
    active: bool,
    on_pick: EventHandler<()>,
) -> Element {
    let class = if active {
        "bg-(--clr-accent-soft) text-(color:--clr-accent)"
    } else {
        "text-(color:--clr-text-muted) hover:bg-(--clr-hover-wash) hover:text-(color:--clr-text)"
    };

    rsx! {
        button {
            r#type: "button",
            class: "inline-flex h-8 cursor-pointer items-center gap-(--space-2) rounded-(--round-full) px-(--space-3) text-(length:--fs-sm) font-(weight:--fw-medium) whitespace-nowrap transition-colors {class}",
            "aria-pressed": "{active}",
            onclick: move |_| on_pick.call(()),
            "{label}"
            span { class: "text-(length:--fs-xs) tabular-nums opacity-70", "{count}" }
        }
    }
}

// Filter row of the Runs page: runtime and status pills with counts, a chip for the project
// of a `View runs` link, `Clear filters`, the search and the Columns menu
#[component]
pub fn FilterRow(
    applied: Signal<HistoryFilters>,
    search_draft: Signal<String>,
    counts: Counts,
    prefs: Signal<ColumnPrefs>,
    on_clear: EventHandler<()>,
) -> Element {
    let mut applied = applied;
    let mut search_draft = search_draft;
    let mut ticket = use_signal(|| 0u64);

    let current = applied.read().clone();
    let active = is_active(&current);

    // A runtime stays in the row while chosen, even when it has no run for the other filters
    let runtimes: Vec<(&'static str, &'static str, u64)> = RUNTIMES
        .iter()
        .filter(|(code, _)| counts.runtime(code) > 0 || current.runtime == *code)
        .map(|(code, name)| (*code, *name, counts.runtime(code)))
        .collect();
    let statuses: [(&'static str, &'static str, u64); 3] = [
        ("done", "Done", counts.status("done")),
        ("failed", "Failed", counts.status("failed")),
        ("cancelled", "Cancelled", counts.status("cancelled")),
    ];
    let any_runtime = counts.any_runtime();
    let any_status = counts.any_status();
    let project = current.target_path.trim().to_string();
    let project_name = path_tail(&project);

    rsx! {
        div { class: "flex flex-wrap items-center gap-(--space-2)",
            Pill {
                label: "All",
                count: any_runtime,
                active: current.runtime.is_empty(),
                on_pick: move |_| {
                    let mut next = applied.peek().clone();
                    next.runtime = String::new();
                    applied.set(next);
                },
            }
            for (code , name , count) in runtimes {
                Pill {
                    key: "{code}",
                    label: name,
                    count,
                    active: current.runtime == code,
                    on_pick: move |_| {
                        let mut next = applied.peek().clone();
                        next.runtime = code.to_string();
                        applied.set(next);
                    },
                }
            }
            span { class: "mx-(--space-1) h-5 w-px bg-(--clr-border)" }
            Pill {
                label: "Any status",
                count: any_status,
                active: current.status.is_empty(),
                on_pick: move |_| {
                    let mut next = applied.peek().clone();
                    next.status = String::new();
                    applied.set(next);
                },
            }
            for (code , name , count) in statuses {
                Pill {
                    key: "{code}",
                    label: name,
                    count,
                    active: current.status == code,
                    on_pick: move |_| {
                        let mut next = applied.peek().clone();
                        next.status = code.to_string();
                        applied.set(next);
                    },
                }
            }
            if !project.is_empty() {
                span {
                    class: "inline-flex h-8 items-center gap-(--space-1) rounded-(--round-full) bg-(--clr-surface-hover) pr-(--space-1) pl-(--space-3) text-(length:--fs-sm) text-(color:--clr-text)",
                    title: "{project}",
                    "Project {project_name}"
                    button {
                        r#type: "button",
                        class: "inline-flex size-6 cursor-pointer items-center justify-center rounded-(--round-full) text-(color:--clr-text-muted) transition-colors hover:bg-(--clr-hover-wash) hover:text-(color:--clr-text)",
                        "aria-label": "Remove the project filter",
                        onclick: move |_| {
                            let mut next = applied.peek().clone();
                            next.target_path = String::new();
                            applied.set(next);
                        },
                        Icon { kind: IconKind::Close, class: "size-3.5" }
                    }
                }
            }
            if active {
                button {
                    r#type: "button",
                    class: "cursor-pointer rounded-(--round-md) px-(--space-2) py-(--space-1) text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-accent) transition-colors hover:bg-(--clr-accent-soft)",
                    onclick: move |_| on_clear.call(()),
                    "Clear filters"
                }
            }
            div { class: "ml-auto flex items-center gap-(--space-2)",
                div { class: "relative flex h-9 w-[180px] items-center rounded-(--round-md) border border-transparent bg-(--clr-surface-hover) transition-[width,background-color,border-color,box-shadow] duration-350 ease-(--ease-out) focus-within:w-[260px] focus-within:border-(color:--clr-accent) focus-within:bg-(--clr-surface) focus-within:shadow-(--halo-focus)",
                    Icon {
                        kind: IconKind::Search,
                        class: "pointer-events-none absolute left-(--space-3) size-4 text-(color:--clr-text-faint)",
                    }
                    input {
                        r#type: "text",
                        class: "h-full w-full min-w-0 bg-transparent pr-(--space-3) pl-9 text-(length:--fs-sm) text-(color:--clr-text) outline-none placeholder:text-(color:--clr-text-faint)",
                        placeholder: "Search path or run id",
                        "aria-label": "Search runs by path or run id",
                        value: "{search_draft}",
                        oninput: move |event| {
                            let text = event.value();
                            search_draft.set(text.clone());

                            let next_ticket = *ticket.peek() + 1;
                            ticket.set(next_ticket);

                            spawn(async move {
                                TimeoutFuture::new(SEARCH_DELAY_MS).await;

                                if *ticket.peek() != next_ticket {
                                    return;
                                }

                                let trimmed = text.trim().to_string();
                                let mut next = applied.peek().clone();

                                if next.search != trimmed {
                                    next.search = trimmed;
                                    applied.set(next);
                                }
                            });
                        },
                    }
                }
                ColumnsMenu { prefs }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_filter_set_is_not_active() {
        assert!(!is_active(&HistoryFilters::default()));
    }

    #[test]
    fn any_filter_makes_the_set_active() {
        let runtime = HistoryFilters {
            runtime: "bun".to_string(),
            ..HistoryFilters::default()
        };
        let blank_search = HistoryFilters {
            search: "  ".to_string(),
            ..HistoryFilters::default()
        };
        let project = HistoryFilters {
            target_path: "/srv/shop".to_string(),
            ..HistoryFilters::default()
        };

        assert!(is_active(&runtime));
        assert!(!is_active(&blank_search));
        assert!(is_active(&project));
    }
}
