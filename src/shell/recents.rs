use std::collections::HashSet;

use dioxus::prelude::*;

use super::state::use_shell;
use crate::{
    app::Route,
    components::{Collapse, Icon, IconKind, Popover, PopoverAlign, RuntimeTag, Skeleton},
    features::{
        history::{fetch_page, runtime_label, HistoryFilters, RunSummary},
        live::ACTIVE_RUN,
    },
};

// Runs loaded into the sidebar
const RECENT_LIMIT: u32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusFilter {
    All,
    Done,
    Failed,
    Cancelled,
}

impl StatusFilter {
    const OPTIONS: [Self; 4] = [Self::All, Self::Done, Self::Failed, Self::Cancelled];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Done => "Done",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }

    // Value of the API `status` parameter, empty for every status
    fn api_value(self) -> &'static str {
        match self {
            Self::All => "",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupBy {
    None,
    Runtime,
    Status,
}

impl GroupBy {
    const OPTIONS: [Self; 3] = [Self::None, Self::Runtime, Self::Status];

    fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Runtime => "Runtime",
            Self::Status => "Status",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortBy {
    LastActivity,
    Throughput,
    Name,
}

impl SortBy {
    const OPTIONS: [Self; 3] = [Self::LastActivity, Self::Throughput, Self::Name];

    fn label(self) -> &'static str {
        match self {
            Self::LastActivity => "Last activity",
            Self::Throughput => "Throughput",
            Self::Name => "Name",
        }
    }
}

// One block of the list, `title` is empty when the list is not grouped
#[derive(Debug, Clone, PartialEq)]
struct RunGroup {
    title: String,
    runs: Vec<RunSummary>,
}

fn status_label(status: &str) -> String {
    let mut chars = status.chars();

    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

// Sorts the runs, then splits them into groups in the order the groups first appear
fn arrange(runs: &[RunSummary], group_by: GroupBy, sort_by: SortBy) -> Vec<RunGroup> {
    let mut sorted = runs.to_vec();

    match sort_by {
        SortBy::LastActivity => {}
        SortBy::Throughput => sorted.sort_by(|a, b| {
            let left = a.summary.requests_per_second.unwrap_or(f64::NEG_INFINITY);
            let right = b.summary.requests_per_second.unwrap_or(f64::NEG_INFINITY);

            right.total_cmp(&left)
        }),
        SortBy::Name => sorted.sort_by_key(|run| run.display_name.to_lowercase()),
    }

    let mut groups: Vec<RunGroup> = Vec::new();

    for run in sorted {
        let title = match group_by {
            GroupBy::None => String::new(),
            GroupBy::Runtime => run
                .runtime_code()
                .map_or_else(|| "Unknown".to_string(), runtime_label),
            GroupBy::Status => status_label(&run.status),
        };

        match groups.iter_mut().find(|group| group.title == title) {
            Some(group) => group.runs.push(run),
            None => groups.push(RunGroup {
                title,
                runs: vec![run],
            }),
        }
    }

    groups
}

// Toggle button of a section header, the whole row folds and unfolds the section
#[component]
pub fn SectionToggle(
    #[props(into)] label: String,
    open: bool,
    on_toggle: EventHandler<()>,
) -> Element {
    let chevron_class = if open { "rotate-90" } else { "" };

    rsx! {
        button {
            r#type: "button",
            class: "flex h-8 w-full cursor-pointer items-center gap-(--space-1) rounded-(--round-md) px-(--space-3) text-left text-(length:--fs-xs) font-(weight:--fw-medium) text-(color:--clr-text-muted) transition-colors hover:bg-(--clr-hover-wash)",
            "aria-expanded": "{open}",
            onclick: move |_| on_toggle.call(()),
            "{label}"
            Icon {
                kind: IconKind::ChevronRight,
                class: "size-3.5 transition-transform duration-300 ease-(--ease-out) {chevron_class}",
            }
        }
    }
}

// One run in a list: status dot, name and runtime tag
#[component]
pub fn RecentRun(run: RunSummary) -> Element {
    let route = use_route::<Route>();
    let active = matches!(&route, Route::RunDetail { run_id } if *run_id == run.run_id);
    let running = ACTIVE_RUN
        .read()
        .as_ref()
        .is_some_and(|active_run| active_run.run_id == run.run_id);

    let dot_class = if running {
        "bg-(--clr-accent) pulse-dot text-(color:--clr-accent)"
    } else {
        match run.status.as_str() {
            "done" => "bg-(--clr-success)",
            "failed" => "bg-(--clr-danger)",
            _ => "bg-(--clr-warning)",
        }
    };
    let state_class = if active {
        "bg-(--clr-accent-soft) text-(color:--clr-accent)"
    } else {
        "text-(color:--clr-text) hover:bg-(--clr-hover-wash)"
    };
    let runtime = run.runtime_code().map(runtime_label);
    let shell = use_shell();

    rsx! {
        Link {
            to: Route::RunDetail {
                run_id: run.run_id.clone(),
            },
            class: "flex h-9 items-center gap-(--space-2) rounded-(--round-md) px-(--space-3) text-(length:--fs-sm) transition-colors {state_class}",
            title: "{run.title()}",
            onclick: move |_| shell.close_peek(),
            span { class: "size-(--dot-size) shrink-0 rounded-(--round-full) {dot_class}" }
            span { class: "min-w-0 flex-1 truncate", "{run.display_name}" }
            if let Some(runtime) = runtime {
                RuntimeTag { runtime }
            }
        }
    }
}

const MENU_ROW: &str = "flex h-9 w-full cursor-pointer items-center gap-(--space-2) rounded-(--round-md) px-(--space-2) text-left text-(length:--fs-sm) text-(color:--clr-text) transition-colors hover:bg-(--clr-hover-wash)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuPage {
    Root,
    Status,
    Group,
    Sort,
}

// A list of options with a check mark on the selected one and a back arrow
#[component]
fn OptionList(
    #[props(into)] title: String,
    options: Vec<String>,
    selected: usize,
    on_pick: EventHandler<usize>,
    on_back: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "p-(--space-2)",
            button {
                r#type: "button",
                class: "{MENU_ROW} font-(weight:--fw-medium)",
                onclick: move |_| on_back.call(()),
                Icon { kind: IconKind::ArrowLeft }
                "{title}"
            }
            for (index , option) in options.into_iter().enumerate() {
                button {
                    key: "{index}",
                    r#type: "button",
                    class: "{MENU_ROW}",
                    onclick: move |_| on_pick.call(index),
                    span { class: "w-4 text-(color:--clr-accent)",
                        if index == selected {
                            Icon { kind: IconKind::Check }
                        }
                    }
                    "{option}"
                }
            }
        }
    }
}

// The Recents section: newest runs with a filter menu (status, group by, sort by)
#[component]
pub fn RecentsSection() -> Element {
    let shell = use_shell();
    let mut open = use_signal(|| true);
    let mut menu_open = use_signal(|| false);
    let mut page = use_signal(|| MenuPage::Root);
    let mut status = use_signal(|| StatusFilter::All);
    let mut group_by = use_signal(|| GroupBy::None);
    let mut sort_by = use_signal(|| SortBy::LastActivity);
    let mut folded = use_signal(HashSet::<String>::new);

    // The floating sidebar stays open while the filter menu is open
    use_effect(move || shell.set_hold_open(menu_open()));

    let mut runs = use_resource(move || {
        let filters = HistoryFilters {
            status: status().api_value().to_string(),
            ..HistoryFilters::default()
        };
        let _ = shell.recents_version();

        async move { fetch_page(&filters, None, RECENT_LIMIT).await }
    });

    let filtered = status() != StatusFilter::All
        || group_by() != GroupBy::None
        || sort_by() != SortBy::LastActivity;

    let button_opacity = if filtered || menu_open() {
        "opacity-100"
    } else {
        "opacity-0"
    };

    let menu: Element = match page() {
        MenuPage::Root => rsx! {
            div { class: "p-(--space-2)",
                button {
                    r#type: "button",
                    class: "{MENU_ROW} justify-between",
                    onclick: move |_| page.set(MenuPage::Status),
                    "Status"
                    span { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)",
                        "{status().label()}"
                    }
                }
                button {
                    r#type: "button",
                    class: "{MENU_ROW} justify-between",
                    onclick: move |_| page.set(MenuPage::Group),
                    "Group by"
                    span { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)",
                        "{group_by().label()}"
                    }
                }
                button {
                    r#type: "button",
                    class: "{MENU_ROW} justify-between",
                    onclick: move |_| page.set(MenuPage::Sort),
                    "Sort by"
                    span { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)",
                        "{sort_by().label()}"
                    }
                }
            }
        },
        MenuPage::Status => rsx! {
            OptionList {
                title: "Status",
                options: StatusFilter::OPTIONS.iter().map(|option| option.label().to_string()).collect::<Vec<_>>(),
                selected: StatusFilter::OPTIONS.iter().position(|option| *option == status()).unwrap_or(0),
                on_pick: move |index: usize| {
                    status.set(StatusFilter::OPTIONS[index]);
                    page.set(MenuPage::Root);
                },
                on_back: move |_| page.set(MenuPage::Root),
            }
        },
        MenuPage::Group => rsx! {
            OptionList {
                title: "Group by",
                options: GroupBy::OPTIONS.iter().map(|option| option.label().to_string()).collect::<Vec<_>>(),
                selected: GroupBy::OPTIONS.iter().position(|option| *option == group_by()).unwrap_or(0),
                on_pick: move |index: usize| {
                    group_by.set(GroupBy::OPTIONS[index]);
                    page.set(MenuPage::Root);
                },
                on_back: move |_| page.set(MenuPage::Root),
            }
        },
        MenuPage::Sort => rsx! {
            OptionList {
                title: "Sort by",
                options: SortBy::OPTIONS.iter().map(|option| option.label().to_string()).collect::<Vec<_>>(),
                selected: SortBy::OPTIONS.iter().position(|option| *option == sort_by()).unwrap_or(0),
                on_pick: move |index: usize| {
                    sort_by.set(SortBy::OPTIONS[index]);
                    page.set(MenuPage::Root);
                },
                on_back: move |_| page.set(MenuPage::Root),
            }
        },
    };

    let body: Element = match runs() {
        None => rsx! {
            div { class: "flex flex-col gap-(--space-1) px-(--space-3)",
                for index in 0..4 {
                    Skeleton { key: "{index}", class: "h-9 w-full" }
                }
            }
        },
        Some(Err(_)) => rsx! {
            div { class: "flex items-center justify-between gap-(--space-2) px-(--space-3) py-(--space-2) text-(length:--fs-xs) text-(color:--clr-danger-text)",
                "Couldn't load runs"
                button {
                    r#type: "button",
                    class: "cursor-pointer font-(weight:--fw-medium) underline underline-offset-2",
                    onclick: move |_| runs.restart(),
                    "Try again"
                }
            }
        },
        Some(Ok(page_data)) if page_data.items.is_empty() => rsx! {
            p { class: "px-(--space-3) py-(--space-2) text-(length:--fs-xs) text-(color:--clr-text-faint)",
                "No runs yet"
            }
        },
        Some(Ok(page_data)) => {
            let groups = arrange(&page_data.items, group_by(), sort_by());
            let folded_titles = folded();

            rsx! {
                for group in groups {
                    div { key: "{group.title}", class: "flex flex-col",
                        if !group.title.is_empty() {
                            {
                                let title = group.title.clone();
                                let group_open = !folded_titles.contains(&title);
                                let count = group.runs.len();
                                rsx! {
                                    SectionToggle {
                                        label: format!("{title} · {count}"),
                                        open: group_open,
                                        on_toggle: move |_| {
                                            folded
                                                .with_mut(|set| {
                                                    if !set.remove(&title) {
                                                        set.insert(title.clone());
                                                    }
                                                });
                                        },
                                    }
                                }
                            }
                        }
                        Collapse { open: group.title.is_empty() || !folded_titles.contains(&group.title),
                            for run in group.runs {
                                RecentRun { key: "{run.run_id}", run }
                            }
                        }
                    }
                }
            }
        }
    };

    rsx! {
        div { class: "flex min-h-0 flex-1 flex-col",
            div { class: "group/hdr relative",
                SectionToggle {
                    label: "Recents",
                    open: open(),
                    on_toggle: move |_| open.set(!open()),
                }
                // No transform here: a transformed parent would shrink the click-away layer of the menu
                div { class: "absolute inset-y-0 right-(--space-2) flex items-center",
                    button {
                        r#type: "button",
                        class: "relative inline-flex size-6 cursor-pointer items-center justify-center rounded-(--round-sm) text-(color:--clr-text-muted) transition-[opacity,background-color] hover:bg-(--clr-hover-wash) hover:text-(color:--clr-text) group-hover/hdr:opacity-100 focus-visible:opacity-100 {button_opacity}",
                        "aria-label": "Filter runs",
                        "aria-expanded": "{menu_open()}",
                        onclick: move |_| {
                            page.set(MenuPage::Root);
                            menu_open.set(!menu_open());
                        },
                        Icon { kind: IconKind::Filter, class: "size-3.5" }
                        if filtered {
                            span { class: "absolute top-0.5 right-0.5 size-1.5 rounded-(--round-full) bg-(--clr-accent)" }
                        }
                    }
                    if menu_open() {
                        Popover {
                            align: PopoverAlign::Left,
                            rise: true,
                            width: "w-56",
                            on_close: move |_| menu_open.set(false),
                            {menu}
                        }
                    }
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto",
                Collapse { open: open(), {body} }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(
        id: &str,
        name: &str,
        runtime: Option<&str>,
        status: &str,
        rps: Option<f64>,
    ) -> RunSummary {
        let runtime = runtime.map_or("null".to_string(), |code| format!("\"{code}\""));
        let rps = rps.map_or("null".to_string(), |value| value.to_string());
        let body = format!(
            r#"{{
                "run_id": "{id}", "status": "{status}", "display_name": "{name}",
                "target": {{ "path": "/p/{name}" }},
                "discovery": {{ "runtime": {runtime} }},
                "created_at": "2026-10-08T05:11:22.123Z",
                "summary": {{ "requests_per_second": {rps} }}
            }}"#
        );

        serde_json::from_str(&body).expect("parses")
    }

    fn sample() -> Vec<RunSummary> {
        vec![
            run("1", "shop", Some("rust"), "done", Some(100.0)),
            run("2", "api", Some("bun"), "failed", None),
            run("3", "shop", Some("rust"), "done", Some(300.0)),
            run("4", "blog", None, "cancelled", Some(200.0)),
        ]
    }

    fn ids(group: &RunGroup) -> Vec<&str> {
        group.runs.iter().map(|run| run.run_id.as_str()).collect()
    }

    #[test]
    fn keeps_api_order_without_sort_or_group() {
        let groups = arrange(&sample(), GroupBy::None, SortBy::LastActivity);

        assert_eq!(groups.len(), 1);
        assert_eq!(ids(&groups[0]), ["1", "2", "3", "4"]);
    }

    #[test]
    fn sorts_by_throughput_with_missing_values_last() {
        let groups = arrange(&sample(), GroupBy::None, SortBy::Throughput);

        assert_eq!(ids(&groups[0]), ["3", "4", "1", "2"]);
    }

    #[test]
    fn sorts_by_name_ignoring_case() {
        let groups = arrange(&sample(), GroupBy::None, SortBy::Name);

        assert_eq!(ids(&groups[0]), ["2", "4", "1", "3"]);
    }

    #[test]
    fn groups_by_runtime_in_first_seen_order() {
        let groups = arrange(&sample(), GroupBy::Runtime, SortBy::LastActivity);
        let titles: Vec<&str> = groups.iter().map(|group| group.title.as_str()).collect();

        assert_eq!(titles, ["Rust", "Bun", "Unknown"]);
        assert_eq!(ids(&groups[0]), ["1", "3"]);
    }

    #[test]
    fn groups_by_status() {
        let groups = arrange(&sample(), GroupBy::Status, SortBy::LastActivity);
        let titles: Vec<&str> = groups.iter().map(|group| group.title.as_str()).collect();

        assert_eq!(titles, ["Done", "Failed", "Cancelled"]);
    }
}
