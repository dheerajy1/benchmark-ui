use dioxus::prelude::*;

use super::{
    cards::DataCard,
    kpi::KpiTile,
    models::{ComparisonData, MetricDiff, WorkspaceComparison},
    results::{pair_group, pair_series},
    rows::workspace_rows,
    tables::MetricTable,
};
use crate::{
    features::charts::{BarChart, BarGroup, BarSeries, ChartTone, DivergeRow, DivergingBars},
    format::ValueFormat,
};

// Picks one measurement of a workspace
type Pick = fn(&WorkspaceComparison) -> &MetricDiff;

fn build_time(workspace: &WorkspaceComparison) -> &MetricDiff {
    &workspace.build_duration_ms
}

fn startup_time(workspace: &WorkspaceComparison) -> &MetricDiff {
    &workspace.startup_duration_ms
}

fn peak_memory(workspace: &WorkspaceComparison) -> &MetricDiff {
    &workspace.peak_rss_bytes
}

fn average_cpu(workspace: &WorkspaceComparison) -> &MetricDiff {
    &workspace.cpu_average_percent
}

fn image_size(workspace: &WorkspaceComparison) -> &MetricDiff {
    &workspace.image_bytes
}

// The measurements that native and Docker are set against, with how each one is formatted
const MEASUREMENTS: [(&str, Pick, ValueFormat); 4] = [
    ("Build time", build_time, ValueFormat::DurationMs),
    ("Startup", startup_time, ValueFormat::DurationMs),
    ("Peak memory", peak_memory, ValueFormat::Bytes),
    ("Average CPU", average_cpu, ValueFormat::Percent),
];

// Workspace names in the order they first appear
fn names(data: &ComparisonData) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();

    for workspace in &data.metrics.workspaces {
        if !names.contains(&workspace.name) {
            names.push(workspace.name.clone());
        }
    }

    names
}

fn find<'a>(data: &'a ComparisonData, name: &str, mode: &str) -> Option<&'a WorkspaceComparison> {
    data.metrics
        .workspaces
        .iter()
        .find(|workspace| workspace.name == name && workspace.mode == mode)
}

// How much more Docker uses than native in percent, negative when it uses less
fn percent_over(native: Option<f64>, docker: Option<f64>) -> Option<f64> {
    let native = native.filter(|value| *value > 0.0)?;

    Some((docker? - native) / native * 100.0)
}

fn series_four() -> Vec<BarSeries> {
    [
        ("Baseline native", ChartTone::Slate),
        ("Baseline Docker", ChartTone::Teal),
        ("Comparison native", ChartTone::Blue),
        ("Comparison Docker", ChartTone::Violet),
    ]
    .into_iter()
    .map(|(name, tone)| BarSeries {
        name: name.to_string(),
        tone,
    })
    .collect()
}

// One group per workspace holding native and Docker of both runs
fn mode_groups(data: &ComparisonData, pick: Pick) -> Vec<BarGroup> {
    names(data)
        .into_iter()
        .map(|name| {
            let native = find(data, &name, "native").map(pick);
            let docker = find(data, &name, "docker").map(pick);

            BarGroup {
                label: name,
                values: vec![
                    native.and_then(|diff| diff.baseline_value),
                    docker.and_then(|diff| diff.baseline_value),
                    native.and_then(|diff| diff.comparison_value),
                    docker.and_then(|diff| diff.comparison_value),
                ],
            }
        })
        .collect()
}

// What Docker costs over native for one run, biggest saving first. `baseline` picks the run.
fn cost_rows(data: &ComparisonData, baseline: bool) -> Vec<DivergeRow> {
    let names = names(data);
    let side = |diff: &MetricDiff| {
        if baseline {
            diff.baseline_value
        } else {
            diff.comparison_value
        }
    };
    let mut rows = Vec::new();

    for name in &names {
        let native = find(data, name, "native");
        let docker = find(data, name, "docker");

        for (label, pick, _) in MEASUREMENTS {
            let Some(percent) = percent_over(
                native.and_then(|workspace| side(pick(workspace))),
                docker.and_then(|workspace| side(pick(workspace))),
            ) else {
                continue;
            };

            rows.push(DivergeRow {
                label: if names.len() > 1 {
                    format!("{name} · {label}")
                } else {
                    label.to_string()
                },
                // Docker using less than native counts as better
                improvement: -percent,
                change: percent,
                lower_is_better: true,
            });
        }
    }

    rows.sort_by(|a, b| b.improvement.total_cmp(&a.improvement));
    rows
}

fn docker_workspace(data: &ComparisonData) -> Option<&WorkspaceComparison> {
    data.metrics
        .workspaces
        .iter()
        .find(|workspace| workspace.mode == "docker")
}

// Whether either run measured a Docker workspace
pub fn has_docker(data: &ComparisonData) -> bool {
    docker_workspace(data).is_some()
}

// Subscriber section: its own big numbers, the Docker image, native against Docker, and what
// Docker costs
#[component]
pub fn SubscriberSection(data: ComparisonData) -> Element {
    let all_rows = workspace_rows(&data);
    let image_rows: Vec<_> = all_rows
        .iter()
        .filter(|row| row.label == "Image size")
        .cloned()
        .collect();
    let other_rows: Vec<_> = all_rows
        .iter()
        .filter(|row| row.label != "Image size")
        .cloned()
        .collect();

    let docker = docker_workspace(&data).cloned();
    let image_groups: Vec<BarGroup> = data
        .metrics
        .workspaces
        .iter()
        .filter(|workspace| workspace.mode == "docker")
        .map(|workspace| pair_group(&workspace.name, image_size(workspace)))
        .collect();

    let multiples: Vec<(&'static str, Vec<BarGroup>, ValueFormat)> = MEASUREMENTS
        .iter()
        .map(|(label, pick, format)| (*label, mode_groups(&data, *pick), *format))
        .collect();
    let baseline_costs = cost_rows(&data, true);
    let comparison_costs = cost_rows(&data, false);
    let has_costs = !baseline_costs.is_empty() || !comparison_costs.is_empty();

    rsx! {
        if let Some(docker) = docker {
            div { class: "grid grid-cols-1 gap-(--space-3) sm:grid-cols-2 xl:grid-cols-4",
                KpiTile {
                    label: "Image size",
                    diff: docker.image_bytes.clone(),
                    format: ValueFormat::Bytes,
                }
                KpiTile {
                    label: "Docker build",
                    diff: docker.build_duration_ms.clone(),
                    format: ValueFormat::DurationMs,
                }
                KpiTile {
                    label: "Docker startup",
                    diff: docker.startup_duration_ms.clone(),
                    format: ValueFormat::DurationMs,
                }
                KpiTile {
                    label: "Docker peak memory",
                    diff: docker.peak_rss_bytes.clone(),
                    format: ValueFormat::Bytes,
                }
            }
        }
        if !image_groups.is_empty() {
            DataCard {
                title: "Image size",
                subtitle: "Size of the Docker image of each run.",
                chart: Some(rsx! {
                    BarChart {
                        title: "Image size",
                        groups: image_groups,
                        series: pair_series(),
                        format: ValueFormat::Bytes,
                    }
                }),
                table: Some(rsx! {
                    MetricTable { rows: image_rows, show_group: true }
                }),
            }
        }
        DataCard {
            title: "Native vs Docker",
            subtitle: "The same workspace run natively and in a container, for both runs.",
            chart: Some(rsx! {
                div { class: "grid grid-cols-1 gap-(--space-5) lg:grid-cols-2",
                    for (label , groups , format) in multiples {
                        div { key: "{label}", class: "flex flex-col gap-(--space-2)",
                            h4 { class: "text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text)",
                                "{label}"
                            }
                            BarChart {
                                title: "{label}",
                                groups,
                                series: series_four(),
                                format,
                            }
                        }
                    }
                }
            }),
            table: Some(rsx! {
                MetricTable { rows: other_rows, show_group: true }
            }),
        }
        if has_costs {
            DataCard {
                title: "What Docker costs",
                subtitle: "How much more Docker uses than native. Right and green means Docker was cheaper.",
                chart: Some(rsx! {
                    div { class: "grid grid-cols-1 gap-(--space-5) lg:grid-cols-2",
                        div { class: "flex flex-col gap-(--space-3)",
                            h4 { class: "text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text)",
                                "Baseline run"
                            }
                            DivergingBars { rows: baseline_costs }
                        }
                        div { class: "flex flex-col gap-(--space-3)",
                            h4 { class: "text-(length:--fs-sm) font-(weight:--fw-medium) text-(color:--clr-text)",
                                "Comparison run"
                            }
                            DivergingBars { rows: comparison_costs }
                        }
                    }
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{super::models::fixtures::diff, *};

    fn workspace(name: &str, mode: &str, startup: (f64, f64)) -> WorkspaceComparison {
        let same = || diff(Some(1.0), Some(1.0), "lower_is_better");

        WorkspaceComparison {
            name: name.to_string(),
            mode: mode.to_string(),
            baseline_status: Some("success".to_string()),
            comparison_status: Some("success".to_string()),
            image_bytes: same(),
            build_duration_ms: same(),
            startup_duration_ms: diff(Some(startup.0), Some(startup.1), "lower_is_better"),
            baseline_rss_bytes: same(),
            peak_rss_bytes: same(),
            delta_rss_bytes: same(),
            cpu_baseline_percent: same(),
            cpu_average_percent: same(),
            cpu_peak_percent: same(),
        }
    }

    fn data(workspaces: Vec<WorkspaceComparison>) -> ComparisonData {
        let mut data = super::super::models::fixtures::data();
        data.metrics.workspaces = workspaces;
        data
    }

    #[test]
    fn computes_how_much_more_docker_uses() {
        assert_eq!(percent_over(Some(100.0), Some(150.0)), Some(50.0));
        assert_eq!(percent_over(Some(100.0), Some(80.0)), Some(-20.0));
        assert_eq!(percent_over(Some(0.0), Some(80.0)), None);
        assert_eq!(percent_over(Some(100.0), None), None);
        assert_eq!(percent_over(None, Some(1.0)), None);
    }

    #[test]
    fn lists_workspace_names_once_in_order() {
        let data = data(vec![
            workspace("ts", "native", (1.0, 1.0)),
            workspace("ts", "docker", (1.0, 1.0)),
            workspace("rs", "native", (1.0, 1.0)),
        ]);

        assert_eq!(names(&data), ["ts", "rs"]);
    }

    #[test]
    fn groups_native_and_docker_of_both_runs() {
        let data = data(vec![
            workspace("ts", "native", (100.0, 80.0)),
            workspace("ts", "docker", (150.0, 120.0)),
        ]);

        let groups = mode_groups(&data, startup_time);

        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0].values,
            vec![Some(100.0), Some(150.0), Some(80.0), Some(120.0)]
        );
    }

    #[test]
    fn costs_are_sorted_with_the_biggest_saving_first() {
        let data = data(vec![
            workspace("ts", "native", (100.0, 100.0)),
            workspace("ts", "docker", (150.0, 100.0)),
        ]);

        let rows = cost_rows(&data, true);

        assert_eq!(rows.len(), 4);
        assert_eq!(rows.last().map(|row| row.label.as_str()), Some("Startup"));
        assert_eq!(rows.last().map(|row| row.change), Some(50.0));
        assert!(rows
            .windows(2)
            .all(|pair| pair[0].improvement >= pair[1].improvement));
    }

    #[test]
    fn skips_costs_when_one_mode_is_missing() {
        let data = data(vec![workspace("ts", "docker", (150.0, 100.0))]);

        assert!(cost_rows(&data, true).is_empty());
        assert!(has_docker(&data));
    }
}
