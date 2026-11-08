use dioxus::prelude::*;

use crate::{
    components::{Badge, Card, CardHeader, Tone},
    features::charts::{ChartTone, Reference, SeriesPoint, TrendChart},
    format::{format_optional, ValueFormat},
};

// Readings that decide the direction: the mean of the newest ones against the ones before
const WINDOW: usize = 4;
// Relative change that counts as rising or falling
const DIRECTION_LIMIT: f64 = 0.03;
// Percent change under which a run counts as the same as the last one
const SAME_LIMIT: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Rising,
    Steady,
    Falling,
}

impl Direction {
    fn label(self) -> &'static str {
        match self {
            Self::Rising => "Rising",
            Self::Steady => "Steady",
            Self::Falling => "Falling",
        }
    }

    // A reading that keeps growing is the one to look at
    fn tone(self) -> Tone {
        match self {
            Self::Rising => Tone::Danger,
            Self::Steady => Tone::Neutral,
            Self::Falling => Tone::Success,
        }
    }
}

// Whether the newest readings are above, level with or below the ones before them
pub fn direction_of(values: &[f64]) -> Direction {
    if values.len() < WINDOW * 2 {
        return Direction::Steady;
    }

    let mean = |slice: &[f64]| slice.iter().sum::<f64>() / slice.len() as f64;
    let end = values.len();
    let recent = mean(&values[end - WINDOW..]);
    let before = mean(&values[end - WINDOW * 2..end - WINDOW]);

    if before.abs() < f64::EPSILON {
        return Direction::Steady;
    }

    let change = (recent - before) / before;

    if change > DIRECTION_LIMIT {
        Direction::Rising
    } else if change < -DIRECTION_LIMIT {
        Direction::Falling
    } else {
        Direction::Steady
    }
}

// `+8% peak vs last run`, green when it is better and red when it is worse
pub fn change_pill(
    current: f64,
    last: f64,
    higher_is_better: bool,
    subject: &str,
) -> Option<(String, Tone)> {
    if last <= 0.0 {
        return None;
    }

    let percent = (current - last) / last * 100.0;

    if percent.abs() < SAME_LIMIT {
        return Some((format!("{subject} same as last run"), Tone::Neutral));
    }

    let tone = if (percent > 0.0) == higher_is_better {
        Tone::Success
    } else {
        Tone::Danger
    };

    Some((format!("{percent:+.0}% {subject} vs last run"), tone))
}

// A live chart in its own card. The dashed line is the same number from the last run, the
// chips say where the readings are heading and how they compare with that run.
#[component]
pub fn LiveChartCard(
    #[props(into)] title: String,
    #[props(into, default)] subtitle: String,
    points: Vec<SeriesPoint>,
    format: ValueFormat,
    tone: ChartTone,
    // The number from the last run and what it is called on the line
    #[props(default)] last: Option<f64>,
    #[props(into, default)] last_label: String,
    #[props(default)] ghost: Vec<f64>,
    #[props(default)] live: bool,
    // Shows the Rising, Steady or Falling chip
    #[props(default)] show_direction: bool,
    // Compares the newest reading with the last run instead of the highest one
    #[props(default)] compare_latest: bool,
    #[props(default)] higher_is_better: bool,
    // Drawn inside a group card, so without a card of its own
    #[props(default)] bare: bool,
) -> Element {
    let values: Vec<f64> = points.iter().filter_map(|point| point.value).collect();
    let peak = values.iter().copied().reduce(f64::max);
    let current = if compare_latest {
        values.last().copied()
    } else {
        peak
    };
    let subject = if compare_latest { "now" } else { "peak" };

    let direction = show_direction.then(|| direction_of(&values));
    let pill = current
        .zip(last)
        .and_then(|(current, last)| change_pill(current, last, higher_is_better, subject));
    let reference = last.map(|value| Reference {
        value,
        label: format!("{last_label} {}", format_optional(Some(value), format)),
    });

    if bare {
        return rsx! {
            div { class: "flex min-w-0 flex-col gap-(--space-2)",
                div { class: "flex flex-wrap items-center justify-between gap-(--space-2)",
                    h4 { class: "text-(length:--fs-sm) font-(weight:--fw-semibold) text-(color:--clr-text)",
                        "{title}"
                    }
                    div { class: "flex flex-wrap items-center justify-end gap-(--space-2)",
                        if let Some(direction) = direction {
                            Badge { label: direction.label(), tone: direction.tone() }
                        }
                        if let Some((text , tone)) = pill {
                            Badge { label: text, tone }
                        }
                    }
                }
                TrendChart {
                    title,
                    points,
                    format,
                    tone,
                    reference,
                    ghost,
                    live,
                    mark_peak: true,
                }
            }
        };
    }

    rsx! {
        Card {
            CardHeader {
                title: title.clone(),
                subtitle,
                trailing: Some(rsx! {
                    div { class: "flex flex-wrap items-center justify-end gap-(--space-2)",
                        if let Some(direction) = direction {
                            Badge { label: direction.label(), tone: direction.tone() }
                        }
                        if let Some((text , tone)) = pill {
                            Badge { label: text, tone }
                        }
                    }
                }),
            }
            div { class: "p-(--space-4)",
                TrendChart {
                    title,
                    points,
                    format,
                    tone,
                    reference,
                    ghost,
                    live,
                    mark_peak: true,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn few_readings_are_steady() {
        assert_eq!(direction_of(&[1.0, 2.0, 3.0]), Direction::Steady);
    }

    #[test]
    fn readings_that_climb_are_rising_and_those_that_drop_are_falling() {
        let up = [10.0, 10.0, 10.0, 10.0, 12.0, 12.0, 12.0, 12.0];
        let down = [12.0, 12.0, 12.0, 12.0, 10.0, 10.0, 10.0, 10.0];
        let level = [10.0, 10.0, 10.0, 10.0, 10.1, 10.0, 10.1, 10.0];

        assert_eq!(direction_of(&up), Direction::Rising);
        assert_eq!(direction_of(&down), Direction::Falling);
        assert_eq!(direction_of(&level), Direction::Steady);
    }

    #[test]
    fn a_zero_baseline_is_steady() {
        assert_eq!(direction_of(&[0.0; 8]), Direction::Steady);
    }

    #[test]
    fn the_pill_names_the_change_and_whether_it_is_better() {
        assert_eq!(
            change_pill(108.0, 100.0, false, "peak"),
            Some(("+8% peak vs last run".to_string(), Tone::Danger))
        );
        assert_eq!(
            change_pill(90.0, 100.0, false, "peak"),
            Some(("-10% peak vs last run".to_string(), Tone::Success))
        );
        assert_eq!(
            change_pill(1_200.0, 1_000.0, true, "now"),
            Some(("+20% now vs last run".to_string(), Tone::Success))
        );
    }

    #[test]
    fn a_small_change_is_the_same_and_no_last_run_means_no_pill() {
        assert_eq!(
            change_pill(101.0, 100.0, false, "peak"),
            Some(("peak same as last run".to_string(), Tone::Neutral))
        );
        assert_eq!(change_pill(5.0, 0.0, false, "peak"), None);
    }
}
