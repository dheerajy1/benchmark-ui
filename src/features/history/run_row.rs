use dioxus::prelude::*;

use super::models::RunSummary;
use crate::{
    app::Route,
    components::{Badge, Tone},
    format::{format_optional, format_timestamp, path_tail, short_id, ValueFormat, MISSING},
};

const CELL: &str = "px-(--space-3) py-(--space-2) align-middle";
const NUMBER_CELL: &str = "px-(--space-3) py-(--space-2) text-right align-middle whitespace-nowrap tabular-nums text-(color:--clr-text)";

// One table row of the history list
#[component]
pub fn RunRow(run: RunSummary, selected: bool, on_toggle: EventHandler<String>) -> Element {
    let id = run.run_id.clone();
    let short = short_id(&run.run_id);
    let created = format_timestamp(&run.created_at);
    let tail = path_tail(&run.target.path);
    let runtime = run.discovery.runtime.clone();
    let metrics = &run.summary;

    let build = format_optional(
        metrics.build_time_ms.map(|v| v as f64),
        ValueFormat::DurationMs,
    );
    let startup = format_optional(
        metrics.startup_time_ms.map(|v| v as f64),
        ValueFormat::DurationMs,
    );
    let memory = format_optional(
        metrics.memory_peak_bytes.map(|v| v as f64),
        ValueFormat::Bytes,
    );
    let cpu = format_optional(metrics.cpu_average_percent, ValueFormat::Percent);
    let throughput = format_optional(metrics.requests_per_second, ValueFormat::PerSecond);
    let p95 = format_optional(metrics.p95_ms, ValueFormat::LatencyMs);

    let row_class = if selected {
        "border-t border-(color:--clr-border) bg-(--clr-accent-soft) shadow-[inset_2px_0_0_var(--clr-accent)]"
    } else {
        "border-t border-(color:--clr-border) transition-colors hover:bg-(--clr-surface-hover)"
    };

    rsx! {
        tr { class: "{row_class}",
            td { class: "{CELL}",
                input {
                    r#type: "checkbox",
                    class: "size-4 cursor-pointer accent-(--clr-primary)",
                    checked: selected,
                    "aria-label": "Select run {short} for comparison",
                    onchange: move |_| on_toggle.call(id.clone()),
                }
            }
            td { class: "{CELL}",
                Link {
                    to: Route::RunDetail {
                        run_id: run.run_id.clone(),
                    },
                    class: "font-(family-name:--font-code) text-(length:--fs-sm) text-(color:--clr-accent) hover:underline",
                    "{short}"
                }
                div { class: "text-(length:--fs-2xs) whitespace-nowrap text-(color:--clr-text-faint)", "{created}" }
            }
            td { class: "{CELL}",
                span {
                    class: "block max-w-(--cell-max) truncate text-(color:--clr-text)",
                    title: "{run.target.path}",
                    "{tail}"
                }
            }
            td { class: "{CELL}",
                if let Some(runtime) = runtime {
                    Badge { label: runtime, tone: Tone::Accent }
                } else {
                    span { class: "text-(color:--clr-text-faint)", "{MISSING}" }
                }
            }
            td { class: "{NUMBER_CELL}", "{build}" }
            td { class: "{NUMBER_CELL}", "{startup}" }
            td { class: "{NUMBER_CELL}", "{memory}" }
            td { class: "{NUMBER_CELL}", "{cpu}" }
            td { class: "{NUMBER_CELL}", "{throughput}" }
            td { class: "{NUMBER_CELL}", "{p95}" }
        }
    }
}
