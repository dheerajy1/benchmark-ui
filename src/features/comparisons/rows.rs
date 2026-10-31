use super::models::{ComparisonData, MetricDiff, WorkspaceComparison};
use crate::format::{format_optional, ValueFormat};

// One line of the comparison table
#[derive(Debug, Clone, PartialEq)]
pub struct MetricRow {
    pub group: String,
    pub label: &'static str,
    pub format: ValueFormat,
    pub diff: MetricDiff,
}

impl MetricRow {
    pub fn baseline_text(&self) -> String {
        format_optional(self.diff.baseline_value, self.format)
    }

    pub fn comparison_text(&self) -> String {
        format_optional(self.diff.comparison_value, self.format)
    }

    // Signed difference, the sign is shown separately so units never see negative numbers
    pub fn difference_text(&self) -> String {
        match self.diff.absolute_difference {
            None => crate::format::MISSING.to_string(),
            Some(0.0) => "0".to_string(),
            Some(value) => {
                let sign = if value > 0.0 { "+" } else { "−" };

                format!("{sign}{}", self.format.format(value.abs()))
            }
        }
    }
}

fn row(
    group: &'static str,
    label: &'static str,
    format: ValueFormat,
    diff: &MetricDiff,
) -> MetricRow {
    MetricRow {
        group: group.to_string(),
        label,
        format,
        diff: diff.clone(),
    }
}

// The rows of one workspace and mode. The group names both, so the TypeScript and Rust
// subscribers sit under the same heading in each run and read side by side.
fn workspace_rows_of(workspace: &WorkspaceComparison) -> Vec<MetricRow> {
    let group = format!(
        "{} · {}",
        workspace.name,
        if workspace.mode == "docker" {
            "Docker"
        } else {
            "Native"
        }
    );

    let mut rows = Vec::new();

    let mut push = |label: &'static str, format: ValueFormat, diff: &MetricDiff| {
        rows.push(MetricRow {
            group: group.clone(),
            label,
            format,
            diff: diff.clone(),
        });
    };

    // An image only exists for a container
    if workspace.mode == "docker" {
        push("Image size", ValueFormat::Bytes, &workspace.image_bytes);
    }

    push(
        "Build time",
        ValueFormat::DurationMs,
        &workspace.build_duration_ms,
    );
    push(
        "Startup time",
        ValueFormat::DurationMs,
        &workspace.startup_duration_ms,
    );
    push(
        "Baseline RSS",
        ValueFormat::Bytes,
        &workspace.baseline_rss_bytes,
    );
    push("Peak RSS", ValueFormat::Bytes, &workspace.peak_rss_bytes);
    push("RSS delta", ValueFormat::Bytes, &workspace.delta_rss_bytes);
    push(
        "Baseline CPU",
        ValueFormat::Percent,
        &workspace.cpu_baseline_percent,
    );
    push(
        "Average CPU",
        ValueFormat::Percent,
        &workspace.cpu_average_percent,
    );
    push(
        "Peak CPU",
        ValueFormat::Percent,
        &workspace.cpu_peak_percent,
    );

    rows
}

// Workspace rows, shown after the project's own rows. Empty when neither run had a workspace.
pub fn workspace_rows(data: &ComparisonData) -> Vec<MetricRow> {
    data.metrics
        .workspaces
        .iter()
        .flat_map(workspace_rows_of)
        .collect()
}

// Every compared metric in a stable display order
pub fn metric_rows(data: &ComparisonData) -> Vec<MetricRow> {
    let metrics = &data.metrics;
    let artifact = &metrics.artifact_size;

    vec![
        row(
            "Timing",
            "Build time",
            ValueFormat::DurationMs,
            &metrics.build_time.duration_ms,
        ),
        row(
            "Timing",
            "Startup time",
            ValueFormat::DurationMs,
            &metrics.startup_time.duration_ms,
        ),
        row(
            "HTTP",
            "Requests per second",
            ValueFormat::PerSecond,
            &metrics.http_latency.requests_per_second,
        ),
        row(
            "HTTP",
            "p50 latency",
            ValueFormat::LatencyMs,
            &metrics.http_latency.p50_ms,
        ),
        row(
            "HTTP",
            "p95 latency",
            ValueFormat::LatencyMs,
            &metrics.http_latency.p95_ms,
        ),
        row(
            "HTTP",
            "p99 latency",
            ValueFormat::LatencyMs,
            &metrics.http_latency.p99_ms,
        ),
        row(
            "Memory",
            "Baseline RSS",
            ValueFormat::Bytes,
            &metrics.memory.baseline_rss_bytes,
        ),
        row(
            "Memory",
            "Peak RSS",
            ValueFormat::Bytes,
            &metrics.memory.peak_rss_bytes,
        ),
        row(
            "Memory",
            "RSS delta",
            ValueFormat::Bytes,
            &metrics.memory.delta_rss_bytes,
        ),
        row(
            "CPU",
            "Baseline CPU",
            ValueFormat::Percent,
            &metrics.cpu.baseline_percent,
        ),
        row(
            "CPU",
            "Average CPU",
            ValueFormat::Percent,
            &metrics.cpu.average_percent,
        ),
        row(
            "CPU",
            "Peak CPU",
            ValueFormat::Percent,
            &metrics.cpu.peak_percent,
        ),
        row(
            "Artifacts",
            "Source size",
            ValueFormat::Bytes,
            &artifact.source.bytes,
        ),
        row(
            "Artifacts",
            "Source files",
            ValueFormat::Count,
            &artifact.source.file_count,
        ),
        row(
            "Artifacts",
            "Build size",
            ValueFormat::Bytes,
            &artifact.build.bytes,
        ),
        row(
            "Artifacts",
            "Build files",
            ValueFormat::Count,
            &artifact.build.file_count,
        ),
        row(
            "Artifacts",
            "Excluded size",
            ValueFormat::Bytes,
            &artifact.excluded.bytes,
        ),
        row(
            "Artifacts",
            "Excluded files",
            ValueFormat::Count,
            &artifact.excluded.file_count,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::super::models::fixtures::data;
    use super::*;

    #[test]
    fn lists_every_documented_metric() {
        assert_eq!(metric_rows(&data()).len(), 18);
    }

    #[test]
    fn a_run_without_workspaces_adds_no_rows() {
        assert!(workspace_rows(&data()).is_empty());
    }

    #[test]
    fn a_docker_workspace_adds_an_image_row_and_a_native_one_does_not() {
        use super::super::models::fixtures::diff;

        let lower = || diff(Some(2.0), Some(1.0), "lower_is_better");
        let workspace = |mode: &str| WorkspaceComparison {
            name: "subscriber".to_string(),
            mode: mode.to_string(),
            baseline_status: Some("success".to_string()),
            comparison_status: Some("success".to_string()),
            image_bytes: lower(),
            build_duration_ms: lower(),
            startup_duration_ms: lower(),
            baseline_rss_bytes: lower(),
            peak_rss_bytes: lower(),
            delta_rss_bytes: lower(),
            cpu_baseline_percent: lower(),
            cpu_average_percent: lower(),
            cpu_peak_percent: lower(),
        };

        let docker = workspace_rows_of(&workspace("docker"));
        let native = workspace_rows_of(&workspace("native"));

        assert_eq!(docker.len(), 9);
        assert_eq!(native.len(), 8);
        assert_eq!(docker[0].label, "Image size");
        assert_eq!(docker[0].group, "subscriber · Docker");
        assert_eq!(native[0].group, "subscriber · Native");
    }

    #[test]
    fn formats_signed_differences() {
        let rows = metric_rows(&data());

        assert_eq!(rows[0].difference_text(), "−1.62 s");
        assert_eq!(rows[2].difference_text(), "+20.0 /s");
    }

    #[test]
    fn unavailable_sides_show_the_missing_marker() {
        let rows = metric_rows(&data());

        assert_eq!(rows[1].baseline_text(), crate::format::MISSING);
        assert_eq!(rows[1].difference_text(), crate::format::MISSING);
    }
}
