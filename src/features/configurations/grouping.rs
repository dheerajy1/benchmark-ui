use std::collections::BTreeSet;

use super::models::ConfigurationEntry;

// The history list shows at most this many rows
const MAX_ROWS: usize = 20;

// Runs with identical settings, shown as one row
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigurationGroup {
    // The newest run of the group, "Use these settings" fills the form from it
    pub latest: ConfigurationEntry,
    pub count: usize,
}

// Collapses runs with identical configurations. The input is newest first, so each group's
// latest run is its first member. Runs stored before configurations were recorded group only
// with runs that share their path, start command and readiness.
pub fn group(entries: &[ConfigurationEntry]) -> Vec<ConfigurationGroup> {
    let mut groups: Vec<(String, ConfigurationGroup)> = Vec::new();

    for entry in entries {
        let key = group_key(entry);

        match groups.iter().position(|(existing, _)| *existing == key) {
            Some(index) => groups[index].1.count += 1,
            None => groups.push((
                key,
                ConfigurationGroup {
                    latest: entry.clone(),
                    count: 1,
                },
            )),
        }
    }

    groups
        .into_iter()
        .map(|(_, group)| group)
        .take(MAX_ROWS)
        .collect()
}

// The settings that identify a configuration. The run id and the time are left out on purpose.
// A key that cannot be built falls back to the run id, so it never merges with another run.
fn group_key(entry: &ConfigurationEntry) -> String {
    serde_json::to_string(&(
        &entry.path,
        &entry.startup,
        entry.recorded,
        &entry.configuration,
    ))
    .unwrap_or_else(|_| entry.run_id.clone())
}

// Distinct header names of a run, used for the header count
pub fn header_names_of(entry: &ConfigurationEntry) -> BTreeSet<String> {
    let mut names = BTreeSet::new();

    let readiness = entry
        .startup
        .as_ref()
        .and_then(|startup| startup.readiness.as_ref())
        .and_then(|readiness| readiness.header_names.as_ref());

    if let Some(readiness) = readiness {
        names.extend(readiness.iter().cloned());
    }

    if let Some(workload) = entry
        .configuration
        .as_ref()
        .and_then(|configuration| configuration.workload.as_ref())
    {
        names.extend(workload.header_names.iter().cloned());
    }

    names
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn entry(run_id: &str, path: &str, recorded: bool) -> ConfigurationEntry {
        serde_json::from_value(json!({
            "run_id": run_id,
            "created_at": "2026-10-20T14:03:11+00:00",
            "path": path,
            "startup": null,
            "recorded": recorded,
            "configuration": null
        }))
        .expect("the fixture is a valid entry")
    }

    #[test]
    fn identical_runs_collapse_into_one_row_with_their_count() {
        let entries = vec![
            entry("newest", "/a", false),
            entry("older", "/a", false),
            entry("other", "/b", false),
        ];

        let groups = group(&entries);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].latest.run_id, "newest");
        assert_eq!(groups[0].count, 2);
        assert_eq!(groups[1].count, 1);
    }

    #[test]
    fn at_most_twenty_rows_are_shown() {
        let entries: Vec<ConfigurationEntry> = (0..25)
            .map(|index| entry(&format!("run-{index}"), &format!("/p{index}"), false))
            .collect();

        assert_eq!(group(&entries).len(), MAX_ROWS);
    }

    #[test]
    fn distinct_header_names_are_counted_once() {
        let entry: ConfigurationEntry = serde_json::from_value(json!({
            "run_id": "r",
            "created_at": "2026-10-20T14:03:11+00:00",
            "path": "/p",
            "startup": {
                "program": "bun",
                "args": ["run"],
                "readiness": {
                    "readiness_type": "http", "host": "127.0.0.1", "port": 5000,
                    "path": "/h", "method": "GET", "expected_status": 200,
                    "header_names": ["X-Client-Secret"]
                }
            },
            "recorded": true,
            "configuration": {
                "source": "custom", "target_id": null, "target_name": null,
                "target_path": "/p", "startup": null,
                "workload": {
                    "method": "GET", "path": "/w", "mode": "requests",
                    "requests": 10, "duration_seconds": null, "concurrency": 2,
                    "header_names": ["X-Client-Secret", "X-Client-Id"]
                }
            }
        }))
        .expect("valid");

        assert_eq!(header_names_of(&entry).len(), 2);
    }
}
