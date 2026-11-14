//! Compact formatting of numbers.

/// Rounds a number to the given number of decimals, without a trailing zero or a lone `-`.
fn trimmed(value: f64, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        &text
    };
    match text {
        "-0" => "0".to_string(),
        text => text.to_string(),
    }
}

/// Formats a number in a compact way, to fit in a few characters.
///
/// Finite values with a magnitude from 0.001 to 1e10 give at most 5 characters.
///
/// * Values from 10 up to 1000 are rounded to integers: `50`, `999`.
/// * Between 1 and 10 and below 1, one and two decimals are kept, without trailing zeros:
///   `2.5`, `0.25`.
/// * Larger values use a `k`, `M` or `G` suffix: `1.0k`, `12k`, `1.5M`.
/// * Values very close to zero give `0`. `NaN` and infinite values are printed as `NaN`, `inf`
///   and `-inf`.
///
/// This is the default formatter of [`Chart::y_labels`](crate::views::Chart::y_labels).
///
/// # Examples
///
/// ```rust
/// use cursive_core::utils::format_compact;
///
/// assert_eq!(format_compact(0.0), "0");
/// assert_eq!(format_compact(50.0), "50");
/// assert_eq!(format_compact(2.5), "2.5");
/// assert_eq!(format_compact(1000.0), "1.0k");
/// assert_eq!(format_compact(12_000.0), "12k");
/// assert_eq!(format_compact(1_500_000.0), "1.5M");
/// assert_eq!(format_compact(-2500.0), "-2.5k");
/// ```
pub fn format_compact(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }

    let abs = value.abs();
    // Switch to the next unit a bit early, so that rounding does not give "1000" or "1000k".
    for (unit, suffix) in [(1e9, 'G'), (1e6, 'M'), (1e3, 'k')] {
        if abs >= unit * 0.9995 {
            let scaled = value / unit;
            return if scaled.abs() < 9.95 {
                format!("{scaled:.1}{suffix}")
            } else {
                format!("{scaled:.0}{suffix}")
            };
        }
    }

    if abs >= 10.0 {
        trimmed(value, 0)
    } else if abs >= 1.0 {
        trimmed(value, 1)
    } else {
        trimmed(value, 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_compact_values() {
        for (value, expected) in [
            (0.0, "0"),
            (-0.0, "0"),
            (7.0, "7"),
            (50.0, "50"),
            (100.0, "100"),
            (999.0, "999"),
            (1000.0, "1.0k"),
            (2000.0, "2.0k"),
            (2500.0, "2.5k"),
            (9940.0, "9.9k"),
            (9999.0, "10k"),
            (12000.0, "12k"),
            (999_400.0, "999k"),
            (999_600.0, "1.0M"),
            (1_500_000.0, "1.5M"),
            (25_000_000.0, "25M"),
            (3e9, "3.0G"),
            (0.5, "0.5"),
            (0.25, "0.25"),
            (0.05, "0.05"),
            (0.001, "0"),
            (2.5, "2.5"),
            (3.04, "3"),
            (37.5, "38"),
            (-1.0, "-1"),
            (-0.5, "-0.5"),
            (-2500.0, "-2.5k"),
            (-0.001, "0"),
            (f64::NAN, "NaN"),
            (f64::INFINITY, "inf"),
        ] {
            assert_eq!(format_compact(value), expected, "for {value}");
        }
    }

    #[test]
    fn labels_are_short() {
        let mut value = 0.001;
        while value < 1e10 {
            for value in [value, -value, value * 1.7, value * 9.99, value * 5.55] {
                let label = format_compact(value);
                assert!(label.chars().count() <= 5, "{label} for {value}");
            }
            value *= 1.37;
        }
    }

    #[test]
    fn labels_fit_five_columns_and_never_round_up_to_a_unit_boundary() {
        let mut bad = Vec::new();
        let mut value = 0.001f64;
        while value < 1e10 {
            for v in [
                value,
                value * 0.9995,
                value * 0.99995,
                value * 1.00005,
                value * 9.949,
                value * 9.951,
                value * 9.9949,
                value * 9.995,
                value * 99.95,
                value * 999.5,
                value * 999.95,
            ] {
                for v in [v, -v] {
                    if v.abs() >= 1e10 || v.abs() < 0.001 {
                        continue;
                    }
                    let label = format_compact(v);
                    if label.chars().count() > 5 {
                        bad.push(format!("{v} -> {label}"));
                    }
                    if label.starts_with("1000")
                        || label.starts_with("-1000")
                        || label.contains("10.0")
                    {
                        bad.push(format!("{v} -> {label} (unit boundary)"));
                    }
                    if label == "-0" || label == "-0.0" {
                        bad.push(format!("{v} -> {label}"));
                    }
                }
            }
            value *= 1.0137;
        }
        bad.dedup();
        assert!(
            bad.is_empty(),
            "{} bad, first: {:#?}",
            bad.len(),
            &bad[..bad.len().min(15)]
        );
    }
}
