use dioxus::prelude::*;

use crate::{
    features::{
        comparisons::Comparisons, history::History, overview::Overview, run_detail::RunDetail,
        submission::Submission,
    },
    shell::{AppShell, NotFound},
};

// Every client-side route, each variant renders a feature's top-level component
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppShell)]
        #[route("/")]
        Overview {},
        #[route("/runs")]
        History {},
        #[route("/runs/:run_id")]
        RunDetail { run_id: String },
        #[route("/compare?:baseline&:comparison")]
        Comparisons { baseline: String, comparison: String },
        #[route("/new")]
        Submission {},
        #[route("/:..segments")]
        NotFound { segments: Vec<String> },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

// Root component, loads the generated Tailwind stylesheet and mounts the router
#[component]
pub fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        Router::<Route> {}
    }
}
