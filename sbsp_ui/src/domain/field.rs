// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! What committing a text field means, ported from the `save()` functions
//! of `NumberInput.vue`, `TimeInput.vue` and `VolumeInput.vue`.
//!
//! A field edits a string; when the edit is finished (Enter, focus lost)
//! the string is turned back into a value here. Every function is pure:
//! it gets the text and the current value and says what should happen, so
//! the widgets stay dumb and the rules are testable without a window.

use super::time::{format_seconds, parse_seconds};

/// What committing a field does to its value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Commit<T> {
    /// Nothing changed (or the text was not usable): show the current
    /// value again.
    Unchanged,
    /// The field was cleared, which is allowed to mean "no value".
    Null,
    Value(T),
}

// ---------------------------------------------------------------------
// Numbers

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumberSpec {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: f64,
    pub precision: usize,
    /// Whether an empty field means `None` rather than being rejected.
    pub accept_null: bool,
}

impl Default for NumberSpec {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
            step: 1.0,
            precision: 0,
            accept_null: false,
        }
    }
}

pub fn format_number(value: Option<f64>, precision: usize) -> String {
    value.map_or_else(String::new, |value| format!("{value:.precision$}"))
}

/// JavaScript's `parseFloat`: the longest numeric prefix, so "12abc" is 12
/// and "abc" is not a number. (Rust's `parse` would reject the former.)
pub fn parse_float_prefix(source: &str) -> Option<f64> {
    let source = source.trim_start();
    let bytes = source.as_bytes();
    let mut end = 0;

    if matches!(bytes.first(), Some(b'+' | b'-')) {
        end += 1;
    }
    let digits_from = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    let mut digits = end - digits_from;
    if bytes.get(end) == Some(&b'.') {
        let fraction_from = end + 1;
        let mut fraction_end = fraction_from;
        while bytes.get(fraction_end).is_some_and(u8::is_ascii_digit) {
            fraction_end += 1;
        }
        // "1." is 1, and ".5" is 0.5, but "." alone is nothing.
        if digits > 0 || fraction_end > fraction_from {
            end = fraction_end;
            digits += fraction_end - fraction_from;
        }
    }
    if digits == 0 {
        return None;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let mut exp_end = end + 1;
        if matches!(bytes.get(exp_end), Some(b'+' | b'-')) {
            exp_end += 1;
        }
        let exp_digits_from = exp_end;
        while bytes.get(exp_end).is_some_and(u8::is_ascii_digit) {
            exp_end += 1;
        }
        if exp_end > exp_digits_from {
            end = exp_end;
        }
    }

    source[..end].parse().ok()
}

fn clamp_to(value: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    match (min, max) {
        (_, Some(max)) if value > max => max,
        (Some(min), _) if value < min => min,
        _ => value,
    }
}

/// `Number(x.toFixed(precision))`: round to the field's precision.
fn round_to(value: f64, precision: usize) -> f64 {
    format!("{value:.precision$}").parse().unwrap_or(value)
}

pub fn commit_number(text: &str, current: Option<f64>, spec: &NumberSpec) -> Commit<f64> {
    let text = text.trim();
    if text == format_number(current, spec.precision) {
        return Commit::Unchanged;
    }
    if text.is_empty() {
        return if spec.accept_null {
            Commit::Null
        } else {
            Commit::Unchanged
        };
    }
    let Some(parsed) = parse_float_prefix(text) else {
        return Commit::Unchanged;
    };

    let value = round_to(clamp_to(parsed, spec.min, spec.max), spec.precision);
    if Some(value) == current {
        // Typed something that lands on the current value (e.g. "5" for
        // "5.00"); only the display needs refreshing.
        Commit::Unchanged
    } else {
        Commit::Value(value)
    }
}

/// The +/- buttons. Starts from the lower bound (up) or upper bound
/// (down) when there is no value yet, as the original does.
pub fn step_number(current: Option<f64>, spec: &NumberSpec, direction: i8) -> f64 {
    if direction >= 0 {
        let base = current.or(spec.min).unwrap_or(0.0);
        let next = base + spec.step;
        spec.max.map_or(next, |max| next.min(max))
    } else {
        let base = current.or(spec.max).unwrap_or(0.0);
        let next = base - spec.step;
        spec.min.map_or(next, |min| next.max(min))
    }
}

// ---------------------------------------------------------------------
// Times

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeSpec {
    pub accept_minus: bool,
    /// The field shows `value * multiply` (e.g. a duration stored in
    /// seconds but edited in some other unit).
    pub multiply: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// What an emptied field becomes.
    pub default: f64,
}

impl Default for TimeSpec {
    fn default() -> Self {
        Self {
            accept_minus: false,
            multiply: 1.0,
            min: None,
            max: None,
            default: 0.0,
        }
    }
}

pub fn format_time(value: Option<f64>, spec: &TimeSpec) -> String {
    format_seconds(value.map(|value| value * spec.multiply))
}

pub fn commit_time(text: &str, current: Option<f64>, spec: &TimeSpec) -> Commit<f64> {
    let text = text.trim();
    if text == format_time(current, spec) {
        return Commit::Unchanged;
    }

    let value = if text.is_empty() {
        spec.default
    } else {
        parse_seconds(text, spec.accept_minus) / spec.multiply
    };
    let value = clamp_to(value, spec.min, spec.max);

    if Some(value) == current {
        Commit::Unchanged
    } else {
        Commit::Value(value)
    }
}

// ---------------------------------------------------------------------
// Volume

/// Where "silence" starts: at or below this the field shows `-∞`.
pub const VOLUME_FLOOR_DB: f64 = -60.0;

pub fn format_volume(decibels: f64) -> String {
    if decibels <= VOLUME_FLOOR_DB {
        "-∞".to_string()
    } else {
        format!("{decibels:.2}")
    }
}

/// An empty field is 0 dB (JavaScript's `Number("")`); text that is not a
/// number, including the `-∞` the field itself shows, reverts.
pub fn commit_volume(text: &str, current: f64) -> Commit<f64> {
    let text = text.trim();
    if text == format_volume(current) {
        return Commit::Unchanged;
    }
    let value = if text.is_empty() {
        0.0
    } else {
        match text.parse::<f64>() {
            Ok(value) if value.is_finite() => value,
            _ => return Commit::Unchanged,
        }
    };

    if value == current {
        Commit::Unchanged
    } else {
        Commit::Value(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(min: Option<f64>, max: Option<f64>, precision: usize) -> NumberSpec {
        NumberSpec {
            min,
            max,
            precision,
            ..NumberSpec::default()
        }
    }

    #[test]
    fn parse_float_prefix_reads_the_numeric_prefix_like_javascript() {
        assert_eq!(parse_float_prefix("12"), Some(12.0));
        assert_eq!(parse_float_prefix("12abc"), Some(12.0));
        assert_eq!(parse_float_prefix("  -3.5x"), Some(-3.5));
        assert_eq!(parse_float_prefix(".5"), Some(0.5));
        assert_eq!(parse_float_prefix("1."), Some(1.0));
        assert_eq!(parse_float_prefix("1e2"), Some(100.0));
        assert_eq!(parse_float_prefix("1e"), Some(1.0));
        assert_eq!(parse_float_prefix("abc"), None);
        assert_eq!(parse_float_prefix("."), None);
        assert_eq!(parse_float_prefix("-"), None);
        assert_eq!(parse_float_prefix(""), None);
    }

    #[test]
    fn a_number_is_formatted_to_its_precision() {
        assert_eq!(format_number(Some(2.5), 2), "2.50");
        assert_eq!(format_number(Some(3.0), 0), "3");
        assert_eq!(format_number(None, 2), "");
    }

    #[test]
    fn committing_the_shown_text_changes_nothing() {
        let spec = spec(None, None, 2);
        assert_eq!(commit_number("2.50", Some(2.5), &spec), Commit::Unchanged);
        assert_eq!(commit_number("  2.50 ", Some(2.5), &spec), Commit::Unchanged);
    }

    #[test]
    fn a_new_number_is_clamped_and_rounded() {
        let spec = spec(Some(0.0), Some(10.0), 1);
        assert_eq!(commit_number("4.26", Some(1.0), &spec), Commit::Value(4.3));
        assert_eq!(commit_number("99", Some(1.0), &spec), Commit::Value(10.0));
        assert_eq!(commit_number("-5", Some(1.0), &spec), Commit::Value(0.0));
    }

    #[test]
    fn a_number_that_rounds_to_the_current_value_is_unchanged() {
        let spec = spec(None, None, 0);
        assert_eq!(commit_number("5.2", Some(5.0), &spec), Commit::Unchanged);
    }

    #[test]
    fn unparsable_text_reverts() {
        let spec = spec(None, None, 0);
        assert_eq!(commit_number("abc", Some(5.0), &spec), Commit::Unchanged);
    }

    #[test]
    fn an_empty_number_is_null_only_when_allowed() {
        let mut spec = spec(None, None, 0);
        assert_eq!(commit_number("", Some(5.0), &spec), Commit::Unchanged);

        spec.accept_null = true;
        assert_eq!(commit_number("", Some(5.0), &spec), Commit::Null);
        // Already empty: nothing to do.
        assert_eq!(commit_number("", None, &spec), Commit::Unchanged);
    }

    #[test]
    fn the_buttons_step_and_stop_at_the_bounds() {
        let spec = NumberSpec {
            min: Some(0.0),
            max: Some(1.0),
            step: 0.25,
            ..NumberSpec::default()
        };
        assert_eq!(step_number(Some(0.5), &spec, 1), 0.75);
        assert_eq!(step_number(Some(0.9), &spec, 1), 1.0);
        assert_eq!(step_number(Some(0.1), &spec, -1), 0.0);
        // No value yet: up starts from min, down from max.
        assert_eq!(step_number(None, &spec, 1), 0.25);
        assert_eq!(step_number(None, &spec, -1), 0.75);
    }

    #[test]
    fn a_time_is_parsed_clamped_and_compared() {
        let spec = TimeSpec {
            max: Some(100.0),
            ..TimeSpec::default()
        };
        assert_eq!(commit_time("01:30.00", Some(0.0), &spec), Commit::Value(90.0));
        assert_eq!(commit_time("10:00", Some(0.0), &spec), Commit::Value(100.0));
        assert_eq!(commit_time("00:00.00", Some(0.0), &spec), Commit::Unchanged);
    }

    #[test]
    fn an_empty_time_becomes_the_default() {
        let spec = TimeSpec {
            default: 5.0,
            ..TimeSpec::default()
        };
        assert_eq!(commit_time("", Some(1.0), &spec), Commit::Value(5.0));
    }

    #[test]
    fn a_negative_time_needs_accept_minus() {
        let plain = TimeSpec::default();
        assert_eq!(commit_time("-00:10.00", Some(3.0), &plain), Commit::Value(0.0));

        let signed = TimeSpec {
            accept_minus: true,
            ..TimeSpec::default()
        };
        assert_eq!(commit_time("-00:10.00", Some(3.0), &signed), Commit::Value(-10.0));
    }

    #[test]
    fn the_time_multiplier_applies_both_ways() {
        let spec = TimeSpec {
            multiply: 2.0,
            ..TimeSpec::default()
        };
        // Shown value is value * 2.
        assert_eq!(format_time(Some(30.0), &spec), "01:00.00");
        assert_eq!(commit_time("02:00.00", Some(30.0), &spec), Commit::Value(60.0));
    }

    #[test]
    fn the_volume_floor_shows_as_minus_infinity() {
        assert_eq!(format_volume(-60.0), "-∞");
        assert_eq!(format_volume(-75.0), "-∞");
        assert_eq!(format_volume(-12.346), "-12.35");
        assert_eq!(format_volume(0.0), "0.00");
    }

    #[test]
    fn a_volume_commits_numbers_and_reverts_text() {
        assert_eq!(commit_volume("-6", 0.0), Commit::Value(-6.0));
        assert_eq!(commit_volume("0.00", 0.0), Commit::Unchanged);
        assert_eq!(commit_volume("-∞", -60.0), Commit::Unchanged);
        assert_eq!(commit_volume("loud", 0.0), Commit::Unchanged);
        assert_eq!(commit_volume("", -6.0), Commit::Value(0.0));
    }
}
