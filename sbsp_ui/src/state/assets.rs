// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! Ported from `stores/assetResult.ts`.
//!
//! `sbsp_backend::asset_processor::{AssetData, AssetMetadata}` have no
//! feature gate, same pattern as the rest of Phase 2's state modules.
//!
//! Scoped to the local cache only (`add`/`addError`/`addMetadata`/
//! `resetError`/`clear`, plus a plain lookup by path). The original's
//! `get(cueId)`/`getMetadata(cueId)` do three things at once: resolve a
//! cue id to its audio target path via the show model, look that path up
//! in the cache, and -- as a side effect, if missing -- call the backend
//! to start processing it. Those are three different concerns (model
//! lookup, pure cache read, and an API call), so they're split here
//! instead of combined:
//! - the cue-id-to-path resolution is just matching the cue's `CueParam`
//!   at the call site (which already has both `ShowModelState` and
//!   `AssetResults` available), not something this module needs to do;
//! - [`AssetResults::get`]/[`AssetResults::get_metadata`] take the path
//!   directly and do only the cache read, no side effects;
//! - triggering processing is a `Task` issued through `Port` once that
//!   exists (Phase 4+), not state mutation -- [`AssetResults::should_request`]
//!   exposes the "is it worth asking" check that decision needs,
//!   and [`AssetResults::mark_processing`] is how the caller records that
//!   it did.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use sbsp_backend::asset_processor::{AssetData, AssetMetadata};

#[derive(Default)]
pub struct AssetResults {
    pub metadatas: HashMap<PathBuf, AssetMetadata>,
    pub results: HashMap<PathBuf, AssetData>,
    pub processing: HashSet<PathBuf>,
    pub failed: HashSet<PathBuf>,
}

impl AssetResults {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, path: PathBuf, data: AssetData) {
        self.metadatas.insert(path.clone(), data.metadata.clone());
        self.results.insert(path.clone(), data);
        self.processing.remove(&path);
    }

    pub fn add_error(&mut self, path: PathBuf) {
        self.processing.remove(&path);
        self.failed.insert(path);
    }

    pub fn add_metadata(&mut self, path: PathBuf, data: AssetMetadata) {
        self.metadatas.insert(path, data);
    }

    pub fn reset_error(&mut self, path: &Path) {
        self.failed.remove(path);
    }

    /// `processing` is deliberately left untouched here, matching the
    /// original: `clear()` resets known results/metadata/failures, but
    /// does not cancel any processing request already in flight.
    pub fn clear(&mut self) {
        self.results.clear();
        self.metadatas.clear();
        self.failed.clear();
    }

    pub fn get(&self, path: &Path) -> Option<&AssetData> {
        self.results.get(path)
    }

    pub fn get_metadata(&self, path: &Path) -> Option<&AssetMetadata> {
        self.metadatas.get(path)
    }

    /// Whether `path` is worth requesting processing for: not already
    /// in flight, and hasn't already failed (matching the original's
    /// `requestProcess` guard).
    pub fn should_request(&self, path: &Path) -> bool {
        !self.processing.contains(path) && !self.failed.contains(path)
    }

    pub fn mark_processing(&mut self, path: PathBuf) {
        self.processing.insert(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(path: &Path) -> AssetMetadata {
        AssetMetadata {
            path: path.to_path_buf(),
            duration: Some(10.0),
            channel_count: Some(2),
            sample_rate: 48000,
        }
    }

    fn data(path: &Path) -> AssetData {
        AssetData {
            metadata: metadata(path),
            waveform: Vec::new(),
            integrated_lufs: None,
            peak: 0.0,
            start_time: None,
            end_time: None,
        }
    }

    #[test]
    fn add_populates_both_maps_and_clears_processing() {
        let path = PathBuf::from("/music/kick.wav");
        let mut assets = AssetResults::new();
        assets.mark_processing(path.clone());

        assets.add(path.clone(), data(&path));

        assert!(assets.get(&path).is_some());
        assert!(assets.get_metadata(&path).is_some());
        assert!(!assets.processing.contains(&path));
    }

    #[test]
    fn add_error_marks_failed_and_clears_processing() {
        let path = PathBuf::from("/music/missing.wav");
        let mut assets = AssetResults::new();
        assets.mark_processing(path.clone());

        assets.add_error(path.clone());

        assert!(assets.failed.contains(&path));
        assert!(!assets.processing.contains(&path));
    }

    #[test]
    fn should_request_is_false_once_processing_or_failed() {
        let path = PathBuf::from("/music/kick.wav");
        let mut assets = AssetResults::new();
        assert!(assets.should_request(&path));

        assets.mark_processing(path.clone());
        assert!(!assets.should_request(&path));

        assets.add_error(path.clone());
        assert!(!assets.should_request(&path));
    }

    #[test]
    fn reset_error_allows_requesting_again() {
        let path = PathBuf::from("/music/kick.wav");
        let mut assets = AssetResults::new();
        assets.add_error(path.clone());
        assert!(!assets.should_request(&path));

        assets.reset_error(&path);
        assert!(assets.should_request(&path));
    }

    #[test]
    fn clear_does_not_touch_processing() {
        let path = PathBuf::from("/music/kick.wav");
        let mut assets = AssetResults::new();
        assets.mark_processing(path.clone());
        assets.add(
            PathBuf::from("/music/other.wav"),
            data(&PathBuf::from("/music/other.wav")),
        );

        assets.clear();

        assert!(assets.results.is_empty());
        assert!(assets.metadatas.is_empty());
        assert!(assets.processing.contains(&path));
    }
}
