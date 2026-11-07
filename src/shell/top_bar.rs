use dioxus::prelude::*;

use super::state::use_shell;
use crate::{
    app::Route,
    components::{button_class, ButtonVariant, Icon, IconKind, Tooltip},
    features::{
        connection::ApiPill,
        live::{progress_text, ACTIVE_RUN},
    },
    format::short_id,
};

// Breadcrumb items from the first to the current page. Only the last one is not a link.
fn breadcrumbs(route: &Route) -> Vec<(String, Option<Route>)> {
    match route {
        Route::Overview { .. } => vec![("Overview".to_string(), None)],
        Route::History { .. } => vec![
            ("Runs".to_string(), Some(Route::runs())),
            ("History".to_string(), None),
        ],
        Route::RunDetail { run_id } => vec![
            ("Runs".to_string(), Some(Route::runs())),
            (format!("Run {}", short_id(run_id)), None),
        ],
        Route::LiveRun { run_id } => vec![
            ("New benchmark".to_string(), Some(Route::new_benchmark())),
            (format!("Live run {}", short_id(run_id)), None),
        ],
        Route::Comparisons { .. } => vec![("Compare".to_string(), None)],
        Route::Submission { .. } | Route::SubmissionFrom { .. } => {
            vec![("New benchmark".to_string(), None)]
        }
        Route::NotFound { .. } => vec![("Not found".to_string(), None)],
    }
}

// Search field. Enter opens the Runs page filtered by the text.
#[component]
fn SearchField() -> Element {
    let shell = use_shell();
    let nav = use_navigator();
    let mut term = use_signal(String::new);

    rsx! {
        div { class: "group/search relative hidden h-9 w-[190px] items-center rounded-(--round-md) border border-transparent bg-(--clr-surface-hover) transition-[width,background-color,border-color,box-shadow] duration-350 ease-(--ease-out) focus-within:w-[300px] focus-within:border-(color:--clr-accent) focus-within:bg-(--clr-surface) focus-within:shadow-(--halo-focus) md:flex",
            Icon {
                kind: IconKind::Search,
                class: "pointer-events-none absolute left-(--space-3) size-4 text-(color:--clr-text-faint)",
            }
            input {
                r#type: "text",
                class: "h-full w-full min-w-0 bg-transparent pr-16 pl-9 text-(length:--fs-sm) text-(color:--clr-text) outline-none placeholder:text-(color:--clr-text-faint)",
                placeholder: "Search runs",
                "aria-label": "Search runs",
                value: "{term}",
                oninput: move |event| term.set(event.value()),
                onmounted: move |event| shell.register_search(event.data()),
                onkeydown: move |event| {
                    if event.key() == Key::Enter {
                        let query = term().trim().to_string();
                        if !query.is_empty() {
                            let _ = nav
                                .push(Route::History {
                                    target_path: String::new(),
                                    runtime: String::new(),
                                    status: String::new(),
                                    search: query,
                                });
                        }
                    }
                },
            }
            span { class: "pointer-events-none absolute right-(--space-3) rounded-(--round-sm) border border-(color:--clr-border) bg-(--clr-surface) px-(--space-1) text-(length:--fs-2xs) text-(color:--clr-text-faint) transition-opacity duration-200 group-focus-within/search:opacity-0",
                "Ctrl K"
            }
        }
    }
}

// Pulsing chip that opens the live page while a run executes
#[component]
fn ActiveRunChip() -> Element {
    let Some(run) = ACTIVE_RUN.read().clone() else {
        return rsx! {};
    };

    let progress = progress_text(&run);
    let label = if progress.is_empty() {
        "Run in progress".to_string()
    } else {
        format!("Run in progress · {progress}")
    };

    rsx! {
        Link {
            to: Route::LiveRun {
                run_id: run.run_id,
            },
            class: "inline-flex h-8 items-center gap-(--space-2) rounded-(--round-full) bg-(--clr-accent-soft) px-(--space-3) text-(length:--fs-xs) font-(weight:--fw-medium) whitespace-nowrap text-(color:--clr-accent) transition-colors hover:bg-(--clr-accent-soft)/70",
            span { class: "size-(--dot-size) rounded-(--round-full) bg-(--clr-accent) pulse-dot text-(color:--clr-accent)" }
            "{label}"
        }
    }
}

// Top bar: the sidebar button and breadcrumbs on the left, search, API pill, active run and
// the New benchmark button on the right
#[component]
pub fn TopBar() -> Element {
    let shell = use_shell();
    let route = use_route::<Route>();
    let crumbs = breadcrumbs(&route);
    let primary = button_class(ButtonVariant::Primary);

    let pinned = shell.is_pinned();
    let (tooltip, button_class_state) = if pinned {
        (
            "Hide sidebar",
            "bg-(--clr-accent-soft) text-(color:--clr-accent)",
        )
    } else {
        (
            "Show sidebar",
            "text-(color:--clr-text-muted) hover:bg-(--clr-hover-wash) hover:text-(color:--clr-text)",
        )
    };

    rsx! {
        header { class: "sticky top-0 z-20 flex h-(--topbar-height) items-center justify-between gap-(--space-3) border-b border-(color:--clr-border) bg-(--clr-surface) px-(--space-4) sm:px-(--space-6)",
            div { class: "flex min-w-0 items-center gap-(--space-2)",
                Tooltip {
                    text: tooltip,
                    hint: "Ctrl+B",
                    start: true,
                    suppressed: !pinned && shell.is_peeking(),
                    button {
                        r#type: "button",
                        class: "inline-flex size-9 cursor-pointer items-center justify-center rounded-(--round-md) transition-colors {button_class_state}",
                        "aria-label": "{tooltip}",
                        "aria-pressed": "{pinned}",
                        onclick: move |_| shell.toggle_pin(),
                        onmouseenter: move |_| shell.peek_in(),
                        onmouseleave: move |_| shell.peek_out(),
                        Icon { kind: IconKind::Panel, class: "size-5" }
                    }
                }
                nav {
                    class: "flex min-w-0 items-center gap-(--space-1) text-(length:--fs-base)",
                    "aria-label": "Breadcrumb",
                    for (index , (label , target)) in crumbs.into_iter().enumerate() {
                        if index > 0 {
                            Icon {
                                kind: IconKind::ChevronRight,
                                class: "size-3.5 text-(color:--clr-text-faint)",
                            }
                        }
                        if let Some(target) = target {
                            Link {
                                to: target,
                                class: "truncate rounded-(--round-sm) px-(--space-2) py-(--space-1) text-(color:--clr-text-muted) transition-colors hover:bg-(--clr-hover-wash)",
                                "{label}"
                            }
                        } else {
                            span {
                                class: "truncate px-(--space-2) font-(weight:--fw-medium) text-(color:--clr-text)",
                                "aria-current": "page",
                                "{label}"
                            }
                        }
                    }
                }
            }
            div { class: "flex shrink-0 items-center gap-(--space-3)",
                SearchField {}
                ActiveRunChip {}
                ApiPill {}
                Link {
                    to: Route::new_benchmark(),
                    class: "group/new {primary}",
                    Icon {
                        kind: IconKind::Plus,
                        class: "size-4 transition-transform duration-300 ease-(--ease-out) group-hover/new:rotate-90",
                    }
                    "New benchmark"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(route: &Route) -> Vec<String> {
        breadcrumbs(route)
            .into_iter()
            .map(|(label, _)| label)
            .collect()
    }

    #[test]
    fn names_each_page() {
        assert_eq!(labels(&Route::overview()), ["Overview"]);
        assert_eq!(labels(&Route::runs()), ["Runs", "History"]);
        assert_eq!(
            labels(&Route::RunDetail {
                run_id: "0192f3a0-0000-7000-8000-00000000abcd".to_string(),
            }),
            ["Runs", "Run 0000abcd"]
        );
        assert_eq!(labels(&Route::new_benchmark()), ["New benchmark"]);
    }

    #[test]
    fn only_the_last_item_is_not_a_link() {
        let crumbs = breadcrumbs(&Route::runs());

        assert!(crumbs[0].1.is_some());
        assert!(crumbs[1].1.is_none());
    }
}
