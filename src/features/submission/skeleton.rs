use dioxus::prelude::*;

use crate::components::{Card, Skeleton};

// Loading shape of the saved target card: picker, runtime tag and the facts below it
#[component]
pub fn TargetSkeleton() -> Element {
    rsx! {
        Card { class: "flex flex-col gap-(--space-4) p-(--space-4)",
            div {
                role: "status",
                "aria-live": "polite",
                class: "flex flex-col gap-(--space-4)",
                span { class: "sr-only", "Loading saved targets" }
                Skeleton { class: "h-4 w-32" }
                Skeleton { class: "h-10 w-full" }
                div { class: "grid grid-cols-1 gap-(--space-3) sm:grid-cols-2",
                    for index in 0..4 {
                        div { key: "{index}", class: "flex flex-col gap-(--space-1)",
                            Skeleton { class: "h-3 w-20" }
                            Skeleton { class: "h-5 w-40 max-w-full" }
                        }
                    }
                }
            }
        }
    }
}
