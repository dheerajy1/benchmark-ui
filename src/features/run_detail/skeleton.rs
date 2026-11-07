use dioxus::prelude::*;

use crate::components::{BlockSkeleton, ChartCardSkeleton, Skeleton, TileSkeleton};

// Loading shape of a run: header, section bar, six tiles and two sections
#[component]
pub fn RunDetailSkeleton() -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-(--space-4)",
            role: "status",
            "aria-live": "polite",
            span { class: "sr-only", "Loading run" }
            BlockSkeleton { class: "h-20" }
            Skeleton { class: "h-9 w-full" }
            div { class: "grid grid-cols-1 gap-(--space-3) sm:grid-cols-2 xl:grid-cols-3",
                for index in 0..6 {
                    TileSkeleton { key: "{index}" }
                }
            }
            div { class: "grid grid-cols-1 gap-(--space-4) lg:grid-cols-2",
                ChartCardSkeleton {}
                ChartCardSkeleton {}
            }
        }
    }
}
