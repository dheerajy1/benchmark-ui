use dioxus::prelude::*;

use super::Tone;
use crate::format::{format_signed_percent, MISSING};

// Changes smaller than this are reported as unchanged, it hides measurement noise
const UNCHANGED_THRESHOLD_PERCENT: f64 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Improved,
    Regressed,
    Unchanged,
}

// Judges a percentage change using the metric's direction
pub fn verdict(change_percent: f64, lower_is_better: bool) -> Verdict {
    if change_percent.abs() < UNCHANGED_THRESHOLD_PERCENT {
        return Verdict::Unchanged;
    }

    let went_down = change_percent < 0.0;

    if went_down == lower_is_better {
        Verdict::Improved
    } else {
        Verdict::Regressed
    }
}

// Signed percentage colored by whether the change is an improvement
#[component]
pub fn DeltaBadge(change: Option<f64>, lower_is_better: bool) -> Element {
    let Some(change) = change else {
        return rsx! {
            span { class: "text-(length:--fs-sm) text-(color:--clr-text-faint)", "{MISSING}" }
        };
    };

    let (tone, word) = match verdict(change, lower_is_better) {
        Verdict::Improved => (Tone::Success, "Improved"),
        Verdict::Regressed => (Tone::Danger, "Regressed"),
        Verdict::Unchanged => (Tone::Neutral, "Unchanged"),
    };

    let label = format_signed_percent(change);
    let tone_class = tone.text_class();
    let badge_class = tone.badge_class();
    let arrow = if change.abs() < UNCHANGED_THRESHOLD_PERCENT {
        "→"
    } else if change > 0.0 {
        "↑"
    } else {
        "↓"
    };

    rsx! {
        span {
            class: "inline-flex items-center justify-end gap-(--space-2) text-(length:--fs-sm) tabular-nums",
            title: "{word}",
            span { class: "inline-flex items-center gap-(--space-1) rounded-(--round-full) px-(--space-2) py-px font-(weight:--fw-medium) {badge_class}",
                span { "aria-hidden": "true", "{arrow}" }
                "{label}"
            }
            span { class: "hidden w-18 text-left text-(length:--fs-xs) lg:inline {tone_class}", "{word}" }
            span { class: "sr-only lg:hidden", "{word}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_is_better_improves_when_value_drops() {
        assert_eq!(verdict(-25.0, true), Verdict::Improved);
        assert_eq!(verdict(10.0, true), Verdict::Regressed);
    }

    #[test]
    fn higher_is_better_improves_when_value_rises() {
        assert_eq!(verdict(12.0, false), Verdict::Improved);
        assert_eq!(verdict(-12.0, false), Verdict::Regressed);
    }

    #[test]
    fn tiny_changes_are_unchanged() {
        assert_eq!(verdict(0.2, true), Verdict::Unchanged);
        assert_eq!(verdict(-0.4, false), Verdict::Unchanged);
    }
}
