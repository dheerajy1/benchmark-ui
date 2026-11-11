use serde::Deserialize;

use super::{form::FormState, target_run::TargetForm};
use crate::{
    features::{
        connection::{get_json, ApiError},
        live::Stage,
        targets::TargetView,
    },
    format::format_duration_ms,
};

// How long each stage took, averaged over the newest done runs of one project
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct StageAverages {
    pub runs_used: u32,
    pub stages: Vec<StageAverage>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StageAverage {
    pub stage: String,
    pub average_ms: u64,
    // The time in the newest run, null when that run has none
    pub last_ms: Option<u64>,
}

// GET /api/v1/benchmarks/stage-averages?target_path=
pub async fn fetch_stage_averages(path: &str) -> Result<StageAverages, ApiError> {
    get_json(
        "/benchmarks/stage-averages",
        &[("target_path", path.to_string())],
    )
    .await
}

// One step of the timeline in the side panel
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineRow {
    pub label: &'static str,
    // The usual time, `off` for a step this run leaves out, an en dash when it is not known
    pub time: String,
    // Length of the bar, 0 to 100, relative to the longest step
    pub share: u32,
    pub class: &'static str,
}

// Every step in run order, and whether this run does it
fn plan(startup: bool, workload: bool, workspaces: bool, cleanup: bool) -> Vec<(Stage, bool)> {
    Stage::ALL
        .into_iter()
        .map(|stage| {
            let on = match stage {
                Stage::Startup => startup,
                Stage::Baseline | Stage::Workload => workload,
                Stage::Workspaces => workspaces,
                Stage::Cleanup => cleanup,
                _ => true,
            };

            (stage, on)
        })
        .collect()
}

// The steps of a custom run, from the switches of the form
pub fn custom_plan(state: &FormState) -> Vec<(Stage, bool)> {
    plan(
        state.startup_enabled,
        state.workload_enabled,
        state.workspace.enabled,
        !state.cleanup_command.trim().is_empty()
            || (state.workspace.enabled
                && state.workspace.docker_enabled
                && !state.workspace.docker_cleanup_command.trim().is_empty()),
    )
}

// The steps of a saved target run
pub fn target_plan(target: &TargetView, form: &TargetForm) -> Vec<(Stage, bool)> {
    plan(
        target.startup.is_some(),
        form.has_workload,
        !target.workspaces.is_empty(),
        target.has_cleanup(),
    )
}

// The usual time of a stage. A stage with parts, such as `workspaces:name:native`, adds them up
// unless the stage itself is listed.
pub fn average_for(stage: Stage, averages: &StageAverages) -> Option<u64> {
    let key = stage.key();
    let prefix = format!("{key}:");

    if let Some(exact) = averages.stages.iter().find(|entry| entry.stage == key) {
        return Some(exact.average_ms);
    }

    let parts: Vec<u64> = averages
        .stages
        .iter()
        .filter(|entry| entry.stage.starts_with(&prefix))
        .map(|entry| entry.average_ms)
        .collect();

    if parts.is_empty() {
        None
    } else {
        Some(parts.iter().sum())
    }
}

// The time of a stage in the newest run, parts of a stage add up like they do for the average
pub fn last_for(stage: Stage, averages: &StageAverages) -> Option<u64> {
    let key = stage.key();
    let prefix = format!("{key}:");

    if let Some(exact) = averages.stages.iter().find(|entry| entry.stage == key) {
        return exact.last_ms;
    }

    let parts: Vec<u64> = averages
        .stages
        .iter()
        .filter(|entry| entry.stage.starts_with(&prefix))
        .filter_map(|entry| entry.last_ms)
        .collect();

    if parts.is_empty() {
        None
    } else {
        Some(parts.iter().sum())
    }
}

// The rows of the timeline. Steps that are off stay in place, greyed out.
pub fn timeline(plan: &[(Stage, bool)], averages: Option<&StageAverages>) -> Vec<TimelineRow> {
    let times: Vec<Option<u64>> = plan
        .iter()
        .map(|(stage, on)| {
            if *on {
                averages.and_then(|known| average_for(*stage, known))
            } else {
                None
            }
        })
        .collect();
    let longest = times.iter().flatten().copied().max().unwrap_or(0);

    plan.iter()
        .zip(times)
        .map(|((stage, on), time)| {
            let share = match (time, longest) {
                (Some(value), longest) if longest > 0 => {
                    ((value as f64 / longest as f64) * 100.0).round() as u32
                }
                _ => 0,
            };
            let text = if !on {
                "off".to_string()
            } else {
                match time {
                    Some(value) => format_duration_ms(value as f64),
                    None => "–".to_string(),
                }
            };

            TimelineRow {
                label: stage.label(),
                time: text,
                share,
                class: if *on { "opacity-100" } else { "opacity-40" },
            }
        })
        .collect()
}

// The usual time of the steps that are on, `None` when no run is known or none has a time
fn total_ms(plan: &[(Stage, bool)], averages: &StageAverages) -> Option<u64> {
    if averages.runs_used == 0 {
        return None;
    }

    let total: u64 = plan
        .iter()
        .filter(|(_, on)| *on)
        .filter_map(|(stage, _)| average_for(*stage, averages))
        .sum();

    (total > 0).then_some(total)
}

// The usual total time of a run that is under way, from what its request includes
pub fn live_total_ms(
    workload: bool,
    workspaces: bool,
    cleanup: bool,
    averages: &StageAverages,
) -> Option<u64> {
    total_ms(&plan(true, workload, workspaces, cleanup), averages)
}

// The estimate for the whole run: the usual time of the steps it does, or `unknown`
pub fn estimate_text(plan: &[(Stage, bool)], averages: Option<&StageAverages>) -> String {
    let Some((known, total)) =
        averages.and_then(|known| total_ms(plan, known).map(|total| (known, total)))
    else {
        return "unknown".to_string();
    };

    let basis = if known.runs_used == 1 {
        "the last run".to_string()
    } else {
        format!("the last {} runs", known.runs_used)
    };

    format!("About {} · from {basis}", format_duration_ms(total as f64))
}

// The rows and the estimate for the side panel. `None` means the answer has not arrived yet.
pub fn panel_parts(
    plan: &[(Stage, bool)],
    answer: Option<&Result<StageAverages, ApiError>>,
) -> (Vec<TimelineRow>, String) {
    let averages = answer.and_then(|result| result.as_ref().ok());
    let text = if answer.is_none() {
        "Estimating…".to_string()
    } else {
        estimate_text(plan, averages)
    };

    (timeline(plan, averages), text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn averages(runs_used: u32, stages: &[(&str, u64)]) -> StageAverages {
        StageAverages {
            runs_used,
            stages: stages
                .iter()
                .map(|(stage, average_ms)| StageAverage {
                    stage: stage.to_string(),
                    average_ms: *average_ms,
                    last_ms: None,
                })
                .collect(),
        }
    }

    fn on(plan: &[(Stage, bool)], stage: Stage) -> bool {
        plan.iter().any(|(entry, on)| *entry == stage && *on)
    }

    #[test]
    fn switches_decide_which_steps_are_on() {
        let mut state = FormState::default();
        let plan = custom_plan(&state);

        assert!(on(&plan, Stage::Build));
        assert!(on(&plan, Stage::Saving));
        assert!(!on(&plan, Stage::Startup));
        assert!(!on(&plan, Stage::Workload));
        assert!(!on(&plan, Stage::Workspaces));

        state.startup_enabled = true;
        state.workload_enabled = true;
        state.workspace.enabled = true;
        let plan = custom_plan(&state);

        assert!(on(&plan, Stage::Startup));
        assert!(on(&plan, Stage::Baseline));
        assert!(on(&plan, Stage::Workload));
        assert!(on(&plan, Stage::Workspaces));
    }

    #[test]
    fn no_runs_means_unknown() {
        let plan = custom_plan(&FormState::default());

        assert_eq!(estimate_text(&plan, None), "unknown");
        assert_eq!(estimate_text(&plan, Some(&averages(0, &[]))), "unknown");
    }

    #[test]
    fn the_estimate_adds_up_only_the_steps_that_are_on() {
        let known = averages(
            3,
            &[("build", 6_000), ("startup", 1_500), ("workload", 30_000)],
        );
        let mut state = FormState::default();

        assert_eq!(
            estimate_text(&custom_plan(&state), Some(&known)),
            "About 6.00 s · from the last 3 runs"
        );

        state.startup_enabled = true;

        assert_eq!(
            estimate_text(&custom_plan(&state), Some(&known)),
            "About 7.50 s · from the last 3 runs"
        );
    }

    #[test]
    fn one_run_is_named_in_the_singular() {
        let known = averages(1, &[("build", 2_000)]);
        let plan = custom_plan(&FormState::default());

        assert!(estimate_text(&plan, Some(&known)).ends_with("from the last run"));
    }

    #[test]
    fn parts_of_a_stage_add_up_unless_the_stage_is_listed() {
        let parts = averages(
            2,
            &[
                ("workspaces:sub:native", 1_000),
                ("workspaces:sub:docker", 2_000),
            ],
        );
        let listed = averages(
            2,
            &[("workspaces", 5_000), ("workspaces:sub:native", 1_000)],
        );

        assert_eq!(average_for(Stage::Workspaces, &parts), Some(3_000));
        assert_eq!(average_for(Stage::Workspaces, &listed), Some(5_000));
        assert_eq!(average_for(Stage::Build, &parts), None);
    }

    #[test]
    fn the_newest_time_of_a_stage_is_read_like_the_average() {
        let mut known = averages(2, &[("build", 5_000), ("workspaces:a:native", 1_000)]);

        known.stages[0].last_ms = Some(4_000);
        known.stages[1].last_ms = Some(900);

        assert_eq!(last_for(Stage::Build, &known), Some(4_000));
        assert_eq!(last_for(Stage::Workspaces, &known), Some(900));
        assert_eq!(last_for(Stage::Startup, &known), None);
    }

    #[test]
    fn the_timeline_greys_out_what_is_off_and_scales_the_bars() {
        let known = averages(
            3,
            &[("build", 5_000), ("startup", 1_000), ("saving", 2_500)],
        );
        let rows = timeline(&custom_plan(&FormState::default()), Some(&known));

        let build = rows.iter().find(|row| row.label == "Build").unwrap();
        let saving = rows.iter().find(|row| row.label == "Save result").unwrap();
        let startup = rows
            .iter()
            .find(|row| row.label == "Start and wait until ready")
            .unwrap();
        let discovery = rows
            .iter()
            .find(|row| row.label == "Detect runtime")
            .unwrap();

        assert_eq!(build.share, 100);
        assert_eq!(saving.share, 50);
        assert_eq!(startup.time, "off");
        assert_eq!(startup.share, 0);
        assert_eq!(startup.class, "opacity-40");
        assert_eq!(discovery.time, "–");
        assert_eq!(rows.len(), Stage::ALL.len());
    }

    #[test]
    fn a_live_run_adds_up_the_steps_of_its_request() {
        let known = averages(
            3,
            &[("build", 6_000), ("workload", 30_000), ("saving", 500)],
        );

        assert_eq!(live_total_ms(false, false, false, &known), Some(6_500));
        assert_eq!(live_total_ms(true, false, false, &known), Some(36_500));
        assert_eq!(live_total_ms(true, true, false, &averages(0, &[])), None);
    }

    #[test]
    fn a_pending_answer_says_estimating() {
        let plan = custom_plan(&FormState::default());
        let (rows, text) = panel_parts(&plan, None);

        assert_eq!(text, "Estimating…");
        assert_eq!(rows.len(), Stage::ALL.len());
    }

    #[test]
    fn reads_the_answer_of_the_api() {
        let parsed: StageAverages = serde_json::from_str(
            r#"{"runs_used":3,"stages":[{"stage":"build","average_ms":6496,"last_ms":6100}],"total_average_ms":9350}"#,
        )
        .unwrap();

        assert_eq!(parsed.runs_used, 3);
        assert_eq!(parsed.stages.len(), 1);
        assert_eq!(parsed.stages[0].average_ms, 6_496);
        assert_eq!(parsed.stages[0].last_ms, Some(6_100));
    }
}
