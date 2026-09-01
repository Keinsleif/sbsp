// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use symphonia::core::codecs::CODEC_TYPE_NULL;

use std::collections::HashSet;
use std::path::PathBuf;
use std::{collections::HashMap, sync::Arc, time::SystemTime};

use ebur128::EbuR128;
use serde::{Deserialize, Serialize};
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, formats::FormatOptions, io::MediaSourceStream,
    meta::MetadataOptions, probe::Hint,
};
use tokio::{
    sync::{RwLock, Semaphore, broadcast, mpsc},
    task::JoinSet,
};

use super::{
    command::AssetProcessorCommand,
    data::{AssetData, AssetMetadata},
    handle::AssetProcessorHandle,
};
use crate::event::BackendEvent;
use crate::manager::ShowModelHandle;

const WAVEFORM_THRESHOLD: usize = 2000;
const AUDIO_THRESHOLD: f32 = 0.001_f32;

#[derive(Serialize, Deserialize, Clone)]
struct CacheEntry {
    last_modified: SystemTime,
    data: AssetData,
}

#[derive(Serialize, Deserialize, Default)]
struct AssetCache {
    entries: HashMap<PathBuf, CacheEntry>,
}

impl AssetCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

#[derive(Default)]
struct ProcessingEntry {
    orig_paths: HashSet<PathBuf>,
    metadata: Option<AssetMetadata>,
}

impl ProcessingEntry {
    fn with_path(path: PathBuf) -> Self {
        Self {
            orig_paths: HashSet::from([path]),
            metadata: None,
        }
    }
}

pub struct AssetProcessor {
    model_handle: ShowModelHandle,

    command_rx: mpsc::Receiver<AssetProcessorCommand>,
    event_tx: broadcast::Sender<BackendEvent>,

    semaphore: Arc<Semaphore>,
    cache: Arc<RwLock<AssetCache>>,
    processing: Arc<RwLock<HashMap<PathBuf, ProcessingEntry>>>,
}

impl AssetProcessor {
    pub fn new(
        model_handle: ShowModelHandle,
        event_tx: broadcast::Sender<BackendEvent>,
    ) -> (Self, AssetProcessorHandle) {
        let (command_tx, command_rx) = mpsc::channel::<AssetProcessorCommand>(32);
        let cache = Arc::new(RwLock::new(AssetCache::new()));
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        (
            Self {
                model_handle,
                command_rx,
                event_tx,
                semaphore: Arc::new(Semaphore::new((cores - 1).max(1))),
                cache: cache.clone(),
                processing: Arc::new(RwLock::new(HashMap::new())),
            },
            AssetProcessorHandle { command_tx },
        )
    }

    pub async fn run(mut self) {
        let mut event_rx = self.event_tx.subscribe();
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    match command {
                        AssetProcessorCommand::RequestFileAssetData{path} => {
                            log::info!("Asset Processing requested. file={:?}", path);
                            self.handle_process_file(path).await;
                        }
                    }
                },
                result = event_rx.recv() => {
                    match result {
                        Ok(BackendEvent::ShowModelLoaded { .. }) |
                        Ok(BackendEvent::ShowModelReset { .. }) => {
                            self.filter_current_assets().await;
                        }
                        Ok(_) => {},
                        Err(broadcast::error::RecvError::Closed) => break,
                        Err(_) => {
                            log::warn!("Event monitoring receiver Lagged.");
                        }
                    }
                },
            }
        }
    }

    async fn handle_process_file(&self, path: PathBuf) {
        let Ok(actual_path) = self.model_handle.get_asset_standard_path(&path).await else {
            if let Err(e) = self.event_tx.send(BackendEvent::AssetResult {
                path,
                result: Err("Failed to resolve path.".to_string()),
            }) {
                log::error!("Failed to send process result to event bus. {}", e);
            }
            return;
        };
        {
            let entry = {
                let cache = self.cache.read().await;
                cache.entries.get(&actual_path).cloned()
            };
            if let Some(entry) = entry {
                let valid = Self::is_cache_entry_valid(&actual_path, entry.last_modified).await;
                if valid {
                    if let Err(e) = self.event_tx.send(BackendEvent::AssetResult {
                        path,
                        result: Ok(entry.data.clone()),
                    }) {
                        log::error!("Failed to send process result to event bus. {}", e);
                    }
                    return;
                } else {
                    self.cache.write().await.entries.remove(&actual_path);
                }
            }
        }
        {
            let mut processing = self.processing.write().await;
            if let Some(entry) = processing.get_mut(&actual_path) {
                entry.orig_paths.insert(path.clone());
                if let Some(metadata) = &entry.metadata
                    && let Err(e) = self.event_tx.send(BackendEvent::AssetMetadata {
                        path,
                        data: metadata.clone(),
                    })
                {
                    log::error!("Failed to send metadata to event bus. {}", e);
                }
                return;
            }
            processing.insert(
                actual_path.clone(),
                ProcessingEntry::with_path(path.clone()),
            );
        }

        let actual_path_clone = actual_path.clone();
        let event_tx = self.event_tx.clone();
        let cache_lock = self.cache.clone();
        let processing_lock = self.processing.clone();
        let semaphore = self.semaphore.clone();
        tokio::spawn(async move {
            let permit = match semaphore.acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => {
                    if let Some(entry) = processing_lock.blocking_write().remove(&actual_path_clone)
                    {
                        for orig_path in entry.orig_paths {
                            if let Err(e) = event_tx.send(BackendEvent::AssetResult {
                                path: orig_path,
                                result: Err("Asset processor is shutting down.".to_string()),
                            }) {
                                log::error!("Failed to send process result to event bus. {}", e);
                            }
                        }
                    }
                    return;
                }
            };
            tokio::task::spawn_blocking(move || {
                let asset_data = Self::process_asset(
                    actual_path_clone.clone(),
                    event_tx.clone(),
                    cache_lock,
                    processing_lock.clone(),
                )
                .map_err(|e| e.to_string());
                if let Some(entry) = processing_lock.blocking_write().remove(&actual_path_clone) {
                    for orig_path in entry.orig_paths {
                        if let Err(e) = event_tx.send(BackendEvent::AssetResult {
                            path: orig_path,
                            result: asset_data.clone(),
                        }) {
                            log::error!("Failed to send process result to event bus. {}", e);
                        }
                    }
                }
                drop(permit);
            });
        });

        log::info!("Asset Processing started. file={:?}", actual_path);
    }

    async fn is_cache_entry_valid(file_path: &PathBuf, last_modified: SystemTime) -> bool {
        match tokio::fs::metadata(file_path).await {
            Ok(metadata) => matches!(metadata.modified(), Ok(m) if m == last_modified),
            Err(_) => false,
        }
    }

    async fn filter_current_assets(&self) {
        let active_paths = self.model_handle.get_all_asset_paths().await;

        let (snapshot, before_count): (Vec<(PathBuf, PathBuf, SystemTime)>, usize) = {
            let mut cache = self.cache.write().await;
            let before_len = cache.entries.len();
            cache.entries.retain(|path, _| active_paths.contains(path));

            (
                cache
                    .entries
                    .iter()
                    .map(|(path, entry)| {
                        (
                            path.clone(),
                            entry.data.metadata.path.clone(),
                            entry.last_modified,
                        )
                    })
                    .collect(),
                before_len,
            )
        };

        let checked_paths: HashSet<PathBuf> = snapshot.iter().map(|(p, _, _)| p.clone()).collect();

        let mut join_set = JoinSet::new();
        for (path, file_path, last_modified) in snapshot {
            join_set.spawn(async move {
                let valid = Self::is_cache_entry_valid(&file_path, last_modified).await;
                (path, valid)
            });
        }

        let mut active_cache_paths = HashSet::new();
        while let Some(result) = join_set.join_next().await {
            if let Ok((path, valid)) = result
                && valid
            {
                active_cache_paths.insert(path);
            }
        }

        let mut cache = self.cache.write().await;
        cache.entries.retain(|path, _| {
            if checked_paths.contains(path) {
                active_cache_paths.contains(path)
            } else {
                true
            }
        });

        let after_count = cache.entries.len();
        log::info!(
            "Asset cache filtered. Freed: {}, Remaining: {}",
            before_count - after_count,
            after_count
        );
    }

    fn process_asset(
        standard_path: PathBuf,
        event_tx: broadcast::Sender<BackendEvent>,
        cache: Arc<RwLock<AssetCache>>,
        processing: Arc<RwLock<HashMap<PathBuf, ProcessingEntry>>>,
    ) -> anyhow::Result<AssetData> {
        let src: std::fs::File = std::fs::File::open(&standard_path)?;
        let last_modified = src.metadata().and_then(|m| m.modified());
        let mss = MediaSourceStream::new(Box::new(src), Default::default());

        let mut hint = Hint::new();
        if let Some(ext_osstr) = standard_path.extension()
            && let Some(ext_str) = ext_osstr.to_str()
        {
            hint.with_extension(ext_str);
        }

        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();
        let decoder_opts: DecoderOptions = Default::default();

        let probed =
            symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;

        let mut format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(symphonia::core::errors::Error::Unsupported(
                "No track with supported codec",
            ))?;

        let track_id = track.id;

        let codec_params = track.codec_params.clone();

        let duration = codec_params
            .time_base
            .zip(codec_params.n_frames)
            .map(calc_secs_from_timebase);

        let sample_rate = codec_params
            .sample_rate
            .ok_or_else(|| anyhow::anyhow!("Sample rate not found."))?;
        let mut channel_count = codec_params.channels.map(|channels| channels.count());

        let mut metadata = AssetMetadata {
            path: standard_path.clone(),
            duration,
            channel_count: channel_count.map(|c| c as u16),
            sample_rate,
        };

        {
            let mut processing_guard = processing.blocking_write();
            if let Some(entry) = processing_guard.get_mut(&standard_path) {
                entry.metadata = Some(metadata.clone());
                for p in &entry.orig_paths {
                    if let Err(e) = event_tx.send(BackendEvent::AssetMetadata {
                        path: p.clone(),
                        data: metadata.clone(),
                    }) {
                        log::error!("Failed to send metadata to event bus. {}", e);
                    }
                }
            }
        }

        let mut decoder = symphonia::default::get_codecs().make(&codec_params, &decoder_opts)?;

        let total_frames = codec_params.n_frames.unwrap_or(0);

        let frames_per_peak = {
            let default_frames_per_peak = (sample_rate as f64 * 0.1).max(1.0) as u64;
            if total_frames > 0 {
                let calculate_interval = total_frames.div_ceil(WAVEFORM_THRESHOLD as u64);
                default_frames_per_peak.max(calculate_interval)
            } else {
                default_frames_per_peak
            }
        };

        let mut sample_buf = None;
        let mut ebur128 = if let Some(channels) = channel_count {
            Some(EbuR128::new(
                channels as u32,
                sample_rate,
                ebur128::Mode::I,
            )?)
        } else {
            None
        };

        let mut first_audio_frame: Option<u64> = None;
        let mut last_audio_frame: Option<u64> = None;
        let mut max_audio_sample: f32 = 0.0;
        let mut waveform = Vec::with_capacity(WAVEFORM_THRESHOLD);
        let mut frame_index: u64 = 0;
        let mut peak_counter: u64 = 0;
        let mut max_in_current_peak: f32 = 0.0;

        let result = loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(err) => break Err(err),
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let decoded_spec = *decoded.spec();
                    let channels = decoded_spec.channels.count();
                    if channel_count.is_none() {
                        channel_count = Some(channels)
                    }
                    if ebur128.is_none() {
                        ebur128 = Some(EbuR128::new(
                            decoded_spec.channels.count() as u32,
                            sample_rate,
                            ebur128::Mode::I,
                        )?);
                    }
                    if sample_buf.is_none() {
                        sample_buf = Some(SampleBuffer::<f32>::new(
                            decoded.capacity() as u64,
                            decoded_spec,
                        ));
                    } else if let Some(buffer) = &mut sample_buf
                        && buffer.capacity() < decoded.capacity() * decoded_spec.channels.count()
                    {
                        *buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, decoded_spec);
                    }

                    if let Some(buffer) = &mut sample_buf {
                        buffer.copy_interleaved_ref(decoded);

                        let samples = buffer.samples();

                        if let Some(ebur) = &mut ebur128 {
                            ebur.add_frames_f32(samples)?;
                        }
                        for frame in samples.chunks(channels) {
                            let frame_max = frame
                                .iter()
                                .map(|s| s.abs())
                                .fold(0.0f32, |acc, val| acc.max(val));
                            if frame_max >= AUDIO_THRESHOLD {
                                if first_audio_frame.is_none() {
                                    first_audio_frame = Some(frame_index);
                                }
                                last_audio_frame = Some(frame_index);
                            }
                            max_in_current_peak = max_in_current_peak.max(frame_max);

                            frame_index += 1;
                            peak_counter += 1;
                            if peak_counter >= frames_per_peak {
                                waveform.push(max_in_current_peak);
                                max_audio_sample = max_audio_sample.max(max_in_current_peak);
                                max_in_current_peak = 0.0;
                                peak_counter = 0;
                            }
                        }
                    }
                }
                Err(symphonia::core::errors::Error::DecodeError(_)) => (),
                Err(err) => break Err(err),
            }
        };

        ignore_end_of_stream_error(result)?;
        if let Some(false) = decoder.finalize().verify_ok {
            anyhow::bail!("Asset verification failed: checksum or stream mismatch");
        }

        if peak_counter > 0 {
            waveform.push(max_in_current_peak);
            max_audio_sample = max_audio_sample.max(max_in_current_peak);
        }

        let start_time = codec_params
            .time_base
            .zip(first_audio_frame)
            .map(calc_secs_from_timebase);
        let end_time = codec_params
            .time_base
            .zip(last_audio_frame)
            .map(calc_secs_from_timebase);

        let integrated_lufs = ebur128.and_then(|ebur| {
            ebur.loudness_global()
                .inspect_err(|e| log::warn!("Failed to calculate integrated LUFS: {}", e))
                .ok()
        });

        metadata.channel_count = channel_count.map(|c| c as u16);

        let asset_data = AssetData {
            metadata,
            waveform,
            integrated_lufs,
            peak: if max_audio_sample > 0.0 {
                max_audio_sample.log10() * 20.0
            } else {
                -60.0
            },
            start_time,
            end_time,
        };

        if let Ok(lm_time) = last_modified {
            cache.blocking_write().entries.insert(
                standard_path,
                CacheEntry {
                    last_modified: lm_time,
                    data: asset_data.clone(),
                },
            );
        }

        Ok(asset_data)
    }
}

fn calc_secs_from_timebase((base, spans): (symphonia::core::units::TimeBase, u64)) -> f64 {
    let t = base.calc_time(spans);
    t.seconds as f64 + t.frac
}

fn ignore_end_of_stream_error(
    result: symphonia::core::errors::Result<()>,
) -> symphonia::core::errors::Result<()> {
    match result {
        Err(symphonia::core::errors::Error::IoError(err))
            if err.kind() == std::io::ErrorKind::UnexpectedEof
                && err.to_string() == "end of stream" =>
        {
            Ok(())
        }
        _ => result,
    }
}
