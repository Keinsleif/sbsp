// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Ported from `utils.ts`: `easingToCurve`, `curveToEasing`.
//!
//! `sbsp_backend::model::cue::audio::Easing` already carries its intensity
//! inline per-variant (`InPow(f64)` etc.), unlike the old `ts-rs`-generated
//! TS type, which the Vue frontend bridged to a flat `{ type, power }`
//! shape for its curve-editor widget (a dropdown for the curve kind, plus
//! one shared number input for the intensity, since `Linear` has none).
//! [`Curve`] is that same flat shape, kept because the editor widget
//! (Phase 6) will likely want it for the same reason, not because the
//! Easing type itself needs bridging for serialization anymore.
//!
//! `sbsp_backend::model` has no feature gate, so this works on every
//! target (host, remote, and eventually web) without needing `backend` or
//! `protocol`.

use sbsp_backend::model::cue::audio::Easing;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveKind {
    Linear,
    InPow,
    OutPow,
    InOutPow,
}

/// The curve-editor widget's flat view of an [`Easing`]. `power` is `None`
/// only for `Linear`, which has no intensity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Curve {
    pub kind: CurveKind,
    pub power: Option<f64>,
}

impl From<Easing> for Curve {
    fn from(easing: Easing) -> Self {
        match easing {
            Easing::Linear => Curve {
                kind: CurveKind::Linear,
                power: None,
            },
            Easing::InPow(power) => Curve {
                kind: CurveKind::InPow,
                power: Some(power),
            },
            Easing::OutPow(power) => Curve {
                kind: CurveKind::OutPow,
                power: Some(power),
            },
            Easing::InOutPow(power) => Curve {
                kind: CurveKind::InOutPow,
                power: Some(power),
            },
        }
    }
}

impl From<Curve> for Easing {
    /// A missing `power` (only expected for a non-`Linear` kind if the
    /// widget's number input was somehow left empty) defaults to `2.0`,
    /// matching the original's fallback.
    fn from(curve: Curve) -> Self {
        let power = curve.power.unwrap_or(2.0);
        match curve.kind {
            CurveKind::Linear => Easing::Linear,
            CurveKind::InPow => Easing::InPow(power),
            CurveKind::OutPow => Easing::OutPow(power),
            CurveKind::InOutPow => Easing::InOutPow(power),
        }
    }
}

impl Curve {
    /// How far a fade has got (0..=1) after `x` (0..=1) of its time. The
    /// same curve as the backend's `Easing::get_factor`, which only exists
    /// with the `backend` feature; the editor draws it on every target.
    pub fn factor(&self, x: f64) -> f64 {
        let power = self.power.unwrap_or(2.0);
        match self.kind {
            CurveKind::Linear => x,
            CurveKind::InPow => x.powf(power),
            CurveKind::OutPow => 1.0 - (1.0 - x).powf(power),
            CurveKind::InOutPow => {
                let doubled = x * 2.0;
                if doubled < 1.0 {
                    0.5 * doubled.powf(power)
                } else {
                    0.5 * (1.0 - (2.0 - doubled).powf(power)) + 0.5
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_curve_runs_from_zero_to_one() {
        for kind in [
            CurveKind::Linear,
            CurveKind::InPow,
            CurveKind::OutPow,
            CurveKind::InOutPow,
        ] {
            let curve = Curve { kind, power: Some(3.0) };
            assert!((curve.factor(0.0)).abs() < 1e-12, "{kind:?} starts at 0");
            assert!((curve.factor(1.0) - 1.0).abs() < 1e-12, "{kind:?} ends at 1");
        }
    }

    #[test]
    fn the_shapes_differ_where_they_should() {
        let at = |kind, power| Curve { kind, power }.factor(0.5);
        assert_eq!(at(CurveKind::Linear, None), 0.5);
        assert_eq!(at(CurveKind::InPow, Some(2.0)), 0.25);
        assert_eq!(at(CurveKind::OutPow, Some(2.0)), 0.75);
        assert!((at(CurveKind::InOutPow, Some(2.0)) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn linear_has_no_power() {
        let curve: Curve = Easing::Linear.into();
        assert_eq!(
            curve,
            Curve {
                kind: CurveKind::Linear,
                power: None
            }
        );
    }

    #[test]
    fn pow_variants_carry_intensity() {
        let curve: Curve = Easing::InOutPow(3.0).into();
        assert_eq!(
            curve,
            Curve {
                kind: CurveKind::InOutPow,
                power: Some(3.0)
            }
        );
    }

    #[test]
    fn round_trips() {
        for easing in [
            Easing::Linear,
            Easing::InPow(1.5),
            Easing::OutPow(2.5),
            Easing::InOutPow(4.0),
        ] {
            let curve: Curve = easing.into();
            assert_eq!(Easing::from(curve), easing);
        }
    }

    #[test]
    fn missing_power_defaults_to_two() {
        let curve = Curve {
            kind: CurveKind::InPow,
            power: None,
        };
        assert_eq!(Easing::from(curve), Easing::InPow(2.0));
    }
}
