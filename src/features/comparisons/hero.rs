use dioxus::prelude::*;

use super::models::{ComparisonData, MetricDiff};
use crate::{
    components::{button_class, use_image_saver, verdict, ButtonVariant, DeltaBadge, Verdict},
    features::{charts::ChartTone, history::runtime_label},
    format::{format_optional, path_tail, ValueFormat},
};

// Under this factor a difference is described in percent, "4% faster" reads better than "1.0x faster"
const FACTOR_FROM: f64 = 1.1;

// DOM id of the hero card and the name of its picture
const HERO_ID: &str = "card-summary-hero";
const HERO_FILE: &str = "benchmark-comparison.png";

// The big sentence of the hero, for example `3.4x faster`
#[derive(Debug, Clone, PartialEq)]
pub struct Headline {
    pub text: String,
    pub verdict: Verdict,
}

// How many times better the comparison is, above 1 is better. None when a side is missing or zero.
fn gain(diff: &MetricDiff) -> Option<f64> {
    let baseline = diff.baseline_value.filter(|value| *value > 0.0)?;
    let comparison = diff.comparison_value.filter(|value| *value > 0.0)?;

    Some(if diff.lower_is_better() {
        baseline / comparison
    } else {
        comparison / baseline
    })
}

fn headline_of(gain: f64) -> Headline {
    if (gain - 1.0).abs() < 0.005 {
        return Headline {
            text: "About the same".to_string(),
            verdict: Verdict::Unchanged,
        };
    }

    let better = gain > 1.0;
    let factor = if better { gain } else { 1.0 / gain };
    let word = if better { "faster" } else { "slower" };
    let text = if factor >= FACTOR_FROM {
        format!("{factor:.1}x {word}")
    } else {
        format!("{:.0}% {word}", (factor - 1.0) * 100.0)
    };

    Headline {
        text,
        verdict: if better {
            Verdict::Improved
        } else {
            Verdict::Regressed
        },
    }
}

// Throughput decides the headline, p95 latency steps in when there was no workload throughput
pub fn headline(data: &ComparisonData) -> Headline {
    let http = &data.metrics.http_latency;

    gain(&http.requests_per_second)
        .or_else(|| gain(&http.p95_ms))
        .map_or_else(
            || Headline {
                text: "Nothing to compare".to_string(),
                verdict: Verdict::Unchanged,
            },
            headline_of,
        )
}

// "41% lower", None when the change is missing or smaller than the noise
fn phrase(diff: &MetricDiff) -> Option<String> {
    let change = diff.percentage_change?;

    if verdict(change, diff.lower_is_better()) == Verdict::Unchanged {
        return Some("unchanged".to_string());
    }

    let direction = if change > 0.0 { "higher" } else { "lower" };

    Some(format!("{:.0}% {direction}", change.abs()))
}

fn join_parts(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        [init @ .., last] => format!("{}, and {last}", init.join(", ")),
    }
}

// One sentence under the headline, for example
// `Throughput is 12,400 /s against 3,640 /s, p95 latency is 62% lower, and peak memory is 41% higher.`
pub fn summary_sentence(data: &ComparisonData) -> String {
    let metrics = &data.metrics;
    let http = &metrics.http_latency;
    let mut parts = Vec::new();

    if let (Some(baseline), Some(comparison)) = (
        http.requests_per_second.baseline_value,
        http.requests_per_second.comparison_value,
    ) {
        parts.push(format!(
            "throughput is {} against {}",
            ValueFormat::PerSecond.format(comparison),
            ValueFormat::PerSecond.format(baseline)
        ));
    }

    if let Some(text) = phrase(&http.p95_ms) {
        parts.push(format!("p95 latency is {text}"));
    }

    if let Some(text) = phrase(&metrics.memory.peak_rss_bytes) {
        parts.push(format!("peak memory is {text}"));
    }

    if parts.is_empty() {
        return "Neither run has numbers that can be compared.".to_string();
    }

    let sentence = join_parts(&parts);
    let mut letters = sentence.chars();
    let first = letters.next().map(|c| c.to_uppercase().to_string());

    format!("{}{}.", first.unwrap_or_default(), letters.as_str())
}

// Small line above the headline, for example `Rust vs TypeScript, same target`
pub fn context_line(data: &ComparisonData) -> String {
    let name = |runtime: &Option<String>, path: &str| {
        runtime
            .as_deref()
            .map_or_else(|| path_tail(path), runtime_label)
    };
    let baseline = name(&data.baseline.runtime, &data.baseline.target.path);
    let comparison = name(&data.comparison.runtime, &data.comparison.target.path);
    let target = if data.baseline.target.path == data.comparison.target.path {
        "same target"
    } else {
        "different targets"
    };

    format!("{baseline} vs {comparison}, {target}")
}

// Widths in percent of the longer bar of the pair, an existing bar is never thinner than 1.5%
fn pair_widths(baseline: Option<f64>, comparison: Option<f64>) -> (f64, f64) {
    let highest = [baseline, comparison]
        .into_iter()
        .flatten()
        .filter(|value| value.is_finite())
        .fold(0.0_f64, f64::max);

    let width = |value: Option<f64>| match value {
        Some(value) if highest > 0.0 && value.is_finite() => {
            (value / highest * 100.0).clamp(1.5, 100.0)
        }
        _ => 0.0,
    };

    (width(baseline), width(comparison))
}

// One metric of the paired bars
struct PairRow {
    label: &'static str,
    format: ValueFormat,
    diff: MetricDiff,
}

fn pair_rows(data: &ComparisonData) -> Vec<PairRow> {
    let metrics = &data.metrics;

    vec![
        PairRow {
            label: "Throughput",
            format: ValueFormat::PerSecond,
            diff: metrics.http_latency.requests_per_second.clone(),
        },
        PairRow {
            label: "p95 latency",
            format: ValueFormat::LatencyMs,
            diff: metrics.http_latency.p95_ms.clone(),
        },
        PairRow {
            label: "Peak memory",
            format: ValueFormat::Bytes,
            diff: metrics.memory.peak_rss_bytes.clone(),
        },
        PairRow {
            label: "Startup",
            format: ValueFormat::DurationMs,
            diff: metrics.startup_time.duration_ms.clone(),
        },
    ]
}

// One row of the paired bars with every text already formatted
struct PairView {
    label: &'static str,
    baseline_text: String,
    comparison_text: String,
    baseline_style: String,
    comparison_style: String,
    change: Option<f64>,
    lower_is_better: bool,
}

// Headline card of the page: how much faster or slower, one sentence, and four paired bars
#[component]
pub fn Hero(data: ComparisonData) -> Element {
    let top = headline(&data);
    let sentence = summary_sentence(&data);
    let context = context_line(&data);
    let headline_class = match top.verdict {
        Verdict::Improved => "text-(color:--clr-accent)",
        Verdict::Regressed => "text-(color:--clr-danger-text)",
        Verdict::Unchanged => "text-(color:--clr-text)",
    };
    let save_image = use_image_saver();
    let secondary = button_class(ButtonVariant::Secondary);
    let baseline_color = ChartTone::Slate.color_var();
    let comparison_color = ChartTone::Blue.color_var();
    let baseline_swatch = ChartTone::Slate.swatch_class();
    let comparison_swatch = ChartTone::Blue.swatch_class();

    let rows: Vec<PairView> = pair_rows(&data)
        .into_iter()
        .map(|row| {
            let (baseline_width, comparison_width) =
                pair_widths(row.diff.baseline_value, row.diff.comparison_value);

            PairView {
                label: row.label,
                baseline_text: format_optional(row.diff.baseline_value, row.format),
                comparison_text: format_optional(row.diff.comparison_value, row.format),
                baseline_style: format!(
                    "width: {baseline_width:.1}%; background-color: {baseline_color}"
                ),
                comparison_style: format!(
                    "width: {comparison_width:.1}%; background-color: {comparison_color}"
                ),
                change: row.diff.percentage_change,
                lower_is_better: row.diff.lower_is_better(),
            }
        })
        .collect();

    rsx! {
        section {
            id: HERO_ID,
            class: "fade-up flex flex-col gap-(--space-5) rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) p-(--space-5) shadow-(--shadow-card)",
            "aria-label": "Summary of the comparison",
            div { class: "flex flex-col gap-(--space-2)",
                div { class: "flex items-start justify-between gap-(--space-3)",
                    p { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)", "{context}" }
                    button {
                        r#type: "button",
                        class: "shrink-0 {secondary}",
                        "data-capture-skip": "",
                        onclick: move |_| {
                            save_image.call((HERO_ID.to_string(), HERO_FILE.to_string()));
                        },
                        "Save as image"
                    }
                }
                h2 { class: "text-(length:--fs-hero) leading-tight font-(weight:--fw-semibold) tabular-nums {headline_class}",
                    "{top.text}"
                }
                p { class: "max-w-2xl text-(length:--fs-base) text-(color:--clr-text-muted)", "{sentence}" }
            }
            div { class: "group/pairs flex flex-col gap-(--space-4)", role: "list",
                for row in rows {
                    div {
                        key: "{row.label}",
                        role: "listitem",
                        class: "flex flex-col gap-(--space-2) transition-opacity duration-150 group-hover/pairs:opacity-40 hover:opacity-100! md:flex-row md:items-center md:gap-(--space-4)",
                        span { class: "text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text) md:w-28 md:shrink-0",
                            "{row.label}"
                        }
                        div { class: "flex min-w-0 flex-1 flex-col gap-(--space-1)",
                            div { class: "flex items-center gap-(--space-3)",
                                div { class: "h-3 min-w-0 flex-1 overflow-hidden rounded-(--round-full) bg-(--clr-surface-hover)",
                                    div {
                                        class: "grow-x h-full rounded-(--round-full)",
                                        style: "{row.baseline_style}",
                                    }
                                }
                                span { class: "w-28 shrink-0 text-right text-(length:--fs-sm) tabular-nums text-(color:--clr-text-muted)",
                                    "{row.baseline_text}"
                                }
                            }
                            div { class: "flex items-center gap-(--space-3)",
                                div { class: "h-3 min-w-0 flex-1 overflow-hidden rounded-(--round-full) bg-(--clr-surface-hover)",
                                    div {
                                        class: "grow-x h-full rounded-(--round-full)",
                                        style: "{row.comparison_style}",
                                    }
                                }
                                span { class: "w-28 shrink-0 text-right text-(length:--fs-sm) font-(weight:--fw-medium) tabular-nums text-(color:--clr-text)",
                                    "{row.comparison_text}"
                                }
                            }
                        }
                        div { class: "md:w-48 md:shrink-0 md:text-right",
                            DeltaBadge { change: row.change, lower_is_better: row.lower_is_better }
                        }
                    }
                }
            }
            div { class: "flex flex-wrap items-center gap-(--space-4) text-(length:--fs-xs) text-(color:--clr-text-muted)",
                span { class: "inline-flex items-center gap-(--space-2)",
                    span { class: "inline-block size-(--dot-size) rounded-(--round-full) {baseline_swatch}" }
                    "Baseline"
                }
                span { class: "inline-flex items-center gap-(--space-2)",
                    span { class: "inline-block size-(--dot-size) rounded-(--round-full) {comparison_swatch}" }
                    "Comparison"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{super::models::fixtures::diff, *};

    fn higher(baseline: f64, comparison: f64) -> MetricDiff {
        diff(Some(baseline), Some(comparison), "higher_is_better")
    }

    #[test]
    fn counts_faster_when_throughput_grows() {
        let result = headline_of(gain(&higher(1_000.0, 3_400.0)).expect("gain"));

        assert_eq!(result.text, "3.4x faster");
        assert_eq!(result.verdict, Verdict::Improved);
    }

    #[test]
    fn counts_slower_when_throughput_drops() {
        let result = headline_of(gain(&higher(3_400.0, 1_000.0)).expect("gain"));

        assert_eq!(result.text, "3.4x slower");
        assert_eq!(result.verdict, Verdict::Regressed);
    }

    #[test]
    fn lower_latency_counts_as_faster() {
        let latency = diff(Some(100.0), Some(50.0), "lower_is_better");

        assert_eq!(
            headline_of(gain(&latency).expect("gain")).text,
            "2.0x faster"
        );
    }

    #[test]
    fn small_differences_are_percent_and_tiny_ones_are_the_same() {
        assert_eq!(headline_of(1.04).text, "4% faster");
        assert_eq!(headline_of(1.0 / 1.04).text, "4% slower");
        assert_eq!(headline_of(1.001).verdict, Verdict::Unchanged);
    }

    #[test]
    fn a_missing_or_zero_side_has_no_gain() {
        assert_eq!(gain(&diff(None, Some(1.0), "lower_is_better")), None);
        assert_eq!(gain(&higher(0.0, 5.0)), None);
    }

    #[test]
    fn phrases_the_change_with_its_direction() {
        let lower = diff(Some(100.0), Some(59.0), "lower_is_better");
        let same = diff(Some(100.0), Some(100.1), "lower_is_better");

        assert_eq!(phrase(&lower).as_deref(), Some("41% lower"));
        assert_eq!(phrase(&same).as_deref(), Some("unchanged"));
        assert_eq!(phrase(&diff(None, None, "lower_is_better")), None);
    }

    #[test]
    fn joins_parts_with_and() {
        let parts = |count: usize| -> Vec<String> { (0..count).map(|n| format!("p{n}")).collect() };

        assert_eq!(join_parts(&parts(1)), "p0");
        assert_eq!(join_parts(&parts(2)), "p0 and p1");
        assert_eq!(join_parts(&parts(3)), "p0, p1, and p2");
    }

    #[test]
    fn scales_a_pair_to_the_longer_bar() {
        assert_eq!(pair_widths(Some(50.0), Some(100.0)), (50.0, 100.0));
        assert_eq!(pair_widths(None, Some(100.0)), (0.0, 100.0));
        assert_eq!(pair_widths(Some(0.0), Some(0.0)), (0.0, 0.0));
        assert_eq!(pair_widths(Some(1.0), Some(1_000.0)).0, 1.5);
    }
}
