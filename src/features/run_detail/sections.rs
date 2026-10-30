use dioxus::prelude::*;

use super::models::{ArtifactSize, Cpu, HttpLatency, Memory, StepResult};
use crate::{
    components::{Card, CardHeader, Definition, EmptyState, StatusBadge},
    features::charts::{BarChart, BarGroup, BarSeries, ChartTone},
    format::{
        format_bytes, format_duration_ms, format_number, format_timestamp, ValueFormat, MISSING,
    },
};

fn single_series(name: &str, tone: ChartTone) -> Vec<BarSeries> {
    vec![BarSeries {
        name: name.to_string(),
        tone,
    }]
}

fn group(label: &str, value: Option<f64>) -> BarGroup {
    BarGroup {
        label: label.to_string(),
        values: vec![value],
    }
}

// Request counts and latency percentiles of the HTTP workload
#[component]
pub fn HttpSection(latency: Option<HttpLatency>) -> Element {
    let Some(latency) = latency else {
        return rsx! {
            EmptyState {
                title: "No HTTP workload",
                description: "This run did not include a workload, so there are no latency or throughput results.",
                span { class: "text-(length:--fs-sm) text-(color:--clr-text-faint)", "Add a workload when submitting a benchmark." }
            }
        };
    };

    let groups = vec![
        group("min", latency.min_ms),
        group("mean", latency.mean_ms),
        group("p50", latency.p50_ms),
        group("p95", latency.p95_ms),
        group("p99", latency.p99_ms),
        group("max", latency.max_ms),
    ];
    let partial = latency.status == "target_exited";
    let total = format_number(latency.total_requests as f64, 0);
    let succeeded = format_number(latency.successful_requests as f64, 0);
    let failed = format_number(latency.failed_requests as f64, 0);
    let duration = format_duration_ms(latency.duration_ms as f64);
    let rate = format_number(latency.requests_per_second, 1);

    rsx! {
        Card {
            CardHeader {
                title: "HTTP latency",
                subtitle: "Response time percentiles under the configured workload.",
            }
            div { class: "flex flex-col gap-(--space-5) p-(--space-5)",
                dl { class: "grid grid-cols-2 gap-(--space-4) md:grid-cols-5",
                    Definition { label: "Status", StatusBadge { status: latency.status.clone() } }
                    Definition { label: "Requests", "{total}" }
                    Definition { label: "Succeeded", "{succeeded}" }
                    Definition { label: "Failed", "{failed}" }
                    Definition { label: "Duration", "{duration}" }
                }
                if partial {
                    p { class: "rounded-(--round-md) bg-(--clr-warning-soft) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-warning)",
                        "The target exited during the workload, so these numbers are partial and not a completed benchmark."
                    }
                }
                p { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)",
                    "Throughput "
                    span { class: "font-(weight:--fw-semibold) text-(color:--clr-text)", "{rate} requests/s" }
                }
                BarChart {
                    title: "Latency percentiles",
                    groups,
                    series: single_series("Latency", ChartTone::Amber),
                    format: ValueFormat::LatencyMs,
                }
            }
        }
    }
}

// Memory and CPU side by side
#[component]
pub fn ResourceSection(memory: Option<Memory>, cpu: Option<Cpu>) -> Element {
    let memory_groups = memory.map(|memory| {
        vec![
            group("baseline", Some(memory.baseline.rss_bytes as f64)),
            group("peak", Some(memory.peak.rss_bytes as f64)),
            group("delta", Some(memory.delta.rss_bytes as f64)),
        ]
    });
    let cpu_groups = cpu.map(|cpu| {
        vec![
            group("baseline", Some(cpu.baseline_percent)),
            group("average", Some(cpu.average_percent)),
            group("peak", Some(cpu.peak_percent)),
        ]
    });

    rsx! {
        div { class: "grid grid-cols-1 gap-(--space-4) lg:grid-cols-2",
            Card {
                CardHeader { title: "Memory", subtitle: "Resident set size of the target process group." }
                div { class: "p-(--space-4)",
                    if let Some(groups) = memory_groups {
                        BarChart {
                            title: "Memory",
                            groups,
                            series: single_series("RSS", ChartTone::Violet),
                            format: ValueFormat::Bytes,
                        }
                    } else {
                        p { class: "py-(--space-8) text-center text-(length:--fs-sm) text-(color:--clr-text-faint)",
                            "No memory result, this run had no workload."
                        }
                    }
                }
            }
            Card {
                CardHeader { title: "CPU", subtitle: "100% is one fully used core." }
                div { class: "p-(--space-4)",
                    if let Some(groups) = cpu_groups {
                        BarChart {
                            title: "CPU",
                            groups,
                            series: single_series("CPU", ChartTone::Cyan),
                            format: ValueFormat::Percent,
                        }
                    } else {
                        p { class: "py-(--space-8) text-center text-(length:--fs-sm) text-(color:--clr-text-faint)",
                            "No CPU result, this run had no workload."
                        }
                    }
                }
            }
        }
    }
}

// Details of one lifecycle step, build or startup
#[component]
fn StepCard(#[props(into)] title: String, step: StepResult) -> Element {
    let duration = step
        .duration_ms
        .map_or_else(|| MISSING.to_string(), |ms| format_duration_ms(ms as f64));
    let command = step.command.as_ref().map(|command| {
        std::iter::once(command.program.clone())
            .chain(command.args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ")
    });
    let started = step.started_at.as_deref().map(format_timestamp);
    let finished = step
        .finished_at
        .as_deref()
        .or(step.ready_at.as_deref())
        .map(format_timestamp);
    let exit_code = step.exit_code.map(|code| code.to_string());
    let readiness = step.readiness.as_ref().map(|readiness| {
        format!(
            "{} {} {}:{}{} expects {}",
            readiness.readiness_type,
            readiness.method,
            readiness.host,
            readiness.port,
            readiness.path,
            readiness.expected_status
        )
    });

    rsx! {
        Card {
            CardHeader { title }
            dl { class: "grid grid-cols-2 gap-(--space-4) p-(--space-5)",
                Definition { label: "Status", StatusBadge { status: step.status.clone() } }
                Definition { label: "Duration", "{duration}" }
                if let Some(command) = command {
                    div { class: "col-span-2",
                        Definition { label: "Command",
                            code { class: "font-(family-name:--font-code) text-(length:--fs-sm)", "{command}" }
                        }
                    }
                }
                if let Some(directory) = step.working_directory.clone() {
                    div { class: "col-span-2",
                        Definition { label: "Working directory",
                            code { class: "font-(family-name:--font-code) text-(length:--fs-sm)", "{directory}" }
                        }
                    }
                }
                if let Some(readiness) = readiness {
                    div { class: "col-span-2",
                        Definition { label: "Readiness probe",
                            code { class: "font-(family-name:--font-code) text-(length:--fs-sm)", "{readiness}" }
                        }
                    }
                }
                if let Some(started) = started {
                    Definition { label: "Started", "{started}" }
                }
                if let Some(finished) = finished {
                    Definition { label: "Finished", "{finished}" }
                }
                if let Some(code) = exit_code {
                    Definition { label: "Exit code", "{code}" }
                }
            }
        }
    }
}

// Build and startup steps
#[component]
pub fn StepsSection(build_step: StepResult, startup_step: StepResult) -> Element {
    rsx! {
        div { class: "grid grid-cols-1 gap-(--space-4) lg:grid-cols-2",
            StepCard { title: "Build", step: build_step }
            StepCard { title: "Startup", step: startup_step }
        }
    }
}

// Size and file count of the source, build and excluded artifacts
#[component]
pub fn ArtifactSection(artifacts: ArtifactSize) -> Element {
    let categories = [
        ("Source", artifacts.source),
        ("Build", artifacts.build),
        ("Excluded", artifacts.excluded),
    ];
    let groups: Vec<BarGroup> = categories
        .iter()
        .map(|(label, category)| group(label, Some(category.bytes as f64)))
        .collect();
    let rows: Vec<(&str, ArtifactRow)> = categories
        .iter()
        .map(|(label, category)| {
            (
                *label,
                ArtifactRow {
                    size: category.size.clone(),
                    exact: format_bytes(category.bytes as f64),
                    files: format_number(category.file_count as f64, 0),
                },
            )
        })
        .collect();

    rsx! {
        Card {
            CardHeader { title: "Artifact size", subtitle: "What was measured on disk for the target." }
            div { class: "grid grid-cols-1 gap-(--space-5) p-(--space-5) lg:grid-cols-2",
                div { class: "overflow-x-auto",
                    table { class: "w-full text-left text-(length:--fs-sm)",
                        thead {
                            tr { class: "text-(length:--fs-xs) uppercase tracking-wider text-(color:--clr-text-faint)",
                                th { class: "py-(--space-2) pr-(--space-3) font-(weight:--fw-medium)", "Category" }
                                th { class: "py-(--space-2) pr-(--space-3) text-right font-(weight:--fw-medium)", "Size" }
                                th { class: "py-(--space-2) text-right font-(weight:--fw-medium)", "Files" }
                            }
                        }
                        tbody {
                            for (label , category) in rows {
                                tr { class: "border-t border-(color:--clr-border)",
                                    td { class: "py-(--space-3) pr-(--space-3)", "{label}" }
                                    td { class: "py-(--space-3) pr-(--space-3) text-right tabular-nums",
                                        span { title: "{category.exact}", "{category.size}" }
                                    }
                                    td { class: "py-(--space-3) text-right tabular-nums", "{category.files}" }
                                }
                            }
                        }
                    }
                }
                BarChart {
                    title: "Artifact size by category",
                    groups,
                    series: single_series("Size", ChartTone::Blue),
                    format: ValueFormat::Bytes,
                }
            }
        }
    }
}

// Artifact table row with every value already formatted
struct ArtifactRow {
    size: String,
    exact: String,
    files: String,
}
