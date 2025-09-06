mod handle;

pub use handle::AssetProcessorHandle;

use std::{collections::HashMap, path::PathBuf, sync::Arc, time::SystemTime};

use serde::{Deserialize, Serialize};
use symphonia::core::{
    audio::{SampleBuffer, SignalSpec},
    codecs::{DecoderOptions, FinalizeResult},
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};
use tokio::sync::{RwLock, broadcast, mpsc, oneshot};

use crate::{event::UiEvent, manager::ShowModelHandle, model::cue::CueParam};

const WAVEFORM_THRESHOLD: usize = 2000;

pub enum AssetCommand {
    RequestFileAssetData {
        path: PathBuf,
        responder: oneshot::Sender<Result<AssetData, String>>,
    },
    ProcessAll,
}

#[derive(Clone, Debug)]
struct ProcessResult {
    path: PathBuf,
    data: Result<AssetData, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AssetData {
    pub path: PathBuf,
    pub duration: Option<f64>,
    pub waveform: Vec<f32>,
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

    command_rx: mpsc::Receiver<AssetCommand>,
    event_rx: broadcast::Receiver<UiEvent>,

    process_tx: broadcast::Sender<ProcessResult>,
    process_rx: broadcast::Receiver<ProcessResult>,
    cache: Arc<RwLock<AssetCache>>,
    processing: Arc<RwLock<Vec<PathBuf>>>,
}

impl AssetProcessor {
    pub fn new(
        model_handle: ShowModelHandle,
        event_rx: broadcast::Receiver<UiEvent>,
    ) -> (Self, AssetProcessorHandle) {
        let (command_tx, command_rx) = mpsc::channel::<AssetCommand>(32);
        let cache = Arc::new(RwLock::new(AssetCache::new()));
        let (process_tx, process_rx) = broadcast::channel(8);
        (
            Self {
                model_handle,
                command_rx,
                event_rx,
                process_tx,
                process_rx,
                cache: cache.clone(),
                processing: Arc::new(RwLock::new(Vec::new())),
            },
            AssetProcessorHandle { command_tx },
        )
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(command) = self.command_rx.recv() => {
                    match command {
                        AssetCommand::RequestFileAssetData{path, responder} => {
                            self.handle_process_file(path, responder).await;
                        }
                        AssetCommand::ProcessAll => {
                            self.handle_process_all().await;
                        }
                    }
                },
                Ok(result) = self.process_rx.recv() => {
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
        path: PathBuf,
        responder: oneshot::Sender<Result<AssetData, String>>,
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
            responder.send(Ok(entry.data.clone())).unwrap();
        } else {
            let mut processing = self.processing.write().await;

            if processing.contains(&filepath) {
                let mut receiver = self.process_tx.subscribe();
                let path_clone = filepath.clone();
                tokio::spawn(async move {
                    while let Ok(result) = receiver.recv().await {
                        if result.path == path_clone {
                            responder.send(result.data).ok();
                            break;
                        }
                    }
                });
            } else {
                processing.push(filepath.clone());
                drop(processing);

                let path_clone = filepath.clone();
                let process_tx = self.process_tx.clone();
                tokio::spawn(async move {
                    let asset_data = Self::process_asset(path_clone.clone())
                        .await
                        .map_err(|e| e.to_string());
                    responder.send(asset_data.clone()).unwrap();
                    process_tx
                        .send(ProcessResult {
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
            let process_tx = self.process_tx.clone();
            tokio::spawn(async move {
                let asset_data = Self::process_asset(path_clone.clone())
                    .await
                    .map_err(|e| e.to_string());
                process_tx
                    .send(ProcessResult {
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
            && let Some(ext_str) = ext_osstr.to_str() {
                hint.with_extension(ext_str);
            }

        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();
        let decoder_opts: DecoderOptions = Default::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)?;

        let mut format = probed.format;

        let track = format.default_track().unwrap();

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decoder_opts)?;

        let track_id = track.id;

        let duration = track.codec_params.time_base.zip(track.codec_params.n_frames).map(|(base, spans)| {
            let symphonia_time = base.calc_time(spans);
            symphonia_time.seconds as f64 + symphonia_time.frac
        });

        let mut sample_buf = None;
        let mut spec: Option<SignalSpec> = None;
        let mut samples = vec![];

        let result = loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(err) => break Err(err),
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(audio_buf) => {
                    if sample_buf.is_none() {
                        let spec_local = *audio_buf.spec();

                        let duration = audio_buf.capacity() as u64;

                        sample_buf = Some(SampleBuffer::<f32>::new(duration, spec_local));
                        spec = Some(spec_local);
                    }

                    if let Some(buf) = &mut sample_buf {
                        buf.copy_interleaved_ref(audio_buf);

                        for &sample in buf.samples() {
                            samples.push(sample);
                        }
                    }
                }
                Err(symphonia::core::errors::Error::DecodeError(_)) => (),
                Err(err) => break Err(err),
            }
        };

        ignore_end_of_stream_error(result)?;
        do_verification(decoder.finalize())?;

        let spec_data = spec.unwrap();
        let mut window = (spec_data.rate / 10) as usize;

        let step = if samples.len() < window {
            1
        } else if samples.len() / window < 5000 {
            (samples.len() - (samples.len() % window)) / window + 1
        } else {
            window = (samples.len() - (samples.len() % (WAVEFORM_THRESHOLD - 1))) / (WAVEFORM_THRESHOLD - 1);
            WAVEFORM_THRESHOLD
        };

        let mut peaks = Vec::new();

        for i in 0..step {
            let start = i * window;
            if samples.len() < start + window {
                peaks.push(
                    samples[start..]
                        .iter()
                        .copied()
                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap(),
                );
            } else {
                peaks.push(
                    samples[start..(start + window)]
                        .iter()
                        .copied()
                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap(),
                );
            }
        }

        Ok(AssetData {
            path,
            duration,
            waveform: peaks,
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
