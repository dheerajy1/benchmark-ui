use dioxus::prelude::*;

use super::{
    chips::{chips_for, Chip},
    last_run::LastRun,
    live_chart::LiveChartCard,
    state::{mode_title, workspace_series, LiveState, Stage, StageState, WorkspaceLive},
};
use crate::{
    components::{Badge, Card, Tone},
    features::{
        charts::ChartTone,
        submission::{last_for, StageAverages},
    },
    format::{format_duration_ms, ValueFormat},
};

// Share of the last build time that has passed, 0 to 100, `None` when there is no last build
pub fn build_percent(elapsed_ms: u64, last_ms: Option<u64>) -> Option<u32> {
    let last = last_ms.filter(|last| *last > 0)?;

    Some(((elapsed_ms as f64 / last as f64 * 100.0) as u32).min(100))
}

// What the groups draw from: the snapshot taken when the view was paused, else the live state
fn view_of(state: Signal<LiveState>, frozen: Signal<Option<LiveState>>) -> LiveState {
    frozen
        .read()
        .clone()
        .unwrap_or_else(|| state.read().clone())
}

// One group of the run: its name, where it ran, how long it took and the numbers it states,
// with its charts below. A group that has not started is the same card, dimmed.
#[component]
fn GroupCard(
    #[props(into)] title: String,
    #[props(into, default)] note: String,
    #[props(into, default)] duration: String,
    #[props(default)] live: bool,
    #[props(default)] dimmed: bool,
    #[props(default)] chips: Vec<Chip>,
    children: Element,
) -> Element {
    let tone = if dimmed { "opacity-60" } else { "" };

    rsx! {
        Card { class: "flex flex-1 flex-col gap-(--space-3) p-(--space-4) {tone}",
            div { class: "flex items-baseline justify-between gap-(--space-3)",
                div { class: "flex min-w-0 items-baseline gap-(--space-2)",
                    h3 { class: "text-(length:--fs-md) font-(weight:--fw-semibold) text-(color:--clr-text)",
                        "{title}"
                    }
                    if !note.is_empty() {
                        span { class: "truncate text-(length:--fs-xs) text-(color:--clr-text-faint)",
                            "{note}"
                        }
                    }
                }
                if live {
                    Badge { label: "Live", tone: Tone::Accent }
                } else if !duration.is_empty() {
                    span { class: "shrink-0 text-(length:--fs-xs) tabular-nums text-(color:--clr-text-muted)",
                        "{duration}"
                    }
                }
            }
            if !chips.is_empty() {
                div { class: "flex flex-wrap gap-(--space-1)",
                    for chip in chips {
                        Badge { key: "{chip.text}", label: chip.text.clone(), tone: chip.tone }
                    }
                }
            }
            {children}
        }
    }
}

// The space of a chart that does not exist yet, so nothing moves when it arrives
#[component]
fn Waiting(#[props(into)] text: String) -> Element {
    rsx! {
        div { class: "flex min-h-40 flex-1 items-center justify-center rounded-(--round-md) border border-dashed border-(color:--clr-border-strong) px-(--space-4) text-center text-(length:--fs-sm) text-(color:--clr-text-faint)",
            "{text}"
        }
    }
}

// Before the target runs there is nothing to chart, so the build shows how far it is
#[component]
fn BuildProgress(view: LiveState, last_build: Option<u64>, clock: Signal<u64>) -> Element {
    let elapsed = view
        .stage_elapsed_ms(Stage::Build, clock())
        .unwrap_or_default();
    let percent = build_percent(elapsed, last_build);

    let line = match last_build {
        Some(last) => format!(
            "Building: {} so far · the last build took {}",
            format_duration_ms(elapsed as f64),
            format_duration_ms(last as f64)
        ),
        None => format!(
            "Building: {} so far · no earlier build to compare with",
            format_duration_ms(elapsed as f64)
        ),
    };

    rsx! {
        div { class: "flex min-h-40 flex-1 flex-col justify-center gap-(--space-2) rounded-(--round-md) border border-dashed border-(color:--clr-border-strong) p-(--space-4)",
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

// The workload: throughput on the full width, memory and CPU side by side under it. It also
// stands in for the build before the workload starts, and says so when there is no workload.
#[component]
pub fn WorkloadGroup(
    state: Signal<LiveState>,
    frozen: Signal<Option<LiveState>>,
    last_run: Option<LastRun>,
    averages: Option<StageAverages>,
    clock: Signal<u64>,
) -> Element {
    let view = view_of(state, frozen);
    let running = matches!(view.stage(Stage::Workload), StageState::Running { .. });
    let last = last_run.unwrap_or_default();

    let duration = match view.stage(Stage::Workload) {
        StageState::Done { duration_ms, .. } => duration_ms
            .map(|ms| format_duration_ms(ms as f64))
            .unwrap_or_default(),
        _ => String::new(),
    };

    let chips = match view.latest_sample() {
        Some(newest) => {
            let peak = view
                .samples()
                .iter()
                .filter_map(|sample| sample.rss_bytes)
                .max()
                .map(|bytes| bytes as f64);

            chips_for(
                peak,
                newest.cpu_average,
                Some((newest.requests_per_second, newest.completed)),
                last.memory_peak,
            )
        }
        None => Vec::new(),
    };

    let building = matches!(view.stage(Stage::Build), StageState::Running { .. });
    let has_samples = !view.samples().is_empty();
    let waiting_text = if view.workload_requested() == Some(false) {
        "No workload was requested"
    } else if view.is_terminal() {
        "The workload did not run"
    } else {
        "Starts after the build and the baseline"
    };

    let last_build = averages
        .as_ref()
        .and_then(|known| last_for(Stage::Build, known));

    let memory = view.chart_series(|sample| sample.rss_bytes.map(|bytes| bytes as f64));
    let cpu = view.chart_series(|sample| sample.cpu_interval);
    let throughput = view.chart_series(|sample| Some(sample.requests_per_second));

    rsx! {
        GroupCard {
            title: "Workload",
            duration,
            live: running,
            dimmed: !has_samples && !building,
            chips,
            if has_samples {
                div { class: "flex flex-1 flex-col gap-(--space-4)",
                    LiveChartCard {
                        bare: true,
                        title: "Throughput",
                        points: throughput,
                        format: ValueFormat::PerSecond,
                        tone: ChartTone::Blue,
                        last: last.throughput,
                        last_label: "Last run average",
                        live: running,
                        compare_latest: true,
                        higher_is_better: true,
                    }
                    div { class: "grid grid-cols-1 gap-(--space-4) md:grid-cols-2",
                        LiveChartCard {
                            bare: true,
                            title: "Memory (RSS)",
                            points: memory,
                            format: ValueFormat::Bytes,
                            tone: ChartTone::Cyan,
                            last: last.memory_peak,
                            last_label: "Last run peak",
                            live: running,
                            show_direction: true,
                        }
                        LiveChartCard {
                            bare: true,
                            title: "CPU",
                            points: cpu,
                            format: ValueFormat::Percent,
                            tone: ChartTone::Amber,
                            last: last.cpu_peak,
                            last_label: "Last run peak",
                            live: running,
                        }
                    }
                }
            } else if building {
                BuildProgress { view: view.clone(), last_build, clock }
            } else {
                Waiting { text: waiting_text }
            }
        }
    }
}

// Native and Docker next to each other. A mode that has not started is a dimmed card of the
// same size, so the row never changes shape when its readings arrive.
#[component]
pub fn ModeGroups(
    state: Signal<LiveState>,
    frozen: Signal<Option<LiveState>>,
    last_run: Option<LastRun>,
) -> Element {
    let view = view_of(state, frozen);
    let modes = view.workspace_modes();

    if view.workspaces_requested() == Some(false) {
        return rsx! {
            Waiting { text: "No workspace was requested" }
        };
    }

    let terminal = view.is_terminal();

    // The modes a workspace run goes through that have not reported yet
    let missing: Vec<&'static str> = if view.workspaces_requested() == Some(true) && !terminal {
        ["native", "docker"]
            .into_iter()
            .filter(|mode| !modes.iter().any(|known| known.mode == *mode))
            .collect()
    } else {
        Vec::new()
    };

    let cards: Vec<(WorkspaceLive, Vec<f64>)> = modes
        .iter()
        .map(|mode| (mode.clone(), ghost_of(mode, &modes)))
        .collect();

    rsx! {
        div { class: "grid flex-1 grid-cols-1 gap-(--space-4) md:grid-cols-2",
            for (index , (mode , ghost)) in cards.into_iter().enumerate() {
                ModeCard {
                    key: "{index}",
                    mode,
                    ghost,
                    last_run: last_run.clone(),
                    terminal,
                }
            }
            for name in missing {
                GroupCard { key: "{name}", title: mode_title(name), dimmed: true,
                    Waiting { text: "Starts after the workload" }
                }
            }
        }
    }
}

// The Native memory readings of the same workspace, drawn as a grey dashed line behind Docker
fn ghost_of(live: &WorkspaceLive, modes: &[WorkspaceLive]) -> Vec<f64> {
    if live.mode != "docker" {
        return Vec::new();
    }

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
}

#[component]
fn ModeCard(
    mode: WorkspaceLive,
    ghost: Vec<f64>,
    last_run: Option<LastRun>,
    terminal: bool,
) -> Element {
    let last = last_run
        .as_ref()
        .and_then(|known| known.mode(&mode.workspace, &mode.mode));
    let last_memory = last.and_then(|entry| entry.memory_peak);
    let last_cpu = last.and_then(|entry| entry.cpu_peak);

    let sampling = !terminal && mode.phase == "sampling";
    let newest = mode.points.last();

    let duration = if mode.phase == "finished" {
        newest
            .map(|point| format_duration_ms(point.elapsed_ms as f64))
            .unwrap_or_default()
    } else {
        String::new()
    };

    let peak = mode
        .points
        .iter()
        .filter_map(|point| point.rss_bytes)
        .max()
        .map(|bytes| bytes as f64);
    let chips = match newest {
        Some(point) => chips_for(peak, point.cpu_average, None, last_memory),
        None => Vec::new(),
    };

    let memory = workspace_series(&mode, |point| point.rss_bytes.map(|bytes| bytes as f64));
    let cpu = workspace_series(&mode, |point| point.cpu_interval);
    let waiting_text = if terminal {
        "No readings were taken"
    } else {
        "Building and starting, nothing to sample yet"
    };

    rsx! {
        GroupCard {
            title: mode_title(&mode.mode),
            note: mode.workspace.clone(),
            duration,
            live: sampling,
            chips,
            if mode.points.is_empty() {
                Waiting { text: waiting_text }
            } else {
                div { class: "grid flex-1 grid-cols-1 gap-(--space-4) lg:grid-cols-2",
                    LiveChartCard {
                        bare: true,
                        title: "Memory (RSS)",
                        points: memory,
                        format: ValueFormat::Bytes,
                        tone: ChartTone::Cyan,
                        last: last_memory,
                        last_label: "Last run peak",
                        ghost,
                        live: sampling,
                        show_direction: true,
                    }
                    LiveChartCard {
                        bare: true,
                        title: "CPU",
                        points: cpu,
                        format: ValueFormat::Percent,
                        tone: ChartTone::Amber,
                        last: last_cpu,
                        last_label: "Last run peak",
                        live: sampling,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::live::state::WorkspacePoint;

    #[test]
    fn the_build_is_measured_against_the_last_one() {
        assert_eq!(build_percent(3_000, Some(6_000)), Some(50));
        assert_eq!(build_percent(9_000, Some(6_000)), Some(100));
        assert_eq!(build_percent(3_000, None), None);
        assert_eq!(build_percent(3_000, Some(0)), None);
    }

    fn mode(workspace: &str, mode: &str, rss: &[u64]) -> WorkspaceLive {
        WorkspaceLive {
            workspace: workspace.to_string(),
            mode: mode.to_string(),
            phase: "sampling".to_string(),
            status: None,
            points: rss
                .iter()
                .map(|bytes| WorkspacePoint {
                    elapsed_ms: 0,
                    rss_bytes: Some(*bytes),
                    peak_rss_bytes: None,
                    cpu_interval: None,
                    cpu_average: None,
                    cpu_peak: None,
                })
                .collect(),
        }
    }

    #[test]
    fn docker_keeps_the_native_readings_of_its_workspace_as_a_ghost() {
        let modes = vec![
            mode("api", "native", &[10, 20]),
            mode("other", "native", &[99]),
            mode("api", "docker", &[30]),
        ];

        assert_eq!(ghost_of(&modes[2], &modes), vec![10.0, 20.0]);
        assert!(ghost_of(&modes[0], &modes).is_empty());
    }
}
