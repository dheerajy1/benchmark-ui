use dioxus::prelude::*;

use crate::{
    components::{button_class, ButtonVariant, Card, CardHeader},
    features::{
        configurations::{header_names_of, ConfigurationEntry, ConfigurationGroup, StartupSummary},
        targets::TargetsView,
    },
};

use super::prefill::{prefill_for, Prefill};
use crate::format::relative_time;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

// One row of the history list, already worded for display
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryRow {
    pub index: usize,
    pub when: String,
    // The exact time, shown on hover
    pub absolute: String,
    pub runs: String,
    pub source: String,
    pub project: String,
    pub command: String,
    pub workload: String,
    pub headers: String,
    // Why "Use these settings" is unavailable for this row
    pub disabled_reason: Option<String>,
}

// "2026-10-21T14:03:11+00:00" becomes "21 Oct, 14:03". Anything else is shown unchanged.
pub fn format_when(created_at: &str) -> String {
    let (Some(date), Some(time)) = (created_at.get(0..10), created_at.get(11..16)) else {
        return created_at.to_string();
    };

    let parts: Vec<&str> = date.split('-').collect();

    let [_year, month, day] = parts.as_slice() else {
        return created_at.to_string();
    };

    let month_name = month
        .parse::<usize>()
        .ok()
        .and_then(|number| number.checked_sub(1))
        .and_then(|index| MONTHS.get(index));

    match month_name {
        Some(month_name) => format!("{} {month_name}, {time}", day.trim_start_matches('0')),
        None => created_at.to_string(),
    }
}

// The current time in milliseconds since the Unix epoch, from the browser clock
pub fn now_epoch_ms() -> i64 {
    js_sys::Date::now() as i64
}

// The row for one history group, `index` is its position in the list
pub fn row_for(
    index: usize,
    group: &ConfigurationGroup,
    targets: &TargetsView,
    now_ms: i64,
) -> HistoryRow {
    let entry = &group.latest;

    let disabled_reason = match prefill_for(entry, targets) {
        Prefill::Unavailable { reason } => Some(reason),
        _ => None,
    };

    HistoryRow {
        index,
        when: relative_time(&entry.created_at, now_ms),
        absolute: format_when(&entry.created_at),
        runs: if group.count == 1 {
            "1 run".to_string()
        } else {
            format!("{} runs", group.count)
        },
        source: source_label(entry),
        project: project_name(&entry.path),
        command: command_summary(entry.startup.as_ref()),
        workload: workload_summary(entry),
        headers: headers_summary(entry),
        disabled_reason,
    }
}

fn source_label(entry: &ConfigurationEntry) -> String {
    match &entry.configuration {
        None => "Partly recorded".to_string(),
        Some(configuration) if configuration.source == "target" => {
            let name = configuration
                .target_name
                .clone()
                .or_else(|| configuration.target_id.clone())
                .unwrap_or_default();

            format!("Saved target: {name}")
        }
        Some(_) => "Custom".to_string(),
    }
}

fn project_name(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string()
}

fn command_summary(startup: Option<&StartupSummary>) -> String {
    let Some(startup) = startup else {
        return "No start command".to_string();
    };

    let mut parts: Vec<String> = Vec::new();

    if let Some(program) = &startup.program {
        let args = startup.args.clone().unwrap_or_default().join(" ");

        parts.push(format!("{program} {args}").trim().to_string());
    }

    if let Some(readiness) = &startup.readiness {
        parts.push(format!(":{} {}", readiness.port, readiness.path));
    }

    if parts.is_empty() {
        "No start command".to_string()
    } else {
        parts.join(", ")
    }
}

fn workload_summary(entry: &ConfigurationEntry) -> String {
    match entry
        .configuration
        .as_ref()
        .and_then(|configuration| configuration.workload.as_ref())
    {
        Some(workload) if workload.mode == "duration" => format!(
            "{} s, {} concurrent",
            workload.duration_seconds.unwrap_or_default(),
            workload.concurrency
        ),
        Some(workload) => format!(
            "{} requests, {} concurrent",
            workload.requests.unwrap_or_default(),
            workload.concurrency
        ),
        None if entry.recorded => "No workload".to_string(),
        None => "Workload not recorded".to_string(),
    }
}

fn headers_summary(entry: &ConfigurationEntry) -> String {
    if !entry.recorded {
        return "Headers not recorded".to_string();
    }

    format!("{} headers", header_names_of(entry).len())
}

// The list of previous configurations. Escape closes it.
#[component]
pub fn HistoryPanel(
    rows: Vec<HistoryRow>,
    on_use: EventHandler<usize>,
    on_close: EventHandler<()>,
) -> Element {
    let primary = button_class(ButtonVariant::Primary);
    let secondary = button_class(ButtonVariant::Secondary);

    rsx! {
        div {
            onkeydown: move |event| {
                if format!("{:?}", event.key()) == "Escape" {
                    on_close.call(());
                }
            },
            Card {
                CardHeader {
                    title: "Previous runs",
                    subtitle: "Runs with the same settings are grouped. Choosing one fills the form. Nothing starts until you press Run.",
                }
                if rows.is_empty() {
                    p { class: "p-(--space-4) text-(length:--fs-sm) text-(color:--clr-text-muted)",
                        "No benchmark runs yet."
                    }
                }
                div { class: "flex flex-col divide-y divide-(color:--clr-border)",
                    for row in rows.iter() {
                        {
                            let index = row.index;
                            rsx! {
                                div { key: "{index}", class: "flex flex-col gap-(--space-2) p-(--space-4)",
                                    div { class: "flex flex-wrap items-center gap-(--space-2) text-(length:--fs-sm) text-(color:--clr-text)",
                                        span { class: "font-(weight:--fw-medium)", title: "{row.absolute}", "{row.when}" }
                                        span { class: "text-(color:--clr-text-muted)", "{row.runs}" }
                                        span { class: "rounded-(--round-sm) border border-(color:--clr-border) px-(--space-2) text-(length:--fs-xs) text-(color:--clr-text-muted)",
                                            "{row.source}"
                                        }
                                    }
                                    p { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)",
                                        "{row.project} · {row.command}"
                                    }
                                    p { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)",
                                        "{row.workload} · {row.headers}"
                                    }
                                    if let Some(reason) = &row.disabled_reason {
                                        p { class: "text-(length:--fs-xs) text-(color:--clr-danger)", "{reason}" }
                                    }
                                    div {
                                        button {
                                            r#type: "button",
                                            class: "{primary}",
                                            disabled: row.disabled_reason.is_some(),
                                            onclick: move |_| on_use.call(index),
                                            "Use these settings"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "flex justify-end border-t border-(color:--clr-border) p-(--space-3)",
                    button {
                        r#type: "button",
                        class: "{secondary}",
                        onclick: move |_| on_close.call(()),
                        "Close"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_the_time_of_a_run_for_the_list() {
        assert_eq!(format_when("2026-10-21T14:03:11+00:00"), "21 Oct, 14:03");
        assert_eq!(format_when("not a date"), "not a date");
    }
}
