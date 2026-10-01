// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Ported from `utils.ts`: `secondsToFormat`, `formatToSeconds`,
//! `secondsToHMR`.

/// Formats seconds as `MM:SS.CC` (or `HH:MM:SS.CC` once an hour is
/// reached), for display next to a cue (time fields, the transport bar).
///
/// `None` (the original's `NaN`/`null` case -- unknown duration) renders as
/// `--:--.--`.
pub fn format_seconds(source_seconds: Option<f64>) -> String {
    let Some(seconds) = source_seconds.filter(|s| !s.is_nan()) else {
        return "--:--.--".to_string();
    };

    let (hour, minute, second, centisecond) = split(seconds);

    if hour > 0 {
        format!("{hour:02}:{minute:02}:{second:02}.{centisecond:02}")
    } else {
        format!("{minute:02}:{second:02}.{centisecond:02}")
    }
}

/// Formats seconds in a human-readable, variable-precision form, used for
/// things like the default `{duration}` wait-cue name (`buildCueName`'s
/// `secondsToHMR` helper): `1h 02m 03s`, `2m 03.5s`, `3s`, `3.5s`.
pub fn format_human(source_seconds: f64) -> String {
    let (hour, minute, second, centisecond) = split(source_seconds);

    if hour > 0 {
        format!("{hour:02}h {minute:02}m {second:02}s")
    } else if minute > 0 {
        // Seconds are zero-padded here (matching the original, which uses
        // its pre-padded `ss` string in this branch) but not in the two
        // branches below, which use the raw number. Also note this branch
        // always appends a decimal, even when it trims down to a single
        // "0" (exact whole seconds, e.g. 90.0 -> "01m 30.0s", not
        // "01m 30s") -- there is no separate "no decimal" case in here the
        // way there is for the two branches below.
        format!("{minute:02}m {}s", trim_trailing_zero(second, centisecond, true))
    } else if centisecond == 0 {
        format!("{second}s")
    } else {
        format!("{}s", trim_trailing_zero(second, centisecond, false))
    }
}

/// Parses a `[[HH:]MM:]SS[.cc]` string back into seconds. A leading `-` is
/// honored only when `accept_minus` is true (matching the original's
/// per-field toggle for inputs that must stay non-negative); otherwise a
/// negative input parses as `0.0`.
///
/// Returns `0.0` for an empty string or a token that doesn't parse as a
/// non-negative number, matching the original's silent-stop-and-return
/// behavior (it was written for a text input that filters keystrokes, so a
/// malformed value should not occur, but is not an error if it does).
pub fn parse_seconds(source: &str, accept_minus: bool) -> f64 {
    let source = source.trim();
    let (is_minus, source) = match source.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, source),
    };

    let tokens: Vec<&str> = source.split(':').collect();
    let mut result = 0.0;

    for (i, token) in tokens.iter().enumerate() {
        let Ok(value) = token.parse::<f64>() else {
            break;
        };
        if value < 0.0 {
            break;
        }
        let place = (tokens.len() - i - 1) as i32;
        result += 60f64.powi(place) * value;
    }

    if is_minus {
        if accept_minus { -result } else { 0.0 }
    } else {
        result
    }
}

/// Splits seconds into (hour, minute, second, centisecond), truncating
/// rather than rounding (matching the original's `Math.floor` throughout,
/// so e.g. 59.996s displays as `59.99`, not as `60.00`/`1:00.00`).
fn split(source_seconds: f64) -> (u64, u64, u64, u64) {
    let total = source_seconds.max(0.0);
    let hour = (total / 3600.0).floor();
    let minute = ((total - 3600.0 * hour) / 60.0).floor();
    let second = (total - 3600.0 * hour - 60.0 * minute).floor();
    let centisecond = ((total - 3600.0 * hour - 60.0 * minute - second) * 100.0).floor();

    (hour as u64, minute as u64, second as u64, centisecond as u64)
}

/// `(3, 50)` -> `"3.5"` (or `"03.5"` with `pad_second`), matching the
/// original's one-trailing-zero trim (it only ever strips a single zero,
/// not every trailing zero -- e.g. a hypothetical `.50` stays `.5`, not
/// `.05` -> `""`; centiseconds are always exactly two digits coming in, so
/// this never needs to strip more than one).
fn trim_trailing_zero(second: u64, centisecond: u64, pad_second: bool) -> String {
    let ms = format!("{centisecond:02}");
    let ms = ms.strip_suffix('0').unwrap_or(&ms);
    if pad_second {
        format!("{second:02}.{ms}")
    } else {
        format!("{second}.{ms}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_seconds_handles_missing_value() {
        assert_eq!(format_seconds(None), "--:--.--");
        assert_eq!(format_seconds(Some(f64::NAN)), "--:--.--");
    }

    #[test]
    fn format_seconds_without_hour() {
        assert_eq!(format_seconds(Some(0.0)), "00:00.00");
        assert_eq!(format_seconds(Some(65.5)), "01:05.50");
    }

    #[test]
    fn format_seconds_with_hour() {
        assert_eq!(format_seconds(Some(3661.25)), "01:01:01.25");
    }

    #[test]
    fn format_seconds_truncates_not_rounds() {
        assert_eq!(format_seconds(Some(59.996)), "00:59.99");
    }

    #[test]
    fn format_human_variants() {
        assert_eq!(format_human(3723.0), "01h 02m 03s");
        assert_eq!(format_human(123.5), "02m 03.5s");
        assert_eq!(format_human(3.0), "3s");
        assert_eq!(format_human(3.5), "3.5s");
    }

    #[test]
    fn parse_seconds_roundtrips_plain_numbers() {
        assert_eq!(parse_seconds("12.5", true), 12.5);
    }

    #[test]
    fn parse_seconds_handles_colons() {
        assert_eq!(parse_seconds("1:30", true), 90.0);
        assert_eq!(parse_seconds("1:01:01.25", true), 3661.25);
    }

    #[test]
    fn parse_seconds_minus_sign() {
        assert_eq!(parse_seconds("-5", true), -5.0);
        assert_eq!(parse_seconds("-5", false), 0.0);
    }

    #[test]
    fn parse_seconds_stops_at_first_bad_token() {
        // Matches the original: a malformed token breaks the loop and
        // whatever accumulated so far is returned, rather than erroring.
        assert_eq!(parse_seconds("1:xx", true), 60.0);
    }

    #[test]
    fn parse_seconds_empty_is_zero() {
        assert_eq!(parse_seconds("", true), 0.0);
    }
}
