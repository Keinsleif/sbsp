mod data;
mod handle;

pub use data::AssetData;
pub use handle::AssetProcessorHandle;

use std::path::PathBuf;
use std::{collections::HashMap, sync::Arc, time::SystemTime};

use serde::{Deserialize, Serialize};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{DecoderOptions, FinalizeResult},
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};
use tokio::sync::{RwLock, broadcast, mpsc};
use ebur128::EbuR128;

use crate::event::UiEvent;
use crate::manager::ShowModelHandle;

const WAVEFORM_THRESHOLD: usize = 2000;
const AUDIO_THRESHOLD: f32 = 0.001_f32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetProcessorCommand {
    RequestFileAssetData {
        path: PathBuf,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessResult {
    pub path: PathBuf,
    pub data: Result<AssetData, String>,
}

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

pub struct AssetProcessor {
    model_handle: ShowModelHandle,

    command_rx: mpsc::Receiver<AssetProcessorCommand>,
    event_tx: broadcast::Sender<UiEvent>,
    result_tx: broadcast::Sender<ProcessResult>,

    cache: Arc<RwLock<AssetCache>>,
    processing: Arc<RwLock<Vec<PathBuf>>>,
}

impl AssetProcessor {
    pub fn new(
        model_handle: ShowModelHandle,
        event_tx: broadcast::Sender<UiEvent>,
    ) -> (Self, AssetProcessorHandle) {
        let (command_tx, command_rx) = mpsc::channel::<AssetProcessorCommand>(32);
        let cache = Arc::new(RwLock::new(AssetCache::new()));
        let (result_tx, _) = broadcast::channel(32);
        (
            Self {
                model_handle,
                command_rx,
                event_tx,
                result_tx,
                cache: cache.clone(),
                processing: Arc::new(RwLock::new(Vec::new())),
            },
            AssetProcessorHandle {
                command_tx,
            },
        )
    }

    pub async fn run(mut self) {
        let mut result_rx = self.result_tx.subscribe();
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    match command {
                        AssetProcessorCommand::RequestFileAssetData{path} => {
                            log::info!("Asset Process requested. file={:?}", path);
                            self.handle_process_file(path).await;
                        }
                    }
                },
                Ok(result) = result_rx.recv() => {
                    let mut cache = self.cache.write().await;
                    if let Ok(data) = &result.data {
                        if let Ok(metadata) = tokio::fs::metadata(data.path.clone()).await && let Ok(last_modified) = metadata.modified() {
                            cache.entries.insert(data.path.clone(), CacheEntry { last_modified, data: data.clone() });
                        } else {
                            cache.entries.insert(data.path.clone(), CacheEntry { last_modified: SystemTime::now(), data: data.clone() });
                        }
                    }
                    self.processing.write().await.retain(|value| *value != result.path);
                    if let Err(e) = self.event_tx.send(UiEvent::AssetResult { path: result.path, data: result.data }) {
                        log::error!("Failed to send process result to event bus. {}", e);
                    }
                }
            }
        }
    }

    async fn handle_process_file(
        &self,
        path: PathBuf,
    ) {
        let actual_path = {
            let model = self.model_handle.read().await;
            if let Some(model_path) = self.model_handle.get_current_file_path().await.as_ref() {
                model_path.join(&model.settings.general.copy_assets_destination).join(&path)
            } else {
                path.clone()
            }
        };
        let cache = self.cache.read().await;
        if let Some(entry) = cache.entries.get(&actual_path) {
            self.event_tx.send(UiEvent::AssetResult { path, data: Ok(entry.data.clone()) }).unwrap();
            return;
        }
        let mut processing = self.processing.write().await;
        if !processing.contains(&actual_path) {
            processing.push(actual_path.clone());
            return;
        }

        let actual_path_clone = actual_path.clone();
        let result_tx = self.result_tx.clone();
        tokio::task::spawn_blocking(move || {
            let asset_data = Self::process_asset(actual_path_clone.clone())
                .map_err(|e| e.to_string());
            result_tx
            .send(ProcessResult {
                path,
                data: asset_data,
            })
            .unwrap();
        });
        log::info!("Asset Process started. file={:?}", actual_path);
    }

    fn process_asset(path: PathBuf) -> anyhow::Result<AssetData> {
        let src: std::fs::File = std::fs::File::open(&path)?;
        let mss = MediaSourceStream::new(Box::new(src), Default::default());

        let mut hint = Hint::new();
        if let Some(ext_osstr) = path.extension()
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

        let track = format.default_track().unwrap();
        let codec_params = track.codec_params.clone();

        let mut decoder =
            symphonia::default::get_codecs().make(&codec_params, &decoder_opts)?;

        let track_id = track.id;

        let duration = codec_params
            .time_base
            .zip(codec_params.n_frames)
            .map(|(base, spans)| {
                let symphonia_time = base.calc_time(spans);
                symphonia_time.seconds as f64 + symphonia_time.frac
            }
        );

        let sample_rate = codec_params.sample_rate.ok_or_else(|| anyhow::anyhow!("Sample rate not found."))?;
        let mut channel_count = codec_params.channels.map(|channels| channels.count());
        let total_samples = codec_params.n_frames.unwrap_or(0);

        let mut samples_per_peaks = (sample_rate as f64 * 0.1).max(1.0) as u64;

        if total_samples > 0 && (total_samples / samples_per_peaks) > WAVEFORM_THRESHOLD as u64 {
            samples_per_peaks = (total_samples - (total_samples % (WAVEFORM_THRESHOLD - 1) as u64)) / (WAVEFORM_THRESHOLD - 1) as u64;
        }

        let mut ebur128 = if let Some(channels) = channel_count {
            Some(EbuR128::new(channels as u32, sample_rate, ebur128::Mode::I)?)
        } else {
            None
        };
        let mut first_audio_sample = None;
        let mut last_audio_sample = None;
        let mut max_audio_sample: f32 = 0.0;
        let mut waveform = Vec::with_capacity(WAVEFORM_THRESHOLD);
        let mut sample_index = 0;
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
                    if channel_count.is_none() {
                        channel_count = Some(decoded_spec.channels.count())
                    }
                    if ebur128.is_none() {
                        ebur128 = Some(EbuR128::new(decoded_spec.channels.count() as u32, sample_rate, ebur128::Mode::I)?);
                    }
                    let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, decoded_spec);

                    sample_buf.copy_interleaved_ref(decoded);

                    if let Some(ebur) = &mut ebur128 {
                        ebur.add_frames_f32(sample_buf.samples())?;
                    }

                    for &sample in sample_buf.samples() {
                        if sample.abs() >= AUDIO_THRESHOLD {
                            if first_audio_sample.is_none() {
                                first_audio_sample = Some(sample_index);
                            }
                            last_audio_sample = Some(sample_index);
                        }
                        max_in_current_peak = max_in_current_peak.max(sample.abs());

                        sample_index += 1;
                        if sample_index % samples_per_peaks == 0 {
                            waveform.push(max_in_current_peak);
                            if max_in_current_peak > max_audio_sample {
                                max_audio_sample = max_in_current_peak;
                            }
                            max_in_current_peak = 0.0;
                        }
                    }
                }
                Err(symphonia::core::errors::Error::DecodeError(_)) => (),
                Err(err) => break Err(err),
            }
        };

        ignore_end_of_stream_error(result)?;
        do_verification(decoder.finalize())?;

        let channels = channel_count.unwrap_or(2);

        if sample_index % samples_per_peaks > 0 {
            waveform.push(max_in_current_peak);
        }

        let start_time = codec_params
            .time_base
            .zip(first_audio_sample.map(|samples| (samples as f64 / channels as f64).floor() as u64))
            .map(|(base, spans)| {
                let symphonia_time = base.calc_time(spans);
                symphonia_time.seconds as f64 + symphonia_time.frac
            }
        );
        let end_time = codec_params
            .time_base
            .zip(last_audio_sample.map(|samples| (samples as f64 / channels as f64).floor() as u64))
            .map(|(base, spans)| {
                let symphonia_time = base.calc_time(spans);
                symphonia_time.seconds as f64 + symphonia_time.frac
            }
        );

        let integrated_lufs = ebur128.map(|ebur| ebur.loudness_global().unwrap());

        Ok(AssetData {
            path,
            duration,
            waveform,
            integrated_lufs,
            peak: max_audio_sample.log10() * 20.0,
            start_time,
            end_time,
        })
    }
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

fn do_verification(finalization: FinalizeResult) -> symphonia::core::errors::Result<i32> {
    match finalization.verify_ok {
        Some(is_ok) => {
            log::debug!("verification: {}", if is_ok { "passed" } else { "failed" });

            Ok(i32::from(!is_ok))
        }
        _ => Ok(0),
    }
}
