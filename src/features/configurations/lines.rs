use super::models::{ConfigurationEntry, RecordedConfiguration, StartupSummary};

// The settings of a run as label and value pairs. Header names only, never values.

// "bun run dev", or "None" when the run had no start command
pub fn command_text(startup: Option<&StartupSummary>) -> String {
    match startup.and_then(|startup| startup.program.as_ref()) {
        Some(program) => {
            let args = startup
                .and_then(|startup| startup.args.clone())
                .unwrap_or_default()
                .join(" ");

            format!("{program} {args}").trim().to_string()
        }
        None => "None".to_string(),
    }
}

// "127.0.0.1:5000 /health (GET → 200)", or "None" when no readiness was recorded
pub fn readiness_text(startup: Option<&StartupSummary>) -> String {
    match startup.and_then(|startup| startup.readiness.as_ref()) {
        Some(readiness) => format!(
            "{}:{} {} ({} → {})",
            readiness.host,
            readiness.port,
            readiness.path,
            readiness.method,
            readiness.expected_status
        ),
        None => "None".to_string(),
    }
}

// The source of a recorded run. A saved target shows its name and its id.
fn source_text(config: &RecordedConfiguration) -> String {
    if config.source != "target" {
        return "Custom".to_string();
    }

    match (config.target_name.as_deref(), config.target_id.as_deref()) {
        (Some(name), Some(id)) => format!("Saved target: {name} ({id})"),
        (None, Some(id)) => format!("Saved target: {id}"),
        (Some(name), None) => format!("Saved target: {name}"),
        (None, None) => "Saved target".to_string(),
    }
}

fn readiness_headers_text(startup: Option<&StartupSummary>) -> String {
    match startup.and_then(|startup| startup.readiness.as_ref()) {
        Some(readiness) => match &readiness.header_names {
            Some(names) => names_text(names),
            None => "Server health credentials".to_string(),
        },
        None => "None".to_string(),
    }
}

// Every setting of a run with a recorded configuration
pub fn configuration_lines(config: &RecordedConfiguration) -> Vec<(String, String)> {
    let startup = config.startup.as_ref();

    let mut lines = vec![
        ("Source".to_string(), source_text(config)),
        ("Path".to_string(), config.target_path.clone()),
        ("Start command".to_string(), command_text(startup)),
        ("Readiness".to_string(), readiness_text(startup)),
        (
            "Readiness headers".to_string(),
            readiness_headers_text(startup),
        ),
    ];

    match &config.workload {
        Some(workload) => {
            let amount = if workload.mode == "duration" {
                format!("{} s", workload.duration_seconds.unwrap_or_default())
            } else {
                format!("{} requests", workload.requests.unwrap_or_default())
            };

            lines.push((
                "Workload".to_string(),
                format!("{} {}", workload.method, workload.path),
            ));
            lines.push(("Amount".to_string(), amount));
            lines.push(("Concurrency".to_string(), workload.concurrency.to_string()));
            lines.push((
                "Workload headers".to_string(),
                names_text(&workload.header_names),
            ));
        }
        None => lines.push(("Workload".to_string(), "None".to_string())),
    }

    lines
}

// The settings of a run stored before configurations were recorded, from the existing tables
pub fn legacy_lines(entry: &ConfigurationEntry) -> Vec<(String, String)> {
    vec![
        ("Source".to_string(), "Unknown".to_string()),
        ("Path".to_string(), entry.path.clone()),
        (
            "Start command".to_string(),
            command_text(entry.startup.as_ref()),
        ),
        (
            "Readiness".to_string(),
            readiness_text(entry.startup.as_ref()),
        ),
        ("Readiness headers".to_string(), "Not recorded".to_string()),
        (
            "Workload".to_string(),
            "Not recorded (made before configurations were recorded)".to_string(),
        ),
    ]
}

fn names_text(names: &[String]) -> String {
    if names.is_empty() {
        "None".to_string()
    } else {
        names.join(", ")
    }
}

// One setting that differs between two runs
#[derive(Debug, Clone, PartialEq)]
pub struct FieldChange {
    pub label: String,
    pub baseline: String,
    pub comparison: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigurationComparison {
    // Both runs recorded the same settings
    Same,
    // Both runs recorded their settings, and these settings differ
    Changed(Vec<FieldChange>),
    // A run has no recorded workload. Only the start command and readiness were compared, and
    // `changes` lists the differences in them. `unrecorded` holds the ids of those runs.
    Partial {
        unrecorded: Vec<String>,
        changes: Vec<FieldChange>,
    },
}

pub fn compare_entries(
    baseline: &ConfigurationEntry,
    comparison: &ConfigurationEntry,
) -> ConfigurationComparison {
    match (&baseline.configuration, &comparison.configuration) {
        (Some(left), Some(right)) => {
            let changes = differences(&configuration_lines(left), &configuration_lines(right));

            if changes.is_empty() {
                ConfigurationComparison::Same
            } else {
                ConfigurationComparison::Changed(changes)
            }
        }
        _ => {
            let mut unrecorded = Vec::new();

            if baseline.configuration.is_none() {
                unrecorded.push(baseline.run_id.clone());
            }

            if comparison.configuration.is_none() {
                unrecorded.push(comparison.run_id.clone());
            }

            let changes = differences(&partial_lines(baseline), &partial_lines(comparison));

            ConfigurationComparison::Partial {
                unrecorded,
                changes,
            }
        }
    }
}

// The start command and readiness of a run, the only settings every run has
fn partial_lines(entry: &ConfigurationEntry) -> Vec<(String, String)> {
    let all = match &entry.configuration {
        Some(config) => configuration_lines(config),
        None => legacy_lines(entry),
    };

    all.into_iter()
        .filter(|(label, _)| label.as_str() == "Start command" || label.as_str() == "Readiness")
        .collect()
}

// Labels whose values differ, the labels of the first run first, then labels only the second has
fn differences(left: &[(String, String)], right: &[(String, String)]) -> Vec<FieldChange> {
    let mut labels: Vec<&String> = left.iter().map(|(label, _)| label).collect();

    for (label, _) in right {
        if !labels.contains(&label) {
            labels.push(label);
        }
    }

    let mut changes = Vec::new();

    for label in labels {
        let baseline = value_of(left, label);
        let comparison = value_of(right, label);

        if baseline != comparison {
            changes.push(FieldChange {
                label: label.clone(),
                baseline: baseline.unwrap_or_else(|| "None".to_string()),
                comparison: comparison.unwrap_or_else(|| "None".to_string()),
            });
        }
    }

    changes
}

fn value_of(lines: &[(String, String)], label: &str) -> Option<String> {
    lines
        .iter()
        .find(|(existing, _)| existing.as_str() == label)
        .map(|(_, value)| value.clone())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn entry(run_id: &str, concurrency: Option<u64>) -> ConfigurationEntry {
        let configuration = match concurrency {
            Some(concurrency) => json!({
                "source": "custom", "target_id": null, "target_name": null,
                "target_path": "/p", "startup": null,
                "workload": {
                    "method": "GET", "path": "/w", "mode": "requests",
                    "requests": 10, "duration_seconds": null,
                    "concurrency": concurrency, "header_names": []
                }
            }),
            None => json!(null),
        };

        serde_json::from_value(json!({
            "run_id": run_id,
            "created_at": "2026-10-20T14:03:11+00:00",
            "path": "/p",
            "startup": null,
            "recorded": concurrency.is_some(),
            "configuration": configuration
        }))
        .expect("valid entry")
    }

    #[test]
    fn identical_recorded_runs_are_the_same() {
        assert_eq!(
            compare_entries(&entry("a", Some(5)), &entry("b", Some(5))),
            ConfigurationComparison::Same
        );
    }

    #[test]
    fn a_changed_setting_shows_both_values() {
        assert_eq!(
            compare_entries(&entry("a", Some(5)), &entry("b", Some(7))),
            ConfigurationComparison::Changed(vec![FieldChange {
                label: "Concurrency".to_string(),
                baseline: "5".to_string(),
                comparison: "7".to_string(),
            }])
        );
    }

    #[test]
    fn an_unrecorded_run_compares_only_its_start_command_and_readiness() {
        let comparison = compare_entries(&entry("old", None), &entry("new", Some(5)));

        assert_eq!(
            comparison,
            ConfigurationComparison::Partial {
                unrecorded: vec!["old".to_string()],
                changes: Vec::new(),
            }
        );
    }
}
