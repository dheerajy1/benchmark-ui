use dioxus::prelude::*;

use super::{
    api::fetch_run,
    models::BenchmarkResult,
    sections::{ArtifactSection, HttpSection, ResourceSection, StepsSection},
};
use crate::{
    app::Route,
    components::{
        button_class, Badge, ButtonVariant, EmptyState, Icon, IconKind, LoadingPanel, PageHeader,
        StatTile, Tone,
    },
    features::{configurations::ConfigurationCard, connection::ApiErrorPanel},
    format::{
        format_bytes, format_duration_ms, format_latency_ms, format_per_second, format_percent,
        short_id, MISSING,
    },
};

// Headline numbers of a stored run
#[component]
fn Headline(result: BenchmarkResult) -> Element {
    let metrics = &result.metrics;

    let throughput = metrics.http_latency.as_ref().map_or_else(
        || MISSING.to_string(),
        |h| format_per_second(h.requests_per_second),
    );
    let p95 = metrics
        .http_latency
        .as_ref()
        .and_then(|h| h.p95_ms)
        .map_or_else(|| MISSING.to_string(), format_latency_ms);
    let memory = metrics.memory.as_ref().map_or_else(
        || MISSING.to_string(),
        |m| format_bytes(m.peak.rss_bytes as f64),
    );
    let cpu = metrics.cpu.as_ref().map_or_else(
        || MISSING.to_string(),
        |c| format_percent(c.average_percent),
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

// Full result of one run, fetched by id from GET /api/v1/benchmarks/{run_id}
#[component]
pub fn RunDetail(run_id: String) -> Element {
    let reactive_id = run_id.clone();
    let mut run = use_resource(use_reactive!(|(reactive_id,)| async move {
        fetch_run(&reactive_id).await
    }));

    let secondary = button_class(ButtonVariant::Secondary);
    let primary = button_class(ButtonVariant::Primary);
    let short = short_id(&run_id);

    let body: Element = match run() {
        None => rsx! {
            LoadingPanel { label: "Loading run" }
        },
        Some(Err(error)) if error.is_not_found() => rsx! {
            EmptyState {
                title: "Run not found",
                description: "The API has no run with this id. It may have been mistyped.",
                Link { to: Route::History {}, class: "{secondary}", "Back to runs" }
            }
        },
        Some(Err(error)) => rsx! {
            ApiErrorPanel { error, on_retry: move |_| run.restart() }
        },
        Some(Ok(result)) => {
            let runtime = result.discovery.runtime.clone();
            let metrics = result.metrics.clone();

            rsx! {
                div { class: "flex flex-col gap-(--space-4)",
                    div { class: "flex flex-wrap items-center gap-(--space-3) text-(length:--fs-xs) text-(color:--clr-text-muted)",
                        if let Some(runtime) = runtime {
                            Badge { label: runtime, tone: Tone::Accent }
                        }
                        span { class: "min-w-0 break-all font-(family-name:--font-code)", "{result.target.path}" }
                    }
                    Headline { result: result.clone() }
                    ConfigurationCard { run_id: run_id.clone() }
                    HttpSection { latency: metrics.http_latency.clone() }
                    ResourceSection { memory: metrics.memory.clone(), cpu: metrics.cpu.clone() }
                    StepsSection {
                        build_step: metrics.build_time.clone(),
                        startup_step: metrics.startup_time.clone(),
                    }
                    ArtifactSection { artifacts: metrics.artifact_size.clone() }
                }
            }
        }
    };

    rsx! {
        document::Title { "Run {short} · Benchmark" }
        PageHeader {
            title: "Run {short}",
            description: "Full measurements for a single stored benchmark run.",
            Link { to: Route::History {}, class: "{secondary}", "All runs" }
            Link {
                to: Route::Comparisons {
                    baseline: run_id.clone(),
                    comparison: String::new(),
                },
                class: "{primary}",
                Icon { kind: IconKind::Compare, class: "size-(--icon-sm)" }
                "Compare with another run"
            }
        }
        {body}
    }
}
