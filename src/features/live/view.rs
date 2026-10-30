use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use super::{
    driver::{follow, FeedState},
    pipeline::Pipeline,
    state::{LiveState, Progress, SamplePoint, Terminal},
};
use crate::{
    app::Route,
    components::{
        button_class, Badge, ButtonVariant, Card, CardHeader, EmptyState, ErrorState, LoadingPanel,
        PageHeader, StatTile, Tone,
    },
    features::{
        charts::{ChartTone, TrendChart},
        run_detail::BenchmarkResult,
    },
    format::{
        format_bytes, format_duration_ms, format_latency_ms, format_number, format_per_second,
        format_percent, path_tail, short_id, ValueFormat, MISSING,
    },
};

const BANNER_WARNING: &str =
    "border-(color:--clr-warning) bg-(--clr-warning-soft) text-(color:--clr-warning)";
const BANNER_INFO: &str = "border-(color:--clr-info) bg-(--clr-info-soft) text-(color:--clr-info)";
const BANNER_DANGER: &str =
    "border-(color:--clr-danger) bg-(--clr-danger-soft) text-(color:--clr-danger)";

// Live view of one run, the page of /runs/live/:run_id
#[component]
pub fn LiveRun(run_id: String) -> Element {
    // A different run id remounts the page, so no state leaks from one run into another
    rsx! {
        LivePage { key: "{run_id}", run_id: run_id.clone() }
    }
}

#[component]
fn LivePage(run_id: String) -> Element {
    let initial_id = run_id.clone();
    let state = use_signal(move || LiveState::new(initial_id.clone()));
    let feed = use_signal(|| FeedState::Connecting);
    let mut clock = use_signal(|| 0u64);
    let restart = use_signal(|| 0u32);

    // Local time in milliseconds, it only fills the time between two events
    use_future(move || async move {
        loop {
            TimeoutFuture::new(1_000).await;

            clock += 1_000;
        }
    });

    // Follows the run until it ends. Changing `restart` starts it again from the last event.
    let follow_id = run_id.clone();
    use_future(move || {
        let _restart = restart();
        let run_id = follow_id.clone();

        async move { follow(run_id, state, feed, clock).await }
    });

    let short = short_id(&run_id);
    let secondary = button_class(ButtonVariant::Secondary);

    let waiting = {
        let snapshot = state.read();

        !snapshot.has_events() && !snapshot.is_terminal()
    };
    let lost = matches!(feed(), FeedState::Lost);

    let body: Element = if lost {
        rsx! {
            EmptyState {
                title: "This run is no longer available",
                description: "The API no longer has this run live and it was not stored. It may have failed, or the API may have restarted while it ran.",
                Link { to: Route::Submission {}, class: "{secondary}", "New benchmark" }
                Link { to: Route::History {}, class: "{secondary}", "All runs" }
            }
        }
    } else if waiting && matches!(feed(), FeedState::Connecting) {
        rsx! {
            LoadingPanel { label: "Connecting to the live run" }
        }
    } else {
        rsx! {
            div { class: "flex flex-col gap-(--space-4)",
                RunBar { state, feed, clock }
                FeedBanner { feed, restart }
                Outcome { state }
                if !state.read().is_recovered() {
                    Pipeline { state, clock }
                    LiveTiles { state }
                    LiveCharts { state }
                    EventLog { state }
                }
            }
        }
    };

    rsx! {
        document::Title { "Live run {short} · Benchmark" }
        PageHeader {
            title: "Live run {short}",
            description: "A benchmark that is running now. Its steps and measurements update as they happen, and the result is stored when it finishes.",
            Link { to: Route::History {}, class: "{secondary}", "All runs" }
        }
        {body}
    }
}

// Badge text and tone of the whole run
fn run_status(state: &LiveState, feed: &FeedState) -> (&'static str, Tone) {
    match state.terminal() {
        Some(Terminal::Completed(_)) => ("Completed", Tone::Success),
        Some(Terminal::Failed { .. }) => ("Failed", Tone::Danger),
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

// Status, target and elapsed time of the run
#[component]
fn RunBar(state: Signal<LiveState>, feed: Signal<FeedState>, clock: Signal<u64>) -> Element {
    let state = state.read();
    let feed = feed();

    let (status, tone) = run_status(&state, &feed);
    let elapsed = format_duration_ms(state.run_elapsed_ms(clock()) as f64);
    let target = state
        .plan()
        .map(|plan| (path_tail(&plan.path), plan.path.clone()));
    let short = short_id(state.run_id());

    rsx! {
        Card {
            div { class: "flex flex-wrap items-center gap-x-(--space-4) gap-y-(--space-2) px-(--space-4) py-(--space-3)",
                Badge { label: status, tone }
                span { class: "font-(family-name:--font-code) text-(length:--fs-sm) text-(color:--clr-text)",
                    "{short}"
                }
                if let Some((name , full)) = target {
                    span {
                        class: "min-w-0 truncate text-(length:--fs-sm) text-(color:--clr-text-muted)",
                        title: "{full}",
                        "{name}"
                    }
                }
                span { class: "ml-auto text-(length:--fs-sm) tabular-nums text-(color:--clr-text-muted)",
                    "Elapsed {elapsed}"
                }
            }
        }
    }
}

// How the page is getting its events, silent while the live stream works
#[component]
fn FeedBanner(feed: Signal<FeedState>, restart: Signal<u32>) -> Element {
    let mut restart = restart;
    let secondary = button_class(ButtonVariant::Secondary);

    let (class, text, can_retry) = match feed() {
        FeedState::Reconnecting { attempt, reason } => (
            BANNER_WARNING,
            format!("Live connection lost ({reason}), reconnecting, attempt {attempt}. Nothing is missed: the stream resumes after the last event."),
            false,
        ),
        FeedState::Polling => (
            BANNER_INFO,
            "A live connection is not possible from here, so this page checks for news every 2 seconds instead.".to_string(),
            false,
        ),
        FeedState::TooManyViewers => (
            BANNER_WARNING,
            "This run already has the most viewers it allows. Trying again shortly.".to_string(),
            false,
        ),
        FeedState::Offline(detail) => (
            BANNER_DANGER,
            format!("The API cannot be reached. {detail}"),
            true,
        ),
        FeedState::Connecting | FeedState::Live | FeedState::Finished | FeedState::Lost => {
            return rsx! {};
        }
    };

    rsx! {
        div {
            class: "flex flex-wrap items-center justify-between gap-(--space-3) rounded-(--round-lg) border px-(--space-4) py-(--space-3) text-(length:--fs-sm) {class}",
            role: "status",
            span { class: "min-w-0 break-words", "{text}" }
            if can_retry {
                button {
                    r#type: "button",
                    class: "{secondary}",
                    onclick: move |_| restart += 1,
                    "Try again"
                }
            }
        }
    }
}

// Headline numbers of a stored result, the same as the run detail page
#[component]
fn FinalTiles(result: BenchmarkResult) -> Element {
    let metrics = &result.metrics;

    let throughput = metrics.http_latency.as_ref().map_or_else(
        || MISSING.to_string(),
        |latency| format_per_second(latency.requests_per_second),
    );
    let p95 = metrics
        .http_latency
        .as_ref()
        .and_then(|latency| latency.p95_ms)
        .map_or_else(|| MISSING.to_string(), format_latency_ms);
    let memory = metrics.memory.as_ref().map_or_else(
        || MISSING.to_string(),
        |memory| format_bytes(memory.peak.rss_bytes as f64),
    );
    let cpu = metrics.cpu.as_ref().map_or_else(
        || MISSING.to_string(),
        |cpu| format_percent(cpu.average_percent),
    );
    let startup = metrics
        .startup_time
        .duration_ms
        .map_or_else(|| MISSING.to_string(), |ms| format_duration_ms(ms as f64));
    let build = metrics
        .build_time
        .duration_ms
        .map_or_else(|| MISSING.to_string(), |ms| format_duration_ms(ms as f64));

    rsx! {
        div { class: "grid grid-cols-2 gap-(--space-3) md:grid-cols-3 xl:grid-cols-6",
            StatTile { label: "Throughput", value: throughput }
            StatTile { label: "p95 latency", value: p95 }
            StatTile { label: "Peak memory", value: memory }
            StatTile { label: "CPU average", value: cpu }
            StatTile { label: "Startup", value: startup }
            StatTile { label: "Build", value: build }
        }
    }
}

// The end of the run: the stored result with a button to open it, or the reason it failed.
// There is no automatic redirect, the final numbers stay on screen until the user moves on.
#[component]
fn Outcome(state: Signal<LiveState>) -> Element {
    let secondary = button_class(ButtonVariant::Secondary);
    let primary = button_class(ButtonVariant::Primary);
    let nav = use_navigator();

    let (terminal, run_id, recovered) = {
        let state = state.read();

        (
            state.terminal().cloned(),
            state.run_id().to_string(),
            state.is_recovered(),
        )
    };

    match terminal {
        None => rsx! {},
        Some(Terminal::Completed(result)) => {
            let short = short_id(&run_id);
            let subtitle = if recovered {
                "The live record had expired, this is the stored result."
            } else {
                "The result is stored and appears in the run history."
            };

            rsx! {
                Card {
                    CardHeader { title: "Benchmark completed", subtitle }
                    div { class: "flex flex-col gap-(--space-4) p-(--space-4)",
                        if let Some(result) = result {
                            FinalTiles { result: *result }
                        }
                        div { class: "flex flex-wrap items-center gap-(--space-2)",
                            Link {
                                to: Route::RunDetail {
                                    run_id: run_id.clone(),
                                },
                                class: "{primary}",
                                "View run {short}"
                            }
                            Link { to: Route::Submission {}, class: "{secondary}", "Run another benchmark" }
                        }
                    }
                }
            }
        }
        Some(Terminal::Failed {
            code,
            message,
            stage,
        }) => {
            let mut detail = vec!["The result was not stored.".to_string()];

            if let Some(stage) = stage {
                detail.insert(0, format!("Stopped in the {stage} step."));
            }

            if let Some(code) = code {
                detail.push(code);
            }

            rsx! {
                ErrorState {
                    title: "The benchmark failed",
                    message,
                    detail: detail.join(" "),
                    on_retry: move |_| {
                        let _ = nav.push(Route::Submission {});
                    },
                }
            }
        }
    }
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

// The latest measurements of the running workload, nothing is shown before a sample exists
#[component]
fn LiveTiles(state: Signal<LiveState>) -> Element {
    let Some(sample) = state.read().latest_sample().cloned() else {
        return rsx! {};
    };

    let memory = sample
        .rss_bytes
        .map_or_else(|| MISSING.to_string(), |bytes| format_bytes(bytes as f64));
    let memory_hint = sample
        .peak_rss_bytes
        .map(|bytes| format!("peak {}", format_bytes(bytes as f64)))
        .unwrap_or_default();
    let (cpu, cpu_hint) = cpu_text(&sample);
    let throughput = format_per_second(sample.requests_per_second);
    let completed = format_number(sample.completed as f64, 0);
    let failed_hint = if sample.failed > 0 {
        format!("{} failed", format_number(sample.failed as f64, 0))
    } else {
        "none failed".to_string()
    };
    let workload_time = format_duration_ms(sample.elapsed_ms as f64);
    let progress = match &sample.progress {
        Progress::Requests { .. } => String::new(),
        Progress::Duration { total_ms, .. } => {
            format!("of {}", format_duration_ms(*total_ms as f64))
        }
    };

    rsx! {
        div {
            class: "grid grid-cols-2 gap-(--space-3) md:grid-cols-3 xl:grid-cols-5",
            "aria-label": "Live measurements",
            StatTile { label: "Memory (RSS)", value: memory, hint: memory_hint }
            StatTile { label: "CPU", value: cpu, hint: cpu_hint }
            StatTile { label: "Throughput", value: throughput, hint: "running average" }
            StatTile { label: "Requests done", value: completed, hint: failed_hint }
            StatTile { label: "Workload time", value: workload_time, hint: progress }
        }
    }
}

// Live charts of the last minute of samples
#[component]
fn LiveCharts(state: Signal<LiveState>) -> Element {
    let (memory, cpu, throughput, windowed, count) = {
        let state = state.read();

        (
            state.chart_series(|sample| sample.rss_bytes.map(|bytes| bytes as f64)),
            state.chart_series(|sample| sample.cpu_interval),
            state.chart_series(|sample| Some(sample.requests_per_second)),
            state.chart_is_windowed(),
            state.samples().len(),
        )
    };

    if count == 0 {
        return rsx! {};
    }

    let subtitle = if windowed {
        "The most recent minute of samples, one every 500 ms."
    } else {
        "Samples since the workload started, one every 500 ms."
    };

    rsx! {
        div { class: "grid gap-(--space-4) lg:grid-cols-3",
            Card {
                CardHeader { title: "Memory (RSS)", subtitle }
                div { class: "p-(--space-4)",
                    TrendChart {
                        title: "Memory (RSS)",
                        points: memory,
                        format: ValueFormat::Bytes,
                        tone: ChartTone::Cyan,
                    }
                }
            }
            Card {
                CardHeader { title: "CPU", subtitle }
                div { class: "p-(--space-4)",
                    TrendChart {
                        title: "CPU",
                        points: cpu,
                        format: ValueFormat::Percent,
                        tone: ChartTone::Amber,
                    }
                }
            }
            Card {
                CardHeader { title: "Throughput", subtitle }
                div { class: "p-(--space-4)",
                    TrendChart {
                        title: "Throughput",
                        points: throughput,
                        format: ValueFormat::PerSecond,
                        tone: ChartTone::Blue,
                    }
                }
            }
        }
    }
}

// What happened, newest first so it never needs scrolling to follow
#[component]
fn EventLog(state: Signal<LiveState>) -> Element {
    let lines: Vec<(String, String, &'static str)> = state
        .read()
        .log()
        .iter()
        .rev()
        .map(|line| {
            (
                format!("+{}", format_duration_ms(line.t_ms as f64)),
                line.text.clone(),
                line.tone.text_class(),
            )
        })
        .collect();

    rsx! {
        Card {
            CardHeader { title: "Events", subtitle: "Newest first, times are since the run was accepted." }
            ol { class: "flex max-h-72 flex-col gap-(--space-1) overflow-y-auto px-(--space-4) py-(--space-3)",
                for (index , (time , text , tone_class)) in lines.into_iter().enumerate() {
                    li {
                        key: "{index}",
                        class: "flex items-baseline gap-(--space-3) text-(length:--fs-sm)",
                        span { class: "w-20 shrink-0 font-(family-name:--font-code) text-(length:--fs-xs) tabular-nums text-(color:--clr-text-faint)",
                            "{time}"
                        }
                        span { class: "min-w-0 break-words {tone_class}", "{text}" }
                    }
                }
            }
        }
    }
}
