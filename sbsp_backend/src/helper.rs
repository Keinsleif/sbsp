use std::collections::{BTreeSet, HashMap};

use anyhow::Result;
use rodio::{DeviceTrait, cpal::{SampleFormat, SupportedBufferSize, traits::HostTrait}};
use serde::{Deserialize, Serialize};

const FRAME_SIZES: &[u32] = &[32, 64, 128, 256, 512, 1024, 2048, 4096];

#[derive(Serialize, Deserialize)]
pub struct SupportedHardware {
    default: String,
    devices: HashMap<String, DeviceInformation>
}

#[derive(Serialize, Deserialize)]
pub struct DeviceInformation {
    name: String,
    supported_configs: Vec<FrameConfig>,
    default_channel_count: u16,
    default_sample_rate: u32,
}

#[derive(Serialize, Deserialize)]
pub struct FrameConfig {
    channel_count: u16,
    sample_rates: BTreeSet<u32>,
    buffer_sizes: BTreeSet<u32>,
}

fn get_buffer_sizes(buf_conf: SupportedBufferSize) -> BTreeSet<u32> {
    match buf_conf {
        SupportedBufferSize::Range { min, max } => {
            FRAME_SIZES.iter()
                .filter(|&&size| size >= min && size <= max)
                .cloned()
                .collect()
        },
        SupportedBufferSize::Unknown => {
            BTreeSet::new()
        }
    }
}

pub fn get_supported_hardware() -> Result<SupportedHardware> {
    let host = rodio::cpal::default_host();
    let devices = host.devices()?;
    let default = host.default_output_device().ok_or(anyhow::anyhow!("Failed to get default device."))?.id()?;
    let mut hardwares = HashMap::new();
    for device in devices {
        if let Ok(id) = device.id()
        && let Ok(name) = device.description()
        && let Ok(supported_confs) = device.supported_output_configs()
        && let Ok(default_config) = device.default_output_config() {
            let mut configs = HashMap::new();
            for config in supported_confs {
                if config.sample_format() != SampleFormat::F32 {
                    continue;
                }
                let entry = configs.entry(config.channels()).or_insert((BTreeSet::new(), *(config.buffer_size())));
                entry.0.insert(config.max_sample_rate());
            }
            if !configs.is_empty() {
                hardwares.insert(id.to_string(), DeviceInformation {
                    name: name.to_string(),
                    supported_configs: configs.into_iter().map(|(channel_count, (sample_rates, buffer_sizes))| {
                        FrameConfig {
                            channel_count,
                            sample_rates,
                            buffer_sizes: get_buffer_sizes(buffer_sizes)
                        }
                    }).collect(),
                    default_channel_count: default_config.channels(),
                    default_sample_rate: default_config.sample_rate(),
                });
            }
        }
    }
    Ok(SupportedHardware {
        default: default.to_string(),
        devices: hardwares,
    })
}