// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The results of processing audio files: waveforms and peaks, metadata,
//! and the files that failed, keyed by path.
//!
//! This is a cache the app reads from. [`AssetResults::get`] and
//! [`AssetResults::get_metadata`] only look a path up; they never ask the
//! backend for anything. Finding the path of a cue's audio is the caller's
//! job (it matches the cue's `CueParam`), and asking the backend to process
//! a file is a `Task` sent through the port. To decide whether that is
//! worth doing, [`AssetResults::should_request`] says whether the path is
//! neither in flight nor already failed, and
//! [`AssetResults::mark_processing`] records that it was asked for.

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

    /// Forgets the known results, metadata and failures. Requests already in
    /// flight are not cancelled, so `processing` is left as it is.
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

    /// Whether `path` is worth requesting processing for: not already in
    /// flight, and not already failed.
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
