use dioxus::prelude::*;

use super::{
    last_run::LastRun,
    live_chart::change_pill,
    state::{workspace_series, LiveState, Stage, StageState},
};
use crate::{
    components::{Badge, Tone},
    features::charts::Sparkline,
    format::{format_bytes, format_duration_ms, format_per_second, format_percent, ValueFormat},
};

// A phase that has finished and can be opened again
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseId {
    Workload,
    // Position in the list of workspace modes of the run
    Mode(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chip {
    pub text: String,
    pub tone: Tone,
}

// One finished phase, folded into a card
#[derive(Debug, Clone, PartialEq)]
pub struct ResultCard {
    pub id: PhaseId,
    pub name: String,
    pub duration: String,
    pub chips: Vec<Chip>,
    // Memory readings of the phase for the small trend line
    pub values: Vec<Option<f64>>,
    pub labels: Vec<String>,
}

// The numbers a card states, and how the peak memory compares with the last run
fn chips_for(
    peak_memory: Option<f64>,
    cpu_average: Option<f64>,
    throughput: Option<f64>,
    last_peak: Option<f64>,
) -> Vec<Chip> {
    let mut chips = Vec::new();
    let neutral = |text: String| Chip {
        text,
        tone: Tone::Neutral,
    };

    if let Some(peak) = peak_memory {
        chips.push(neutral(format!("Peak memory {}", format_bytes(peak))));
    }

    if let Some(average) = cpu_average {
        chips.push(neutral(format!("CPU average {}", format_percent(average))));
    }

    if let Some(rate) = throughput {
        chips.push(neutral(format!("{} average", format_per_second(rate))));
    }

    if let Some((text, tone)) = peak_memory
        .zip(last_peak)
        .and_then(|(peak, last)| change_pill(peak, last, false, "peak"))
    {
        chips.push(Chip { text, tone });
    }

    chips
}

// Cards of the phases that are over: the workload and every workspace mode that finished
pub fn cards_for(view: &LiveState, last: Option<&LastRun>) -> Vec<ResultCard> {
    let mut cards = Vec::new();
    let samples = view.samples();

    if let StageState::Done { duration_ms, .. } = view.stage(Stage::Workload) {
        if let Some(newest) = samples.last() {
            let peak = samples
                .iter()
                .filter_map(|sample| sample.rss_bytes)
                .max()
                .map(|bytes| bytes as f64);
            let series = view.chart_series(|sample| sample.rss_bytes.map(|bytes| bytes as f64));

            cards.push(ResultCard {
                id: PhaseId::Workload,
                name: "Workload".to_string(),
                duration: format_duration_ms(duration_ms.unwrap_or(newest.elapsed_ms) as f64),
                chips: chips_for(
                    peak,
                    newest.cpu_average,
                    Some(newest.requests_per_second),
                    last.and_then(|known| known.memory_peak),
                ),
                values: series.iter().map(|point| point.value).collect(),
                labels: series
                    .iter()
                    .map(|point| point.detail_label.clone())
                    .collect(),
            });
        }
    }

    for (index, mode) in view.workspace_modes().iter().enumerate() {
        let Some(newest) = mode.points.last() else {
            continue;
        };

        if mode.phase != "finished" {
            continue;
        }

        let peak = mode
            .points
            .iter()
            .filter_map(|point| point.rss_bytes)
            .max()
            .map(|bytes| bytes as f64);
        let series = workspace_series(mode, |point| point.rss_bytes.map(|bytes| bytes as f64));

        cards.push(ResultCard {
            id: PhaseId::Mode(index),
            name: format!(
                "{} · {}",
                mode.workspace,
                super::state::mode_title(&mode.mode)
            ),
            duration: format_duration_ms(newest.elapsed_ms as f64),
            chips: chips_for(
                peak,
                newest.cpu_average,
                None,
                last.and_then(|known| known.mode(&mode.workspace, &mode.mode))
                    .and_then(|entry| entry.memory_peak),
            ),
            values: series.iter().map(|point| point.value).collect(),
            labels: series
                .iter()
                .map(|point| point.detail_label.clone())
                .collect(),
        });
    }

    cards
}

// The finished phases as cards. Choosing one opens its charts, choosing it again closes them.
#[component]
pub fn ResultCards(
    cards: Vec<ResultCard>,
    selected: Option<PhaseId>,
    on_pick: EventHandler<PhaseId>,
) -> Element {
    if cards.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            class: "grid grid-cols-1 gap-(--space-3) sm:grid-cols-2",
            "aria-label": "Finished phases",
            for card in cards {
                {
                    let id = card.id;
                    let chosen = selected == Some(id);
                    let ring = if chosen {
                        "border-(color:--clr-accent)"
                    } else {
                        "border-(color:--clr-border) hover:border-(color:--clr-border-strong)"
                    };

                    rsx! {
                        button {
                            key: "{card.name}",
                            r#type: "button",
                            class: "group fade-up flex cursor-pointer flex-col gap-(--space-2) rounded-(--round-lg) border bg-(--clr-surface) p-(--space-3) text-left transition-colors {ring}",
                            "aria-pressed": "{chosen}",
                            onclick: move |_| on_pick.call(id),
                            div { class: "flex items-baseline justify-between gap-(--space-2)",
                                span { class: "min-w-0 truncate text-(length:--fs-sm) font-(weight:--fw-semibold) text-(color:--clr-text)",
                                    "{card.name}"
                                }
                                span { class: "shrink-0 text-(length:--fs-xs) tabular-nums text-(color:--clr-text-muted)",
                                    "{card.duration}"
                                }
                            }
                            div { class: "flex flex-wrap gap-(--space-1)",
                                for chip in card.chips {
                                    Badge { key: "{chip.text}", label: chip.text.clone(), tone: chip.tone }
                                }
                            }
                            if card.values.len() >= 2 {
                                Sparkline {
                                    values: card.values,
                                    labels: card.labels,
                                    format: ValueFormat::Bytes,
                                    class: "h-8",
                                }
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
    fn a_card_states_the_numbers_and_the_change() {
        let chips = chips_for(Some(110.0), Some(12.0), Some(2_000.0), Some(100.0));

        assert_eq!(chips.len(), 4);
        assert!(chips[0].text.starts_with("Peak memory"));
        assert!(chips[1].text.starts_with("CPU average"));
        assert!(chips[2].text.ends_with("average"));
        assert_eq!(chips[3].text, "+10% peak vs last run");
        assert_eq!(chips[3].tone, Tone::Danger);
    }

    #[test]
    fn missing_numbers_are_left_out_not_zero() {
        assert!(chips_for(None, None, None, Some(100.0)).is_empty());

        let chips = chips_for(Some(100.0), None, None, None);

        assert_eq!(chips.len(), 1);
    }
}
