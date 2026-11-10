use super::models::TrendPoint;
use crate::{
    features::charts::{ChartTone, SeriesPoint},
    format::{date_part, format_number, format_timestamp, ValueFormat, MISSING},
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

// The average number of requests of the newest runs that sent any, for the Throughput chip.
// It stays a dash with the same words when no run has a count, so the chip never moves.
pub fn requests_chip(trends: &[TrendPoint]) -> String {
    let counts: Vec<u64> = newest(trends)
        .iter()
        .filter_map(|point| point.total_requests)
        .collect();

    if counts.is_empty() {
        return format!("{MISSING} requests avg");
    }

    let average = counts.iter().sum::<u64>() as f64 / counts.len() as f64;

    format!("{} requests avg", format_number(average, 0))
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
            total_requests: rps.map(|_| 1000),
            p95_ms: p95,
        }
    }

    #[test]
    fn the_requests_chip_averages_the_runs_that_sent_requests() {
        let mut first = point("a", "2026-10-08T05:00:00.000Z", Some(1.0), None);
        let mut second = point("b", "2026-10-07T05:00:00.000Z", Some(1.0), None);
        let none = point("c", "2026-10-06T05:00:00.000Z", None, None);

        first.total_requests = Some(1000);
        second.total_requests = Some(1400);

        assert_eq!(requests_chip(&[first, second, none]), "1,200 requests avg");
    }

    #[test]
    fn the_requests_chip_keeps_its_words_without_a_count() {
        let none = point("c", "2026-10-06T05:00:00.000Z", None, None);

        assert_eq!(requests_chip(&[none]), format!("{MISSING} requests avg"));
        assert_eq!(requests_chip(&[]), format!("{MISSING} requests avg"));
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
