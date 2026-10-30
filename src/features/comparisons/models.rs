use serde::Deserialize;

// Response model of GET /api/v1/comparisons/summary, see docs/api.md in benchmark-api

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RunReference {
    pub run_id: String,
    pub target: Target,
    #[serde(default)]
    pub runtime: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Target {
    pub path: String,
}

// One compared metric, a side that is unavailable is null and so are the differences
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default)]
pub struct MetricDiff {
    pub baseline_value: Option<f64>,
    pub comparison_value: Option<f64>,
    pub absolute_difference: Option<f64>,
    pub percentage_change: Option<f64>,
    // "lower_is_better" or "higher_is_better", decided by the backend
    pub direction: String,
}

impl MetricDiff {
    pub fn lower_is_better(&self) -> bool {
        self.direction != "higher_is_better"
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StepComparison {
    pub baseline_status: String,
    pub comparison_status: String,
    pub duration_ms: MetricDiff,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MemoryComparison {
    pub baseline_rss_bytes: MetricDiff,
    pub peak_rss_bytes: MetricDiff,
    pub delta_rss_bytes: MetricDiff,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CpuComparison {
    pub baseline_percent: MetricDiff,
    pub average_percent: MetricDiff,
    pub peak_percent: MetricDiff,
}

// Statuses are null when the run had no HTTP workload
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HttpComparison {
    #[serde(default)]
    pub baseline_status: Option<String>,
    #[serde(default)]
    pub comparison_status: Option<String>,
    pub requests_per_second: MetricDiff,
    pub p50_ms: MetricDiff,
    pub p95_ms: MetricDiff,
    pub p99_ms: MetricDiff,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ArtifactCategoryComparison {
    pub bytes: MetricDiff,
    pub file_count: MetricDiff,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ArtifactComparison {
    pub source: ArtifactCategoryComparison,
    pub build: ArtifactCategoryComparison,
    pub excluded: ArtifactCategoryComparison,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ComparisonMetrics {
    pub build_time: StepComparison,
    pub startup_time: StepComparison,
    pub memory: MemoryComparison,
    pub cpu: CpuComparison,
    pub http_latency: HttpComparison,
    pub artifact_size: ArtifactComparison,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ComparisonData {
    pub baseline: RunReference,
    pub comparison: RunReference,
    pub metrics: ComparisonMetrics,
}

#[cfg(test)]
pub mod fixtures {
    use super::*;

    pub fn diff(baseline: Option<f64>, comparison: Option<f64>, direction: &str) -> MetricDiff {
        let absolute = baseline.zip(comparison).map(|(b, c)| c - b);
        let percentage = baseline
            .zip(comparison)
            .filter(|(b, _)| *b != 0.0)
            .map(|(b, c)| (c - b) / b * 100.0);

        MetricDiff {
            baseline_value: baseline,
            comparison_value: comparison,
            absolute_difference: absolute,
            percentage_change: percentage,
            direction: direction.to_string(),
        }
    }

    pub fn step(baseline: Option<f64>, comparison: Option<f64>) -> StepComparison {
        StepComparison {
            baseline_status: "success".to_string(),
            comparison_status: "success".to_string(),
            duration_ms: diff(baseline, comparison, "lower_is_better"),
        }
    }

    pub fn data() -> ComparisonData {
        let lower = |b, c| diff(Some(b), Some(c), "lower_is_better");
        let category = || ArtifactCategoryComparison {
            bytes: lower(10.0, 5.0),
            file_count: lower(2.0, 1.0),
        };
        let reference = |id: &str| RunReference {
            run_id: id.to_string(),
            target: Target {
                path: "/p".to_string(),
            },
            runtime: Some("bun".to_string()),
            created_at: "2026-10-08T05:11:22.123Z".to_string(),
        };

        ComparisonData {
            baseline: reference("a"),
            comparison: reference("b"),
            metrics: ComparisonMetrics {
                build_time: step(Some(6496.0), Some(4872.0)),
                startup_time: step(None, Some(1000.0)),
                memory: MemoryComparison {
                    baseline_rss_bytes: lower(1.0, 1.0),
                    peak_rss_bytes: lower(2.0, 3.0),
                    delta_rss_bytes: lower(1.0, 2.0),
                },
                cpu: CpuComparison {
                    baseline_percent: lower(1.0, 1.0),
                    average_percent: lower(2.0, 3.0),
                    peak_percent: lower(3.0, 4.0),
                },
                http_latency: HttpComparison {
                    baseline_status: Some("completed".to_string()),
                    comparison_status: Some("completed".to_string()),
                    requests_per_second: diff(Some(100.0), Some(120.0), "higher_is_better"),
                    p50_ms: lower(2.0, 2.0),
                    p95_ms: lower(10.0, 9.0),
                    p99_ms: lower(12.0, 11.0),
                },
                artifact_size: ArtifactComparison {
                    source: category(),
                    build: category(),
                    excluded: category(),
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_documented_metric() {
        let body = r#"{
            "baseline_value": 6496, "comparison_value": 4872,
            "absolute_difference": -1624, "percentage_change": -25.0,
            "direction": "lower_is_better"
        }"#;

        let metric: MetricDiff = serde_json::from_str(body).expect("parses");

        assert_eq!(metric.absolute_difference, Some(-1624.0));
        assert!(metric.lower_is_better());
    }

    #[test]
    fn unavailable_sides_stay_null() {
        let body = r#"{
            "baseline_value": null, "comparison_value": 10,
            "absolute_difference": null, "percentage_change": null,
            "direction": "higher_is_better"
        }"#;

        let metric: MetricDiff = serde_json::from_str(body).expect("parses");

        assert_eq!(metric.baseline_value, None);
        assert_eq!(metric.percentage_change, None);
        assert!(!metric.lower_is_better());
    }
}
