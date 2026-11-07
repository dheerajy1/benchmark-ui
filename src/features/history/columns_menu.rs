use dioxus::prelude::*;

use super::columns::{Column, ColumnPrefs};
use crate::components::{Icon, IconKind, Popover, PopoverAlign, Switch};

// A column, whether it is shown, and whether it cannot be hidden
type MenuItem = (Column, bool, bool);

// A group name with its columns in table order
type MenuGroup = (&'static str, Vec<MenuItem>);

// `Columns` button with a count chip and the menu that switches and reorders the columns
#[component]
pub fn ColumnsMenu(prefs: Signal<ColumnPrefs>) -> Element {
    let mut prefs = prefs;
    let mut open = use_signal(|| false);
    let mut dragging = use_signal(|| None::<Column>);

    let shown_count = prefs.read().shown_count();
    let total = Column::ALL.len();
    let ordered = prefs.read().ordered();

    let groups: Vec<MenuGroup> = Column::GROUPS
        .iter()
        .map(|(name, members)| {
            let items = ordered
                .iter()
                .filter(|column| members.contains(column))
                .map(|column| {
                    let shown = prefs.read().is_shown(*column);

                    (*column, shown, shown && shown_count == 1)
                })
                .collect();

            (*name, items)
        })
        .collect();

    let button_class = if open() {
        "border-(color:--clr-accent) bg-(--clr-accent-soft) text-(color:--clr-accent) shadow-(--halo-focus)"
    } else {
        "border-(color:--clr-border) bg-(--clr-surface) text-(color:--clr-text) hover:bg-(--clr-surface-hover)"
    };

    rsx! {
        div { class: "relative",
            button {
                r#type: "button",
                class: "group/columns inline-flex h-9 cursor-pointer items-center gap-(--space-2) rounded-(--round-md) border px-(--space-3) text-(length:--fs-sm) font-(weight:--fw-medium) transition-colors {button_class}",
                "aria-haspopup": "menu",
                "aria-expanded": "{open()}",
                onclick: move |_| open.set(!open()),
                Icon {
                    kind: IconKind::Columns,
                    class: "size-4 transition-transform duration-300 ease-(--ease-out) group-hover/columns:rotate-90",
                }
                "Columns"
                span { class: "rounded-(--round-full) bg-(--clr-surface-hover) px-(--space-2) text-(length:--fs-xs) tabular-nums text-(color:--clr-text-muted)",
                    "{shown_count}"
                }
            }
            if open() {
                Popover {
                    align: PopoverAlign::Right,
                    width: "w-[290px]",
                    on_close: move |_| open.set(false),
                    div { class: "flex items-center justify-between px-(--space-4) pt-(--space-3) pb-(--space-2)",
                        div {
                            p { class: "text-(length:--fs-sm) font-(weight:--fw-semibold) text-(color:--clr-text)",
                                "Columns"
                            }
                            p { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)",
                                "{shown_count} of {total} shown"
                            }
                        }
                        button {
                            r#type: "button",
                            class: "cursor-pointer rounded-(--round-md) px-(--space-2) py-(--space-1) text-(length:--fs-xs) font-(weight:--fw-medium) text-(color:--clr-accent) transition-colors hover:bg-(--clr-accent-soft)",
                            onclick: move |_| prefs.set(ColumnPrefs::standard()),
                            "Reset"
                        }
                    }
                    div { class: "flex max-h-96 flex-col gap-(--space-2) overflow-y-auto px-(--space-2) pb-(--space-2)",
                        for (name , items) in groups {
                            div { key: "{name}",
                                p { class: "px-(--space-2) pt-(--space-2) pb-(--space-1) text-(length:--fs-2xs) font-(weight:--fw-medium) tracking-wider text-(color:--clr-text-faint) uppercase",
                                    "{name}"
                                }
                                for (column , shown , locked) in items {
                                    div {
                                        key: "{column.label()}",
                                        class: "flex h-9 items-center gap-(--space-2) rounded-(--round-md) px-(--space-2) transition-colors hover:bg-(--clr-hover-wash)",
                                        draggable: "true",
                                        ondragstart: move |_| dragging.set(Some(column)),
                                        ondragover: move |event: DragEvent| event.prevent_default(),
                                        ondrop: move |event: DragEvent| {
                                            event.prevent_default();
                                            if let Some(dragged) = dragging() {
                                                prefs.write().move_to(dragged, column);
                                            }
                                            dragging.set(None);
                                        },
                                        ondragend: move |_| dragging.set(None),
                                        Icon {
                                            kind: IconKind::Grip,
                                            class: "size-4 cursor-grab text-(color:--clr-text-faint)",
                                        }
                                        button {
                                            r#type: "button",
                                            class: "min-w-0 flex-1 cursor-pointer truncate text-left text-(length:--fs-sm) text-(color:--clr-text)",
                                            disabled: locked,
                                            onclick: move |_| prefs.write().toggle(column),
                                            "{column.label()}"
                                        }
                                        Switch {
                                            on: shown,
                                            locked,
                                            label: format!("Show {}", column.label()),
                                            on_toggle: move |_| prefs.write().toggle(column),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
