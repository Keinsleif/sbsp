// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use crate::model::cue::audio::{Decibels, FadeParam};

type FadeCompleted = bool;

struct VolumeFadeInfo {
    from: Decibels,
    to: Decibels,
    elapsed: f64,
    fade_param: FadeParam,
}

pub struct Volume {
    pub level: Decibels,
    fade_info: Option<VolumeFadeInfo>,
}

impl Volume {
    pub fn new(level: Decibels) -> Self {
        Self {
            level,
            fade_info: None,
        }
    }

    pub fn new_with_fade(init: Decibels, target: Decibels, fade_param: FadeParam) -> Self {
        Self {
            level: init,
            fade_info: Some(VolumeFadeInfo {
                from: init,
                to: target,
                elapsed: 0.0,
                fade_param,
            }),
        }
    }

    #[inline]
    pub fn set_volume(&mut self, volume: Decibels, fade_param: FadeParam) {
        if let Some(info) = self.fade_info.as_mut() {
            info.from = self.level;
            info.to = volume;
            info.elapsed = 0.0;
            info.fade_param = fade_param;
        } else {
            self.fade_info = Some(VolumeFadeInfo {
                from: self.level,
                to: volume,
                elapsed: 0.0,
                fade_param,
            });
        }
    }

    #[inline]
    pub fn update(&mut self, dt: f64) -> FadeCompleted {
        let info = match self.fade_info.as_mut() {
            Some(i) => i,
            None => return false,
        };

        if info.elapsed >= info.fade_param.duration {
            self.level = info.to;
            self.fade_info = None;
            true
        } else {
            let progress =
                info.fade_param
                    .easing
                    .get_factor(info.elapsed / info.fade_param.duration) as f32;

            self.level = info.from + (info.to - info.from) * progress;
            info.elapsed += dt;
            false
        }
    }
}
