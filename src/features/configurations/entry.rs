use super::{
    api::fetch_run_configuration,
    models::{ConfigurationEntry, ReadinessSummary, StartupSummary},
};
use crate::features::{
    connection::ApiError,
    run_detail::{fetch_run, BenchmarkResult},
};

// The history entry of a run stored before configurations were recorded, built from its stored
// result. Only the path, the start command and the readiness are known. The workload and the
// header names are not stored for these runs, so they are left out rather than invented.
pub fn entry_from_result(result: &BenchmarkResult) -> ConfigurationEntry {
    let step = &result.metrics.startup_time;
    let command = step.command.as_ref();
    let readiness = step.readiness.as_ref();

    let startup = if command.is_none() && readiness.is_none() {
        None
    } else {
        Some(StartupSummary {
            program: command.map(|command| command.program.clone()),
            args: command.map(|command| command.args.clone()),
            readiness: readiness.map(|readiness| ReadinessSummary {
                readiness_type: readiness.readiness_type.clone(),
                host: readiness.host.clone(),
                port: readiness.port,
                path: readiness.path.clone(),
                method: readiness.method.clone(),
                expected_status: readiness.expected_status,
                header_names: None,
            }),
        })
    };

    ConfigurationEntry {
        run_id: result.run_id.clone(),
        // The creation time is not part of the stored result, so it stays unknown
        created_at: String::new(),
        path: result.target.path.clone(),
        startup,
        recorded: false,
        configuration: None,
    }
}

// The history entry of any stored run, including runs outside the 200 most recent. A run with a
// recorded configuration uses it. Any other run uses its stored result.
pub async fn fetch_entry(run_id: &str) -> Result<ConfigurationEntry, ApiError> {
    let detail = fetch_run_configuration(run_id).await?;

    match detail.configuration {
        Some(config) => Ok(ConfigurationEntry {
            run_id: detail.run_id,
            // The creation time is not part of the configuration, so it stays unknown
            created_at: String::new(),
            path: config.target_path.clone(),
            startup: config.startup.clone(),
            recorded: true,
            configuration: Some(config),
        }),
        None => {
            let result = fetch_run(run_id).await?;

            Ok(entry_from_result(&result))
        }
    }
}
