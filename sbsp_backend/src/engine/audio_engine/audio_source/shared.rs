// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

use super::AudioPlaybackState;

pub struct AudioSourceShared {
    state: AtomicU8,
    position: AtomicU64,
    repeat: AtomicBool,
}

impl AudioSourceShared {
    pub const fn new(repeat: bool) -> Self {
        Self {
            state: AtomicU8::new(AudioPlaybackState::Loaded as u8),
            position: AtomicU64::new(0),
            repeat: AtomicBool::new(repeat),
        }
    }

    #[inline]
    pub fn load_state(&self) -> AudioPlaybackState {
        let raw = self.state.load(Ordering::Acquire);

        unsafe { std::mem::transmute(raw) }
    }

    #[inline]
    pub fn store_state(&self, state: AudioPlaybackState) {
        self.state.store(state as u8, Ordering::Release);
    }

    #[inline]
    pub fn load_position(&self) -> f64 {
        f64::from_bits(self.position.load(Ordering::Acquire))
    }

    #[inline]
    pub fn store_position(&self, position: f64) {
        self.position.store(position.to_bits(), Ordering::Release);
    }

    #[inline]
    pub fn load_repeat(&self) -> bool {
        self.repeat.load(Ordering::Acquire)
    }

    #[inline]
    pub fn store_repeat(&self, repeat: bool) {
        self.repeat.store(repeat, Ordering::Release);
    }
}
