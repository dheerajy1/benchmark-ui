// Shared display formatting for benchmark values, every formatter is pure and tested

// Shown wherever a measurement is not available, a missing value is never shown as zero
pub const MISSING: &str = "—";

// Which unit a numeric series is expressed in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueFormat {
    Bytes,
    DurationMs,
    LatencyMs,
    Percent,
    PerSecond,
    Count,
}

impl ValueFormat {
    pub fn format(self, value: f64) -> String {
        match self {
            Self::Bytes => format_bytes(value),
            Self::DurationMs => format_duration_ms(value),
            Self::LatencyMs => format_latency_ms(value),
            Self::Percent => format_percent(value),
            Self::PerSecond => format_per_second(value),
            Self::Count => format_number(value, 0),
        }
    }

    // Compact form used on chart axes
    pub fn format_axis(self, value: f64) -> String {
        match self {
            Self::Bytes => format_bytes(value),
            Self::DurationMs => format_duration_ms(value),
            Self::LatencyMs => format_latency_ms(value),
            Self::Percent => format!("{}%", trim_decimals(value, 1)),
            Self::PerSecond => trim_decimals(value, 1),
            Self::Count => format_number(value, 0),
        }
    }
}

// Formats an optional value, `None` becomes the missing marker
pub fn format_optional(value: Option<f64>, format: ValueFormat) -> String {
    value.map_or_else(|| MISSING.to_string(), |value| format.format(value))
}

// Groups digits with thousands separators and keeps a fixed number of decimals
pub fn format_number(value: f64, decimals: usize) -> String {
    if !value.is_finite() {
        return MISSING.to_string();
    }

    let raw = format!("{:.*}", decimals, value.abs());
    let (whole, fraction) = match raw.split_once('.') {
        Some((whole, fraction)) => (whole.to_string(), Some(fraction.to_string())),
        None => (raw.clone(), None),
    };

    let mut grouped = String::new();
    for (index, digit) in whole.chars().enumerate() {
        if index > 0 && (whole.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }

    let sign = if value < 0.0 && raw_has_non_zero(&grouped, fraction.as_deref()) {
        "-"
    } else {
        ""
    };

    match fraction {
        Some(fraction) => format!("{sign}{grouped}.{fraction}"),
        None => format!("{sign}{grouped}"),
    }
}

fn raw_has_non_zero(whole: &str, fraction: Option<&str>) -> bool {
    whole.chars().any(|c| c.is_ascii_digit() && c != '0')
        || fraction.is_some_and(|f| f.chars().any(|c| c != '0'))
}

// Removes trailing zeros so axis ticks stay short
fn trim_decimals(value: f64, decimals: usize) -> String {
    if !value.is_finite() {
        return MISSING.to_string();
    }

    let text = format!("{:.*}", decimals, value);
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    }
}

// Binary units, 1 KiB is 1024 bytes
pub fn format_bytes(value: f64) -> String {
    if !value.is_finite() || value < 0.0 {
        return MISSING.to_string();
    }

    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut scaled = value;
    let mut unit = 0;

    while scaled >= 1024.0 && unit < UNITS.len() - 1 {
        scaled /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{} B", scaled.round() as u64)
    } else if scaled >= 100.0 {
        format!("{scaled:.0} {}", UNITS[unit])
    } else if scaled >= 10.0 {
        format!("{scaled:.1} {}", UNITS[unit])
    } else {
        format!("{scaled:.2} {}", UNITS[unit])
    }
}

// Build and startup durations
pub fn format_duration_ms(value: f64) -> String {
    if !value.is_finite() || value < 0.0 {
        return MISSING.to_string();
    }

    if value < 1000.0 {
        format!("{} ms", value.round() as u64)
    } else if value < 60_000.0 {
        format!("{:.2} s", value / 1000.0)
    } else {
        let total_seconds = (value / 1000.0).round() as u64;
        format!("{}m {:02}s", total_seconds / 60, total_seconds % 60)
    }
}

// Request latencies keep sub-millisecond precision
pub fn format_latency_ms(value: f64) -> String {
    if !value.is_finite() || value < 0.0 {
        return MISSING.to_string();
    }

    if value < 10.0 {
        format!("{value:.2} ms")
    } else if value < 100.0 {
        format!("{value:.1} ms")
    } else if value < 1000.0 {
        format!("{value:.0} ms")
    } else {
        format!("{:.2} s", value / 1000.0)
    }
}

pub fn format_percent(value: f64) -> String {
    if !value.is_finite() {
        return MISSING.to_string();
    }

    format!("{value:.1}%")
}

pub fn format_per_second(value: f64) -> String {
    if !value.is_finite() || value < 0.0 {
        return MISSING.to_string();
    }

    if value >= 1000.0 {
        format!("{} /s", format_number(value, 0))
    } else {
        format!("{value:.1} /s")
    }
}

// Explicit sign so improvements and regressions read at a glance
pub fn format_signed_percent(value: f64) -> String {
    if !value.is_finite() {
        return MISSING.to_string();
    }

    if value > 0.0 {
        format!("+{value:.1}%")
    } else {
        format!("{value:.1}%")
    }
}

// "2026-10-08T05:11:22.123Z" becomes "2026-10-08 05:11 UTC"
pub fn format_timestamp(value: &str) -> String {
    let bytes = value.as_bytes();
    let looks_like_iso = value.is_char_boundary(16)
        && value.is_char_boundary(11)
        && bytes.len() >= 16
        && bytes.get(10) == Some(&b'T')
        && bytes.get(4) == Some(&b'-');

    if !looks_like_iso {
        return value.to_string();
    }

    format!("{} {} UTC", &value[..10], &value[11..16])
}

// "2026-10-08T05:11:22.123Z" becomes "2026-10-08"
pub fn date_part(value: &str) -> String {
    value.get(..10).unwrap_or(value).to_string()
}

// First segment of a UUID, enough to tell runs apart in a list
pub fn short_id(value: &str) -> String {
    value.chars().take(8).collect()
}

// Last path component, the full path stays available as a tooltip
pub fn path_tail(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);

    trimmed
        .rsplit(['/', '\\'])
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or(path)
        .to_string()
}

// Milliseconds since the Unix epoch of an ISO 8601 time such as "2026-10-08T05:11:22.123Z".
// A UTC offset such as +02:00 is honoured, and a time without an offset counts as UTC.
pub fn parse_epoch_ms(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();

    if bytes.len() < 19
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || !bytes
            .get(10)
            .is_some_and(|byte| *byte == b'T' || *byte == b' ')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return None;
    }

    let year: i64 = value.get(0..4)?.parse().ok()?;
    let month: i64 = value.get(5..7)?.parse().ok()?;
    let day: i64 = value.get(8..10)?.parse().ok()?;
    let hour: i64 = value.get(11..13)?.parse().ok()?;
    let minute: i64 = value.get(14..16)?.parse().ok()?;
    let second: i64 = value.get(17..19)?.parse().ok()?;

    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }

    let mut rest = value.get(19..)?;

    // Fraction of a second, only the first three digits count
    let mut millis: i64 = 0;
    if let Some(fraction) = rest.strip_prefix('.') {
        let digits: String = fraction
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect();

        if digits.is_empty() {
            return None;
        }

        let kept = &digits[..digits.len().min(3)];
        millis = format!("{kept:0<3}").parse().ok()?;
        rest = &fraction[digits.len()..];
    }

    let offset_minutes: i64 = match rest {
        "" | "Z" | "z" => 0,
        other => {
            let sign: i64 = match *other.as_bytes().first()? {
                b'+' => 1,
                b'-' => -1,
                _ => return None,
            };

            let (hours, minutes) = other.get(1..)?.split_once(':')?;
            let hours: i64 = hours.parse().ok()?;
            let minutes: i64 = minutes.parse().ok()?;

            sign * (hours * 60 + minutes)
        }
    };

    let seconds = days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second
        - offset_minutes * 60;

    Some(seconds * 1_000 + millis)
}

// Days since 1970-01-01 of a civil date in the proleptic Gregorian calendar
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let year_of_era = y - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

    era * 146_097 + day_of_era - 719_468
}

// "just now", "5 min ago", "2 h ago" or "3 days ago", relative to `now_ms`. Older than 30 days
// it shows the date, and a time that cannot be read is shown unchanged.
pub fn relative_time(created_at: &str, now_ms: i64) -> String {
    let Some(created_ms) = parse_epoch_ms(created_at) else {
        return created_at.to_string();
    };

    let seconds = (now_ms - created_ms).max(0) / 1_000;
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;

    if seconds < 60 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{minutes} min ago")
    } else if hours < 24 {
        format!("{hours} h ago")
    } else if days < 30 {
        if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{days} days ago")
        }
    } else {
        date_part(created_at)
    }
}

// Human label for a backend status code such as no_build_command
pub fn humanize_code(code: &str) -> String {
    let mut text = code.replace('_', " ");

    if let Some(first) = text.get(..1) {
        let upper = first.to_uppercase();
        text.replace_range(..1, &upper);
    }

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_bytes_with_binary_units() {
        assert_eq!(format_bytes(512.0), "512 B");
        assert_eq!(format_bytes(73_400_320.0), "70.0 MiB");
        assert_eq!(format_bytes(1024.0), "1.00 KiB");
    }

    #[test]
    fn formats_durations() {
        assert_eq!(format_duration_ms(842.0), "842 ms");
        assert_eq!(format_duration_ms(1842.0), "1.84 s");
        assert_eq!(format_duration_ms(72_000.0), "1m 12s");
    }

    #[test]
    fn formats_latency_with_adaptive_precision() {
        assert_eq!(format_latency_ms(8.7), "8.70 ms");
        assert_eq!(format_latency_ms(45.25), "45.2 ms");
        assert_eq!(format_latency_ms(1500.0), "1.50 s");
    }

    #[test]
    fn groups_thousands() {
        assert_eq!(format_number(1234567.891, 2), "1,234,567.89");
        assert_eq!(format_number(12.0, 0), "12");
        assert_eq!(format_number(-1500.0, 0), "-1,500");
    }

    #[test]
    fn missing_values_are_never_zero() {
        assert_eq!(format_optional(None, ValueFormat::Bytes), MISSING);
        assert_eq!(format_optional(Some(0.0), ValueFormat::Count), "0");
        assert_eq!(format_bytes(f64::NAN), MISSING);
    }

    #[test]
    fn signs_percent_changes() {
        assert_eq!(format_signed_percent(12.34), "+12.3%");
        assert_eq!(format_signed_percent(-25.0), "-25.0%");
    }

    #[test]
    fn formats_timestamps() {
        assert_eq!(
            format_timestamp("2026-10-08T05:11:22.123Z"),
            "2026-10-08 05:11 UTC"
        );
        assert_eq!(format_timestamp("not a date"), "not a date");
    }

    #[test]
    fn extracts_the_date() {
        assert_eq!(date_part("2026-10-08T05:11:22.123Z"), "2026-10-08");
        assert_eq!(date_part("short"), "short");
    }

    #[test]
    fn extracts_path_tail_and_short_id() {
        assert_eq!(path_tail("/home/me/projects/my-api"), "my-api");
        assert_eq!(path_tail("/home/me/projects/my-api/"), "my-api");
        assert_eq!(short_id("0192f3a0-0000-7000-8000-000000000000"), "0192f3a0");
    }

    #[test]
    fn parses_iso_times_to_epoch_milliseconds() {
        assert_eq!(parse_epoch_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_epoch_ms("1970-01-01T00:00:01.5Z"), Some(1_500));
        assert_eq!(
            parse_epoch_ms("2026-10-08T07:11:22+02:00"),
            parse_epoch_ms("2026-10-08T05:11:22Z")
        );
        assert_eq!(parse_epoch_ms("not a date"), None);
    }

    #[test]
    fn describes_how_long_ago_a_run_was() {
        let now = parse_epoch_ms("2026-10-08T07:11:22Z").expect("valid time");

        assert_eq!(relative_time("2026-10-08T07:11:00Z", now), "just now");
        assert_eq!(relative_time("2026-10-08T07:06:22Z", now), "5 min ago");
        assert_eq!(relative_time("2026-10-08T05:11:22Z", now), "2 h ago");
        assert_eq!(relative_time("2026-10-07T07:11:22Z", now), "1 day ago");
        assert_eq!(relative_time("2026-10-05T07:11:22Z", now), "3 days ago");
        assert_eq!(relative_time("2026-09-01T07:11:22Z", now), "2026-09-01");
        assert_eq!(relative_time("garbage", now), "garbage");
    }

    #[test]
    fn humanizes_status_codes() {
        assert_eq!(humanize_code("no_build_command"), "No build command");
        assert_eq!(humanize_code("success"), "Success");
    }
}
