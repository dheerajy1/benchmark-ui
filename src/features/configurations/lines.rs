use super::models::{ConfigurationDetail, RecordedConfiguration};

// The settings of a recorded run as label and value pairs. Header names only, never values.
pub fn configuration_lines(config: &RecordedConfiguration) -> Vec<(String, String)> {
    let source = if config.source == "target" {
        let name = config
            .target_name
            .clone()
            .or_else(|| config.target_id.clone())
            .unwrap_or_default();

        format!("Saved target: {name}")
    } else {
        "Custom".to_string()
    };

    let mut lines = vec![
        ("Source".to_string(), source),
        ("Path".to_string(), config.target_path.clone()),
    ];

    let startup = config.startup.as_ref();

    let command = match startup.and_then(|startup| startup.program.as_ref()) {
        Some(program) => {
            let args = startup
                .and_then(|startup| startup.args.clone())
                .unwrap_or_default()
                .join(" ");

            format!("{program} {args}").trim().to_string()
        }
        None => "None".to_string(),
    };

    lines.push(("Start command".to_string(), command));

    let readiness = startup.and_then(|startup| startup.readiness.as_ref());

    let readiness_text = match readiness {
        Some(readiness) => format!(
            "{}:{} {} ({} → {})",
            readiness.host,
            readiness.port,
            readiness.path,
            readiness.method,
            readiness.expected_status
        ),
        None => "None".to_string(),
    };

    lines.push(("Readiness".to_string(), readiness_text));

    let readiness_headers = match readiness {
        Some(readiness) => match &readiness.header_names {
            Some(names) => names_text(names),
            None => "Server health credentials".to_string(),
        },
        None => "None".to_string(),
    };

    lines.push(("Readiness headers".to_string(), readiness_headers));

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

fn names_text(names: &[String]) -> String {
    if names.is_empty() {
        "None".to_string()
    } else {
        names.join(", ")
    }
}

// Whether two runs used the same settings
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigurationDifference {
    // Both runs recorded the same settings
    Same,
    // At least one run has no recorded configuration, so the settings cannot be checked
    NotComparable,
    // The recorded settings differ in these fields, by label
    Differs(Vec<String>),
}

pub fn compare_configurations(
    baseline: &ConfigurationDetail,
    comparison: &ConfigurationDetail,
) -> ConfigurationDifference {
    let (Some(left), Some(right)) = (&baseline.configuration, &comparison.configuration) else {
        return ConfigurationDifference::NotComparable;
    };

    let left_lines = configuration_lines(left);
    let right_lines = configuration_lines(right);

    let mut changed: Vec<String> = Vec::new();

    for (label, value) in &left_lines {
        let other = right_lines
            .iter()
            .find(|(other_label, _)| other_label == label)
            .map(|(_, other_value)| other_value);

        if other != Some(value) {
            changed.push(label.clone());
        }
    }

    for (label, _) in &right_lines {
        if !left_lines.iter().any(|(existing, _)| existing == label) {
            changed.push(label.clone());
        }
    }

    if changed.is_empty() {
        ConfigurationDifference::Same
    } else {
        ConfigurationDifference::Differs(changed)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn detail(concurrency: u64) -> ConfigurationDetail {
        serde_json::from_value(json!({
            "run_id": "r",
            "recorded": true,
            "configuration": {
                "source": "custom", "target_id": null, "target_name": null,
                "target_path": "/p", "startup": null,
                "workload": {
                    "method": "GET", "path": "/w", "mode": "requests",
                    "requests": 10, "duration_seconds": null,
                    "concurrency": concurrency, "header_names": []
                }
            }
        }))
        .expect("valid detail")
    }

    fn not_recorded() -> ConfigurationDetail {
        serde_json::from_value(json!({ "run_id": "old", "recorded": false, "configuration": null }))
            .expect("valid detail")
    }

    #[test]
    fn identical_settings_are_the_same() {
        assert_eq!(
            compare_configurations(&detail(5), &detail(5)),
            ConfigurationDifference::Same
        );
    }

    #[test]
    fn a_changed_field_is_named() {
        assert_eq!(
            compare_configurations(&detail(5), &detail(7)),
            ConfigurationDifference::Differs(vec!["Concurrency".to_string()])
        );
    }

    #[test]
    fn a_run_without_a_configuration_is_not_comparable() {
        assert_eq!(
            compare_configurations(&not_recorded(), &detail(5)),
            ConfigurationDifference::NotComparable
        );
    }
}
