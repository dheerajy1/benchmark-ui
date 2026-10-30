use dioxus::prelude::*;

use super::{api::fetch_overview, dashboard::Dashboard};
use crate::{
    app::Route,
    components::{
        button_class, ButtonVariant, EmptyState, Icon, IconKind, LoadingPanel, PageHeader,
    },
    features::connection::ApiErrorPanel,
};

// Dashboard landing page backed by GET /api/v1/benchmarks/overview
#[component]
pub fn Overview() -> Element {
    let mut overview = use_resource(move || async move { fetch_overview().await });

    let primary = button_class(ButtonVariant::Primary);
    let secondary = button_class(ButtonVariant::Secondary);

    let body: Element = match overview() {
        None => rsx! {
            LoadingPanel { label: "Loading overview" }
        },
        Some(Err(error)) => rsx! {
            ApiErrorPanel { error, on_retry: move |_| overview.restart() }
        },
        Some(Ok(data)) => {
            if data.latest.is_none() {
                rsx! {
                    EmptyState {
                        title: "No benchmark runs yet",
                        description: "Run your first benchmark to see throughput, latency, memory and startup trends here.",
                        Link { to: Route::Submission {}, class: "{primary}", "Run a benchmark" }
                    }
                }
            } else {
                rsx! {
                    Dashboard { data }
                }
            }
        }
    };

    rsx! {
        document::Title { "Overview · Benchmark" }
        PageHeader {
            title: "Overview",
            description: "Latest results and how they trend across your most recent runs.",
            Link { to: Route::History {}, class: "{secondary}", "All runs" }
            Link { to: Route::Submission {}, class: "{primary}",
                Icon { kind: IconKind::Play, class: "size-(--icon-sm)" }
                "New benchmark"
            }
        }
        {body}
    }
}
