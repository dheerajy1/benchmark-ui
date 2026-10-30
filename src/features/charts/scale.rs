// Scales and tick generation, pure numeric code with no rendering

#[derive(Debug, Clone, PartialEq)]
pub struct Domain {
    pub min: f64,
    pub max: f64,
    pub ticks: Vec<f64>,
}

// Most ticks a domain may contain, protects against runaway loops on odd input
const MAX_TICKS: usize = 24;

// Rounds a span to 1, 2, 5 or 10 times a power of ten
fn nice_number(span: f64, round: bool) -> f64 {
    let exponent = span.log10().floor();
    let fraction = span / 10f64.powf(exponent);

    let nice = if round {
        match fraction {
            f if f < 1.5 => 1.0,
            f if f < 3.0 => 2.0,
            f if f < 7.0 => 5.0,
            _ => 10.0,
        }
    } else {
        match fraction {
            f if f <= 1.0 => 1.0,
            f if f <= 2.0 => 2.0,
            f if f <= 5.0 => 5.0,
            _ => 10.0,
        }
    };

    nice * 10f64.powf(exponent)
}

// Removes floating point noise such as 0.30000000000000004
fn clean(value: f64) -> f64 {
    (value * 1e9).round() / 1e9
}

// Expands [min, max] to round bounds and returns evenly spaced ticks.
// Handles empty, non-finite and single-value input without producing NaN.
pub fn nice_domain(min: f64, max: f64, include_zero: bool, target_ticks: usize) -> Domain {
    let valid = min.is_finite() && max.is_finite() && min <= max;
    let (mut low, mut high) = if valid { (min, max) } else { (0.0, 1.0) };

    if include_zero {
        low = low.min(0.0);
        high = high.max(0.0);
    }

    if (high - low).abs() < f64::EPSILON {
        let pad = if high == 0.0 { 1.0 } else { high.abs() * 0.1 };
        low -= pad;
        high += pad;

        if include_zero && min >= 0.0 {
            low = 0.0;
        }
    }

    let tick_count = target_ticks.max(2);
    let span = nice_number(high - low, false);
    let step = nice_number(span / (tick_count - 1) as f64, true);

    let nice_min = clean((low / step).floor() * step);
    let nice_max = clean((high / step).ceil() * step);
    let count = (((nice_max - nice_min) / step).round() as usize).min(MAX_TICKS);

    let ticks = (0..=count)
        .map(|index| clean(nice_min + step * index as f64))
        .collect();

    Domain {
        min: nice_min,
        max: nice_max,
        ticks,
    }
}

// Maps a value from a data domain to a pixel range
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearScale {
    domain_min: f64,
    domain_max: f64,
    range_start: f64,
    range_end: f64,
}

impl LinearScale {
    pub fn new(domain_min: f64, domain_max: f64, range_start: f64, range_end: f64) -> Self {
        Self {
            domain_min,
            domain_max,
            range_start,
            range_end,
        }
    }

    pub fn map(&self, value: f64) -> f64 {
        let span = self.domain_max - self.domain_min;

        if span.abs() < f64::EPSILON {
            return (self.range_start + self.range_end) / 2.0;
        }

        let ratio = (value - self.domain_min) / span;

        self.range_start + ratio * (self.range_end - self.range_start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_to_round_bounds_from_zero() {
        let domain = nice_domain(0.0, 97.0, true, 5);

        assert_eq!(domain.ticks.first(), Some(&0.0));
        assert_eq!(domain.ticks.last(), Some(&100.0));
        assert_eq!(domain.max, 100.0);
    }

    #[test]
    fn pads_a_flat_series() {
        let domain = nice_domain(5.0, 5.0, false, 5);

        assert!(domain.min < 5.0);
        assert!(domain.max > 5.0);
        assert!(domain.ticks.len() >= 2);
    }

    #[test]
    fn handles_all_zero_data() {
        let domain = nice_domain(0.0, 0.0, true, 5);

        assert_eq!(domain.min, 0.0);
        assert!(domain.max > 0.0);
    }

    #[test]
    fn recovers_from_invalid_input() {
        let domain = nice_domain(f64::NAN, 1.0, false, 5);

        assert!(domain.min.is_finite());
        assert!(domain.max.is_finite());
    }

    #[test]
    fn keeps_negative_values_in_range() {
        let domain = nice_domain(-30.0, 10.0, true, 5);

        assert!(domain.min <= -30.0);
        assert!(domain.max >= 10.0);
    }

    #[test]
    fn maps_values_linearly() {
        let scale = LinearScale::new(0.0, 100.0, 200.0, 0.0);

        assert_eq!(scale.map(0.0), 200.0);
        assert_eq!(scale.map(50.0), 100.0);
        assert_eq!(scale.map(100.0), 0.0);
    }

    #[test]
    fn degenerate_scale_centers_the_value() {
        let scale = LinearScale::new(3.0, 3.0, 0.0, 10.0);

        assert_eq!(scale.map(3.0), 5.0);
    }
}
