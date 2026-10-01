// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Ported from `utils.ts`: `faderToDecibels`, `decibelsToFader`.
//!
//! The UI's fader position is not linear in dB: it stretches the range
//! below -10 dB so quiet levels are easier to set precisely with a slider,
//! using two piecewise-linear segments. Values stay as plain `f64`; which
//! "unit" a value is in is tracked by which function produced it. A newtype
//! wrapper (`FaderPosition(f64)` / `Decibels(f64)`) may be worth adding once
//! there's a concrete mis-mix bug to point to, rather than speculating on a
//! design with no usage yet to validate it against.
//!
//! The two functions are exact inverses of each other (checked by hand,
//! not just by the tests below): both breakpoints -- fader -10 / dB -10,
//! and fader -25 / dB -40 -- line up, and each segment's formula inverts
//! cleanly into the other direction's formula for that segment.

/// Fader position -> dB.
pub fn fader_to_decibels(fader: f64) -> f64 {
    if fader > -10.0 {
        fader
    } else if fader > -25.0 {
        2.0 * (fader + 10.0) - 10.0
    } else {
        4.0 * (fader + 25.0) - 40.0
    }
}

/// dB -> fader position.
pub fn decibels_to_fader(decibels: f64) -> f64 {
    if decibels > -10.0 {
        decibels
    } else if decibels > -40.0 {
        (decibels + 10.0) / 2.0 - 10.0
    } else {
        (decibels + 40.0) / 4.0 - 25.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_above_minus_ten() {
        assert_eq!(fader_to_decibels(0.0), 0.0);
        assert_eq!(fader_to_decibels(-5.0), -5.0);
        assert_eq!(decibels_to_fader(-5.0), -5.0);
    }

    #[test]
    fn breakpoints_are_continuous() {
        // fader -25 is the breakpoint between faderToDecibels' two lower
        // segments: 2*(-25+10)-10 = -40.
        assert_eq!(fader_to_decibels(-25.0), -40.0);
        // -40 dB is decibelsToFader's matching breakpoint, recovering -25.
        assert_eq!(decibels_to_fader(-40.0), -25.0);
    }

    #[test]
    fn round_trips_exactly() {
        for fader in [0.0, -5.0, -10.0, -15.0, -25.0, -30.0, -60.0, -100.0] {
            let db = fader_to_decibels(fader);
            assert_eq!(decibels_to_fader(db), fader, "fader {fader} -> dB {db}");
        }
    }

    #[test]
    fn deep_attenuation() {
        assert_eq!(fader_to_decibels(-60.0), 4.0 * (-60.0 + 25.0) - 40.0);
        assert_eq!(decibels_to_fader(-100.0), (-100.0 + 40.0) / 4.0 - 25.0);
    }
}
