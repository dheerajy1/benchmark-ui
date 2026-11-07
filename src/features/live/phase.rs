use dioxus::prelude::*;

use super::{
    last_run::LastRun,
    live_chart::LiveChartCard,
    result_cards::{cards_for, PhaseId, ResultCards},
    state::{mode_title, workspace_series, LiveState, Stage, StageState},
};
use crate::{
    components::{Card, CardHeader, StatTile},
    features::{
        charts::ChartTone,
        submission::{last_for, StageAverages},
    },
    format::{
        format_bytes, format_duration_ms, format_percent, humanize_code, ValueFormat, MISSING,
    },
};

// Share of the last build time that has passed, 0 to 100, `None` when there is no last build
pub fn build_percent(elapsed_ms: u64, last_ms: Option<u64>) -> Option<u32> {
    let last = last_ms.filter(|last| *last > 0)?;

    Some(((elapsed_ms as f64 / last as f64 * 100.0) as u32).min(100))
}

// The finished phases as cards and the charts of the phase that runs now. Choosing a card shows
// the charts of that phase instead. While the view is paused everything comes from the snapshot
// taken at that moment.
#[component]
pub fn Charts(
    state: Signal<LiveState>,
    frozen: Signal<Option<LiveState>>,
    last_run: Option<LastRun>,
    averages: Option<StageAverages>,
    clock: Signal<u64>,
) -> Element {
    let mut opened = use_signal(|| None::<PhaseId>);

    let view = frozen
        .read()
        .clone()
        .unwrap_or_else(|| state.read().clone());
    let cards = cards_for(&view, last_run.as_ref());

    rsx! {
        div { class: "flex min-w-0 flex-col gap-(--space-4)",
            ResultCards {
                cards,
                selected: opened(),
                on_pick: move |id: PhaseId| {
                    let next = if *opened.peek() == Some(id) { None } else { Some(id) };

                    opened.set(next);
                },
            }
            PhaseCharts {
                view,
                last_run,
                averages,
                clock,
                opened: opened(),
            }
        }
    }
}

#[component]
fn PhaseCharts(
    view: LiveState,
    last_run: Option<LastRun>,
    averages: Option<StageAverages>,
    clock: Signal<u64>,
    opened: Option<PhaseId>,
) -> Element {
    match opened {
        Some(PhaseId::Workload) => {
            return rsx! {
                WorkloadPhase { view, last_run }
            };
        }
        Some(PhaseId::Mode(index)) => {
            return rsx! {
                WorkspacePhase {
                    view,
                    last_run,
                    index: Some(index),
                }
            };
        }
        None => {}
    }

    if matches!(view.stage(Stage::Build), StageState::Running { .. }) {
        let last_build = averages
            .as_ref()
            .and_then(|known| last_for(Stage::Build, known));

        return rsx! {
            BuildCard { view, last_build, clock }
        };
    }

    if matches!(view.stage(Stage::Workload), StageState::Running { .. })
        && !view.samples().is_empty()
    {
        return rsx! {
            WorkloadPhase { view, last_run }
        };
    }

    if matches!(view.stage(Stage::Workspaces), StageState::Running { .. })
        && !view.workspace_modes().is_empty()
    {
        return rsx! {
            WorkspacePhase {
                view,
                last_run,
                index: None,
            }
        };
    }

    rsx! {}
}

// Before the target runs there is nothing to chart, so the build shows how far it is
#[component]
fn BuildCard(view: LiveState, last_build: Option<u64>, clock: Signal<u64>) -> Element {
    let elapsed = view
        .stage_elapsed_ms(Stage::Build, clock())
        .unwrap_or_default();
    let percent = build_percent(elapsed, last_build);

    let line = match last_build {
        Some(last) => format!(
            "{} so far · the last build took {}",
            format_duration_ms(elapsed as f64),
            format_duration_ms(last as f64)
        ),
        None => format!(
            "{} so far · no earlier build to compare with",
            format_duration_ms(elapsed as f64)
        ),
    };

    rsx! {
        Card {
            CardHeader {
                title: "Building",
                subtitle: "Charts start when the target runs.",
            }
            div { class: "flex flex-col gap-(--space-2) p-(--space-4)",
                span { class: "text-(length:--fs-sm) tabular-nums text-(color:--clr-text)", "{line}" }
                if let Some(percent) = percent {
                    div {
                        role: "progressbar",
                        "aria-valuemin": "0",
                        "aria-valuemax": "100",
                        "aria-valuenow": "{percent}",
                        "aria-label": "Build progress against the last build",
                        class: "h-1.5 w-full overflow-hidden rounded-(--round-full) bg-(--clr-surface-raised)",
                        div {
                            class: "h-full rounded-(--round-full) bg-(--clr-accent) transition-[width] duration-500",
                            style: "width: {percent}%",
                        }
                    }
                }
            }
        }
    }
}

// Throughput, memory and CPU of the workload
#[component]
fn WorkloadPhase(view: LiveState, last_run: Option<LastRun>) -> Element {
    let running = matches!(view.stage(Stage::Workload), StageState::Running { .. });
    let last = last_run.unwrap_or_default();

    let memory = view.chart_series(|sample| sample.rss_bytes.map(|bytes| bytes as f64));
    let cpu = view.chart_series(|sample| sample.cpu_interval);
    let throughput = view.chart_series(|sample| Some(sample.requests_per_second));
    let subtitle = if view.chart_is_windowed() {
        "The most recent minute of samples, one every 500 ms."
    } else {
        "Samples since the workload started, one every 500 ms."
    };

    rsx! {
        div { class: "grid grid-cols-1 gap-(--space-4)",
            LiveChartCard {
                title: "Throughput",
                subtitle,
                points: throughput,
                format: ValueFormat::PerSecond,
                tone: ChartTone::Blue,
                last: last.throughput,
                last_label: "Last run average",
                live: running,
                compare_latest: true,
                higher_is_better: true,
            }
            LiveChartCard {
                title: "Memory (RSS)",
                subtitle,
                points: memory,
                format: ValueFormat::Bytes,
                tone: ChartTone::Cyan,
                last: last.memory_peak,
                last_label: "Last run peak",
                live: running,
                show_direction: true,
            }
            LiveChartCard {
                title: "CPU",
                subtitle,
                points: cpu,
                format: ValueFormat::Percent,
                tone: ChartTone::Amber,
                last: last.cpu_peak,
                last_label: "Last run peak",
                live: running,
            }
        }
    }
}

// The workspace mode that is sampled now, or the one opened from its card. A Docker mode keeps the
// Native readings of the same workspace as a grey dashed line.
#[component]
fn WorkspacePhase(view: LiveState, last_run: Option<LastRun>, index: Option<usize>) -> Element {
    let modes = view.workspace_modes();
    // A phase that was opened again from its card is over, whatever the run does now
    let running = !view.is_terminal() && index.is_none();

    let Some(live) = index
        .and_then(|position| modes.get(position))
        .or_else(|| modes.last())
        .cloned()
    else {
        return rsx! {};
    };

    let last = last_run
        .as_ref()
        .and_then(|known| known.mode(&live.workspace, &live.mode))
        .cloned();
    let sampling = running && live.phase == "sampling";

    let latest = live.points.last();

    let now = format!("{} · {}", live.workspace, mode_title(&live.mode));
    let phase = match live.phase.as_str() {
        "started" => "Building and starting, nothing to sample yet".to_string(),
        "sampling" => "Sampling memory and CPU".to_string(),
        _ => format!(
            "Finished · {}",
            humanize_code(live.status.as_deref().unwrap_or("done"))
        ),
    };
    let memory_now = latest
        .and_then(|point| point.rss_bytes)
        .map_or_else(|| MISSING.to_string(), |bytes| format_bytes(bytes as f64));
    let memory_hint = latest
        .and_then(|point| point.peak_rss_bytes)
        .map(|bytes| format!("peak {}", format_bytes(bytes as f64)))
        .unwrap_or_default();
    let cpu_now = latest
        .and_then(|point| point.cpu_interval.or(point.cpu_average))
        .map_or_else(|| MISSING.to_string(), format_percent);
    let cpu_hint = match latest.and_then(|point| point.cpu_average.zip(point.cpu_peak)) {
        Some((average, peak)) => format!(
            "average {} · peak {}",
            format_percent(average),
            format_percent(peak)
        ),
        None => String::new(),
    };
    let sampled = latest.map_or_else(
        || MISSING.to_string(),
        |point| format_duration_ms(point.elapsed_ms as f64),
    );

    let ghost: Vec<f64> = if live.mode == "docker" {
        modes
            .iter()
            .find(|mode| mode.workspace == live.workspace && mode.mode == "native")
            .map(|native| {
                native
                    .points
                    .iter()
                    .filter_map(|point| point.rss_bytes.map(|bytes| bytes as f64))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let memory = workspace_series(&live, |point| point.rss_bytes.map(|bytes| bytes as f64));
    let cpu = workspace_series(&live, |point| point.cpu_interval);

    rsx! {
        div { class: "flex flex-col gap-(--space-4)",
            if running {
                div {
                    class: "grid grid-cols-2 gap-(--space-3)",
                    "aria-label": "Live workspace measurements",
                    StatTile { label: "Workspace now", value: now.clone(), hint: phase }
                    StatTile { label: "Sampled for", value: sampled }
                    StatTile { label: "Memory (RSS)", value: memory_now, hint: memory_hint }
                    StatTile { label: "CPU", value: cpu_now, hint: cpu_hint }
                }
            }
            LiveChartCard {
                title: "{now} memory (RSS)",
                points: memory,
                format: ValueFormat::Bytes,
                tone: ChartTone::Cyan,
                last: last.as_ref().and_then(|entry| entry.memory_peak),
                last_label: "Last run peak",
                ghost,
                live: sampling,
                show_direction: true,
            }
            LiveChartCard {
                title: "{now} CPU",
                points: cpu,
                format: ValueFormat::Percent,
                tone: ChartTone::Amber,
                last: last.as_ref().and_then(|entry| entry.cpu_peak),
                last_label: "Last run peak",
                live: sampling,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_build_is_measured_against_the_last_one() {
        assert_eq!(build_percent(3_000, Some(6_000)), Some(50));
        assert_eq!(build_percent(9_000, Some(6_000)), Some(100));
        assert_eq!(build_percent(3_000, None), None);
        assert_eq!(build_percent(3_000, Some(0)), None);
    }
}
