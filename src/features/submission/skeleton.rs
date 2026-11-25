use dioxus::prelude::*;

use crate::components::Skeleton;

// Loading shape of the new benchmark page in saved target mode. Words that are always there are
// drawn as grey bars of their own length (`Ghost`), values that are not known yet are plain bars,
// and every card has a dashed outline, so the page keeps its final shape while the targets load.

// Widths of the label and value bars of the facts card, one pair per row
const FACT_BARS: [(&str, &str); 5] = [
    ("w-10", "w-2/5"),
    ("w-16", "w-1/2"),
    ("w-14", "w-4/5"),
    ("w-20", "w-3/5"),
    ("w-12", "w-2/3"),
];

// The lines of "What will run" with the width of each value bar
const SUMMARY_BARS: [(&str, &str); 6] = [
    ("Target", "w-2/3"),
    ("Path", "w-11/12"),
    ("Build", "w-3/5"),
    ("Cleanup", "w-1/3"),
    ("Workload", "w-1/2"),
    ("Workspace", "w-1/4"),
];

// The steps of the estimated time, as the run shows them
const STEPS: [&str; 9] = [
    "Detect runtime",
    "Build",
    "Measure artifact size",
    "Start and wait until ready",
    "Baseline",
    "Workload",
    "Measure workspace",
    "Clean up",
    "Save result",
];

// A word that is always there, drawn as a bar of its own length. The page does not jump when the
// real text arrives, and the word is hidden from assistive technology and from selection.
#[component]
fn Ghost(#[props(into)] text: String, #[props(into, default)] class: String) -> Element {
    rsx! {
        span {
            class: "skeleton block w-fit max-w-full rounded-(--round-sm) text-transparent select-none {class}",
            "aria-hidden": "true",
            "{text}"
        }
    }
}

// A round bar, for badges
#[component]
fn Pill(#[props(into, default)] class: String) -> Element {
    rsx! {
        div { class: "skeleton rounded-(--round-full) {class}", "aria-hidden": "true" }
    }
}

// Card with a dashed outline, the loading twin of `Card`
#[component]
fn SkCard(#[props(into, default)] class: String, children: Element) -> Element {
    rsx! {
        section {
            class: "min-w-0 rounded-(--round-lg) border border-dashed border-(color:--clr-border-strong) bg-(--clr-surface) p-(--space-4) {class}",
            {children}
        }
    }
}

// The empty box of a field or a picker
#[component]
fn SkField(#[props(into, default)] class: String, children: Element) -> Element {
    rsx! {
        div {
            class: "flex items-center justify-between gap-(--space-3) rounded-(--round-md) border border-dashed border-(color:--clr-border-strong) px-(--space-3) {class}",
            {children}
        }
    }
}

// Left column: the target picker, what the target does and the workload of the run
#[component]
pub fn FormSkeleton() -> Element {
    rsx! {
        div {
            role: "status",
            "aria-live": "polite",
            class: "flex flex-col gap-(--space-4)",
            span { class: "sr-only", "Loading saved targets" }
            SkCard { class: "flex flex-col gap-(--space-3)",
                Ghost {
                    text: "Target",
                    class: "text-(length:--fs-base) font-(weight:--fw-semibold)",
                }
                SkField { class: "py-(--space-2)",
                    div { class: "flex flex-col gap-(--space-2)",
                        Skeleton { class: "h-4 w-32" }
                        Skeleton { class: "h-3 w-52 max-w-full" }
                    }
                    Skeleton { class: "size-3 shrink-0" }
                }
            }
            SkCard { class: "flex flex-col gap-(--space-3) [--sk-delay:150ms]",
                div { class: "flex items-center justify-between gap-(--space-3)",
                    Ghost {
                        text: "What this target does",
                        class: "text-(length:--fs-base) font-(weight:--fw-semibold)",
                    }
                    Pill { class: "h-5 w-28 shrink-0" }
                }
                div { class: "flex flex-col",
                    for (index , (label , value)) in FACT_BARS.into_iter().enumerate() {
                        div {
                            key: "{index}",
                            class: "flex flex-col gap-(--space-2) border-t border-dotted border-(color:--clr-border-strong) py-(--space-2) first:border-t-0",
                            Skeleton { class: "h-2.5 {label}" }
                            Skeleton { class: "h-3.5 {value}" }
                        }
                    }
                }
                Ghost {
                    text: "Targets are changed by editing the server's targets file, not here.",
                    class: "text-(length:--fs-xs)",
                }
            }
            SkCard { class: "flex flex-col gap-(--space-4) [--sk-delay:300ms]",
                div { class: "flex flex-col gap-(--space-1)",
                    Ghost {
                        text: "Workload for this run",
                        class: "text-(length:--fs-base) font-(weight:--fw-semibold)",
                    }
                    Ghost {
                        text: "Starts from the target's values. Changes apply to this run only.",
                        class: "text-(length:--fs-xs)",
                    }
                }
                div { class: "flex flex-col gap-(--space-1)",
                    Ghost {
                        text: "Run until",
                        class: "text-(length:--fs-xs) font-(weight:--fw-medium)",
                    }
                    div { class: "flex h-8.5 w-54 max-w-full items-center justify-around rounded-(--round-full) border border-dashed border-(color:--clr-border-strong)",
                        for name in ["Duration", "Requests"] {
                            Ghost {
                                key: "{name}",
                                text: name,
                                class: "text-(length:--fs-sm)",
                            }
                        }
                    }
                }
                div { class: "grid grid-cols-1 gap-(--space-4) sm:grid-cols-2",
                    div { class: "flex flex-col gap-(--space-1)",
                        Skeleton { class: "h-3 w-24" }
                        SkField { class: "h-(--control-height)",
                            Skeleton { class: "h-3 w-10" }
                        }
                    }
                    div { class: "flex flex-col gap-(--space-1)",
                        Ghost {
                            text: "Concurrency",
                            class: "text-(length:--fs-xs) font-(weight:--fw-medium)",
                        }
                        SkField { class: "h-(--control-height)",
                            Skeleton { class: "h-3 w-6" }
                        }
                    }
                }
            }
        }
    }
}

// The switch above the form: the two modes as grey words in a dashed pill. The row it sits in
// stretches it to the height of the buttons next to it.
#[component]
pub fn SwitchSkeleton() -> Element {
    rsx! {
        div { class: "flex items-stretch", "aria-hidden": "true",
            div { class: "flex items-center rounded-(--round-full) border border-dashed border-(color:--clr-border-strong) p-1",
                for name in ["Saved target", "Custom"] {
                    div { key: "{name}", class: "flex w-32 justify-center",
                        Ghost { text: name, class: "text-(length:--fs-sm)" }
                    }
                }
            }
        }
    }
}

// Right column: the summary panel and the previous runs. On wide screens it is cut where the
// form column ends, with a blur strip over the contents of its bottom edge to show that there is
// more. The dashed border stays outside the strip.
#[component]
pub fn PanelSkeleton() -> Element {
    rsx! {
        div {
            "aria-hidden": "true",
            class: "relative flex min-w-0 flex-col gap-(--space-4) lg:h-0 lg:min-h-full lg:self-stretch lg:overflow-hidden lg:rounded-b-(--round-lg) lg:border-b lg:border-dashed lg:border-(color:--clr-border-strong)",
            SkCard { class: "flex shrink-0 flex-col gap-(--space-4) [--sk-delay:100ms]",
                div { class: "flex flex-col gap-(--space-2)",
                    Ghost {
                        text: "What will run",
                        class: "text-(length:--fs-base) font-(weight:--fw-semibold)",
                    }
                    div { class: "flex flex-col gap-(--space-2)",
                        for (label , width) in SUMMARY_BARS {
                            div { key: "{label}", class: "flex flex-col gap-1",
                                Ghost {
                                    text: label,
                                    class: "text-(length:--fs-2xs) font-(weight:--fw-medium) uppercase tracking-wider",
                                }
                                Skeleton { class: "h-3.5 {width}" }
                            }
                        }
                    }
                }
                div { class: "flex flex-col gap-(--space-2) border-t border-dotted border-(color:--clr-border-strong) pt-(--space-3)",
                    div { class: "flex flex-col gap-1",
                        Ghost {
                            text: "Estimated time",
                            class: "text-(length:--fs-2xs) font-(weight:--fw-medium) uppercase tracking-wider",
                        }
                        Skeleton { class: "h-3.5 w-14" }
                    }
                    ul { class: "flex flex-col gap-(--space-2)",
                        for step in STEPS {
                            li { key: "{step}", class: "flex flex-col gap-1",
                                div { class: "flex items-center justify-between gap-(--space-2)",
                                    Ghost { text: step, class: "text-(length:--fs-xs)" }
                                    Skeleton { class: "h-2.5 w-8 shrink-0" }
                                }
                                div { class: "h-1 rounded-(--round-full) bg-(--clr-border)" }
                            }
                        }
                    }
                }
                div { class: "flex items-start gap-(--space-2) border-t border-dotted border-(color:--clr-border-strong) pt-(--space-3)",
                    span { class: "mx-1.5 mt-1.5 inline-block size-1.5 shrink-0 rounded-(--round-full) bg-(--clr-border-strong)" }
                    Ghost {
                        text: "A saved target is chosen",
                        class: "text-(length:--fs-sm)",
                    }
                }
                div { class: "flex h-(--control-height) items-center justify-center rounded-(--round-md) border border-dashed border-(color:--clr-border-strong)",
                    Ghost {
                        text: "Run benchmark",
                        class: "text-(length:--fs-sm) font-(weight:--fw-medium)",
                    }
                }
            }
            SkCard { class: "flex shrink-0 flex-col gap-(--space-3) [--sk-delay:250ms]",
                div { class: "flex flex-col",
                    Ghost {
                        text: "Previous runs",
                        class: "text-(length:--fs-base) font-(weight:--fw-semibold)",
                    }
                    Ghost {
                        text: "Runs with the same settings are grouped. Choosing one fills the form.",
                        class: "text-(length:--fs-xs)",
                    }
                }
                for index in 0..2 {
                    div {
                        key: "{index}",
                        class: "flex flex-col gap-(--space-2) rounded-(--round-md) border border-dashed border-(color:--clr-border-strong) bg-(--clr-surface-raised) p-(--space-3)",
                        div { class: "flex items-center justify-between gap-(--space-2)",
                            Skeleton { class: "h-3 w-12" }
                            Skeleton { class: "h-3 w-16" }
                        }
                        Skeleton { class: "h-3.5 w-4/5" }
                        Skeleton { class: "h-3 w-2/3" }
                        div { class: "flex items-center justify-between gap-(--space-2)",
                            Skeleton { class: "h-3 w-14" }
                            Skeleton { class: "h-3 w-8" }
                        }
                        Skeleton { class: "h-8 w-full" }
                    }
                }
            }
            div { class: "blur-strip hidden lg:block" }
        }
    }
}
