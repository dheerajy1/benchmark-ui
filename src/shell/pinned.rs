use dioxus::prelude::*;

use super::{
    recents::{RecentRun, SectionToggle},
    state::use_shell,
};
use crate::{
    components::{Collapse, Icon, IconKind, Skeleton},
    features::{
        history::{fetch_page, runtime_label, HistoryFilters},
        targets::{fetch_targets, TargetView},
    },
};

// Runs loaded for one saved target, the newest also names its runtime
const TARGET_RUNS: u32 = 5;

// One saved target: folder row that expands to its newest runs under a thin line
#[component]
fn PinnedTarget(target: TargetView) -> Element {
    let mut expanded = use_signal(|| false);
    let path = target.path.clone();
    let shell = use_shell();

    let runs = use_resource(move || {
        let filters = HistoryFilters {
            target_path: path.clone(),
            ..HistoryFilters::default()
        };
        // A deleted run leaves this list too
        let _ = shell.recents_version();

        async move { fetch_page(&filters, None, TARGET_RUNS).await }
    });

    let page = runs().and_then(Result::ok);
    let runtime = page
        .as_ref()
        .and_then(|page| page.items.first())
        .and_then(|run| run.runtime_code().map(runtime_label));
    let label = match runtime {
        Some(runtime) => format!("{}, {runtime}", target.name),
        None => target.name.clone(),
    };
    let chevron_class = if expanded() { "rotate-90" } else { "" };

    let target_runs: Element = match page {
        None => rsx! {
            Skeleton { class: "m-(--space-2) h-5" }
        },
        Some(page) if page.items.is_empty() => rsx! {
            p { class: "px-(--space-3) py-(--space-2) text-(length:--fs-xs) text-(color:--clr-text-faint)",
                "No runs yet"
            }
        },
        Some(page) => rsx! {
            for run in page.items {
                RecentRun { key: "{run.run_id}", run }
            }
        },
    };

    rsx! {
        div { class: "flex flex-col",
            button {
                r#type: "button",
                class: "group/row flex h-9 w-full cursor-pointer items-center gap-(--space-2) rounded-(--round-md) px-(--space-3) text-left text-(length:--fs-sm) text-(color:--clr-text) transition-colors hover:bg-(--clr-hover-wash)",
                "aria-expanded": "{expanded()}",
                title: "{target.path}",
                onclick: move |_| expanded.set(!expanded()),
                span { class: "relative size-4 shrink-0",
                    Icon {
                        kind: IconKind::Folder,
                        class: "absolute inset-0 size-4 text-(color:--clr-text-muted) transition-opacity duration-150 group-hover/row:opacity-0",
                    }
                    Icon {
                        kind: IconKind::ChevronRight,
                        class: "absolute inset-0 size-4 text-(color:--clr-text-muted) opacity-0 transition-[opacity,transform] duration-300 ease-(--ease-out) group-hover/row:opacity-100 {chevron_class}",
                    }
                }
                span { class: "min-w-0 flex-1 truncate", "{label}" }
            }
            Collapse { open: expanded(),
                div { class: "ml-(--space-5) flex flex-col border-l border-(color:--clr-border) pl-(--space-1)",
                    {target_runs}
                }
            }
        }
    }
}

// The Pinned section: the saved targets of the server
#[component]
pub fn PinnedSection() -> Element {
    let mut open = use_signal(|| true);
    let mut targets = use_resource(fetch_targets);

    let body: Element = match targets() {
        None => rsx! {
            div { class: "px-(--space-3)",
                Skeleton { class: "h-9 w-full" }
            }
        },
        Some(Err(_)) => rsx! {
            div { class: "flex items-center justify-between gap-(--space-2) px-(--space-3) py-(--space-2) text-(length:--fs-xs) text-(color:--clr-danger-text)",
                "Couldn't load targets"
                button {
                    r#type: "button",
                    class: "cursor-pointer font-(weight:--fw-medium) underline underline-offset-2",
                    onclick: move |_| targets.restart(),
                    "Try again"
                }
            }
        },
        Some(Ok(view)) if view.targets.is_empty() => rsx! {
            p { class: "px-(--space-3) py-(--space-2) text-(length:--fs-xs) text-(color:--clr-text-faint)",
                "No saved targets"
            }
        },
        Some(Ok(view)) => rsx! {
            for target in view.targets {
                PinnedTarget { key: "{target.id}", target }
            }
        },
    };

    rsx! {
        div { class: "flex shrink-0 flex-col",
            SectionToggle {
                label: "Pinned",
                open: open(),
                on_toggle: move |_| open.set(!open()),
            }
            div { class: "max-h-56 overflow-y-auto",
                Collapse { open: open(), {body} }
            }
        }
    }
}
