use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use super::{
    api::fetch_run,
    models::BenchmarkResult,
    sections::{ArtifactSection, HttpSection, ResourceSection, StepsSection},
    skeleton::RunDetailSkeleton,
    trend::{column_values, tile_values, window_ending_at},
    workspaces::WorkspacesSection,
};
use crate::{
    app::Route,
    components::{
        button_class, copy_text, use_section_spy, ButtonVariant, ErrorKind, ErrorState, Icon,
        IconKind, RuntimeTag, SectionBar, SectionBlock, StatusBadge,
    },
    features::{
        charts::{ChartTone, TrendTile},
        configurations::ConfigurationCard,
        connection::ApiErrorPanel,
        history::{fetch_page, Column, HistoryFilters},
    },
    format::{format_timestamp, humanize_code, path_tail, relative_time, short_id, ValueFormat},
};

// How long the copy button shows "Copied"
const COPIED_MS: u32 = 1_400;
const HISTORY_LIMIT: u32 = 100;

// One tile of the overview: the column it reads and how it is drawn
type TileSpec = (Column, ValueFormat, ChartTone, bool);

const TILES: [TileSpec; 6] = [
    (
        Column::Throughput,
        ValueFormat::PerSecond,
        ChartTone::Blue,
        false,
    ),
    (Column::P95, ValueFormat::LatencyMs, ChartTone::Amber, true),
    (
        Column::PeakMemory,
        ValueFormat::Bytes,
        ChartTone::Cyan,
        true,
    ),
    (
        Column::CpuAverage,
        ValueFormat::Percent,
        ChartTone::Violet,
        true,
    ),
    (
        Column::Startup,
        ValueFormat::DurationMs,
        ChartTone::Teal,
        true,
    ),
    (
        Column::BuildTime,
        ValueFormat::DurationMs,
        ChartTone::Slate,
        true,
    ),
];

// Titles of the sections that the page shows for a run
fn section_titles(has_workspaces: bool) -> Vec<&'static str> {
    let mut titles = vec!["Overview", "Latency", "Resources", "Build and size"];

    if has_workspaces {
        titles.push("Subscriber");
    }

    titles.push("Configuration");
    titles
}

// Red box under the header when the run did not finish
#[component]
fn FailureBanner(status: String, stage: Option<String>, message: Option<String>) -> Element {
    let headline = match (status.as_str(), stage.as_deref()) {
        ("cancelled", Some(stage)) => format!("Cancelled during {}", humanize_code(stage)),
        ("cancelled", None) => "Cancelled before it finished".to_string(),
        (_, Some(stage)) => format!("Failed during {}", humanize_code(stage)),
        (_, None) => "Failed before it finished".to_string(),
    };

    rsx! {
        div {
            role: "alert",
            class: "flex items-start gap-(--space-3) rounded-(--round-lg) bg-(--clr-danger-soft) p-(--space-4) text-(color:--clr-danger-text)",
            Icon { kind: IconKind::Alert, class: "mt-px size-(--icon-md)" }
            div { class: "min-w-0",
                p { class: "text-(length:--fs-base) font-(weight:--fw-medium)", "{headline}" }
                if let Some(message) = message {
                    p { class: "mt-(--space-1) text-(length:--fs-sm) break-words", "{message}" }
                }
            }
        }
    }
}

// Header of the page: name, runtime, when, id and the two actions
#[component]
fn RunHeader(run_id: String, result: BenchmarkResult, created_at: Option<String>) -> Element {
    let mut copied = use_signal(|| false);
    let secondary = button_class(ButtonVariant::Secondary);
    let short = short_id(&run_id);

    let name = if result.display_name.trim().is_empty() {
        path_tail(&result.target.path)
    } else {
        result.display_name.clone()
    };
    let when = created_at.map(|created_at| {
        format!(
            "{} · {}",
            relative_time(&created_at, js_sys::Date::now() as i64),
            format_timestamp(&created_at)
        )
    });
    let runtime = result.discovery.runtime.clone();
    let finished = result.status == "done";
    let copy_id = run_id.clone();
    let copy_class = if copied() {
        "text-(color:--clr-success-text)"
    } else {
        "text-(color:--clr-text-muted) hover:text-(color:--clr-text)"
    };

    rsx! {
        header { class: "flex flex-col gap-(--space-3) pb-(--space-4) lg:flex-row lg:items-start lg:justify-between",
            div { class: "min-w-0 flex flex-col gap-(--space-2)",
                div { class: "flex flex-wrap items-center gap-(--space-2)",
                    h1 { class: "min-w-0 text-(length:--fs-xl) font-(weight:--fw-semibold) break-words text-(color:--clr-text)",
                        "{name}"
                    }
                    if let Some(runtime) = runtime {
                        RuntimeTag { runtime }
                    }
                    if !finished {
                        StatusBadge { status: result.status.clone() }
                    }
                }
                div { class: "flex flex-wrap items-center gap-x-(--space-2) gap-y-(--space-1) text-(length:--fs-xs) text-(color:--clr-text-muted)",
                    if let Some(when) = when {
                        span { "{when}" }
                        span { "aria-hidden": "true", "·" }
                    }
                    button {
                        r#type: "button",
                        class: "cursor-pointer font-(family-name:--font-code) transition-colors {copy_class}",
                        title: "Copy the full run id",
                        "aria-label": "Copy run id {run_id}",
                        onclick: move |_| {
                            copy_text(&copy_id);
                            copied.set(true);
                            spawn(async move {
                                TimeoutFuture::new(COPIED_MS).await;
                                copied.set(false);
                            });
                        },
                        if copied() {
                            "Copied"
                        } else {
                            "{short}"
                        }
                    }
                    span { "aria-hidden": "true", "·" }
                    span { class: "min-w-0 break-all font-(family-name:--font-code)", "{result.target.path}" }
                }
            }
            div { class: "flex flex-wrap items-center gap-(--space-2)",
                Link {
                    to: Route::SubmissionFrom {
                        run_id: run_id.clone(),
                    },
                    class: "group/action {secondary}",
                    Icon {
                        kind: IconKind::Refresh,
                        class: "size-(--icon-sm) transition-transform duration-300 group-hover/action:-rotate-45",
                    }
                    "Run again"
                }
                Link {
                    to: Route::Comparisons {
                        baseline: run_id.clone(),
                        comparison: String::new(),
                    },
                    class: "group/action {secondary}",
                    Icon {
                        kind: IconKind::Compare,
                        class: "size-(--icon-sm) transition-transform duration-300 group-hover/action:rotate-12",
                    }
                    "Compare with…"
                }
            }
        }
    }
}

// Everything under the header of a loaded run
#[component]
fn RunBody(run_id: String, result: BenchmarkResult) -> Element {
    let target_path = result.target.path.clone();
    let history = use_resource(use_reactive!(|(target_path,)| async move {
        fetch_page(
            &HistoryFilters {
                target_path,
                ..HistoryFilters::default()
            },
            None,
            HISTORY_LIMIT,
        )
        .await
    }));

    let runs = match history() {
        Some(Ok(page)) => page.items,
        _ => Vec::new(),
    };
    let created_at = runs
        .iter()
        .find(|run| run.run_id == run_id)
        .map(|run| run.created_at.clone());
    let window = window_ending_at(&runs, &run_id);

    let metrics = result.metrics.clone();
    let has_workspaces = !metrics.workspaces.is_empty();
    let titles = section_titles(has_workspaces);
    let configuration_index = titles.len() - 1;

    let spy = use_section_spy();

    let tiles = TILES.map(|(column, format, tone, lower_is_better)| {
        let values = tile_values(&window, &result, column);
        rsx! {
            TrendTile {
                key: "{column.label()}",
                label: column.label(),
                values,
                format,
                tone,
                lower_is_better,
            }
        }
    });

    let series = |column: Column| column_values(&window, column);

    rsx! {
        if result.status != "done" {
            div { class: "pb-(--space-4)",
                FailureBanner {
                    status: result.status.clone(),
                    stage: result.stopped_stage.clone(),
                    message: result.failure_message.clone(),
                }
            }
        }
        RunHeader {
            run_id: run_id.clone(),
            result: result.clone(),
            created_at,
        }
        SectionBar { titles, spy }
        div { class: "flex flex-col gap-(--space-6) pt-(--space-4)",
            SectionBlock { index: 0, spy, title: "Overview",
                div { class: "grid grid-cols-1 gap-(--space-3) sm:grid-cols-2 xl:grid-cols-3",
                    for tile in tiles {
                        {tile}
                    }
                }
            }
            SectionBlock { index: 1, spy, title: "Latency",
                HttpSection { latency: metrics.http_latency.clone() }
            }
            SectionBlock { index: 2, spy, title: "Resources",
                ResourceSection { memory: metrics.memory.clone(), cpu: metrics.cpu.clone() }
            }
            SectionBlock { index: 3, spy, title: "Build and size",
                StepsSection {
                    build_step: metrics.build_time.clone(),
                    startup_step: metrics.startup_time.clone(),
                    build_trend: series(Column::BuildTime),
                    startup_trend: series(Column::Startup),
                }
                ArtifactSection {
                    artifacts: metrics.artifact_size.clone(),
                    trend: series(Column::BuildOutput),
                }
            }
            if has_workspaces {
                SectionBlock { index: 4, spy, title: "Subscriber",
                    WorkspacesSection {
                        workspaces: metrics.workspaces.clone(),
                        image_trend: series(Column::ImageSize),
                    }
                }
            }
            SectionBlock { index: configuration_index, spy, title: "Configuration",
                ConfigurationCard { run_id: run_id.clone(), run: result.clone() }
            }
        }
    }
}

// Full result of one run, fetched by id from GET /api/v1/benchmarks/{run_id}
#[component]
pub fn RunDetail(run_id: String) -> Element {
    let reactive_id = run_id.clone();
    let mut run = use_resource(use_reactive!(|(reactive_id,)| async move {
        fetch_run(&reactive_id).await
    }));

    let secondary = button_class(ButtonVariant::Secondary);
    let short = short_id(&run_id);

    let body: Element = match run() {
        None => rsx! {
            RunDetailSkeleton {}
        },
        Some(Err(error)) if error.is_not_found() => rsx! {
            ErrorState {
                title: "This run doesn't exist",
                message: "The API has no run with the id {short}. It may have been mistyped.",
                kind: ErrorKind::Grey,
                retryable: false,
                actions: Some(rsx! {
                    Link { to: Route::runs(), class: "{secondary}", "All runs" }
                    Link { to: Route::new_benchmark(), class: "{secondary}", "New benchmark" }
                }),
                on_retry: move |_| {},
            }
        },
        Some(Err(error)) => rsx! {
            ApiErrorPanel { error, on_retry: move |_| run.restart() }
        },
        Some(Ok(result)) => rsx! {
            RunBody { key: "{run_id}", run_id: run_id.clone(), result }
        },
    };

    rsx! {
        document::Title { "Run {short} · Benchmark" }
        {body}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_the_subscriber_section_only_with_workspaces() {
        assert_eq!(section_titles(false).len(), 5);
        assert_eq!(section_titles(true).len(), 6);
        assert_eq!(section_titles(true)[4], "Subscriber");
        assert_eq!(section_titles(true)[5], "Configuration");
    }
}
