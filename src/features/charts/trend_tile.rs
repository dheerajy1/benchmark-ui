use dioxus::prelude::*;

use super::{palette::ChartTone, sparkline::Sparkline};
use crate::{
    components::{verdict, Tone, Verdict},
    format::{format_optional, format_signed_percent, ValueFormat},
};

// `Latest`, `1 run ago`, `3 runs ago` for each of `count` chronological points
pub fn ago_labels(count: usize) -> Vec<String> {
    (0..count)
        .map(|index| match count - 1 - index {
            0 => "Latest".to_string(),
            1 => "1 run ago".to_string(),
            age => format!("{age} runs ago"),
        })
        .collect()
}

// "482 MB" becomes ("482", "MB"), a value without a unit keeps an empty unit
pub fn split_unit(text: &str) -> (String, String) {
    match text.split_once(' ') {
        Some((number, unit)) => (number.to_string(), unit.to_string()),
        None => (text.to_string(), String::new()),
    }
}

// Percentage change of the newest value against the one before it, `None` when either is
// missing or the earlier one is zero
pub fn change_of_last(values: &[Option<f64>]) -> Option<f64> {
    let latest = (*values.last()?)?;
    let previous = (*values.get(values.len().checked_sub(2)?)?)?;

    if previous == 0.0 {
        return None;
    }

    Some((latest - previous) / previous * 100.0)
}

// One headline measurement: name and change pill, the newest value with its unit, and the
// values of the last runs as a small chart with the Peak and the Low marked on it.
// `values` are in chronological order, the last one belongs to the run shown.
#[component]
pub fn TrendTile(
    #[props(into)] label: String,
    values: Vec<Option<f64>>,
    format: ValueFormat,
    tone: ChartTone,
    lower_is_better: bool,
    // Small text at the right of the value, for example how many requests were sent
    #[props(into, default)] chip: String,
) -> Element {
    let labels = ago_labels(values.len());

    let latest = values.last().copied().flatten();
    let (number, unit) = split_unit(&format_optional(latest, format));
    let previous = values
        .len()
        .checked_sub(2)
        .and_then(|index| values.get(index))
        .copied()
        .flatten()
        .map(|value| format.format(value));

    let pill = change_of_last(&values).map(|change| {
        let pill_tone = match verdict(change, lower_is_better) {
            Verdict::Improved => Tone::Success,
            Verdict::Regressed => Tone::Danger,
            Verdict::Unchanged => Tone::Neutral,
        };
        let arrow = if change > 0.0 { "\u{2191}" } else { "\u{2193}" };

        (
            format!("{arrow} {}", format_signed_percent(change)),
            pill_tone.badge_class(),
        )
    });

    rsx! {
        div { class: "group fade-up flex min-w-0 flex-col gap-(--space-2) rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) px-(--space-4) pt-(--space-3) pb-(--space-2) shadow-(--shadow-card)",
            div { class: "flex items-center justify-between gap-(--space-2)",
                span { class: "text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text-muted)",
                    "{label}"
                }
                if let Some((text , pill_class)) = pill {
                    span { class: "group/pill inline-flex items-center rounded-(--round-full) px-(--space-2) py-px text-(length:--fs-xs) font-(weight:--fw-medium) whitespace-nowrap tabular-nums {pill_class}",
                        "{text}"
                        if let Some(previous) = previous {
                            span { class: "max-w-0 overflow-hidden opacity-0 transition-all duration-200 ease-(--ease-out) group-hover/pill:ml-(--space-1) group-hover/pill:max-w-32 group-hover/pill:opacity-100",
                                "vs {previous}"
                            }
                        }
                    }
                }
            }
            div { class: "flex items-baseline gap-(--space-1)",
                span { class: "text-(length:--fs-kpi) font-(weight:--fw-semibold) tabular-nums text-(color:--clr-text)",
                    "{number}"
                }
                if !unit.is_empty() {
                    span { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)", "{unit}" }
                }
                if !chip.is_empty() {
                    span { class: "ml-auto min-w-0 truncate pl-(--space-2) text-(length:--fs-xs) text-(color:--clr-text-muted) tabular-nums",
                        title: "{chip}",
                        "{chip}"
                    }
                }
            }
            Sparkline {
                values,
                labels,
                format,
                tone,
                mark_extremes: true,
                class: "h-20",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_points_by_their_age() {
        assert_eq!(ago_labels(3), ["2 runs ago", "1 run ago", "Latest"]);
        assert!(ago_labels(0).is_empty());
    }

    #[test]
    fn splits_the_unit_from_the_number() {
        assert_eq!(split_unit("482 MB"), ("482".to_string(), "MB".to_string()));
        assert_eq!(
            split_unit("\u{2014}"),
            ("\u{2014}".to_string(), String::new())
        );
        assert_eq!(
            split_unit("1,204 /s"),
            ("1,204".to_string(), "/s".to_string())
        );
    }

    #[test]
    fn computes_the_change_of_the_newest_value() {
        assert_eq!(change_of_last(&[Some(100.0), Some(120.0)]), Some(20.0));
        assert_eq!(change_of_last(&[Some(10.0), Some(9.0)]), Some(-10.0));
    }

    #[test]
    fn no_change_without_two_comparable_values() {
        assert_eq!(change_of_last(&[Some(1.0)]), None);
        assert_eq!(change_of_last(&[None, Some(1.0)]), None);
        assert_eq!(change_of_last(&[Some(1.0), None]), None);
        assert_eq!(change_of_last(&[Some(0.0), Some(1.0)]), None);
        assert_eq!(change_of_last(&[]), None);
    }
}
