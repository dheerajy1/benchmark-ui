use dioxus::prelude::*;

use crate::components::{ChartCardSkeleton, Skeleton, TileSkeleton};

// Loading shape of the Overview: latest run card, six tiles and the chart cards in the same grid
#[component]
pub fn OverviewSkeleton() -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-(--space-4)",
            role: "status",
            "aria-live": "polite",
            span { class: "sr-only", "Loading overview" }
            Skeleton { class: "h-28 w-full rounded-(--round-lg)" }
            div { class: "grid grid-cols-1 gap-(--space-4) sm:grid-cols-2 lg:grid-cols-3",
                for index in 0..6 {
                    TileSkeleton { key: "{index}" }
                }
            }
            div { class: "grid grid-cols-1 gap-(--space-4) lg:grid-cols-3",
                ChartCardSkeleton { class: "lg:col-span-2 lg:row-span-2 lg:[--chart-height:27rem]" }
                ChartCardSkeleton {}
                ChartCardSkeleton {}
            }
            div { class: "grid grid-cols-1 gap-(--space-4) lg:grid-cols-2",
                ChartCardSkeleton {}
                ChartCardSkeleton {}
            }
        }
    }
}
