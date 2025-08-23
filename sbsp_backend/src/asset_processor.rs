use std::{collections::HashMap, path::PathBuf, sync::Arc, time::SystemTime};

use serde::{Deserialize, Serialize};
use symphonia::core::{audio::{SampleBuffer, SignalSpec}, codecs::{DecoderOptions, FinalizeResult}, errors::{Error, Result}, formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions, probe::Hint};
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::manager::ShowModelHandle;

pub enum AssetCommand {
    RequestAssetData {
        path: PathBuf,
        responder: oneshot::Sender<anyhow::Result<AssetData>>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AssetData {
    pub path: PathBuf,
    pub duration: f64,
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

    cache: Arc<RwLock<AssetCache>>,
}

impl AssetProcessor {
    pub fn new(
        model_handle: ShowModelHandle,
    ) -> (Self, AssetProcessorHandle) {
        let (command_tx, command_rx) = mpsc::channel::<AssetCommand>(32);
        let cache = Arc::new(RwLock::new(AssetCache::new()));
        (
            Self {
                model_handle,
                command_rx,
                cache: cache.clone(),
            },
            AssetProcessorHandle { command_tx },
        )
    }

    pub async fn run(mut self) {
        while let Some(command) = self.command_rx.recv().await {
            match command {
                AssetCommand::RequestAssetData{path, responder} => {
                    let mut filepath = self
                        .model_handle
                        .get_current_file_path()
                        .await
                        .unwrap_or(PathBuf::new());
                    filepath.pop();
                    filepath.push(path);
                    let cache = self.cache.read().await;
                    let asset_data;
                    if cache.entries.contains_key(&filepath) {
                        let cache_entry = cache.entries.get(&filepath).unwrap().clone();
                        drop(cache);
                        let modified = filepath.metadata().unwrap().modified().unwrap();
                        if cache_entry.last_modified != modified {
                            asset_data = self.process_asset(filepath.clone()).await;
                            let mut cache = self.cache.write().await;
                            if let Ok(data) = &asset_data {
                                cache.entries.insert(filepath.clone(), CacheEntry { last_modified: filepath.metadata().unwrap().modified().unwrap(), data: data.clone() });
                            }
                        } else {
                            asset_data = Ok(cache_entry.data.clone());
                        }
                    } else {
                        asset_data = self.process_asset(filepath.clone()).await;
                        let mut cache = self.cache.write().await;
                        if let Ok(data) = &asset_data {
                            cache.entries.insert(filepath.clone(), CacheEntry { last_modified: filepath.metadata().unwrap().modified().unwrap(), data: data.clone() });
                        }
                    }

                    responder.send(asset_data).unwrap();
                }
            }
        }
    }

    async fn process_asset(&self, path: PathBuf) -> anyhow::Result<AssetData> {
        let src = std::fs::File::open(&path).unwrap();
        let mss = MediaSourceStream::new(Box::new(src), Default::default());

        let mut hint = Hint::new();
        if let Some(ext_osstr) = path.extension() {
            hint.with_extension(ext_osstr.to_str().unwrap());
        }

        let format_opts: FormatOptions = Default::default();
        let metadata_opts: MetadataOptions = Default::default();
        let decoder_opts: DecoderOptions = Default::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .unwrap();

        let mut format = probed.format;

        let track = format.default_track().unwrap();

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decoder_opts)
            .unwrap();

        let track_id = track.id;

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
                Err(Error::DecodeError(_)) => (),
                Err(err) => break Err(err),
            }
        };

        ignore_end_of_stream_error(result).unwrap();
        do_verification(decoder.finalize()).unwrap();

        let spec_data = spec.unwrap();
        let window = spec_data.rate / 10;

        let step = if samples.len() < window as usize {
            1
        } else {
            (samples.len() - (samples.len() % window as usize)) / window as usize
                + 1
        };

        let mut peaks = Vec::new();

        for i in 0..step {
            let start = i * spec_data.rate as usize;
            if samples.len() < start + spec_data.rate as usize {
                peaks.push(
                    samples[start..]
                        .iter()
                        .copied()
                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap(),
                );
            } else {
                peaks.push(
                    samples[start..(start + spec_data.rate as usize)]
                        .iter()
                        .copied()
                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap(),
                );
            }
        }

        Ok(AssetData { path, duration: samples.len() as f64 / spec_data.rate as f64, waveform: peaks })
    }
}

pub struct AssetProcessorHandle {
    command_tx: mpsc::Sender<AssetCommand>,
}

impl AssetProcessorHandle {
    pub async fn request_asset_data(&self, target: PathBuf) -> anyhow::Result<AssetData> {
        let (result_tx, result_rx) = oneshot::channel();
        self.command_tx
            .send(AssetCommand::RequestAssetData {path: target, responder: result_tx})
            .await
            .unwrap();

        result_rx.await.unwrap_or_else(|_| Err(anyhow::anyhow!("AssetProcessor task may have panicked")))
    }
}

fn ignore_end_of_stream_error(result: Result<()>) -> Result<()> {
    match result {
        Err(Error::IoError(err))
            if err.kind() == std::io::ErrorKind::UnexpectedEof
                && err.to_string() == "end of stream" =>
        {
            Ok(())
        }
        _ => result,
    }
}

fn do_verification(finalization: FinalizeResult) -> Result<i32> {
    match finalization.verify_ok {
        Some(is_ok) => {
            log::debug!("verification: {}", if is_ok { "passed" } else { "failed" });

            Ok(i32::from(!is_ok))
        }
        _ => Ok(0),
    }
}