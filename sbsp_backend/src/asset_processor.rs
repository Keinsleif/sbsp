mod data;
mod handle;

pub use data::AssetData;
pub use handle::AssetProcessorHandle;
use uuid::Uuid;

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

use crate::{event::UiEvent, manager::ShowModelHandle, model::cue::CueParam};

const WAVEFORM_THRESHOLD: usize = 2000;
const AUDIO_THRESHOLD: f32 = 0.001_f32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetProcessorCommand {
    RequestFileAssetData {
        id: Uuid,
        path: PathBuf,
    },
    ProcessAll,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessResult {
    pub id: Uuid,
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
    event_rx: broadcast::Receiver<UiEvent>,
    result_tx: broadcast::Sender<ProcessResult>,
    result_rx: broadcast::Receiver<ProcessResult>,

    cache: Arc<RwLock<AssetCache>>,
    processing: Arc<RwLock<Vec<PathBuf>>>,
}

impl AssetProcessor {
    pub fn new(
        model_handle: ShowModelHandle,
        event_rx: broadcast::Receiver<UiEvent>,
    ) -> (Self, AssetProcessorHandle) {
        let (command_tx, command_rx) = mpsc::channel::<AssetProcessorCommand>(32);
        let cache = Arc::new(RwLock::new(AssetCache::new()));
        let (result_tx, result_rx) = broadcast::channel(8);
        (
            Self {
                model_handle,
                command_rx,
                event_rx,
                result_tx: result_tx.clone(),
                result_rx,
                cache: cache.clone(),
                processing: Arc::new(RwLock::new(Vec::new())),
            },
            AssetProcessorHandle {
                result_rx_factory: result_tx,
                command_tx,
            },
        )
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    match command {
                        AssetProcessorCommand::RequestFileAssetData{id, path} => {
                            self.handle_process_file(id, path).await;
                        }
                        AssetProcessorCommand::ProcessAll => {
                            self.handle_process_all().await;
                        }
                    }
                },
                Ok(result) = self.result_rx.recv() => {
                    let mut cache = self.cache.write().await;
                    if let Ok(data) = &result.data {
                        cache.entries.insert(result.path.clone(), CacheEntry { last_modified: result.path.metadata().unwrap().modified().unwrap(), data: data.clone() });
                    }
                    self.processing.write().await.retain(|value| *value != result.path);
                },
                Ok(event) = self.event_rx.recv() => {
                    if let UiEvent::ShowModelLoaded{..} = event {
                        self.handle_process_all().await;
                    }
                },
            }
        }
    }

    async fn handle_process_file(
        &self,
        id: Uuid,
        path: PathBuf,
    ) {
        let mut filepath = self
            .model_handle
            .get_current_file_path()
            .await
            .unwrap_or(PathBuf::new());
        filepath.pop();
        filepath.push(&path);
        let cache = self.cache.read().await;
        if let Some(entry) = cache.entries.get(&filepath) {
            self.result_tx.send(ProcessResult { id, path: filepath, data: Ok(entry.data.clone()) }).unwrap();
        } else {
            let mut processing = self.processing.write().await;

            if processing.contains(&filepath) {
                let mut receiver = self.result_tx.subscribe();
                let sender = self.result_tx.clone();
                let path_clone = filepath.clone();
                tokio::spawn(async move {
                    while let Ok(result) = receiver.recv().await {
                        if result.path == path_clone {
                            sender.send(ProcessResult { id, path: result.path, data: result.data }).unwrap();
                            break;
                        }
                    }
                });
            } else {
                processing.push(filepath.clone());
                drop(processing);

                let path_clone = filepath.clone();
                let result_tx = self.result_tx.clone();
                tokio::spawn(async move {
                    let asset_data = Self::process_asset(path_clone.clone())
                        .await
                        .map_err(|e| e.to_string());
                    result_tx
                        .send(ProcessResult {
                            id,
                            path: path_clone,
                            data: asset_data,
                        })
                        .unwrap();
                });
            }
        }
    }

    async fn handle_process_all(&self) {
        let model = self.model_handle.read().await;
        let parent = self
            .model_handle
            .get_current_file_path()
            .await
            .unwrap_or(PathBuf::new());

        let paths = model.cues.iter().filter_map(|cue| {
            if let CueParam::Audio(param) = &cue.params {
                let mut filepath = parent.clone();
                filepath.pop();
                filepath.push(param.target.clone());

                Some(filepath)
            } else {
                None
            }
        });
        let mut processing = self.processing.write().await;
        for path in paths {
            if self.cache.read().await.entries.contains_key(&path.clone())
                || processing.contains(&path)
            {
                continue;
            }
            processing.push(path.clone());

            let path_clone = path.clone();
            let process_tx = self.result_tx.clone();
            tokio::spawn(async move {
                let asset_data = Self::process_asset(path_clone.clone())
                    .await
                    .map_err(|e| e.to_string());
                process_tx
                    .send(ProcessResult {
                        id: Uuid::nil(),
                        path: path_clone,
                        data: asset_data,
                    })
                    .unwrap();
            });
        }
    }

    async fn process_asset(path: PathBuf) -> anyhow::Result<AssetData> {
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
        let channel_count = codec_params.channels.map_or(1, |channels| channels.count());
        let total_samples = codec_params.n_frames.unwrap_or(0);

        let mut samples_per_peaks = (sample_rate as f64 * 0.1).max(1.0) as u64;

        if total_samples > 0 && (total_samples / samples_per_peaks) > WAVEFORM_THRESHOLD as u64 {
            samples_per_peaks = (total_samples - (total_samples % (WAVEFORM_THRESHOLD - 1) as u64)) / (WAVEFORM_THRESHOLD - 1) as u64;
        }

        let mut ebur128 = EbuR128::new(channel_count as u32, sample_rate, ebur128::Mode::I)?;
        let mut first_audio_sample = None;
        let mut last_audio_sample = None;
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
                    let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());

                    sample_buf.copy_interleaved_ref(decoded);

                    ebur128.add_frames_f32(sample_buf.samples())?;

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

        if sample_index % samples_per_peaks > 0 {
            waveform.push(max_in_current_peak);
        }

        let start_time = codec_params
            .time_base
            .zip(first_audio_sample.map(|samples| (samples as f64 / channel_count as f64).floor() as u64))
            .map(|(base, spans)| {
                let symphonia_time = base.calc_time(spans);
                symphonia_time.seconds as f64 + symphonia_time.frac
            }
        );
        let end_time = codec_params
            .time_base
            .zip(last_audio_sample.map(|samples| (samples as f64 / channel_count as f64).floor() as u64))
            .map(|(base, spans)| {
                let symphonia_time = base.calc_time(spans);
                symphonia_time.seconds as f64 + symphonia_time.frac
            }
        );

        let integrated_lufs = ebur128.loudness_global().ok();

        Ok(AssetData {
            path,
            duration,
            waveform,
            integrated_lufs,
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
