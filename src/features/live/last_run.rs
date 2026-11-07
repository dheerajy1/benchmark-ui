use crate::features::{
    history::{fetch_page, HistoryFilters},
    run_detail::{fetch_run, BenchmarkResult},
};

// What the newest finished run of the same project measured, for the dashed lines of the charts
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LastRun {
    pub run_id: String,
    pub memory_peak: Option<f64>,
    pub cpu_peak: Option<f64>,
    // Requests per second over the whole workload
    pub throughput: Option<f64>,
    pub modes: Vec<ModeLast>,
}

// The peaks of one workspace mode in that run
#[derive(Debug, Clone, PartialEq)]
pub struct ModeLast {
    pub workspace: String,
    pub mode: String,
    pub memory_peak: Option<f64>,
    pub cpu_peak: Option<f64>,
}

impl LastRun {
    pub fn from_result(result: &BenchmarkResult) -> Self {
        let metrics = &result.metrics;

        Self {
            run_id: result.run_id.clone(),
            memory_peak: metrics
                .memory
                .as_ref()
                .map(|memory| memory.peak.rss_bytes as f64),
            cpu_peak: metrics.cpu.as_ref().map(|cpu| cpu.peak_percent),
            throughput: metrics
                .http_latency
                .as_ref()
                .map(|latency| latency.requests_per_second),
            modes: metrics
                .workspaces
                .iter()
                .map(|workspace| ModeLast {
                    workspace: workspace.name.clone(),
                    mode: workspace.mode.clone(),
                    memory_peak: workspace
                        .memory
                        .as_ref()
                        .map(|memory| memory.peak.rss_bytes as f64),
                    cpu_peak: workspace.cpu.as_ref().map(|cpu| cpu.peak_percent),
                })
                .collect(),
        }
    }

    // The peaks of one workspace mode
    pub fn mode(&self, workspace: &str, mode: &str) -> Option<&ModeLast> {
        self.modes
            .iter()
            .find(|entry| entry.workspace == workspace && entry.mode == mode)
    }
}

// The newest finished run of a project, `None` when there is none or it cannot be read. The
// charts then simply have no dashed line.
pub async fn fetch_last_run(path: &str) -> Option<LastRun> {
    let filters = HistoryFilters {
        target_path: path.to_string(),
        status: "done".to_string(),
        ..HistoryFilters::default()
    };

    let page = fetch_page(&filters, None, 1).await.ok()?;
    let newest = page.items.first()?;
    let result = fetch_run(&newest.run_id).await.ok()?;

    Some(LastRun::from_result(&result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_peaks_of_one_mode() {
        let last = LastRun {
            modes: vec![
                ModeLast {
                    workspace: "sub".to_string(),
                    mode: "native".to_string(),
                    memory_peak: Some(10.0),
                    cpu_peak: None,
                },
                ModeLast {
                    workspace: "sub".to_string(),
                    mode: "docker".to_string(),
                    memory_peak: Some(20.0),
                    cpu_peak: Some(5.0),
                },
            ],
            ..LastRun::default()
        };

        assert_eq!(last.mode("sub", "docker").unwrap().memory_peak, Some(20.0));
        assert!(last.mode("sub", "other").is_none());
        assert!(last.mode("none", "native").is_none());
    }
}
