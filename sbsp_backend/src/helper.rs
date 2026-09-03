// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::collections::{BTreeMap, BTreeSet};

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
const COMMON_SAMPLE_RATES: &[u32] = &[
    8000, 11025, 16000, 22050, 32000, 44100, 48000, 88200, 96000, 176400, 192000,
];

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
    pub sample_rates: BTreeMap<u32, BTreeSet<u32>>,
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
            let mut configs: IndexMap<u16, BTreeMap<u32, BTreeSet<u32>>> = IndexMap::new();
            for config in supported_confs {
                if config.sample_format() != SampleFormat::F32 {
                    continue;
                }
                let entry = configs.entry(config.channels()).or_default();

                for &rate in COMMON_SAMPLE_RATES {
                    if rate >= config.min_sample_rate() && rate <= config.max_sample_rate() {
                        entry
                            .entry(rate)
                            .or_default()
                            .extend(get_buffer_sizes(*config.buffer_size()));
                    }
                }
            }
            if !configs.is_empty() {
                let name = format!(
                    "{}{}",
                    description.name(),
                    description
                        .driver()
                        .map(|d| format!(" ({})", d))
                        .unwrap_or("".to_string())
                );
                hardwares.insert(
                    id.to_string(),
                    DeviceInformation {
                        name,
                        supported_configs: configs
                            .into_iter()
                            .map(|(channel_count, sample_rates)| FrameConfig {
                                channel_count,
                                sample_rates,
                            })
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
