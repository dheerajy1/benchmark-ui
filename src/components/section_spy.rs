use std::rc::Rc;

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

// Every pill has the same width, so the highlight can slide by whole pill widths
const PILL_WIDTH_REM: f64 = 7.5;
// How often the page checks which section is in view
const WATCH_MS: u32 = 150;
// A section counts as the current one once its top is above this many pixels from the viewport top
const ACTIVE_LINE: f64 = 170.0;
// The pill row shows its bottom border once the page has scrolled past this many pixels
const STUCK_LINE: f64 = 120.0;

type SectionRefs = Signal<Vec<Option<Rc<MountedData>>>>;

// Index of the last section whose top has passed the active line, the first one when none has
fn active_section(tops: &[Option<f64>]) -> usize {
    tops.iter()
        .enumerate()
        .filter(|(_, top)| top.is_some_and(|top| top <= ACTIVE_LINE))
        .map(|(index, _)| index)
        .next_back()
        .unwrap_or(0)
}

// Reports the section tops so the pills can follow the scroll position
async fn section_tops(refs: &[Option<Rc<MountedData>>]) -> Vec<Option<f64>> {
    let mut tops = Vec::with_capacity(refs.len());

    for section in refs {
        let top = match section {
            Some(data) => data.get_client_rect().await.ok().map(|rect| rect.origin.y),
            None => None,
        };
        tops.push(top);
    }

    tops
}

// Which section of a long page is in view, shared by the pill row and the section blocks
#[derive(Clone, Copy, PartialEq)]
pub struct SectionSpy {
    refs: SectionRefs,
    active: Signal<usize>,
    stuck: Signal<bool>,
}

impl SectionSpy {
    pub fn active(self) -> usize {
        (self.active)()
    }

    pub fn stuck(self) -> bool {
        (self.stuck)()
    }

    // Highlights the section at once and glides the page to it
    pub fn pick(self, index: usize) {
        let mut active = self.active;
        active.set(index);

        let section = self.refs.peek().get(index).cloned().flatten();
        if let Some(section) = section {
            spawn(async move {
                let _ = section.scroll_to(ScrollBehavior::Smooth).await;
            });
        }
    }
}

// Starts watching the section blocks of the page, polling their positions every 150 ms
pub fn use_section_spy() -> SectionSpy {
    let refs: SectionRefs = use_signal(Vec::new);
    let mut active = use_signal(|| 0_usize);
    let mut stuck = use_signal(|| false);

    use_future(move || async move {
        loop {
            TimeoutFuture::new(WATCH_MS).await;

            let sections = refs.peek().clone();
            let tops = section_tops(&sections).await;

            let current = active_section(&tops);
            if *active.peek() != current {
                active.set(current);
            }

            let past = tops
                .first()
                .copied()
                .flatten()
                .is_some_and(|top| top < STUCK_LINE);
            if *stuck.peek() != past {
                stuck.set(past);
            }
        }
    });

    SectionSpy {
        refs,
        active,
        stuck,
    }
}

// One section of the page that the pill row scrolls to
#[component]
pub fn SectionBlock(
    index: usize,
    spy: SectionSpy,
    #[props(into)] title: String,
    children: Element,
) -> Element {
    let mut refs = spy.refs;

    rsx! {
        section {
            class: "fade-up scroll-mt-36 flex flex-col gap-(--space-3)",
            "aria-label": "{title}",
            onmounted: move |event| {
                let mut list = refs.write();
                if list.len() <= index {
                    list.resize(index + 1, None);
                }
                list[index] = Some(event.data());
            },
            {children}
        }
    }
}

// Sticky row of pills that jump to the sections of the page. The soft highlight slides to the
// active pill, a border fades in once the row is stuck under the top bar. `trailing` sits at the
// right end of the row.
#[component]
pub fn SectionBar(
    titles: Vec<&'static str>,
    spy: SectionSpy,
    #[props(default)] trailing: Option<Element>,
) -> Element {
    let active = spy.active();
    let offset = active as f64 * PILL_WIDTH_REM;
    let border = if spy.stuck() {
        "border-(color:--clr-border)"
    } else {
        "border-transparent"
    };

    let pills: Vec<(usize, &'static str, &'static str, bool)> = titles
        .into_iter()
        .enumerate()
        .map(|(index, title)| {
            let pill_class = if index == active {
                "font-(weight:--fw-medium) text-(color:--clr-accent)"
            } else {
                "text-(color:--clr-text-muted) hover:text-(color:--clr-text)"
            };

            (index, title, pill_class, index == active)
        })
        .collect();

    rsx! {
        nav {
            class: "sticky top-(--topbar-height) z-10 -mx-(--space-1) flex items-center justify-between gap-(--space-3) border-b bg-(--clr-canvas) px-(--space-1) py-(--space-2) transition-colors duration-300 {border}",
            "aria-label": "Sections of this page",
            div { class: "relative inline-flex max-w-full overflow-x-auto rounded-(--round-full) border border-(color:--clr-border) bg-(--clr-surface) p-1",
                span {
                    class: "pointer-events-none absolute top-1 bottom-1 left-1 rounded-(--round-full) bg-(--clr-accent-soft) transition-transform duration-350 ease-(--ease-out)",
                    style: "width: {PILL_WIDTH_REM}rem; transform: translateX({offset}rem)",
                }
                for (index , title , pill_class , current) in pills {
                    button {
                        key: "{title}",
                        r#type: "button",
                        class: "relative z-10 shrink-0 cursor-pointer rounded-(--round-full) py-(--space-1) text-center text-(length:--fs-sm) transition-colors {pill_class}",
                        style: "width: {PILL_WIDTH_REM}rem",
                        "aria-current": "{current}",
                        onclick: move |_| spy.pick(index),
                        "{title}"
                    }
                }
            }
            if let Some(trailing) = trailing {
                {trailing}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_last_section_above_the_line() {
        let tops = [Some(-900.0), Some(-300.0), Some(120.0), Some(640.0)];

        assert_eq!(active_section(&tops), 2);
    }

    #[test]
    fn falls_back_to_the_first_section() {
        assert_eq!(active_section(&[Some(400.0), Some(900.0)]), 0);
        assert_eq!(active_section(&[None, None]), 0);
        assert_eq!(active_section(&[]), 0);
    }
}
