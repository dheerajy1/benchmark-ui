use dioxus::prelude::*;

use crate::{
    app::Route,
    components::{Icon, IconKind, RuntimeTag, INPUT_CLASS},
    features::history::RunSummary,
    format::{date_part, path_tail, relative_time, short_id, MISSING},
};

fn option_label(run: &RunSummary) -> String {
    format!(
        "{} · {} · {} · {}",
        short_id(&run.run_id),
        run.discovery.runtime.as_deref().unwrap_or(MISSING),
        path_tail(&run.target.path),
        date_part(&run.created_at)
    )
}

// Which side of the comparison a card stands for
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Baseline,
    Comparison,
}

impl Side {
    fn label(self) -> &'static str {
        match self {
            Self::Baseline => "Baseline",
            Self::Comparison => "Comparison",
        }
    }

    // Grey tag for the baseline, blue tag for the comparison
    fn tag_class(self) -> &'static str {
        match self {
            Self::Baseline => "bg-(--clr-surface-hover) text-(color:--clr-text-muted)",
            Self::Comparison => "bg-(--clr-accent-soft) text-(color:--clr-accent)",
        }
    }
}

// One side of the comparison: name, runtime, short id and time of the chosen run, and a select
// listing the newest runs. A side without a run is drawn with a dashed border.
#[component]
fn SelectorCard(
    side: Side,
    value: String,
    runs: Vec<RunSummary>,
    on_change: EventHandler<String>,
) -> Element {
    let chosen = runs.iter().find(|run| run.run_id == value).cloned();
    let listed = chosen.is_some();
    let unlisted = if !value.is_empty() && !listed {
        Some(format!("{} · not in the newest runs", short_id(&value)))
    } else {
        None
    };
    let options: Vec<(String, String, bool)> = runs
        .iter()
        .map(|run| (run.run_id.clone(), option_label(run), run.run_id == value))
        .collect();
    let value_for_unlisted = value.clone();
    let border = if value.is_empty() {
        "border-dashed border-(color:--clr-border-strong)"
    } else {
        "border-(color:--clr-border)"
    };
    let tag_class = side.tag_class();
    let label = side.label();
    let now_ms = js_sys::Date::now() as i64;
    let chosen_short = chosen
        .as_ref()
        .map(|run| short_id(&run.run_id))
        .unwrap_or_default();
    let chosen_when = chosen
        .as_ref()
        .map(|run| relative_time(&run.created_at, now_ms))
        .unwrap_or_default();
    let value_short = short_id(&value);

    rsx! {
        div { class: "flex min-w-0 flex-col gap-(--space-3) rounded-(--round-lg) border bg-(--clr-surface) p-(--space-4) {border}",
            span { class: "inline-flex w-fit items-center rounded-(--round-full) px-(--space-2) py-px text-(length:--fs-xs) font-(weight:--fw-medium) {tag_class}",
                "{label}"
            }
            div { class: "flex min-h-14 flex-col gap-(--space-1)",
                if let Some(run) = chosen {
                    div { class: "flex flex-wrap items-center gap-(--space-2)",
                        Link {
                            to: Route::RunDetail {
                                run_id: run.run_id.clone(),
                            },
                            class: "min-w-0 text-(length:--fs-base) font-(weight:--fw-semibold) break-words text-(color:--clr-text) hover:text-(color:--clr-accent)",
                            "{run.display_name}"
                        }
                        if let Some(runtime) = run.runtime_code() {
                            RuntimeTag { runtime: crate::features::history::runtime_label(runtime) }
                        }
                    }
                    p { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)",
                        span { class: "font-(family-name:--font-code)", "{chosen_short}" }
                        " · {chosen_when}"
                    }
                } else if !value.is_empty() {
                    p { class: "font-(family-name:--font-code) text-(length:--fs-base) text-(color:--clr-text)",
                        "{value_short}"
                    }
                    p { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)",
                        "Older than the newest runs"
                    }
                } else {
                    p { class: "text-(length:--fs-base) text-(color:--clr-text-faint)", "No run chosen" }
                }
            }
            select {
                class: "{INPUT_CLASS}",
                "aria-label": "{label} run",
                onchange: move |event| on_change.call(event.value()),
                option { value: "", selected: value.is_empty(), "Choose a run" }
                if let Some(text) = unlisted {
                    option { value: "{value_for_unlisted}", selected: true, "{text}" }
                }
                for (id , text , is_selected) in options {
                    option { value: "{id}", selected: is_selected, "{text}" }
                }
            }
        }
    }
}

// The two run cards with the swap button between them. The selection lives in the address.
#[component]
pub fn RunSelectors(runs: Vec<RunSummary>, baseline: String, comparison: String) -> Element {
    let nav = use_navigator();
    let mut turns = use_signal(|| 0_u32);

    let other_for_baseline = comparison.clone();
    let other_for_comparison = baseline.clone();
    let swap_baseline = comparison.clone();
    let swap_comparison = baseline.clone();
    let angle = turns() * 180;

    rsx! {
        div { class: "flex flex-col items-stretch gap-(--space-3) md:flex-row md:items-center",
            div { class: "min-w-0 md:flex-1",
                SelectorCard {
                    side: Side::Baseline,
                    value: baseline.clone(),
                    runs: runs.clone(),
                    on_change: move |id: String| {
                        let _ = nav
                            .push(Route::Comparisons {
                                baseline: id,
                                comparison: other_for_baseline.clone(),
                            });
                    },
                }
            }
            button {
                r#type: "button",
                class: "group/swap inline-flex size-(--control-height) shrink-0 cursor-pointer items-center justify-center self-center rounded-(--round-md) border border-(color:--clr-border) bg-(--clr-surface) text-(color:--clr-text-muted) transition-[background-color,color,transform] duration-200 hover:bg-(--clr-surface-hover) hover:text-(color:--clr-accent) active:scale-95 focus-visible:outline-none focus-visible:shadow-(--halo-focus)",
                "aria-label": "Swap baseline and comparison",
                title: "Swap baseline and comparison",
                onclick: move |_| {
                    turns += 1;
                    let _ = nav
                        .push(Route::Comparisons {
                            baseline: swap_baseline.clone(),
                            comparison: swap_comparison.clone(),
                        });
                },
                span {
                    class: "inline-flex transition-transform duration-500 ease-(--ease-out)",
                    style: "transform: rotate({angle}deg)",
                    Icon {
                        kind: IconKind::Swap,
                        class: "size-(--icon-sm) transition-transform duration-300 ease-(--ease-out) group-hover/swap:rotate-180",
                    }
                }
            }
            div { class: "min-w-0 md:flex-1",
                SelectorCard {
                    side: Side::Comparison,
                    value: comparison.clone(),
                    runs,
                    on_change: move |id: String| {
                        let _ = nav
                            .push(Route::Comparisons {
                                baseline: other_for_comparison.clone(),
                                comparison: id,
                            });
                    },
                }
            }
        }
    }
}
