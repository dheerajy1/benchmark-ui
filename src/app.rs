use dioxus::prelude::*;

use crate::{
    features::{
        comparisons::Comparisons,
        history::History,
        live::LiveRun,
        overview::Overview,
        run_detail::RunDetail,
        submission::{Submission, SubmissionFrom},
    },
    shell::{AppShell, NotFound},
};

// Every client-side route, each variant renders a feature's top-level component
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppShell)]
        #[route("/?:project")]
        Overview { project: String },
        #[route("/runs?:target_path&:runtime&:status&:search")]
        History { target_path: String, runtime: String, status: String, search: String },
        #[route("/runs/live/:run_id")]
        LiveRun { run_id: String },
        #[route("/runs/:run_id")]
        RunDetail { run_id: String },
        #[route("/compare?:baseline&:comparison")]
        Comparisons { baseline: String, comparison: String },
        #[route("/new?:target")]
        Submission { target: String },
        #[route("/new/from/:run_id")]
        SubmissionFrom { run_id: String },
        #[route("/:..segments")]
        NotFound { segments: Vec<String> },
}

impl Route {
    // The overview of the newest project
    pub fn overview() -> Self {
        Self::Overview {
            project: String::new(),
        }
    }

    // The New benchmark page with nothing chosen
    pub fn new_benchmark() -> Self {
        Self::Submission {
            target: String::new(),
        }
    }

    // The Runs page with no filter
    pub fn runs() -> Self {
        Self::History {
            target_path: String::new(),
            runtime: String::new(),
            status: String::new(),
            search: String::new(),
        }
    }
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
const INTER_FONT: &str =
    "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&display=swap";

// Root component, loads the generated Tailwind stylesheet and mounts the router
#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        document::Link {
            rel: "preconnect",
            href: "https://fonts.gstatic.com",
            crossorigin: "anonymous",
        }
        document::Link { rel: "stylesheet", href: INTER_FONT }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        Router::<Route> {}
    }
}
