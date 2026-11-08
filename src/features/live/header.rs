use dioxus::prelude::*;

use super::{
    cancel_control::CancelControl,
    driver::FeedState,
    state::{LiveState, SamplePoint, Stage, StageState, Terminal},
};
use crate::{
    components::{button_class, Badge, ButtonVariant, Card, RuntimeTag, StatTile, Tone},
    features::history::runtime_label,
    format::{
        format_bytes, format_duration_ms, format_number, format_per_second, format_percent,
        path_tail, short_id, MISSING,
    },
};

// Badge text and tone of the whole run
pub fn run_status(state: &LiveState, feed: &FeedState) -> (&'static str, Tone) {
    match state.terminal() {
        Some(Terminal::Completed(_)) => ("Completed", Tone::Success),
        Some(Terminal::Failed { .. }) => ("Failed", Tone::Danger),
        Some(Terminal::Cancelled { .. }) => ("Cancelled", Tone::Neutral),
        None if state.is_cancelling() => ("Cancelling", Tone::Neutral),
        None => match feed {
            FeedState::Live | FeedState::Polling => ("Running", Tone::Accent),
            FeedState::Connecting => ("Connecting", Tone::Neutral),
            FeedState::Reconnecting { .. } => ("Reconnecting", Tone::Neutral),
            FeedState::TooManyViewers => ("Waiting for a place", Tone::Neutral),
            FeedState::Offline(_) => ("Offline", Tone::Danger),
            FeedState::Finished => ("Finished", Tone::Neutral),
            FeedState::Lost => ("Unavailable", Tone::Danger),
        },
    }
}

// `About 12 s left`, `About 3 min left`, or a plain note when the run takes longer than the
// usual time. Nothing is said when there is no usual time to go by.
pub fn left_text(elapsed_ms: u64, estimate_ms: Option<u64>) -> Option<String> {
    let estimate = estimate_ms?;

    if elapsed_ms >= estimate {
        return Some("Taking longer than usual".to_string());
    }

    let seconds = (estimate - elapsed_ms).div_ceil(1_000);

    Some(if seconds < 90 {
        format!("About {seconds} s left")
    } else {
        format!("About {} min left", seconds.div_ceil(60))
    })
}

// How far the run is, 0 to 99 while it runs. The usual time is the yardstick when it is known,
// otherwise the share of finished steps.
pub fn percent_done(
    finished_steps: usize,
    total_steps: usize,
    elapsed_ms: u64,
    estimate_ms: Option<u64>,
) -> u32 {
    let percent = match estimate_ms.filter(|estimate| *estimate > 0) {
        Some(estimate) => (elapsed_ms as f64 / estimate as f64 * 100.0) as u32,
        None if total_steps == 0 => 0,
        None => (finished_steps * 100 / total_steps) as u32,
    };

    percent.min(99)
}

// Text of the browser tab: the progress while it runs, the outcome after
pub fn tab_title(terminal: Option<&Terminal>, percent: u32, label: &str) -> String {
    match terminal {
        None => format!("{percent}% · {label} benchmark"),
        Some(Terminal::Completed(_)) => format!("Done · {label} benchmark"),
        Some(Terminal::Failed { .. }) => format!("Failed · {label} benchmark"),
        Some(Terminal::Cancelled { .. }) => format!("Cancelled · {label} benchmark"),
    }
}

// The name of the run for people: the saved target's name or the last folder of its path
fn run_name(state: &LiveState) -> Option<String> {
    state.plan().map(|plan| {
        plan.target_name
            .clone()
            .unwrap_or_else(|| path_tail(&plan.path))
    })
}

// Steps that are over, done or left out
fn finished_steps(state: &LiveState) -> usize {
    Stage::ALL
        .iter()
        .filter(|stage| {
            matches!(
                state.stage(**stage),
                StageState::Done { .. } | StageState::Skipped { .. }
            )
        })
        .count()
}

// The title of the browser tab. It sits in its own component so only the title redraws when the
// clock ticks.
#[component]
pub fn TabTitle(
    state: Signal<LiveState>,
    clock: Signal<u64>,
    estimate_ms: Option<u64>,
    short: String,
) -> Element {
    let title = {
        let snapshot = state.read();

        match run_name(&snapshot) {
            None => format!("Live run {short} · Benchmark"),
            Some(name) => {
                let label = snapshot.runtime().map_or(name, runtime_label);
                let percent = percent_done(
                    finished_steps(&snapshot),
                    Stage::ALL.len(),
                    snapshot.run_elapsed_ms(clock()),
                    estimate_ms,
                );

                tab_title(snapshot.terminal(), percent, &label)
            }
        }
    };

    rsx! {
        document::Title { "{title}" }
    }
}

// Name, runtime, status, elapsed time, the time that is left and the Cancel control
#[component]
pub fn HeaderCard(
    state: Signal<LiveState>,
    feed: Signal<FeedState>,
    clock: Signal<u64>,
    token: Signal<Option<String>>,
    estimate_ms: Option<u64>,
    paused: bool,
    on_pause: EventHandler<()>,
) -> Element {
    let secondary = button_class(ButtonVariant::Secondary);
    let (status, tone, elapsed, left, name, full_path, runtime, short, run_id, show_pause) = {
        let snapshot = state.read();
        let (status, tone) = run_status(&snapshot, &feed());
        let elapsed_ms = snapshot.run_elapsed_ms(clock());
        let running = !snapshot.is_terminal() && !snapshot.is_cancelling();

        (
            status,
            tone,
            format_duration_ms(elapsed_ms as f64),
            if running {
                left_text(elapsed_ms, estimate_ms)
            } else {
                None
            },
            run_name(&snapshot),
            snapshot.plan().map(|plan| plan.path.clone()),
            snapshot.runtime().map(runtime_label),
            short_id(snapshot.run_id()),
            snapshot.run_id().to_string(),
            running && (!snapshot.samples().is_empty() || !snapshot.workspace_modes().is_empty()),
        )
    };
    let full_path = full_path.unwrap_or_default();

    rsx! {
        Card {
            div { class: "flex flex-wrap items-center gap-x-(--space-3) gap-y-(--space-2) px-(--space-4) py-(--space-3)",
                if let Some(name) = name {
                    h2 {
                        class: "min-w-0 truncate text-(length:--fs-lg) font-(weight:--fw-semibold) text-(color:--clr-text)",
                        title: "{full_path}",
                        "{name}"
                    }
                }
                if let Some(runtime) = runtime {
                    RuntimeTag { runtime }
                }
                Badge { label: status, tone }
                span { class: "font-(family-name:--font-code) text-(length:--fs-xs) text-(color:--clr-text-faint)",
                    "{short}"
                }
                div { class: "ml-auto flex flex-col items-end text-(length:--fs-sm) tabular-nums",
                    span { class: "text-(color:--clr-text)", "Elapsed {elapsed}" }
                    if let Some(left) = left {
                        span { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)", "{left}" }
                    }
                }
                if show_pause {
                    button {
                        r#type: "button",
                        class: "{secondary}",
                        "aria-pressed": "{paused}",
                        onclick: move |_| on_pause.call(()),
                        if paused {
                            "Resume view"
                        } else {
                            "Pause view"
                        }
                    }
                }
                CancelControl { state, run_id, token }
            }
        }
    }
}

// Throughput, memory and CPU of the run as they are now
struct HudValues {
    throughput: String,
    throughput_hint: String,
    memory: String,
    memory_hint: String,
    cpu: String,
    cpu_hint: String,
}

fn cpu_text(sample: &SamplePoint) -> (String, String) {
    let value = sample
        .cpu_interval
        .or(sample.cpu_average)
        .map_or_else(|| MISSING.to_string(), format_percent);

    let hint = match (sample.cpu_average, sample.cpu_peak) {
        (Some(average), Some(peak)) => {
            format!(
                "average {} · peak {}",
                format_percent(average),
                format_percent(peak)
            )
        }
        (Some(average), None) => format!("average {}", format_percent(average)),
        _ => String::new(),
    };

    (value, hint)
}

fn hud_values(sample: Option<&SamplePoint>) -> HudValues {
    let Some(sample) = sample else {
        return HudValues {
            throughput: MISSING.to_string(),
            throughput_hint: "waiting for the workload".to_string(),
            memory: MISSING.to_string(),
            memory_hint: String::new(),
            cpu: MISSING.to_string(),
            cpu_hint: String::new(),
        };
    };

    let (cpu, cpu_hint) = cpu_text(sample);

    HudValues {
        throughput: format_per_second(sample.requests_per_second),
        throughput_hint: format!(
            "running average · {} sent",
            format_number(sample.completed as f64, 0)
        ),
        memory: sample
            .rss_bytes
            .map_or_else(|| MISSING.to_string(), |bytes| format_bytes(bytes as f64)),
        memory_hint: sample
            .peak_rss_bytes
            .map(|bytes| format!("peak {}", format_bytes(bytes as f64)))
            .unwrap_or_default(),
        cpu,
        cpu_hint,
    }
}

// The three numbers to watch while the run goes
#[component]
pub fn LiveHud(state: Signal<LiveState>) -> Element {
    let values = hud_values(state.read().latest_sample());

    rsx! {
        div {
            class: "grid grid-cols-1 gap-(--space-3) sm:grid-cols-3",
            "aria-label": "Live measurements",
            StatTile {
                label: "Requests per second",
                value: values.throughput,
                hint: values.throughput_hint,
            }
            StatTile {
                label: "Memory now",
                value: values.memory,
                hint: values.memory_hint,
            }
            StatTile {
                label: "CPU now",
                value: values.cpu,
                hint: values.cpu_hint,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn says_how_long_is_left_in_seconds_then_minutes() {
        assert_eq!(left_text(0, None), None);
        assert_eq!(
            left_text(30_000, Some(42_000)).as_deref(),
            Some("About 12 s left")
        );
        assert_eq!(
            left_text(0, Some(200_000)).as_deref(),
            Some("About 4 min left")
        );
    }

    #[test]
    fn a_run_past_its_usual_time_says_so() {
        assert_eq!(
            left_text(50_000, Some(42_000)).as_deref(),
            Some("Taking longer than usual")
        );
    }

    #[test]
    fn progress_follows_the_usual_time_and_never_reaches_100_while_running() {
        assert_eq!(percent_done(0, 8, 21_000, Some(42_000)), 50);
        assert_eq!(percent_done(0, 8, 90_000, Some(42_000)), 99);
    }

    #[test]
    fn without_a_usual_time_progress_follows_the_finished_steps() {
        assert_eq!(percent_done(4, 8, 5_000, None), 50);
        assert_eq!(percent_done(8, 8, 5_000, None), 99);
        assert_eq!(percent_done(0, 0, 0, None), 0);
        assert_eq!(percent_done(4, 8, 5_000, Some(0)), 50);
    }

    #[test]
    fn the_tab_title_shows_progress_then_the_outcome() {
        assert_eq!(tab_title(None, 42, "Rust"), "42% · Rust benchmark");
        assert_eq!(
            tab_title(Some(&Terminal::Completed(None)), 99, "Rust"),
            "Done · Rust benchmark"
        );
        assert_eq!(
            tab_title(
                Some(&Terminal::Cancelled {
                    stage: None,
                    persisted: true
                }),
                10,
                "Rust"
            ),
            "Cancelled · Rust benchmark"
        );
    }

    #[test]
    fn the_hud_shows_dashes_before_the_first_sample() {
        let values = hud_values(None);

        assert_eq!(values.throughput, MISSING);
        assert_eq!(values.throughput_hint, "waiting for the workload");
    }
}
