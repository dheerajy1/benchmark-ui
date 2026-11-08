use super::live_chart::change_pill;
use crate::{
    components::Tone,
    format::{format_bytes, format_per_second, format_percent},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Chip {
    pub text: String,
    pub tone: Tone,
}

// The numbers a card states, and how the peak memory compares with the last run
pub fn chips_for(
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
