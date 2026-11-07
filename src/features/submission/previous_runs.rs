use dioxus::prelude::*;

use super::prefill::{prefill_for, Prefill};
use crate::{
    components::Card,
    features::{
        charts::{ago_labels, Sparkline},
        configurations::{ConfigurationEntry, ConfigurationGroup, StartupSummary},
        history::RunSummary,
        targets::TargetsView,
    },
    format::{format_optional, relative_time, ValueFormat},
};

// Most cards the side panel lists
const MAX_CARDS: usize = 5;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

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

// The numbers of the runs in one group, oldest first
#[derive(Debug, Clone, PartialEq)]
pub struct Trend {
    pub name: &'static str,
    pub format: ValueFormat,
    pub values: Vec<Option<f64>>,
    pub labels: Vec<String>,
    // The newest value, ready to show
    pub latest: String,
}

// One mini card, already worded for display
#[derive(Debug, Clone, PartialEq)]
pub struct RunCard {
    // Position of the group in the full list, what the page fills the form from
    pub index: usize,
    pub when: String,
    pub absolute: String,
    pub source: String,
    // For example `3 runs · 60 s · 50 concurrency`
    pub header: String,
    pub detail: String,
    pub trend: Option<Trend>,
    // Why the card cannot fill the form
    pub disabled_reason: Option<String>,
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

// The workload of a run as `60 s · 50 concurrency`, `None` when it was not recorded
fn workload_text(entry: &ConfigurationEntry) -> Option<String> {
    match entry
        .configuration
        .as_ref()
        .and_then(|configuration| configuration.workload.as_ref())
    {
        Some(workload) if workload.mode == "duration" => Some(format!(
            "{} s · {} concurrency",
            workload.duration_seconds.unwrap_or_default(),
            workload.concurrency
        )),
        Some(workload) => Some(format!(
            "{} requests · {} concurrency",
            workload.requests.unwrap_or_default(),
            workload.concurrency
        )),
        None if entry.recorded => Some("no workload".to_string()),
        None => None,
    }
}

// `3 runs · 60 s · 50 concurrency`
fn header_line(group: &ConfigurationGroup) -> String {
    let runs = if group.count == 1 {
        "1 run".to_string()
    } else {
        format!("{} runs", group.count)
    };

    match workload_text(&group.latest) {
        Some(workload) => format!("{runs} · {workload}"),
        None => runs,
    }
}

fn same_path(left: &str, right: &str) -> bool {
    left.trim_end_matches('/') == right.trim_end_matches('/')
}

// Throughput of each run of the group, oldest first. A group with no throughput at all falls
// back to p95, a group with neither has no trend.
fn trend_for(group: &ConfigurationGroup, items: &[RunSummary]) -> Option<Trend> {
    let runs: Vec<Option<&RunSummary>> = group
        .run_ids
        .iter()
        .rev()
        .map(|id| items.iter().find(|item| item.run_id == *id))
        .collect();

    let series = |pick: fn(&RunSummary) -> Option<f64>| -> Vec<Option<f64>> {
        runs.iter().map(|run| run.and_then(pick)).collect()
    };

    let throughput = series(|run| run.summary.requests_per_second);
    let p95 = series(|run| run.summary.p95_ms);

    let (name, format, values) = if throughput.iter().any(Option::is_some) {
        ("Throughput", ValueFormat::PerSecond, throughput)
    } else if p95.iter().any(Option::is_some) {
        ("p95", ValueFormat::LatencyMs, p95)
    } else {
        return None;
    };

    Some(Trend {
        name,
        format,
        latest: format_optional(values.last().copied().flatten(), format),
        labels: ago_labels(values.len()),
        values,
    })
}

// The cards for the side panel. With a path only the groups of that project are listed.
pub fn cards(
    groups: &[ConfigurationGroup],
    items: &[RunSummary],
    targets: &TargetsView,
    path: &str,
    now_ms: i64,
) -> Vec<RunCard> {
    let path = path.trim();

    groups
        .iter()
        .enumerate()
        .filter(|(_, group)| path.is_empty() || same_path(&group.latest.path, path))
        .take(MAX_CARDS)
        .map(|(index, group)| {
            let entry = &group.latest;
            let disabled_reason = match prefill_for(entry, targets) {
                Prefill::Unavailable { reason } => Some(reason),
                _ => None,
            };

            RunCard {
                index,
                when: relative_time(&entry.created_at, now_ms),
                absolute: format_when(&entry.created_at),
                source: source_label(entry),
                header: header_line(group),
                detail: format!(
                    "{} · {}",
                    project_name(&entry.path),
                    command_summary(entry.startup.as_ref())
                ),
                trend: trend_for(group, items),
                disabled_reason,
            }
        })
        .collect()
}

// Mini cards of earlier runs. Choosing one fills the form, nothing starts.
#[component]
pub fn PreviousRuns(cards: Vec<RunCard>, on_use: EventHandler<usize>) -> Element {
    rsx! {
        Card { class: "flex flex-col gap-(--space-3) p-(--space-4)",
            div { class: "flex flex-col",
                h2 { class: "text-(length:--fs-base) font-(weight:--fw-semibold) text-(color:--clr-text)",
                    "Previous runs"
                }
                p { class: "text-(length:--fs-xs) text-(color:--clr-text-muted)",
                    "Runs with the same settings are grouped. Choosing one fills the form."
                }
            }
            if cards.is_empty() {
                p { class: "text-(length:--fs-sm) text-(color:--clr-text-faint)",
                    "No previous runs for this project yet."
                }
            }
            for card in cards {
                {
                    let index = card.index;
                    let blocked = card.disabled_reason.is_some();
                    let tone = if blocked {
                        "cursor-not-allowed opacity-60"
                    } else {
                        "cursor-pointer hover:border-(color:--clr-accent) hover:bg-(--clr-surface-hover)"
                    };

                    rsx! {
                        button {
                            key: "{index}",
                            r#type: "button",
                            class: "group flex w-full flex-col gap-(--space-2) rounded-(--round-md) border border-(color:--clr-border) bg-(--clr-surface-raised) p-(--space-3) text-left transition-colors {tone}",
                            disabled: blocked,
                            onclick: move |_| on_use.call(index),
                            div { class: "flex items-center justify-between gap-(--space-2) text-(length:--fs-xs)",
                                span {
                                    class: "font-(weight:--fw-medium) text-(color:--clr-text)",
                                    title: "{card.absolute}",
                                    "{card.when}"
                                }
                                span { class: "truncate text-(color:--clr-text-muted)", "{card.source}" }
                            }
                            span { class: "text-(length:--fs-sm) text-(color:--clr-text)", "{card.header}" }
                            span { class: "truncate text-(length:--fs-xs) text-(color:--clr-text-muted)",
                                "{card.detail}"
                            }
                            if let Some(trend) = card.trend {
                                div { class: "flex items-baseline justify-between text-(length:--fs-xs) text-(color:--clr-text-muted)",
                                    span { "{trend.name}" }
                                    span { class: "tabular-nums text-(color:--clr-text)", "{trend.latest}" }
                                }
                                if trend.values.len() >= 2 {
                                    Sparkline {
                                        values: trend.values,
                                        labels: trend.labels,
                                        format: trend.format,
                                        class: "h-8",
                                    }
                                }
                            }
                            if let Some(reason) = card.disabled_reason {
                                span { class: "text-(length:--fs-xs) text-(color:--clr-danger)", "{reason}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::features::configurations::group;

    fn entry(run_id: &str, path: &str, mode: Option<&str>) -> ConfigurationEntry {
        let configuration = mode.map(|mode| {
            json!({
                "source": "custom", "target_id": null, "target_name": null,
                "target_path": path, "startup": null,
                "workload": {
                    "method": "GET", "path": "/w", "mode": mode,
                    "requests": 500, "duration_seconds": 60, "concurrency": 50,
                    "header_names": []
                }
            })
        });

        serde_json::from_value(json!({
            "run_id": run_id,
            "created_at": "2026-10-20T14:03:11+00:00",
            "path": path,
            "startup": null,
            "recorded": mode.is_some(),
            "configuration": configuration,
            "workload": null
        }))
        .expect("the fixture is a valid entry")
    }

    fn summary(run_id: &str, rps: Option<f64>, p95: Option<f64>) -> RunSummary {
        serde_json::from_value(json!({
            "run_id": run_id,
            "status": "done",
            "display_name": "api",
            "target": { "path": "/p" },
            "discovery": { "runtime": null },
            "created_at": "2026-10-20T14:03:11+00:00",
            "summary": { "requests_per_second": rps, "p95_ms": p95 }
        }))
        .expect("the fixture is a valid summary")
    }

    #[test]
    fn formats_the_time_of_a_run_for_the_card() {
        assert_eq!(format_when("2026-10-21T14:03:11+00:00"), "21 Oct, 14:03");
        assert_eq!(format_when("not a date"), "not a date");
    }

    #[test]
    fn the_header_names_the_runs_the_duration_and_the_concurrency() {
        let three = group(&[
            entry("c", "/p", Some("duration")),
            entry("b", "/p", Some("duration")),
            entry("a", "/p", Some("duration")),
        ]);
        let one = group(&[entry("a", "/p", Some("requests"))]);
        let none = group(&[entry("a", "/p", None)]);

        assert_eq!(header_line(&three[0]), "3 runs · 60 s · 50 concurrency");
        assert_eq!(
            header_line(&one[0]),
            "1 run · 500 requests · 50 concurrency"
        );
        assert_eq!(header_line(&none[0]), "1 run");
    }

    #[test]
    fn paths_match_without_a_trailing_slash() {
        assert!(same_path("/p/", "/p"));
        assert!(!same_path("/p", "/q"));
    }

    #[test]
    fn the_trend_runs_from_the_oldest_run_to_the_newest() {
        let groups = group(&[
            entry("c", "/p", Some("duration")),
            entry("b", "/p", Some("duration")),
            entry("a", "/p", Some("duration")),
        ]);
        let items = vec![
            summary("c", Some(300.0), None),
            summary("b", None, None),
            summary("a", Some(100.0), None),
        ];

        let trend = trend_for(&groups[0], &items).expect("there is a trend");

        assert_eq!(trend.name, "Throughput");
        assert_eq!(trend.values, vec![Some(100.0), None, Some(300.0)]);
        assert_eq!(trend.labels.len(), 3);
    }

    #[test]
    fn the_trend_falls_back_to_p95_and_then_to_nothing() {
        let groups = group(&[entry("a", "/p", Some("duration"))]);

        let p95 = trend_for(&groups[0], &[summary("a", None, Some(12.0))]).expect("p95 trend");
        let none = trend_for(&groups[0], &[summary("a", None, None)]);

        assert_eq!(p95.name, "p95");
        assert_eq!(none, None);
        assert_eq!(trend_for(&groups[0], &[]), None);
    }
}
