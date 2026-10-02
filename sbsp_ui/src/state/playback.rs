// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Ported from `stores/showState.ts`.
//!
//! `sbsp_backend::controller::state` (`ActiveCue`/`PlaybackStatus`/
//! `ShowState`/`StateParam`) has no feature gate, so it's available on
//! every target. The event types that feed `handle_sync_event`/
//! `handle_cue_state_event` (`SyncData`/`CueStatusEventParam`, from
//! `sbsp_backend::event`) are gated behind the `backend` feature (host and
//! remote both have it via `server`/`client`; web does not), so those two
//! methods are `#[cfg(any(feature = "host", feature = "remote"))]`
//! (`sbsp_ui`'s own features, not `sbsp_backend`'s -- `#[cfg(feature =
//! "backend")]` here would check this crate's features, which has no such
//! feature, so it would just always be compiled out). `update`/`calculate_position`/
//! `get_position` only need the always-available types and stay
//! unconditional.
//!
//! One deliberate change from the original: every method that needs "now"
//! (`performance.now()` in the original) takes it as an explicit `Instant`
//! parameter instead of calling `Instant::now()` internally. This makes
//! the interpolation math in `calculate_position`/`get_position` testable
//! without a real clock; the caller (Phase 4+ wiring) passes
//! `Instant::now()`.

use std::collections::HashMap;
use std::time::Instant;

#[cfg(any(feature = "host", feature = "remote"))]
use sbsp_backend::event::{CueStatusEventParam, SyncData};
use sbsp_backend::{
    controller::state::{ActiveCue, PlaybackStatus, ShowState, StateParam},
    model::cue::Uuid,
};

#[derive(Debug, Clone)]
pub struct SyncedCue {
    pub position: f64,
    pub status: PlaybackStatus,
    pub last_synced_at: Instant,
}

#[derive(Default)]
pub struct PlaybackState {
    pub active_cues: HashMap<Uuid, ActiveCue>,
    pub synced_data: HashMap<Uuid, SyncedCue>,
    pub latency: f64,
}

impl PlaybackState {
    pub fn new() -> Self {
        Self::default()
    }

    /// A periodic full sync: updates `latency` and each synced cue's
    /// position, then drops any `active_cues` entry no longer present in
    /// `synced_data` (the original's own "status may be broken!" warning
    /// path -- a cue appearing in the sync without existing synced data --
    /// is preserved as a `log::warn!`).
    #[cfg(any(feature = "host", feature = "remote"))]
    pub fn handle_sync_event(&mut self, data: &SyncData, now: Instant) {
        for cue in &data.cues {
            self.latency = data.latency;
            match self.synced_data.get_mut(&cue.id) {
                Some(target) => {
                    target.position = cue.position;
                    target.last_synced_at = now;
                }
                None => {
                    log::warn!("status may be broken!");
                    self.synced_data.insert(
                        cue.id,
                        SyncedCue {
                            position: 0.0,
                            status: PlaybackStatus::Playing,
                            last_synced_at: now,
                        },
                    );
                }
            }
        }

        self.active_cues.retain(|id, _| self.synced_data.contains_key(id));
    }

    /// A full replace from a freshly (re)connected `ShowState` snapshot.
    pub fn update(&mut self, state: &ShowState, now: Instant) {
        let mut new_synced = HashMap::with_capacity(state.active_cues.len());
        let mut new_active = HashMap::with_capacity(state.active_cues.len());

        for (&id, active_cue) in &state.active_cues {
            new_synced.insert(
                id,
                SyncedCue {
                    position: active_cue.position,
                    status: active_cue.status,
                    last_synced_at: now,
                },
            );
            new_active.insert(id, active_cue.clone());
        }

        self.synced_data = new_synced;
        self.active_cues = new_active;
    }

    /// Applies one incremental cue-status event. See `CueStatusEventParam`
    /// for the full set; each arm mirrors the original's `switch` case of
    /// the same name.
    #[cfg(any(feature = "host", feature = "remote"))]
    pub fn handle_cue_state_event(&mut self, data: &CueStatusEventParam, now: Instant) {
        use CueStatusEventParam as E;

        match data {
            E::Triggered { .. } => {}
            E::Loaded { cue_id, position, duration } => {
                if !self.synced_data.contains_key(cue_id) {
                    self.synced_data.insert(
                        *cue_id,
                        SyncedCue { position: *position, status: PlaybackStatus::Loaded, last_synced_at: now },
                    );
                    self.active_cues.insert(
                        *cue_id,
                        ActiveCue {
                            cue_id: *cue_id,
                            position: *position,
                            duration: *duration,
                            status: PlaybackStatus::Loaded,
                            params: StateParam::None,
                        },
                    );
                }
            }
            E::PreWaitStarted { cue_id, duration } => {
                self.synced_data.insert(
                    *cue_id,
                    SyncedCue { position: 0.0, status: PlaybackStatus::PreWaiting, last_synced_at: now },
                );
                self.active_cues.insert(
                    *cue_id,
                    ActiveCue {
                        cue_id: *cue_id,
                        position: 0.0,
                        duration: *duration,
                        status: PlaybackStatus::PreWaiting,
                        params: StateParam::None,
                    },
                );
            }
            E::PreWaitPaused { cue_id, position } => {
                self.synced_data.insert(
                    *cue_id,
                    SyncedCue { position: *position, status: PlaybackStatus::PreWaitPaused, last_synced_at: now },
                );
                if let Some(active) = self.active_cues.get_mut(cue_id) {
                    active.position = *position;
                }
            }
            E::PreWaitResumed { cue_id } => {
                if let Some(target) = self.synced_data.get_mut(cue_id) {
                    target.status = PlaybackStatus::PreWaiting;
                    target.last_synced_at = now;
                }
            }
            // The backend auto-triggers the start cue; nothing to do here.
            E::PreWaitCompleted { .. } => {}
            E::Started { cue_id, position, duration, params } => {
                self.synced_data.insert(
                    *cue_id,
                    SyncedCue { position: *position, status: PlaybackStatus::Playing, last_synced_at: now },
                );
                self.active_cues.insert(
                    *cue_id,
                    ActiveCue {
                        cue_id: *cue_id,
                        position: *position,
                        duration: *duration,
                        status: PlaybackStatus::Playing,
                        params: *params,
                    },
                );
            }
            E::Paused { cue_id, position } => {
                self.synced_data.insert(
                    *cue_id,
                    SyncedCue { position: *position, status: PlaybackStatus::Paused, last_synced_at: now },
                );
                if let Some(active) = self.active_cues.get_mut(cue_id) {
                    active.position = *position;
                }
            }
            E::Resumed { cue_id } => {
                if let Some(target) = self.synced_data.get_mut(cue_id) {
                    target.status = PlaybackStatus::Playing;
                    target.last_synced_at = now;
                }
            }
            E::Stopping { cue_id } => {
                if let Some(target) = self.synced_data.get_mut(cue_id) {
                    target.status = PlaybackStatus::Stopping;
                }
            }
            E::Seeked { cue_id, position } => {
                if let Some(target) = self.synced_data.get_mut(cue_id) {
                    target.position = *position;
                    target.last_synced_at = now;
                }
                if let Some(active) = self.active_cues.get_mut(cue_id) {
                    active.position = *position;
                }
            }
            E::Stopped { cue_id } | E::Completed { cue_id } | E::Error { cue_id, .. } => {
                self.synced_data.remove(cue_id);
                self.active_cues.remove(cue_id);
            }
            E::StateParamUpdated { cue_id, params } => {
                if let Some(active) = self.active_cues.get_mut(cue_id) {
                    active.params = *params;
                }
            }
        }
    }

    /// Every synced cue's interpolated position right now. When
    /// `update_active_cues` is true, each `ActiveCue`'s stored `position`
    /// (and `status`, if it changed) is updated to match -- matching the
    /// original, which is called this way once per animation frame to
    /// drive the UI's own reactive state.
    pub fn calculate_position(&mut self, update_active_cues: bool, now: Instant) -> HashMap<Uuid, f64> {
        let latency = self.latency;
        let mut positions = HashMap::with_capacity(self.synced_data.len());

        for (&cue_id, synced) in &self.synced_data {
            let active = self.active_cues.entry(cue_id).or_insert_with(|| ActiveCue {
                cue_id,
                position: synced.position,
                duration: 0.0,
                status: synced.status,
                params: StateParam::None,
            });

            if active.status != synced.status {
                active.status = synced.status;
            }

            let position = resolve_position(synced, active, latency, now);

            if update_active_cues && position != active.position {
                active.position = position;
            }

            positions.insert(cue_id, position);
        }

        positions
    }

    /// A single cue's interpolated position, without touching stored
    /// state. `None` if `id` isn't synced; also `None` (after a warning,
    /// matching the original) if it's synced but has no matching
    /// `active_cues` entry, which should not normally happen.
    pub fn get_position(&self, id: Uuid, now: Instant) -> Option<f64> {
        let synced = self.synced_data.get(&id)?;
        let Some(active) = self.active_cues.get(&id) else {
            log::warn!("ShowState sync broken.");
            return None;
        };

        Some(resolve_position(synced, active, self.latency, now))
    }
}

/// Shared by `calculate_position`/`get_position`: while a cue is in an
/// "in-flight" status (`PreWaiting`/`Playing`/`Stopping`) and has a known
/// duration, extrapolate forward from the last sync by elapsed time (plus
/// half the measured round-trip latency), wrapping for a repeating audio
/// cue or clamping otherwise. Anything else (not in-flight, or no known
/// duration yet) just reports the last synced position as-is.
fn resolve_position(synced: &SyncedCue, active: &ActiveCue, latency: f64, now: Instant) -> f64 {
    let in_flight = matches!(
        synced.status,
        PlaybackStatus::PreWaiting | PlaybackStatus::Playing | PlaybackStatus::Stopping
    );

    if in_flight && active.duration > 0.0 {
        let elapsed = now.saturating_duration_since(synced.last_synced_at).as_secs_f64();
        let raw = synced.position + latency / 2.0 + elapsed;

        if matches!(active.params, StateParam::Audio(p) if p.repeating) {
            raw % active.duration
        } else {
            raw.min(active.duration)
        }
    } else {
        synced.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbsp_backend::controller::state::AudioStateParam;
    use std::time::Duration;

    fn synced(position: f64, status: PlaybackStatus, last_synced_at: Instant) -> SyncedCue {
        SyncedCue { position, status, last_synced_at }
    }

    fn active(cue_id: Uuid, position: f64, duration: f64, status: PlaybackStatus, params: StateParam) -> ActiveCue {
        ActiveCue { cue_id, position, duration, status, params }
    }

    #[test]
    fn stopped_cue_reports_last_synced_position_unchanged() {
        let id = Uuid::new_v4();
        let t0 = Instant::now();
        let mut state = PlaybackState::new();
        state.synced_data.insert(id, synced(5.0, PlaybackStatus::Paused, t0));
        state.active_cues.insert(id, active(id, 5.0, 10.0, PlaybackStatus::Paused, StateParam::None));

        let later = t0 + Duration::from_secs(3);
        assert_eq!(state.get_position(id, later), Some(5.0));
    }

    #[test]
    fn playing_cue_extrapolates_elapsed_time() {
        let id = Uuid::new_v4();
        let t0 = Instant::now();
        let mut state = PlaybackState::new();
        state.synced_data.insert(id, synced(2.0, PlaybackStatus::Playing, t0));
        state.active_cues.insert(id, active(id, 2.0, 10.0, PlaybackStatus::Playing, StateParam::None));

        let later = t0 + Duration::from_secs(1);
        assert_eq!(state.get_position(id, later), Some(3.0));
    }

    #[test]
    fn playing_cue_clamps_to_duration_when_not_repeating() {
        let id = Uuid::new_v4();
        let t0 = Instant::now();
        let mut state = PlaybackState::new();
        state.synced_data.insert(id, synced(9.5, PlaybackStatus::Playing, t0));
        state.active_cues.insert(id, active(id, 9.5, 10.0, PlaybackStatus::Playing, StateParam::None));

        let later = t0 + Duration::from_secs(5);
        assert_eq!(state.get_position(id, later), Some(10.0));
    }

    #[test]
    fn repeating_audio_cue_wraps_around_duration() {
        let id = Uuid::new_v4();
        let t0 = Instant::now();
        let mut state = PlaybackState::new();
        state.synced_data.insert(id, synced(9.0, PlaybackStatus::Playing, t0));
        state.active_cues.insert(
            id,
            active(
                id,
                9.0,
                10.0,
                PlaybackStatus::Playing,
                StateParam::Audio(AudioStateParam { repeating: true, volume: Default::default() }),
            ),
        );

        // 9.0 + 3.0 elapsed = 12.0, wraps to 2.0 over a 10.0 duration.
        let later = t0 + Duration::from_secs(3);
        assert_eq!(state.get_position(id, later), Some(2.0));
    }

    #[test]
    fn latency_is_halved_and_added() {
        let id = Uuid::new_v4();
        let t0 = Instant::now();
        let mut state = PlaybackState::new();
        state.latency = 0.2;
        state.synced_data.insert(id, synced(0.0, PlaybackStatus::Playing, t0));
        state.active_cues.insert(id, active(id, 0.0, 10.0, PlaybackStatus::Playing, StateParam::None));

        assert_eq!(state.get_position(id, t0), Some(0.1));
    }

    #[test]
    fn zero_duration_does_not_extrapolate() {
        let id = Uuid::new_v4();
        let t0 = Instant::now();
        let mut state = PlaybackState::new();
        state.synced_data.insert(id, synced(1.0, PlaybackStatus::Playing, t0));
        state.active_cues.insert(id, active(id, 1.0, 0.0, PlaybackStatus::Playing, StateParam::None));

        let later = t0 + Duration::from_secs(5);
        assert_eq!(state.get_position(id, later), Some(1.0));
    }

    #[test]
    fn get_position_none_when_not_synced() {
        let state = PlaybackState::new();
        assert_eq!(state.get_position(Uuid::new_v4(), Instant::now()), None);
    }

    #[test]
    fn calculate_position_creates_missing_active_cue() {
        let id = Uuid::new_v4();
        let t0 = Instant::now();
        let mut state = PlaybackState::new();
        state.synced_data.insert(id, synced(4.0, PlaybackStatus::Paused, t0));

        let positions = state.calculate_position(true, t0);

        assert_eq!(positions.get(&id), Some(&4.0));
        assert!(state.active_cues.contains_key(&id));
    }

    #[test]
    fn calculate_position_updates_active_cue_when_requested() {
        let id = Uuid::new_v4();
        let t0 = Instant::now();
        let mut state = PlaybackState::new();
        state.synced_data.insert(id, synced(2.0, PlaybackStatus::Playing, t0));
        state.active_cues.insert(id, active(id, 2.0, 10.0, PlaybackStatus::Playing, StateParam::None));

        let later = t0 + Duration::from_secs(1);
        state.calculate_position(true, later);

        assert_eq!(state.active_cues[&id].position, 3.0);
    }

    #[test]
    fn calculate_position_leaves_active_cue_when_not_requested() {
        let id = Uuid::new_v4();
        let t0 = Instant::now();
        let mut state = PlaybackState::new();
        state.synced_data.insert(id, synced(2.0, PlaybackStatus::Playing, t0));
        state.active_cues.insert(id, active(id, 2.0, 10.0, PlaybackStatus::Playing, StateParam::None));

        let later = t0 + Duration::from_secs(1);
        state.calculate_position(false, later);

        assert_eq!(state.active_cues[&id].position, 2.0);
    }

    #[test]
    fn update_replaces_from_show_state_snapshot() {
        let id = Uuid::new_v4();
        let t0 = Instant::now();
        let mut state = PlaybackState::new();
        state.synced_data.insert(Uuid::new_v4(), synced(1.0, PlaybackStatus::Playing, t0));

        let mut show_state = ShowState::new();
        show_state
            .active_cues
            .insert(id, active(id, 7.0, 20.0, PlaybackStatus::Paused, StateParam::None));

        state.update(&show_state, t0);

        assert_eq!(state.synced_data.len(), 1);
        assert!(state.synced_data.contains_key(&id));
        assert_eq!(state.synced_data[&id].position, 7.0);
    }
}
