use dioxus::prelude::*;

use super::{
    models::WorkspaceResult,
    native_vs_docker::{pairs_of, NativeVsDocker},
    sections::trend_graph,
};
use crate::{
    components::{Card, CardHeader, Definition, StatusBadge},
    features::charts::ChartTone,
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
pub fn WorkspacesSection(
    workspaces: Vec<WorkspaceResult>,
    // Docker image size of the last runs of this target, the run itself last
    #[props(default)] image_trend: Vec<Option<f64>>,
) -> Element {
    if workspaces.is_empty() {
        return rsx! {};
    }

    // The numbers add up the images of every Docker workspace, so a single one can show them
    let single_docker = workspaces
        .iter()
        .filter(|workspace| workspace.mode == "docker")
        .count()
        == 1;

    // Native and Docker of one workspace are compared in one card, when both ran
    let pairs = pairs_of(&workspaces);

    rsx! {
        for (name , native , docker) in pairs {
            NativeVsDocker { key: "{name}", name, native, docker }
        }
        for workspace in workspaces {
            {
                let trend = if single_docker && workspace.mode == "docker" {
                    image_trend.clone()
                } else {
                    Vec::new()
                };

                rsx! {
                    WorkspaceCard { image_trend: trend, workspace }
                }
            }
        }
    }
}

#[component]
fn WorkspaceCard(
    workspace: WorkspaceResult,
    #[props(default)] image_trend: Vec<Option<f64>>,
) -> Element {
    let title = mode_title(&workspace.mode);
    let has_trend = image_trend.iter().flatten().count() >= 2;
    let subtitle = format!(
        "Workspace {} · measured after the project, from the process or the container's main process.",
        workspace.name
    );

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
                title,
                subtitle,
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
                if has_trend {
                    div { class: "grid grid-cols-2 gap-(--space-1)",
                        p { class: "col-span-2 text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text)",
                            "Docker image size"
                        }
                        {trend_graph(image_trend, ValueFormat::Bytes, ChartTone::Cyan)}
                    }
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
