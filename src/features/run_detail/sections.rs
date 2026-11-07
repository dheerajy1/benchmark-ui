use dioxus::prelude::*;

use super::models::{ArtifactSize, Cpu, HttpLatency, Memory, StepResult};
use crate::{
    components::{Card, CardHeader, Definition, QuietNote, StatusBadge},
    features::charts::{
        ago_labels, BarChart, BarGroup, BarRow, BarSeries, ChartTone, HorizontalBars, Sparkline,
    },
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

// Small chart of one measurement over the last runs, nothing when fewer than two runs have it
fn trend_graph(values: Vec<Option<f64>>, format: ValueFormat, tone: ChartTone) -> Element {
    if values.iter().flatten().count() < 2 {
        return rsx! {};
    }

    let labels = ago_labels(values.len());
    let count = values.len();

    rsx! {
        div { class: "group col-span-2",
            p { class: "mb-(--space-1) text-(length:--fs-xs) text-(color:--clr-text-faint)",
                "Last {count} runs of this target"
            }
            Sparkline {
                values,
                labels,
                format,
                tone,
                class: "h-16",
            }
        }
    }
}

// Request counts and latency percentiles of the HTTP workload
#[component]
pub fn HttpSection(latency: Option<HttpLatency>) -> Element {
    let Some(latency) = latency else {
        return rsx! {
            QuietNote { text: "This run had no HTTP workload, so there are no latency or throughput results." }
        };
    };

    let rows = vec![
        BarRow {
            label: "p50".to_string(),
            value: latency.p50_ms,
        },
        BarRow {
            label: "p95".to_string(),
            value: latency.p95_ms,
        },
        BarRow {
            label: "p99".to_string(),
            value: latency.p99_ms,
        },
        BarRow {
            label: "max".to_string(),
            value: latency.max_ms,
        },
    ];
    let partial = latency.status == "target_exited";
    let counts = request_counts(
        latency.total_requests,
        latency.successful_requests,
        latency.failed_requests,
    );
    let failed_class = if counts.failed > 0 {
        "text-(color:--clr-danger-text)"
    } else {
        ""
    };
    let duration = format_duration_ms(latency.duration_ms as f64);
    let rate = format_number(latency.requests_per_second, 1);

    rsx! {
        Card {
            CardHeader {
                title: "Latency",
                subtitle: "Response time percentiles under the configured workload.",
            }
            div { class: "flex flex-col gap-(--space-4) p-(--space-4)",
                p { class: "text-(length:--fs-sm) tabular-nums text-(color:--clr-text-muted)",
                    "{counts.before_failed}"
                    span { class: "{failed_class}", "{counts.failed_text}" }
                }
                dl { class: "grid grid-cols-2 gap-(--space-4) md:grid-cols-3",
                    Definition { label: "Status", StatusBadge { status: latency.status.clone() } }
                    Definition { label: "Duration", "{duration}" }
                    Definition { label: "Throughput", "{rate} /s" }
                }
                if partial {
                    p { class: "rounded-(--round-md) border border-(color:--clr-warning) bg-(--clr-warning-soft) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-warning)",
                        "The target exited during the workload, so these numbers are partial and not a completed benchmark."
                    }
                }
                HorizontalBars { rows, format: ValueFormat::LatencyMs, tone: ChartTone::Amber }
            }
        }
    }
}

// The line under the Latency title, split so the failed part can turn red
#[derive(Debug, Clone, PartialEq)]
struct RequestCounts {
    // `10,000 requests · 9,982 ok · `
    before_failed: String,
    // `18 failed (0.18%)`
    failed_text: String,
    failed: u64,
}

fn request_counts(total: u64, ok: u64, failed: u64) -> RequestCounts {
    let share = if total == 0 {
        0.0
    } else {
        failed as f64 / total as f64 * 100.0
    };

    RequestCounts {
        before_failed: format!(
            "{} requests · {} ok · ",
            format_number(total as f64, 0),
            format_number(ok as f64, 0)
        ),
        failed_text: format!("{} failed ({share:.2}%)", format_number(failed as f64, 0)),
        failed,
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
                div { class: "p-(--space-4) pb-(--space-3)",
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
                div { class: "p-(--space-4) pb-(--space-3)",
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
fn StepCard(
    #[props(into)] title: String,
    step: StepResult,
    #[props(default)] trend: Vec<Option<f64>>,
) -> Element {
    let graph = trend_graph(trend, ValueFormat::DurationMs, ChartTone::Teal);
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
            dl { class: "grid grid-cols-2 gap-(--space-4) p-(--space-4)",
                {graph}
                Definition { label: "Status", StatusBadge { status: step.status.clone() } }
                Definition { label: "Duration", "{duration}" }
                if let Some(command) = command {
                    div { class: "col-span-2",
                        Definition { label: "Command",
                            code { class: "font-(family-name:--font-code) text-(length:--fs-xs) break-all", "{command}" }
                        }
                    }
                }
                if let Some(directory) = step.working_directory.clone() {
                    div { class: "col-span-2",
                        Definition { label: "Working directory",
                            code { class: "font-(family-name:--font-code) text-(length:--fs-xs) break-all", "{directory}" }
                        }
                    }
                }
                if let Some(readiness) = readiness {
                    div { class: "col-span-2",
                        Definition { label: "Readiness probe",
                            code { class: "font-(family-name:--font-code) text-(length:--fs-xs) break-all", "{readiness}" }
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
pub fn StepsSection(
    build_step: StepResult,
    startup_step: StepResult,
    build_trend: Vec<Option<f64>>,
    startup_trend: Vec<Option<f64>>,
) -> Element {
    rsx! {
        div { class: "grid grid-cols-1 gap-(--space-4) lg:grid-cols-2",
            StepCard {
                title: "Build",
                step: build_step,
                trend: build_trend,
            }
            StepCard {
                title: "Startup",
                step: startup_step,
                trend: startup_trend,
            }
        }
    }
}

// Size and file count of the source, build and excluded artifacts
#[component]
pub fn ArtifactSection(artifacts: ArtifactSize, trend: Vec<Option<f64>>) -> Element {
    let graph = trend_graph(trend, ValueFormat::Bytes, ChartTone::Blue);
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
            div { class: "grid grid-cols-1 gap-(--space-5) p-(--space-4) lg:grid-cols-2",
                div { class: "col-span-full empty:hidden", {graph} }
                div { class: "overflow-x-auto",
                    table { class: "w-full text-left text-(length:--fs-sm)",
                        thead {
                            tr { class: "text-(length:--fs-2xs) uppercase tracking-wider text-(color:--clr-text-faint)",
                                th { class: "py-(--space-2) pr-(--space-3) font-(weight:--fw-medium)", "Category" }
                                th { class: "py-(--space-2) pr-(--space-3) text-right font-(weight:--fw-medium)", "Size" }
                                th { class: "py-(--space-2) text-right font-(weight:--fw-medium)", "Files" }
                            }
                        }
                        tbody {
                            for (label , category) in rows {
                                tr { class: "border-t border-(color:--clr-border)",
                                    td { class: "py-(--space-2) pr-(--space-3)", "{label}" }
                                    td { class: "py-(--space-2) pr-(--space-3) text-right tabular-nums",
                                        span { title: "{category.exact}", "{category.size}" }
                                    }
                                    td { class: "py-(--space-2) text-right tabular-nums", "{category.files}" }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_failed_requests_with_their_share() {
        let counts = request_counts(10_000, 9_982, 18);

        assert_eq!(counts.before_failed, "10,000 requests · 9,982 ok · ");
        assert_eq!(counts.failed_text, "18 failed (0.18%)");
        assert_eq!(counts.failed, 18);
    }

    #[test]
    fn a_run_without_requests_has_no_share() {
        assert_eq!(request_counts(0, 0, 0).failed_text, "0 failed (0.00%)");
    }
}
