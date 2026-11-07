use super::models::TrendPoint;
use crate::{
    features::charts::{ChartTone, SeriesPoint},
    format::{date_part, format_timestamp, ValueFormat},
};

// How many of the newest runs the tiles and charts show
pub const RECENT_RUNS: usize = 12;

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

// The newest runs, newest first
pub fn newest(trends: &[TrendPoint]) -> &[TrendPoint] {
    &trends[..trends.len().min(RECENT_RUNS)]
}

// "2026-10-08T05:11:22Z" becomes "10-08", short enough to label every bar of a run chart
pub fn short_day(created_at: &str) -> String {
    created_at.get(5..10).unwrap_or(created_at).to_string()
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
    fn keeps_only_the_newest_twelve() {
        let trends: Vec<TrendPoint> = (0..20)
            .map(|index| point(&format!("r{index}"), "2026-10-08T05:00:00.000Z", None, None))
            .collect();

        assert_eq!(newest(&trends).len(), RECENT_RUNS);
        assert_eq!(newest(&trends)[0].run_id, "r0");
        assert_eq!(newest(&trends[..3]).len(), 3);
    }

    #[test]
    fn shortens_a_timestamp_to_month_and_day() {
        assert_eq!(short_day("2026-10-08T05:11:22Z"), "10-08");
        assert_eq!(short_day("x"), "x");
    }
}
