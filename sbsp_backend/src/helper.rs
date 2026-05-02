use std::collections::BTreeSet;

#[cfg(feature = "backend")]
use anyhow::Result;
use indexmap::IndexMap;
#[cfg(feature = "backend")]
use rodio::{
    DeviceTrait,
    cpal::{SampleFormat, SupportedBufferSize, traits::HostTrait},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "backend")]
const FRAME_SIZES: &[u32] = &[32, 64, 128, 256, 512, 1024, 2048, 4096];
#[cfg(feature = "backend")]
const SUPPORTED_SAMPLE_RATE: u32 = 192000;

#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SupportedHardware {
    pub default: String,
    pub devices: IndexMap<String, DeviceInformation>,
}

#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInformation {
    pub name: String,
    pub supported_configs: Vec<FrameConfig>,
    pub default_channel_count: u16,
    pub default_sample_rate: u32,
}

#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FrameConfig {
    pub channel_count: u16,
    pub sample_rates: BTreeSet<u32>,
    pub buffer_sizes: BTreeSet<u32>,
}

#[cfg(feature = "backend")]
fn get_buffer_sizes(buf_conf: SupportedBufferSize) -> BTreeSet<u32> {
    match buf_conf {
        SupportedBufferSize::Range { min, max } => FRAME_SIZES
            .iter()
            .filter(|&&size| size >= min && size <= max)
            .cloned()
            .collect(),
        SupportedBufferSize::Unknown => BTreeSet::new(),
    }
}

#[cfg(feature = "backend")]
pub fn get_supported_hardware() -> Result<SupportedHardware> {
    let host = rodio::cpal::default_host();
    let devices = host.devices()?;
    let default = host
        .default_output_device()
        .ok_or(anyhow::anyhow!("Failed to get default device."))?
        .id()?;
    let mut hardwares = IndexMap::new();
    for device in devices {
        if let Ok(id) = device.id()
            && let Ok(description) = device.description()
            && let Ok(supported_confs) = device.supported_output_configs()
            && let Ok(default_config) = device.default_output_config()
        {
            let mut configs = IndexMap::new();
            for config in supported_confs {
                if config.sample_format() != SampleFormat::F32
                    || config.max_sample_rate() > SUPPORTED_SAMPLE_RATE
                {
                    continue;
                }
                let entry = configs
                    .entry(config.channels())
                    .or_insert((BTreeSet::new(), *(config.buffer_size())));
                entry.0.insert(config.max_sample_rate());
            }
            if !configs.is_empty() {
                hardwares.insert(
                    id.to_string(),
                    DeviceInformation {
                        name: description.name().to_string(),
                        supported_configs: configs
                            .into_iter()
                            .map(
                                |(channel_count, (sample_rates, buffer_sizes))| FrameConfig {
                                    channel_count,
                                    sample_rates,
                                    buffer_sizes: get_buffer_sizes(buffer_sizes),
                                },
                            )
                            .collect(),
                        default_channel_count: default_config.channels(),
                        default_sample_rate: default_config.sample_rate(),
                    },
                );
            }
        }
    }
    Ok(SupportedHardware {
        default: default.to_string(),
        devices: hardwares,
    })
}
