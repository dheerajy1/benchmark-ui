use dioxus::prelude::*;

use super::{Card, Skeleton};

// Grey chart line that draws itself, stands in for a chart or sparkline while data loads
#[component]
pub fn GhostLine(#[props(into, default)] class: String) -> Element {
    rsx! {
        svg {
            class: "ghost-draw h-full w-full {class}",
            view_box: "0 0 100 40",
            "preserveAspectRatio": "none",
            "aria-hidden": "true",
            path {
                d: "M0 30 L12 24 L24 27 L38 14 L52 19 L66 9 L80 15 L100 6",
                class: "vector-fixed",
                fill: "none",
                stroke: "var(--clr-skeleton)",
                stroke_width: "2.5",
                stroke_linejoin: "round",
                stroke_linecap: "round",
            }
        }
    }
}

// Chart area of the real chart size with a grey line drawing itself
#[component]
pub fn GhostChart() -> Element {
    rsx! {
        div { class: "h-(--chart-height) w-full", GhostLine {} }
    }
}

// Same box as a metric tile: label, big number and sparkline
#[component]
pub fn TileSkeleton() -> Element {
    rsx! {
        div { class: "flex min-w-0 flex-col gap-(--space-2) rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) px-(--space-4) pt-(--space-3) pb-(--space-2) shadow-(--shadow-card)",
            div { class: "flex items-center justify-between gap-(--space-2)",
                Skeleton { class: "h-4 w-24" }
                Skeleton { class: "h-4 w-12" }
            }
            Skeleton { class: "h-8 w-28" }
            div { class: "h-20 w-full", GhostLine {} }
        }
    }
}

// Same box as a chart card: header lines and a drawing grey line
#[component]
pub fn ChartCardSkeleton(#[props(into, default)] class: String) -> Element {
    rsx! {
        Card { class: "{class}",
            div { class: "flex flex-col gap-(--space-1) border-b border-(color:--clr-border) px-(--space-4) py-(--space-3)",
                Skeleton { class: "h-4 w-40" }
                Skeleton { class: "h-3 w-56 max-w-full" }
            }
            div { class: "p-(--space-4) pb-(--space-3)", GhostChart {} }
        }
    }
}

// Block of the given height inside a card, for sections that have no chart
#[component]
pub fn BlockSkeleton(#[props(into)] class: String) -> Element {
    rsx! {
        Card { class: "p-(--space-4)",
            Skeleton { class: "w-full {class}" }
        }
    }
}

// Rows of the Runs table while the first page loads
#[component]
pub fn RowsSkeleton(#[props(default = 8)] count: usize) -> Element {
    rsx! {
        div {
            class: "flex flex-col gap-0.5 rounded-(--round-lg) border border-(color:--clr-border) bg-(--clr-surface) p-1.5",
            role: "status",
            "aria-live": "polite",
            span { class: "sr-only", "Loading run history" }
            for index in 0..count {
                Skeleton { key: "{index}", class: "h-12 w-full" }
            }
        }
    }
}

const GHOST_BOX: &str =
    "rounded-(--round-lg) border border-dashed border-(color:--clr-border-strong)";

// Three dashed tiles with a dash where the number goes, for an Overview with nothing measured
#[component]
pub fn GhostTiles() -> Element {
    rsx! {
        div {
            class: "grid w-full max-w-2xl grid-cols-3 gap-(--space-3)",
            "aria-hidden": "true",
            for index in 0..3 {
                div {
                    key: "{index}",
                    class: "flex flex-col gap-(--space-2) p-(--space-3) {GHOST_BOX}",
                    Skeleton { class: "h-2.5 w-2/3" }
                    span { class: "text-(length:--fs-kpi) leading-none text-(color:--clr-text-faint)",
                        "\u{2014}"
                    }
                }
            }
        }
    }
}

// Three dashed table rows that fade out, for a list with no entries
#[component]
pub fn GhostRows() -> Element {
    rsx! {
        div {
            class: "flex w-full max-w-xl flex-col gap-(--space-2)",
            "aria-hidden": "true",
            for (index , fade) in ["opacity-100", "opacity-60", "opacity-25"].into_iter().enumerate() {
                div {
                    key: "{index}",
                    class: "flex h-9 items-center gap-(--space-3) px-(--space-3) {GHOST_BOX} {fade}",
                    Skeleton { class: "size-4" }
                    Skeleton { class: "h-2.5 w-1/3" }
                    Skeleton { class: "ml-auto h-2.5 w-12" }
                }
            }
        }
    }
}

// Two dashed run selectors, the text of each says what it waits for
#[component]
pub fn GhostSelectors(#[props(into)] left: String, #[props(into)] right: String) -> Element {
    rsx! {
        div {
            class: "grid w-full max-w-xl grid-cols-1 gap-(--space-3) sm:grid-cols-2",
            "aria-hidden": "true",
            for (index , text) in [left, right].into_iter().enumerate() {
                div {
                    key: "{index}",
                    class: "flex h-14 items-center justify-center px-(--space-3) text-(length:--fs-sm) text-(color:--clr-text-faint) {GHOST_BOX}",
                    "{text}"
                }
            }
        }
    }
}

// Chart area with a faint drawing line and a message on top
#[component]
pub fn GhostNote(#[props(into)] title: String, #[props(into)] text: String) -> Element {
    rsx! {
        div { class: "relative h-(--chart-height) w-full",
            div { class: "absolute inset-0 opacity-60", "aria-hidden": "true", GhostLine {} }
            div { class: "absolute inset-0 flex flex-col items-center justify-center gap-(--space-1) text-center",
                p { class: "text-(length:--fs-base) font-(weight:--fw-semibold) text-(color:--clr-text)",
                    "{title}"
                }
                if !text.is_empty() {
                    p { class: "text-(length:--fs-sm) text-(color:--clr-text-muted)", "{text}" }
                }
            }
        }
    }
}

// One quiet dashed line for a part of a run that does not exist
#[component]
pub fn QuietNote(#[props(into)] text: String) -> Element {
    rsx! {
        p { class: "px-(--space-4) py-(--space-3) text-center text-(length:--fs-sm) text-(color:--clr-text-faint) {GHOST_BOX}",
            "{text}"
        }
    }
}
