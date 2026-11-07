use super::{
    models::{ComparisonData, MetricDiff},
    rows::{metric_rows, MetricRow},
};
use crate::features::charts::DivergeRow;

// The twelve metrics of the summary, by group and name as `metric_rows` lists them
const SUMMARY: [(&str, &str); 12] = [
    ("HTTP", "Requests per second"),
    ("HTTP", "p50 latency"),
    ("HTTP", "p95 latency"),
    ("HTTP", "p99 latency"),
    ("Memory", "Baseline RSS"),
    ("Memory", "Peak RSS"),
    ("CPU", "Average CPU"),
    ("CPU", "Peak CPU"),
    ("Timing", "Build time"),
    ("Timing", "Startup time"),
    ("Artifacts", "Source size"),
    ("Artifacts", "Build size"),
];

// Change in percent counted so that positive is always better, None when there is no change
pub fn improvement(diff: &MetricDiff) -> Option<f64> {
    let change = diff.percentage_change?;

    Some(if diff.lower_is_better() {
        -change
    } else {
        change
    })
}

// The summary metrics from the biggest win to the biggest loss, metrics without a change last
pub fn summary_rows(data: &ComparisonData) -> Vec<MetricRow> {
    let all = metric_rows(data);
    let mut rows: Vec<MetricRow> = SUMMARY
        .iter()
        .filter_map(|(group, label)| {
            all.iter()
                .find(|row| row.group == *group && row.label == *label)
                .cloned()
        })
        .collect();

    rows.sort_by(|a, b| match (improvement(&a.diff), improvement(&b.diff)) {
        (Some(first), Some(second)) => second.total_cmp(&first),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    rows
}

// Bars for the rows that have a change, in the order given
pub fn diverge_rows(rows: &[MetricRow]) -> Vec<DivergeRow> {
    rows.iter()
        .filter_map(|row| {
            Some(DivergeRow {
                label: row.label.to_string(),
                improvement: improvement(&row.diff)?,
                change: row.diff.percentage_change?,
                lower_is_better: row.diff.lower_is_better(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{super::models::fixtures, *};

    #[test]
    fn counts_a_drop_as_better_when_lower_is_better() {
        let lower = fixtures::diff(Some(100.0), Some(60.0), "lower_is_better");
        let higher = fixtures::diff(Some(100.0), Some(60.0), "higher_is_better");

        assert_eq!(improvement(&lower), Some(40.0));
        assert_eq!(improvement(&higher), Some(-40.0));
        assert_eq!(
            improvement(&fixtures::diff(None, None, "lower_is_better")),
            None
        );
    }

    #[test]
    fn lists_twelve_rows_from_the_biggest_win() {
        let rows = summary_rows(&fixtures::data());
        let scores: Vec<f64> = rows
            .iter()
            .filter_map(|row| improvement(&row.diff))
            .collect();

        assert_eq!(rows.len(), 12);
        assert!(scores.windows(2).all(|pair| pair[0] >= pair[1]));
    }

    #[test]
    fn leaves_out_rows_without_a_change_from_the_bars() {
        let rows = summary_rows(&fixtures::data());

        assert!(diverge_rows(&rows).len() <= rows.len());
    }
}
