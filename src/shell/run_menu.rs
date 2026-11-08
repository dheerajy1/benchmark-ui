use dioxus::prelude::*;

use super::state::use_shell;
use crate::{
    app::Route,
    components::{button_class, copy_text, use_toasts, ButtonVariant, Icon, IconKind, Modal},
    features::history::delete_run,
};

const MENU_WIDTH: f64 = 192.0;
const MENU_HEIGHT: f64 = 196.0;
// Gap kept between the menu and the edge of the window
const EDGE: f64 = 8.0;

const ITEM: &str = "flex h-9 w-full cursor-pointer items-center gap-(--space-2) rounded-(--round-md) px-(--space-2) text-left text-(length:--fs-sm) text-(color:--clr-text) transition-colors hover:bg-(--clr-hover-wash)";
const DANGER_ITEM: &str = "flex h-9 w-full cursor-pointer items-center gap-(--space-2) rounded-(--round-md) px-(--space-2) text-left text-(length:--fs-sm) text-(color:--clr-danger-text) transition-colors hover:bg-(--clr-hover-wash) disabled:cursor-not-allowed disabled:opacity-50";
const DANGER_BUTTON: &str = "inline-flex h-(--control-height) cursor-pointer items-center justify-center gap-(--space-2) rounded-(--round-md) bg-(--clr-danger) px-(--space-4) text-(length:--fs-sm) font-(weight:--fw-medium) whitespace-nowrap text-white transition-[opacity,transform] duration-200 hover:opacity-90 active:scale-[0.97] disabled:cursor-not-allowed disabled:opacity-50 disabled:active:scale-100 focus-visible:outline-none focus-visible:shadow-(--halo-focus)";

// Top left corner of the menu: at the click, moved back inside the window when it would stick out
fn menu_corner(click_x: f64, click_y: f64, view_width: f64, view_height: f64) -> (f64, f64) {
    let left = click_x.min(view_width - MENU_WIDTH - EDGE).max(EDGE);
    let top = click_y.min(view_height - MENU_HEIGHT - EDGE).max(EDGE);

    (left, top)
}

// Size of the window, a wide one when the browser does not tell
fn window_size() -> (f64, f64) {
    let Some(window) = web_sys::window() else {
        return (1280.0, 800.0);
    };
    let width = window
        .inner_width()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(1280.0);
    let height = window
        .inner_height()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(800.0);

    (width, height)
}

// The three dots of a run in the sidebar, its menu and the confirmation before a delete
#[component]
pub fn RunMenu(run_id: String, name: String, open_here: bool, running: bool) -> Element {
    let shell = use_shell();
    let nav = use_navigator();
    let toasts = use_toasts();
    let mut corner = use_signal(|| None::<(f64, f64)>);
    let mut confirm = use_signal(|| false);
    let mut deleting = use_signal(|| false);

    // Every link of the menu closes it and the floating sidebar
    let leave = move |_: MouseEvent| {
        corner.set(None);
        shell.set_hold_open(false);
        shell.close_peek();
    };

    let copy_id = run_id.clone();
    let delete_id = run_id.clone();

    let on_delete = move |_: MouseEvent| {
        let run_id = delete_id.clone();
        deleting.set(true);

        spawn(async move {
            let result = delete_run(&run_id).await;
            deleting.set(false);

            match result {
                Ok(()) => {
                    confirm.set(false);
                    shell.set_hold_open(false);
                    toasts.show("Run deleted");
                    shell.bump_recents();

                    if open_here {
                        let _ = nav.push(Route::runs());
                    }
                }
                Err(_) => toasts.show_error("Could not delete the run", None),
            }
        });
    };

    let button_opacity = if corner().is_some() {
        "opacity-100"
    } else {
        "opacity-0"
    };

    let menu: Element = match corner() {
        None => rsx! {},
        Some((left, top)) => rsx! {
            div {
                class: "fixed inset-0 z-30",
                onclick: move |_| {
                    corner.set(None);
                    shell.set_hold_open(false);
                },
            }
            div {
                class: "menu-in fixed z-40 w-48 rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) p-(--space-2) shadow-(--shadow-popover) focus:outline-none",
                style: "left: {left}px; top: {top}px",
                tabindex: "-1",
                onmounted: move |event| async move {
                    let _ = event.set_focus(true).await;
                },
                onkeydown: move |event| {
                    if event.key() == Key::Escape {
                        corner.set(None);
                        shell.set_hold_open(false);
                    }
                },
                Link {
                    to: Route::RunDetail {
                        run_id: run_id.clone(),
                    },
                    class: "{ITEM}",
                    onclick: leave,
                    Icon { kind: IconKind::ChevronRight }
                    "Open"
                }
                Link {
                    to: Route::SubmissionFrom {
                        run_id: run_id.clone(),
                    },
                    class: "{ITEM}",
                    onclick: leave,
                    Icon { kind: IconKind::Play }
                    "Run again"
                }
                Link {
                    to: Route::Comparisons {
                        baseline: run_id.clone(),
                        comparison: String::new(),
                    },
                    class: "{ITEM}",
                    onclick: leave,
                    Icon { kind: IconKind::Compare }
                    "Compare"
                }
                button {
                    r#type: "button",
                    class: "{ITEM}",
                    onclick: move |_| {
                        copy_text(&copy_id);
                        toasts.show("Run id copied");
                        corner.set(None);
                        shell.set_hold_open(false);
                    },
                    Icon { kind: IconKind::Copy }
                    "Copy run id"
                }
                div { class: "my-(--space-1) h-px bg-(--clr-border)" }
                button {
                    r#type: "button",
                    class: "{DANGER_ITEM}",
                    disabled: running,
                    title: if running { "A running run cannot be deleted" } else { "" },
                    onclick: move |_| {
                        corner.set(None);
                        confirm.set(true);
                    },
                    Icon { kind: IconKind::Close }
                    "Delete run"
                }
            }
        },
    };

    rsx! {
        div { class: "absolute inset-y-0 right-(--space-1) flex items-center",
            button {
                r#type: "button",
                class: "inline-flex size-7 cursor-pointer items-center justify-center rounded-(--round-sm) text-(color:--clr-text-muted) transition-[opacity,background-color] hover:bg-(--clr-surface-hover) hover:text-(color:--clr-text) group-hover/run:opacity-100 focus-visible:opacity-100 {button_opacity}",
                "aria-label": "Actions for {name}",
                "aria-haspopup": "menu",
                onclick: move |event| {
                    let point = event.client_coordinates();
                    let (width, height) = window_size();

                    corner.set(Some(menu_corner(point.x, point.y, width, height)));
                    shell.set_hold_open(true);
                },
                Icon { kind: IconKind::Dots, class: "size-(--icon-md)" }
            }
            {menu}
        }
        if confirm() {
            Modal {
                title: "Delete run",
                width: "max-w-md",
                on_close: move |_| {
                    if !deleting() {
                        confirm.set(false);
                        shell.set_hold_open(false);
                    }
                },
                div { class: "flex flex-col gap-(--space-4)",
                    p { class: "text-(length:--fs-sm) text-(color:--clr-text)",
                        "Delete "
                        strong { class: "font-(weight:--fw-semibold)", "{name}" }
                        "? Its results and charts are removed for good."
                    }
                    div { class: "flex justify-end gap-(--space-2)",
                        button {
                            r#type: "button",
                            class: button_class(ButtonVariant::Secondary),
                            disabled: deleting(),
                            onclick: move |_| {
                                confirm.set(false);
                                shell.set_hold_open(false);
                            },
                            "Cancel"
                        }
                        button {
                            r#type: "button",
                            class: "{DANGER_BUTTON}",
                            disabled: deleting(),
                            onclick: on_delete,
                            if deleting() {
                                "Deleting…"
                            } else {
                                "Delete run"
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
    fn the_menu_opens_at_the_click_when_it_fits() {
        assert_eq!(menu_corner(100.0, 200.0, 1280.0, 800.0), (100.0, 200.0));
    }

    #[test]
    fn the_menu_moves_back_inside_the_window() {
        let (left, top) = menu_corner(1270.0, 790.0, 1280.0, 800.0);

        assert_eq!(left, 1280.0 - MENU_WIDTH - EDGE);
        assert_eq!(top, 800.0 - MENU_HEIGHT - EDGE);
    }

    #[test]
    fn the_menu_never_leaves_through_the_top_or_left() {
        assert_eq!(menu_corner(-5.0, -5.0, 1280.0, 800.0), (EDGE, EDGE));
    }
}
