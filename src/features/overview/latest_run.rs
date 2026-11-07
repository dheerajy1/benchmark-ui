use dioxus::prelude::*;

use crate::{
    app::Route,
    components::{button_class, Badge, ButtonVariant, Card, Definition, Icon, IconKind, Tone},
    features::history::RunSummary,
    format::{format_timestamp, short_id, MISSING},
};

// Identity of the newest run with shortcuts to its details and to a comparison with the run before it
#[component]
pub fn LatestRun(run: RunSummary, previous: Option<RunSummary>) -> Element {
    let previous_run_id = previous.as_ref().map(|previous| previous.run_id.clone());
    let compared_with = previous.as_ref().map_or_else(
        || "No earlier finished run to compare with.".to_string(),
        |previous| {
            format!(
                "Compared with {}, {}.",
                short_id(&previous.run_id),
                format_timestamp(&previous.created_at)
            )
        },
    );
    let title = run.title();
    let secondary = button_class(ButtonVariant::Secondary);
    let short = short_id(&run.run_id);
    let created = format_timestamp(&run.created_at);
    let runtime = run.discovery.runtime.clone();

    rsx! {
        Card {
            div { class: "flex flex-col gap-(--space-3) border-b border-(color:--clr-border) px-(--space-4) py-(--space-3) sm:flex-row sm:items-center sm:justify-between",
                div { class: "min-w-0",
                    h2 { class: "text-(length:--fs-base) font-(weight:--fw-semibold) text-(color:--clr-text)",
                        "{title}"
                    }
                    p { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)",
                        "{compared_with}"
                    }
                }
                div { class: "flex flex-wrap gap-(--space-2)",
                    Link {
                        to: Route::RunDetail {
                            run_id: run.run_id.clone(),
                        },
                        class: "{secondary}",
                        "View details"
                    }
                    if let Some(previous) = previous_run_id {
                        Link {
                            to: Route::Comparisons {
                                baseline: previous,
                                comparison: run.run_id.clone(),
                            },
                            class: "{secondary}",
                            Icon { kind: IconKind::Compare }
                            "Compare with previous run"
                        }
                    }
                }
            }
            dl { class: "grid grid-cols-2 gap-(--space-4) px-(--space-4) py-(--space-3) lg:grid-cols-4",
                Definition { label: "Run",
                    Link {
                        to: Route::RunDetail {
                            run_id: run.run_id.clone(),
                        },
                        class: "font-(family-name:--font-code) text-(color:--clr-accent) hover:underline",
                        "{short}"
                    }
                }
                Definition { label: "Runtime",
                    if let Some(runtime) = runtime {
                        Badge { label: runtime, tone: Tone::Accent }
                    } else {
                        span { class: "text-(color:--clr-text-faint)", "{MISSING}" }
                    }
                }
                Definition { label: "Recorded", "{created}" }
                Definition { label: "Target",
                    span { class: "font-(family-name:--font-code) text-(length:--fs-xs) break-all",
                        "{run.target.path}"
                    }
                }
            }
        }
    }
}
