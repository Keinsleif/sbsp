// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[repr(u8)]
pub enum AudioPlaybackState {
    Loaded,
    Playing,
    Pausing,
    Paused,
    Resuming,
    SoftStopping,
    HardStopping,
    Stopped,
    Completed,
}

impl AudioPlaybackState {
    #[inline]
    pub fn is_stopped(&self) -> bool {
        match *self {
            AudioPlaybackState::Stopped | AudioPlaybackState::Completed => true,
            AudioPlaybackState::Loaded
            | AudioPlaybackState::Playing
            | AudioPlaybackState::Pausing
            | AudioPlaybackState::Paused
            | AudioPlaybackState::Resuming
            | AudioPlaybackState::SoftStopping
            | AudioPlaybackState::HardStopping => false,
        }
    }

    #[inline]
    pub fn is_advancing(&self) -> bool {
        match *self {
            AudioPlaybackState::Loaded
            | AudioPlaybackState::Paused
            | AudioPlaybackState::Stopped
            | AudioPlaybackState::Completed => false,
            AudioPlaybackState::Playing
            | AudioPlaybackState::Pausing
            | AudioPlaybackState::Resuming
            | AudioPlaybackState::SoftStopping
            | AudioPlaybackState::HardStopping => true,
        }
    }
}

impl PartialEq<AudioPlaybackState> for u8 {
    fn eq(&self, other: &AudioPlaybackState) -> bool {
        *self == (*other as u8)
    }
}
