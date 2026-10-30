use super::models::TrendPoint;
use crate::{
    components::{verdict, Tone, Verdict},
    features::{
        charts::{ChartTone, SeriesPoint},
        history::RunMetrics,
    },
    format::{date_part, format_signed_percent, format_timestamp, ValueFormat},
};

// The six measurements charted over time
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendMetric {
    Throughput,
    P95Latency,
    PeakMemory,
    CpuAverage,
    Startup,
    Build,
}

impl TrendMetric {
    pub const ALL: [TrendMetric; 6] = [
        Self::Throughput,
        Self::P95Latency,
        Self::PeakMemory,
        Self::CpuAverage,
        Self::Startup,
        Self::Build,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Throughput => "Throughput",
            Self::P95Latency => "p95 latency",
            Self::PeakMemory => "Peak memory",
            Self::CpuAverage => "CPU average",
            Self::Startup => "Startup time",
            Self::Build => "Build time",
        }
    }

    pub fn format(self) -> ValueFormat {
        match self {
            Self::Throughput => ValueFormat::PerSecond,
            Self::P95Latency => ValueFormat::LatencyMs,
            Self::PeakMemory => ValueFormat::Bytes,
            Self::CpuAverage => ValueFormat::Percent,
            Self::Startup | Self::Build => ValueFormat::DurationMs,
        }
    }

    pub fn tone(self) -> ChartTone {
        match self {
            Self::Throughput => ChartTone::Blue,
            Self::P95Latency => ChartTone::Amber,
            Self::PeakMemory => ChartTone::Violet,
            Self::CpuAverage => ChartTone::Cyan,
            Self::Startup | Self::Build => ChartTone::Teal,
        }
    }

    // Higher throughput is better, every other metric is better when lower
    pub fn lower_is_better(self) -> bool {
        !matches!(self, Self::Throughput)
    }

    pub fn value(self, point: &TrendPoint) -> Option<f64> {
        match self {
            Self::Throughput => point.requests_per_second,
            Self::P95Latency => point.p95_ms,
            Self::PeakMemory => point.memory_peak_bytes.map(|value| value as f64),
            Self::CpuAverage => point.cpu_average_percent,
            Self::Startup => point.startup_time_ms.map(|value| value as f64),
            Self::Build => point.build_time_ms.map(|value| value as f64),
        }
    }
}

// Value of a metric on the latest run's history summary
pub fn summary_value(metric: TrendMetric, summary: &RunMetrics) -> Option<f64> {
    match metric {
        TrendMetric::Throughput => summary.requests_per_second,
        TrendMetric::P95Latency => summary.p95_ms,
        TrendMetric::PeakMemory => summary.memory_peak_bytes.map(|value| value as f64),
        TrendMetric::CpuAverage => summary.cpu_average_percent,
        TrendMetric::Startup => summary.startup_time_ms.map(|value| value as f64),
        TrendMetric::Build => summary.build_time_ms.map(|value| value as f64),
    }
}

// Chart points in chronological order, the API returns the newest run first
pub fn series_points(trends: &[TrendPoint], metric: TrendMetric) -> Vec<SeriesPoint> {
    trends
        .iter()
        .rev()
        .map(|point| SeriesPoint {
            axis_label: date_part(&point.created_at),
            detail_label: format_timestamp(&point.created_at),
            value: metric.value(point),
        })
        .collect()
}

// Percentage change of the newest run against the one before it, `None` when either is missing
pub fn change_vs_previous(trends: &[TrendPoint], metric: TrendMetric) -> Option<f64> {
    let latest = metric.value(trends.first()?)?;
    let previous = metric.value(trends.get(1)?)?;

    if previous == 0.0 {
        return None;
    }

    Some((latest - previous) / previous * 100.0)
}

// Text and tone for the hint under a headline tile
pub fn change_hint(trends: &[TrendPoint], metric: TrendMetric) -> (String, Tone) {
    let Some(change) = change_vs_previous(trends, metric) else {
        return (String::new(), Tone::Neutral);
    };

    let tone = match verdict(change, metric.lower_is_better()) {
        Verdict::Improved => Tone::Success,
        Verdict::Regressed => Tone::Danger,
        Verdict::Unchanged => Tone::Neutral,
    };

    (
        format!(
            "{} {} vs previous run",
            if change > 0.0 { "↑" } else { "↓" },
            format_signed_percent(change)
        ),
        tone,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(run_id: &str, created_at: &str, rps: Option<f64>, p95: Option<f64>) -> TrendPoint {
        TrendPoint {
            run_id: run_id.to_string(),
            created_at: created_at.to_string(),
            build_time_ms: None,
            startup_time_ms: None,
            memory_peak_bytes: None,
            cpu_average_percent: None,
            requests_per_second: rps,
            p95_ms: p95,
        }
    }

    #[test]
    fn series_are_chronological() {
        let trends = vec![
            point("new", "2026-10-08T05:00:00.000Z", Some(120.0), None),
            point("old", "2026-10-07T05:00:00.000Z", Some(100.0), None),
        ];

        let series = series_points(&trends, TrendMetric::Throughput);

        assert_eq!(series[0].value, Some(100.0));
        assert_eq!(series[1].value, Some(120.0));
        assert_eq!(series[0].axis_label, "2026-10-07");
    }

    #[test]
    fn missing_values_stay_missing_in_series() {
        let trends = vec![point("a", "2026-10-08T05:00:00.000Z", None, None)];

        let series = series_points(&trends, TrendMetric::P95Latency);

        assert_eq!(series[0].value, None);
    }

    #[test]
    fn computes_change_against_the_previous_run() {
        let trends = vec![
            point("new", "2026-10-08T05:00:00.000Z", Some(120.0), Some(9.0)),
            point("old", "2026-10-07T05:00:00.000Z", Some(100.0), Some(10.0)),
        ];

        assert_eq!(
            change_vs_previous(&trends, TrendMetric::Throughput),
            Some(20.0)
        );
        assert_eq!(
            change_vs_previous(&trends, TrendMetric::P95Latency),
            Some(-10.0)
        );
    }

    #[test]
    fn no_change_without_two_comparable_runs() {
        let single = vec![point("a", "2026-10-08T05:00:00.000Z", Some(1.0), None)];
        assert_eq!(change_vs_previous(&single, TrendMetric::Throughput), None);

        let gap = vec![
            point("new", "2026-10-08T05:00:00.000Z", Some(1.0), None),
            point("old", "2026-10-07T05:00:00.000Z", None, None),
        ];
        assert_eq!(change_vs_previous(&gap, TrendMetric::Throughput), None);

        let zero = vec![
            point("new", "2026-10-08T05:00:00.000Z", Some(1.0), None),
            point("old", "2026-10-07T05:00:00.000Z", Some(0.0), None),
        ];
        assert_eq!(change_vs_previous(&zero, TrendMetric::Throughput), None);
    }

    #[test]
    fn hint_uses_metric_direction() {
        let trends = vec![
            point("new", "2026-10-08T05:00:00.000Z", Some(120.0), Some(12.0)),
            point("old", "2026-10-07T05:00:00.000Z", Some(100.0), Some(10.0)),
        ];

        assert_eq!(
            change_hint(&trends, TrendMetric::Throughput).1,
            Tone::Success
        );
        assert_eq!(
            change_hint(&trends, TrendMetric::P95Latency).1,
            Tone::Danger
        );
    }
}
