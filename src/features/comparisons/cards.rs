use dioxus::prelude::*;

use crate::components::{Icon, IconKind, Popover};

// Whether the cards of the page show their chart or their table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Charts,
    Tables,
}

impl ViewMode {
    fn flipped(self) -> Self {
        match self {
            Self::Charts => Self::Tables,
            Self::Tables => Self::Charts,
        }
    }
}

// What a card shows: its own choice while the page switch is where it was when the choice was
// made, otherwise the page switch. Flipping the page switch therefore flips every card.
fn effective_mode(own: Option<(ViewMode, ViewMode)>, page: ViewMode) -> ViewMode {
    match own {
        Some((mode, seen_page)) if seen_page == page => mode,
        _ => page,
    }
}

// Shared by every card of the page, provided once by the results
pub fn use_view_mode_provider() -> Signal<ViewMode> {
    use_context_provider(|| Signal::new(ViewMode::Charts))
}

const THUMB_REM: f64 = 5.5;

// Charts | Tables switch with a sliding thumb
#[component]
pub fn ViewSwitch() -> Element {
    let mut mode = use_context::<Signal<ViewMode>>();
    let offset = if mode() == ViewMode::Tables {
        THUMB_REM
    } else {
        0.0
    };
    let options: Vec<(ViewMode, &'static str, bool, &'static str)> =
        [(ViewMode::Charts, "Charts"), (ViewMode::Tables, "Tables")]
            .into_iter()
            .map(|(option, label)| {
                let active = mode() == option;
                let class = if active {
                    "font-(weight:--fw-medium) text-(color:--clr-accent)"
                } else {
                    "text-(color:--clr-text-muted) hover:text-(color:--clr-text)"
                };

                (option, label, active, class)
            })
            .collect();

    rsx! {
        div {
            class: "relative inline-flex shrink-0 rounded-(--round-full) border border-(color:--clr-border) bg-(--clr-surface) p-1",
            role: "group",
            "aria-label": "Show charts or tables",
            span {
                class: "pointer-events-none absolute top-1 bottom-1 left-1 rounded-(--round-full) bg-(--clr-accent-soft) transition-transform duration-350 ease-(--ease-out)",
                style: "width: {THUMB_REM}rem; transform: translateX({offset}rem)",
            }
            for (option , label , active , class) in options {
                button {
                    key: "{label}",
                    r#type: "button",
                    class: "relative z-10 cursor-pointer rounded-(--round-full) py-(--space-1) text-center text-(length:--fs-sm) transition-colors {class}",
                    style: "width: {THUMB_REM}rem",
                    "aria-pressed": "{active}",
                    onclick: move |_| mode.set(option),
                    "{label}"
                }
            }
        }
    }
}

// One topic of the page. A card with both a chart and a table has a three dots menu that flips
// it on its own, a card with only one of them shows that one.
#[component]
pub fn DataCard(
    #[props(into)] title: String,
    #[props(into, default)] subtitle: String,
    #[props(default)] chart: Option<Element>,
    #[props(default)] table: Option<Element>,
) -> Element {
    let page = use_context::<Signal<ViewMode>>();
    let mut own = use_signal(|| None::<(ViewMode, ViewMode)>);
    let mut open = use_signal(|| false);

    let both = chart.is_some() && table.is_some();
    let mode = if both {
        effective_mode(own(), page())
    } else if chart.is_some() {
        ViewMode::Charts
    } else {
        ViewMode::Tables
    };
    let flip_label = match mode {
        ViewMode::Charts => "Show as table",
        ViewMode::Tables => "Show as chart",
    };
    let content = match mode {
        ViewMode::Charts => chart,
        ViewMode::Tables => table,
    };

    rsx! {
        section { class: "fade-up min-w-0 rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) shadow-(--shadow-card)",
            header { class: "flex items-start justify-between gap-(--space-3) px-(--space-4) pt-(--space-3)",
                div { class: "min-w-0",
                    h3 { class: "text-(length:--fs-base) font-(weight:--fw-semibold) text-(color:--clr-text)",
                        "{title}"
                    }
                    if !subtitle.is_empty() {
                        p { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)", "{subtitle}" }
                    }
                }
                if both {
                    div { class: "relative shrink-0",
                        button {
                            r#type: "button",
                            class: "inline-flex size-8 cursor-pointer items-center justify-center rounded-(--round-md) text-(color:--clr-text-muted) transition-colors hover:bg-(--clr-surface-hover) hover:text-(color:--clr-text)",
                            "aria-label": "Options of {title}",
                            "aria-haspopup": "menu",
                            "aria-expanded": "{open()}",
                            onclick: move |_| open.set(!open()),
                            Icon { kind: IconKind::Dots, class: "size-(--icon-md)" }
                        }
                        if open() {
                            Popover {
                                rise: true,
                                width: "w-44",
                                on_close: move |_| open.set(false),
                                div { role: "menu", class: "flex flex-col p-(--space-1)",
                                    button {
                                        r#type: "button",
                                        role: "menuitem",
                                        class: "cursor-pointer rounded-(--round-md) px-(--space-3) py-(--space-2) text-left text-(length:--fs-sm) text-(color:--clr-text) transition-colors hover:bg-(--clr-surface-hover)",
                                        onclick: move |_| {
                                            own.set(Some((mode.flipped(), page())));
                                            open.set(false);
                                        },
                                        "{flip_label}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "px-(--space-4) pt-(--space-3) pb-(--space-4)",
                if let Some(content) = content {
                    {content}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_the_page_without_an_own_choice() {
        assert_eq!(effective_mode(None, ViewMode::Charts), ViewMode::Charts);
        assert_eq!(effective_mode(None, ViewMode::Tables), ViewMode::Tables);
    }

    #[test]
    fn keeps_an_own_choice_while_the_page_stays() {
        let own = Some((ViewMode::Tables, ViewMode::Charts));

        assert_eq!(effective_mode(own, ViewMode::Charts), ViewMode::Tables);
    }

    #[test]
    fn flipping_the_page_switch_flips_the_card_again() {
        let own = Some((ViewMode::Tables, ViewMode::Charts));

        assert_eq!(effective_mode(own, ViewMode::Tables), ViewMode::Tables);
        assert_eq!(ViewMode::Charts.flipped(), ViewMode::Tables);
    }
}
