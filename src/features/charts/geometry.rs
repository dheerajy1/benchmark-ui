// Path building for line and area marks, pure and tested

// Splits a series into runs of consecutive present points so gaps stay visible
pub fn segments(coordinates: &[Option<(f64, f64)>]) -> Vec<Vec<(f64, f64)>> {
    let mut result: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut current: Vec<(f64, f64)> = Vec::new();

    for coordinate in coordinates {
        match coordinate {
            Some(point) => current.push(*point),
            None => {
                if !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                }
            }
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

// SVG path data for a polyline
pub fn line_path(segment: &[(f64, f64)]) -> String {
    segment
        .iter()
        .enumerate()
        .map(|(index, (x, y))| {
            let command = if index == 0 { "M" } else { "L" };
            format!("{command}{x:.2} {y:.2}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// SVG path data for the filled area between a polyline and a baseline
pub fn area_path(segment: &[(f64, f64)], baseline: f64) -> String {
    let (Some(first), Some(last)) = (segment.first(), segment.last()) else {
        return String::new();
    };

    format!(
        "{} L{:.2} {baseline:.2} L{:.2} {baseline:.2} Z",
        line_path(segment),
        last.0,
        first.0
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_segments_at_missing_values() {
        let coordinates = [
            Some((0.0, 1.0)),
            Some((1.0, 2.0)),
            None,
            Some((3.0, 3.0)),
            None,
            None,
            Some((6.0, 4.0)),
            Some((7.0, 5.0)),
        ];

        let result = segments(&coordinates);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].len(), 2);
        assert_eq!(result[1].len(), 1);
        assert_eq!(result[2].len(), 2);
    }

    #[test]
    fn empty_series_has_no_segments() {
        assert!(segments(&[]).is_empty());
        assert!(segments(&[None, None]).is_empty());
    }

    #[test]
    fn builds_line_and_area_paths() {
        let segment = [(0.0, 10.0), (5.0, 20.0)];

        assert_eq!(line_path(&segment), "M0.00 10.00 L5.00 20.00");
        assert_eq!(
            area_path(&segment, 100.0),
            "M0.00 10.00 L5.00 20.00 L5.00 100.00 L0.00 100.00 Z"
        );
        assert_eq!(area_path(&[], 100.0), "");
    }
}
