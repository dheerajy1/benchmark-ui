use dioxus::prelude::*;

use super::{
    api::fetch_overview,
    dashboard::Dashboard,
    models::{OverviewData, OverviewState},
    project_switcher::ProjectSwitcher,
    skeleton::OverviewSkeleton,
};
use crate::{
    app::Route,
    components::{button_class, ButtonVariant, EmptyState, GhostTiles, Icon, IconKind, PageHeader},
    features::connection::ApiErrorPanel,
};

// Amber note under the header, used when the page has no numbers to show
#[component]
fn Notice(#[props(into)] title: String, #[props(into)] text: String, children: Element) -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-(--space-3) rounded-(--round-lg) border border-(color:--clr-warning)/40 bg-(--clr-warning-soft) p-(--space-4) sm:flex-row sm:items-center sm:justify-between",
            role: "status",
            div { class: "flex min-w-0 items-start gap-(--space-3) text-(color:--clr-warning-text)",
                Icon { kind: IconKind::Alert, class: "mt-px size-(--icon-md) shrink-0" }
                div { class: "min-w-0",
                    p { class: "text-(length:--fs-base) font-(weight:--fw-semibold)", "{title}" }
                    p { class: "text-(length:--fs-sm) break-words", "{text}" }
                }
            }
            div { class: "flex shrink-0 flex-wrap gap-(--space-2)", {children} }
        }
    }
}

// Page content for the response of one project
fn content(data: OverviewData) -> Element {
    let primary = button_class(ButtonVariant::Primary);
    let secondary = button_class(ButtonVariant::Secondary);

    match data.state() {
        OverviewState::NoRuns => rsx! {
            EmptyState {
                title: "Nothing measured yet",
                description: "Run your first benchmark to see throughput, latency, memory and startup trends here.",
                ghost: Some(rsx! {
                    GhostTiles {}
                }),
                Link { to: Route::new_benchmark(), class: "{primary}", "New benchmark" }
            }
        },
        OverviewState::UnknownProject => {
            let path = data
                .project
                .as_ref()
                .map(|project| project.path.clone())
                .unwrap_or_default();

            rsx! {
                Notice {
                    title: format!("No runs found for {path}"),
                    text: "Pick another project from the list above.",
                    Link { to: Route::runs(), class: "{secondary}", "All runs" }
                }
            }
        }
        OverviewState::NoFinishedRuns => {
            let path = data
                .project
                .as_ref()
                .map(|project| project.path.clone())
                .unwrap_or_default();
            let text = format!(
                "{} failed, {} cancelled. Only finished runs have numbers to show.",
                data.totals.failed_runs, data.totals.cancelled_runs
            );

            rsx! {
                Notice { title: "No finished runs for this project yet", text,
                    Link {
                        to: Route::History {
                            target_path: path,
                            runtime: String::new(),
                            status: String::new(),
                            search: String::new(),
                        },
                        class: "{secondary}",
                        "View runs"
                    }
                }
            }
        }
        OverviewState::Ready => rsx! {
            Dashboard { data }
        },
    }
}

// Dashboard landing page backed by GET /api/v1/benchmarks/overview for one project
#[component]
pub fn Overview(project: String) -> Element {
    let mut overview = use_resource(use_reactive!(|(project,)| async move {
        fetch_overview(&project).await
    }));

    let (switcher, body): (Element, Element) = match overview() {
        None => (
            rsx! {},
            rsx! {
                OverviewSkeleton {}
            },
        ),
        Some(Err(error)) => (
            rsx! {},
            rsx! {
                ApiErrorPanel { error, on_retry: move |_| overview.restart() }
            },
        ),
        Some(Ok(data)) => {
            let switcher = if data.projects.is_empty() {
                rsx! {}
            } else {
                rsx! {
                    ProjectSwitcher {
                        selected: data.project.clone(),
                        projects: data.projects.clone(),
                    }
                }
            };

            (switcher, content(data))
        }
    };

    rsx! {
        document::Title { "Overview · Benchmark" }
        PageHeader {
            title: "Overview",
            description: "Latest results and how they trend across the newest runs of one project.",
            {switcher}
        }
        {body}
    }
}
