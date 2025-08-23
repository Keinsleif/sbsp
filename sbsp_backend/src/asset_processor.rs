use std::{collections::HashMap, path::PathBuf, sync::Arc, time::SystemTime};

use serde::{Deserialize, Serialize};
use symphonia::core::{audio::{SampleBuffer, SignalSpec}, codecs::{DecoderOptions, FinalizeResult}, errors::{Error, Result}, formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions, probe::Hint};
use tokio::sync::{RwLock, broadcast, mpsc};

use crate::{event::UiEvent, manager::ShowModelHandle};

#[derive(Debug)]
pub enum AssetCommand {
    RequestAssetData(PathBuf),
}

#[derive(Serialize, Deserialize, Clone)]
struct CacheEntry {
    pub path: PathBuf,
    last_modified: SystemTime,
    pub duration: f64,
    pub waveform: Vec<f32>,
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
    event_tx: broadcast::Sender<UiEvent>,

    cache: Arc<RwLock<AssetCache>>,
}

impl AssetProcessor {
    pub fn new(
        model_handle: ShowModelHandle,
        event_tx: broadcast::Sender<UiEvent>,
    ) -> (Self, AssetProcessorHandle) {
        let (command_tx, command_rx) = mpsc::channel::<AssetCommand>(32);
        let cache = Arc::new(RwLock::new(AssetCache::new()));
        (
            Self {
                model_handle,
                command_rx,
                event_tx,
                cache: cache.clone(),
            },
            AssetProcessorHandle { command_tx },
        )
    }

    pub async fn run(mut self) {
        while let Some(command) = self.command_rx.recv().await {
            match command {
                AssetCommand::RequestAssetData(path_buf) => {
                    let mut filepath = self
                        .model_handle
                        .get_current_file_path()
                        .await
                        .unwrap_or(PathBuf::new());
                    filepath.pop();
                    filepath.push(path_buf);
                    let cache = self.cache.read().await;
                    let entry_option;
                    if cache.entries.contains_key(&filepath) {
                        let cache_entry = cache.entries.get(&filepath).unwrap();
                        let modified = filepath.metadata().unwrap().modified().unwrap();
                        if cache_entry.last_modified != modified {
                            entry_option = self.process_asset(filepath).await;
                        } else {
                            entry_option = Ok(cache_entry.clone());
                        }
                    } else {
                        let result = self.process_asset(filepath).await;
                        entry_option = result;
                    }

                    if let Ok(entry) = entry_option {
                        if self.event_tx.send(UiEvent::AssetDataUpdated {
                            path: entry.path,
                            duration: entry.duration,
                            waveform: entry.waveform,
                        }).is_err() {
                            log::trace!("No UI clients are listening to playback events.");
                        }
                    } else if let Err(e) = entry_option {
                        log::error!("Failed to parse asset: {}", e);
                    }
                }
            }
        }
    }

    async fn process_asset(&self, path: PathBuf) -> anyhow::Result<CacheEntry> {
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

        let last_modified = path.metadata().unwrap().modified().unwrap();

        Ok(CacheEntry {
            path,
            last_modified,
            duration: samples.len() as f64 / spec_data.rate as f64,
            waveform: peaks,
        })
    }
}

pub struct AssetProcessorHandle {
    command_tx: mpsc::Sender<AssetCommand>,
}

impl AssetProcessorHandle {
    pub async fn request_asset_data(&self, target: PathBuf) {
        self.command_tx
            .send(AssetCommand::RequestAssetData(target))
            .await
            .unwrap();
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