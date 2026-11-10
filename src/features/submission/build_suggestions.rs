use dioxus::prelude::*;

use super::form::FormState;
use crate::{components::Skeleton, features::configurations::ConfigurationEntry};

// Most chips shown, they stay on one line
const MAX_SUGGESTIONS: usize = 4;

// The chip row keeps this height in every state, so the form never shifts
const ROW_HEIGHT: &str = "h-7";

// A build the person used in the past, offered as a quick fill
#[derive(Debug, Clone, PartialEq)]
pub struct BuildSuggestion {
    pub command: String,
    // Empty when that run measured the default folder
    pub output_dir: String,
}

// The builds of the successful runs of this project path, newest first, each shown once. `runs`
// is newest first, as the API sends it. No path typed yet means no suggestions.
pub fn suggestions_for(runs: &[ConfigurationEntry], path: &str) -> Vec<BuildSuggestion> {
    let wanted = path.trim().trim_end_matches('/');

    if wanted.is_empty() {
        return Vec::new();
    }

    let mut found: Vec<BuildSuggestion> = Vec::new();

    for entry in runs {
        if entry.status != "done" || entry.path.trim_end_matches('/') != wanted {
            continue;
        }

        let Some(build) = entry
            .configuration
            .as_ref()
            .and_then(|configuration| configuration.build.as_ref())
        else {
            continue;
        };

        let suggestion = BuildSuggestion {
            command: build.command.clone(),
            output_dir: build.output_dir.clone().unwrap_or_default(),
        };

        if !found.contains(&suggestion) {
            found.push(suggestion);
        }

        if found.len() == MAX_SUGGESTIONS {
            break;
        }
    }

    found
}

// The builds of past successful runs of the typed project path. A click fills the build command
// and the output folder. Always four equal slots in one fixed-height row, so nothing moves in any
// state: skeletons while the history loads, a chip in each slot that has a build, and a dashed
// empty pill in each slot that has none.
#[component]
pub fn BuildChips(
    form: Signal<FormState>,
    suggestions: Vec<BuildSuggestion>,
    loading: bool,
    disabled: bool,
) -> Element {
    let mut form = form;
    let state = form.read().clone();
    let empty_slots = MAX_SUGGESTIONS.saturating_sub(suggestions.len());
    let hint = if !loading && suggestions.is_empty() {
        "Past successful builds for this path will appear here."
    } else {
        ""
    };

    rsx! {
        div {
            class: "grid {ROW_HEIGHT} grid-cols-4 gap-(--space-2)",
            title: "{hint}",
            if loading {
                for slot in 0..MAX_SUGGESTIONS {
                    Skeleton {
                        key: "{slot}",
                        class: "{ROW_HEIGHT} w-full rounded-(--round-full)",
                    }
                }
            } else {
                for suggestion in suggestions {
                    {
                        let active = state.build_command.trim() == suggestion.command
                            && state.build_output_dir.trim() == suggestion.output_dir;
                        let class = if active {
                            "border-(color:--clr-accent) bg-(--clr-accent-soft) text-(color:--clr-accent)"
                        } else {
                            "border-(color:--clr-border) text-(color:--clr-text-muted) hover:border-(color:--clr-accent) hover:text-(color:--clr-accent)"
                        };
                        let title = if suggestion.output_dir.is_empty() {
                            suggestion.command.clone()
                        } else {
                            format!("{} · output {}", suggestion.command, suggestion.output_dir)
                        };
                        let label = suggestion.command.clone();
                        let picked = suggestion.clone();

                        rsx! {
                            button {
                                key: "{suggestion.command}|{suggestion.output_dir}",
                                r#type: "button",
                                title: "{title}",
                                disabled,
                                class: "{ROW_HEIGHT} w-full min-w-0 cursor-pointer truncate rounded-(--round-full) border px-(--space-3) font-mono text-(length:--fs-xs) transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-60 {class}",
                                onclick: move |_| {
                                    let picked = picked.clone();

                                    form.with_mut(|state| {
                                        state.build_command = picked.command;
                                        state.build_output_dir = picked.output_dir;
                                    });
                                },
                                "{label}"
                            }
                        }
                    }
                }
                for slot in 0..empty_slots {
                    div {
                        key: "empty-{slot}",
                        aria_hidden: "true",
                        class: "{ROW_HEIGHT} w-full rounded-(--round-full) border border-dashed border-(color:--clr-border)",
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

    fn run(path: &str, status: &str, build: Option<(&str, Option<&str>)>) -> ConfigurationEntry {
        let configuration = match build {
            Some((command, output_dir)) => json!({
                "source": "custom", "target_id": null, "target_name": null,
                "target_path": path, "startup": null, "workload": null,
                "build": { "command": command, "output_dir": output_dir }
            }),
            None => json!(null),
        };

        serde_json::from_value(json!({
            "run_id": "r",
            "created_at": "2026-11-09T10:00:00+00:00",
            "status": status,
            "path": path,
            "startup": null,
            "recorded": true,
            "configuration": configuration
        }))
        .expect("valid entry")
    }

    #[test]
    fn only_successful_runs_of_the_same_path_are_offered() {
        let runs = vec![
            run("/a", "failed", Some(("bad build", None))),
            run("/b", "done", Some(("other project", None))),
            run(
                "/a/",
                "done",
                Some(("cargo xtask build", Some("target/release"))),
            ),
            run("/a", "cancelled", Some(("cancelled build", None))),
            run("/a", "done", None),
        ];

        let found = suggestions_for(&runs, " /a ");

        assert_eq!(
            found,
            vec![BuildSuggestion {
                command: "cargo xtask build".to_string(),
                output_dir: "target/release".to_string(),
            }]
        );
    }

    #[test]
    fn a_build_is_offered_once_and_at_most_four_are_shown() {
        let mut runs = vec![
            run("/a", "done", Some(("one", None))),
            run("/a", "done", Some(("one", None))),
        ];

        for name in ["two", "three", "four", "five"] {
            runs.push(run("/a", "done", Some((name, None))));
        }

        let found = suggestions_for(&runs, "/a");

        assert_eq!(found.len(), MAX_SUGGESTIONS);
        assert_eq!(found[0].command, "one");
        assert_eq!(found[1].command, "two");
    }

    #[test]
    fn no_typed_path_means_no_suggestions() {
        let runs = vec![run("/a", "done", Some(("one", None)))];

        assert!(suggestions_for(&runs, "  ").is_empty());
    }
}
