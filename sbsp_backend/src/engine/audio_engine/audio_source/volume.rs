use crate::model::cue::audio::{Decibels, FadeParam};

struct VolumeFadeInfo {
    pub from: Decibels,
    pub to: Decibels,
    pub elapsed: f64,
    pub fade_param: FadeParam,
}

pub struct Volume {
    pub volume: Decibels,
    fade_info: Option<VolumeFadeInfo>,
}

impl Volume {
    pub fn new(volume: Decibels) -> Self {
        Self {
            volume,
            fade_info: None,
        }
    }

    pub fn new_with_fade(init: Decibels, target: Decibels, fade_param: FadeParam) -> Self {
        Self {
            volume: init,
            fade_info: Some(VolumeFadeInfo {
                from: init,
                to: target,
                elapsed: 0.0,
                fade_param,
            }),
        }
    }

    pub fn set_volume(&mut self, volume: Decibels, fade_param: FadeParam) {
        if let Some(info) = self.fade_info.as_mut() {
            info.from = self.volume;
            info.to = volume;
            info.elapsed = 0.0;
            info.fade_param = fade_param;
        } else {
            self.fade_info = Some(VolumeFadeInfo {
                from: self.volume,
                to: volume,
                elapsed: 0.0,
                fade_param,
            });
        }
    }

    pub fn update(&mut self, dt: f64) -> bool {
        if let Some(info) = self.fade_info.as_mut() {
            if info.elapsed >= info.fade_param.duration {
                self.volume = info.to;
                return true;
            }

            let progress =
                info.fade_param
                    .easing
                    .get_factor(info.elapsed / info.fade_param.duration) as f32;

            self.volume = info.from + (info.to - info.from) * progress;

            info.elapsed += dt;
        }
        false
    }
}
