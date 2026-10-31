use dioxus::prelude::*;

use super::models::WorkspaceResult;
use crate::{
    components::{Card, CardHeader, Definition, StatusBadge},
    features::charts::{BarChart, BarGroup, BarSeries, ChartTone},
    format::{format_bytes, format_duration_ms, format_percent, ValueFormat, MISSING},
};

fn mode_title(mode: &str) -> &'static str {
    match mode {
        "docker" => "Docker",
        _ => "Native",
    }
}

fn duration(value: Option<u64>) -> String {
    value.map_or_else(|| MISSING.to_string(), |ms| format_duration_ms(ms as f64))
}

// One card per workspace and mode: image size, build time, startup time, memory, CPU and the
// load result. A mode that did not succeed shows why and its numbers stay empty.
#[component]
pub fn WorkspacesSection(workspaces: Vec<WorkspaceResult>) -> Element {
    if workspaces.is_empty() {
        return rsx! {};
    }

    // The modes of one workspace are compared side by side, so they are grouped by name
    let mut groups: Vec<(String, Vec<WorkspaceResult>)> = Vec::new();

    for workspace in &workspaces {
        match groups.iter_mut().find(|(name, _)| *name == workspace.name) {
            Some((_, modes)) => modes.push(workspace.clone()),
            None => groups.push((workspace.name.clone(), vec![workspace.clone()])),
        }
    }

    rsx! {
        for (name , modes) in groups {
            WorkspaceCharts { key: "{name}", name, modes }
        }
        for workspace in workspaces {
            WorkspaceCard { workspace }
        }
    }
}

fn bars(
    label: &str,
    modes: &[WorkspaceResult],
    pick: fn(&WorkspaceResult) -> Option<f64>,
) -> BarGroup {
    BarGroup {
        label: label.to_string(),
        values: modes.iter().map(pick).collect(),
    }
}

// Native and Docker of one workspace side by side: memory, CPU and the time it took
#[component]
fn WorkspaceCharts(name: String, modes: Vec<WorkspaceResult>) -> Element {
    let series: Vec<BarSeries> = modes
        .iter()
        .map(|mode| BarSeries {
            name: mode_title(&mode.mode).to_string(),
            tone: if mode.mode == "docker" {
                ChartTone::Cyan
            } else {
                ChartTone::Violet
            },
        })
        .collect();

    let memory = vec![
        bars("baseline", &modes, |m| {
            m.memory.as_ref().map(|x| x.baseline.rss_bytes as f64)
        }),
        bars("peak", &modes, |m| {
            m.memory.as_ref().map(|x| x.peak.rss_bytes as f64)
        }),
    ];
    let cpu = vec![
        bars("average", &modes, |m| {
            m.cpu.as_ref().map(|x| x.average_percent)
        }),
        bars("peak", &modes, |m| m.cpu.as_ref().map(|x| x.peak_percent)),
    ];
    let time = vec![
        bars("build", &modes, |m| {
            m.build.as_ref().map(|x| x.duration_ms as f64)
        }),
        bars("startup", &modes, |m| {
            m.startup.as_ref().map(|x| x.duration_ms as f64)
        }),
    ];

    rsx! {
        div { class: "grid grid-cols-1 gap-(--space-4) lg:grid-cols-3",
            Card {
                CardHeader { title: "{name} · memory", subtitle: "Resident set size, native and Docker side by side." }
                div { class: "p-(--space-4) pb-(--space-3)",
                    BarChart {
                        title: "{name} memory",
                        groups: memory,
                        series: series.clone(),
                        format: ValueFormat::Bytes,
                    }
                }
            }
            Card {
                CardHeader { title: "{name} · CPU", subtitle: "100% is one fully used core." }
                div { class: "p-(--space-4) pb-(--space-3)",
                    BarChart {
                        title: "{name} CPU",
                        groups: cpu,
                        series: series.clone(),
                        format: ValueFormat::Percent,
                    }
                }
            }
            Card {
                CardHeader { title: "{name} · build and startup", subtitle: "A native run without a build command has no build time." }
                div { class: "p-(--space-4) pb-(--space-3)",
                    BarChart {
                        title: "{name} build and startup",
                        groups: time,
                        series,
                        format: ValueFormat::DurationMs,
                    }
                }
            }
        }
    }
}

#[component]
fn WorkspaceCard(workspace: WorkspaceResult) -> Element {
    let title = format!("{} · {}", workspace.name, mode_title(&workspace.mode));

    let image = workspace
        .image
        .as_ref()
        .map_or_else(|| MISSING.to_string(), |image| image.size.clone());
    let disk_usage = workspace
        .image
        .as_ref()
        .and_then(|image| image.disk_usage.clone())
        .unwrap_or_else(|| MISSING.to_string());
    let build = duration(workspace.build.as_ref().map(|build| build.duration_ms));
    let startup = duration(
        workspace
            .startup
            .as_ref()
            .map(|startup| startup.duration_ms),
    );
    let baseline = workspace.memory.as_ref().map_or_else(
        || MISSING.to_string(),
        |memory| format_bytes(memory.baseline.rss_bytes as f64),
    );
    let peak = workspace.memory.as_ref().map_or_else(
        || MISSING.to_string(),
        |memory| format_bytes(memory.peak.rss_bytes as f64),
    );
    let cpu_average = workspace.cpu.as_ref().map_or_else(
        || MISSING.to_string(),
        |cpu| format_percent(cpu.average_percent),
    );
    let cpu_peak = workspace.cpu.as_ref().map_or_else(
        || MISSING.to_string(),
        |cpu| format_percent(cpu.peak_percent),
    );
    let load_duration = duration(workspace.load.as_ref().map(|load| load.duration_ms));
    let load_exit = workspace
        .load
        .as_ref()
        .and_then(|load| load.exit_code)
        .map_or_else(|| MISSING.to_string(), |code| code.to_string());
    let variables = if workspace.env_variable_names.is_empty() {
        "None".to_string()
    } else {
        workspace.env_variable_names.join(", ")
    };

    rsx! {
        Card {
            CardHeader {
                title: "{title}",
                subtitle: "Measured after the project, from the process or the container's main process.",
            }
            div { class: "flex flex-col gap-(--space-4) p-(--space-4)",
                dl { class: "grid grid-cols-2 gap-(--space-4) md:grid-cols-4",
                    Definition { label: "Status", StatusBadge { status: workspace.status.clone() } }
                    if workspace.mode == "docker" {
                        Definition { label: "Image size", "{image}" }
                        Definition { label: "Image disk usage", "{disk_usage}" }
                    }
                    Definition { label: "Build time", "{build}" }
                    Definition { label: "Startup time", "{startup}" }
                    Definition { label: "Memory baseline", "{baseline}" }
                    Definition { label: "Memory peak", "{peak}" }
                    Definition { label: "CPU average", "{cpu_average}" }
                    Definition { label: "CPU peak", "{cpu_peak}" }
                }
                if let Some(load) = workspace.load.clone() {
                    dl { class: "grid grid-cols-2 gap-(--space-4) md:grid-cols-4",
                        Definition { label: "Load command", StatusBadge { status: load.status.clone() } }
                        Definition { label: "Load duration", "{load_duration}" }
                        Definition { label: "Load exit code", "{load_exit}" }
                    }
                } else if let Some(idle) = workspace.idle_seconds {
                    p { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)",
                        "No load command, sampled while idle for {idle} s."
                    }
                }
                p { class: "text-(length:--fs-xs) text-(color:--clr-text-faint)",
                    "Environment variables (names only): {variables}"
                }
                if let Some(message) = workspace.message.clone() {
                    p { class: "rounded-(--round-md) border border-(color:--clr-warning) bg-(--clr-warning-soft) px-(--space-3) py-(--space-2) text-(length:--fs-sm) text-(color:--clr-warning)",
                        "{message}"
                    }
                }
            }
        }
    }
}
