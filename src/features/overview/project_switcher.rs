use dioxus::prelude::*;

use super::models::{Project, ProjectEntry};
use crate::{
    app::Route,
    components::{Icon, IconKind, Popover, PopoverAlign, RuntimeTag},
    features::history::runtime_label,
    format::relative_time,
};

// `path · last run time`, the second line of a menu row
fn subtitle(path: &str, last_run_at: &str, now_ms: i64) -> String {
    format!("{path} · {}", relative_time(last_run_at, now_ms))
}

fn run_count(total: u64) -> String {
    if total == 1 {
        "1 run".to_string()
    } else {
        format!("{total} runs")
    }
}

// Chooses the project the Overview shows. The choice lives in the address (`?project=`).
#[component]
pub fn ProjectSwitcher(selected: Option<Project>, projects: Vec<ProjectEntry>) -> Element {
    let mut open = use_signal(|| false);
    let now_ms = js_sys::Date::now() as i64;
    let selected_path = selected
        .as_ref()
        .map(|project| project.path.clone())
        .unwrap_or_default();

    let button_class = if open() {
        "border-(color:--clr-accent) shadow-(--halo-focus)"
    } else {
        "border-(color:--clr-border) hover:bg-(--clr-surface-hover)"
    };
    let chevron_class = if open() { "-rotate-90" } else { "rotate-90" };

    let label = selected.as_ref().map_or_else(
        || "Choose a project".to_string(),
        |project| project.name.clone(),
    );
    let runtime = selected
        .as_ref()
        .and_then(|project| project.runtime.as_deref())
        .map(runtime_label);

    rsx! {
        div { class: "relative",
            button {
                r#type: "button",
                class: "inline-flex h-10 min-w-60 cursor-pointer items-center gap-(--space-2) rounded-(--round-md) border bg-(--clr-surface) px-(--space-3) text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text) transition-colors {button_class}",
                "aria-haspopup": "listbox",
                "aria-expanded": "{open()}",
                onclick: move |_| open.set(!open()),
                Icon { kind: IconKind::Folder, class: "size-(--icon-sm) text-(color:--clr-text-muted)" }
                span { class: "min-w-0 flex-1 truncate text-left", "{label}" }
                if let Some(runtime) = runtime {
                    RuntimeTag { runtime }
                }
                Icon {
                    kind: IconKind::ChevronRight,
                    class: "size-(--icon-sm) text-(color:--clr-text-faint) transition-transform duration-200 {chevron_class}",
                }
            }
            if open() {
                Popover {
                    align: PopoverAlign::Right,
                    width: "w-[340px]",
                    on_close: move |_| open.set(false),
                    p { class: "px-(--space-4) pt-(--space-3) pb-(--space-1) text-(length:--fs-xs) font-(weight:--fw-medium) text-(color:--clr-text-muted)",
                        "Projects"
                    }
                    div { class: "flex max-h-80 flex-col gap-px overflow-y-auto p-(--space-2)", role: "listbox",
                        for entry in projects {
                            ProjectRow {
                                key: "{entry.path}",
                                selected: entry.path == selected_path,
                                now_ms,
                                entry,
                                on_pick: move |_| open.set(false),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ProjectRow(
    entry: ProjectEntry,
    selected: bool,
    now_ms: i64,
    on_pick: EventHandler<()>,
) -> Element {
    let row_class = if selected {
        "bg-(--clr-accent-soft)"
    } else {
        "hover:bg-(--clr-hover-wash)"
    };
    let second_line = subtitle(&entry.path, &entry.last_run_at, now_ms);
    let count = run_count(entry.total_runs);
    let runtime = entry.runtime.as_deref().map(runtime_label);

    rsx! {
        Link {
            to: Route::Overview {
                project: entry.path.clone(),
            },
            class: "flex items-center gap-(--space-3) rounded-(--round-md) px-(--space-3) py-(--space-2) transition-colors {row_class}",
            onclick: move |_| on_pick.call(()),
            div { class: "min-w-0 flex-1",
                p { class: "truncate text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text)",
                    "{entry.name}"
                }
                p { class: "truncate text-(length:--fs-xs) text-(color:--clr-text-muted)", title: "{second_line}",
                    "{second_line}"
                }
            }
            div { class: "flex shrink-0 flex-col items-end gap-px",
                if let Some(runtime) = runtime {
                    RuntimeTag { runtime }
                }
                span { class: "text-(length:--fs-xs) text-(color:--clr-text-faint)", "{count}" }
            }
            if selected {
                Icon { kind: IconKind::Check, class: "size-(--icon-sm) text-(color:--clr-accent)" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_subtitle_has_the_path_and_the_age() {
        let now = crate::format::parse_epoch_ms("2026-10-08T07:11:00Z").expect("parses");

        assert_eq!(
            subtitle("/srv/shop", "2026-10-08T05:11:00Z", now),
            "/srv/shop · 2 h ago"
        );
    }

    #[test]
    fn counts_runs_in_the_singular_and_plural() {
        assert_eq!(run_count(1), "1 run");
        assert_eq!(run_count(9), "9 runs");
    }
}
