use dioxus::prelude::*;

use crate::components::{BlockSkeleton, ChartCardSkeleton, Skeleton, TileSkeleton};

// Loading shape of the two run selectors
#[component]
pub fn PickersSkeleton() -> Element {
    rsx! {
        div {
            class: "grid grid-cols-1 gap-(--space-3) md:grid-cols-2",
            role: "status",
            "aria-live": "polite",
            span { class: "sr-only", "Loading runs" }
            Skeleton { class: "h-16 w-full rounded-(--round-lg)" }
            Skeleton { class: "h-16 w-full rounded-(--round-lg)" }
        }
    }
}

// Loading shape of a comparison: summary block, KPI tiles and a chart card
#[component]
pub fn CompareSkeleton() -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-(--space-4)",
            role: "status",
            "aria-live": "polite",
            span { class: "sr-only", "Comparing runs" }
            BlockSkeleton { class: "h-24" }
            div { class: "grid grid-cols-1 gap-(--space-3) sm:grid-cols-2 xl:grid-cols-3",
                for index in 0..6 {
                    TileSkeleton { key: "{index}" }
                }
            }
            ChartCardSkeleton {}
        }
    }
}
