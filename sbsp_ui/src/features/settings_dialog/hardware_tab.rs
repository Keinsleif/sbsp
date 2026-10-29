// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

//! The audio hardware tab (host only): the output device and its
//! configuration. Ported from `AudioHardware.vue`.
//!
//! Each list starts with "Default" and goes on with what the chosen device
//! supports. A saved value the hardware does not offer (a device that was
//! unplugged, say) shows as "Default" and is flagged; it stays in the
//! settings until something else is chosen.

use iced::widget::{Space, column, container, row, text, tooltip};
use iced::{Alignment, Element, Length};
use sbsp_backend::helper::{DeviceInformation, SupportedHardware};
use crate::settings::AudioHardwareSettings;

use super::{Choice, Message, choice, picker};
use crate::fl;
use crate::widgets::icon::{self, Icon, Tone};
use crate::widgets::inputs::labeled;

/// What the four lists offer, and which of the saved values they do not.
#[derive(Clone)]
pub struct Options {
    pub devices: Vec<Choice<Option<String>>>,
    pub channel_counts: Vec<Choice<Option<u16>>>,
    pub sample_rates: Vec<Choice<Option<u32>>>,
    pub buffer_sizes: Vec<Choice<Option<u32>>>,
    /// The saved device is not among the devices.
    pub device_missing: bool,
    /// The saved channel count, sample rate or buffer size is not offered.
    pub config_missing: bool,
    /// What each list shows as selected (`None`: Default).
    pub device: Option<String>,
    pub channel_count: Option<u16>,
    pub sample_rate: Option<u32>,
    pub buffer_size: Option<u32>,
}

fn default_label() -> String {
    fl!("general-default")
}

fn kilohertz(rate: u32) -> String {
    format!("{} kHz", f64::from(rate) / 1000.0)
}

/// `current` if the list offers it, else `None` (Default).
fn offered<T: PartialEq + Copy>(list: &[Choice<Option<T>>], current: Option<T>) -> Option<T> {
    current.filter(|value| list.iter().any(|option| option.value == Some(*value)))
}

pub fn options(hardware: &SupportedHardware, saved: &AudioHardwareSettings) -> Options {
    let mut devices = vec![choice(None, default_label())];
    devices.extend(
        hardware
            .devices
            .iter()
            .map(|(id, device)| choice(Some(id.clone()), device.name.clone())),
    );
    let device_id = saved
        .device_id
        .clone()
        .filter(|id| hardware.devices.contains_key(id));
    let device: Option<&DeviceInformation> = hardware
        .devices
        .get(device_id.as_deref().unwrap_or(&hardware.default));

    let Some(device) = device else {
        // No such device at all (not even the default): nothing to pick
        // from but the device list.
        return Options {
            device_missing: saved.device_id.is_some() && device_id.is_none(),
            devices,
            channel_counts: Vec::new(),
            sample_rates: Vec::new(),
            buffer_sizes: Vec::new(),
            device: device_id,
            channel_count: None,
            sample_rate: None,
            buffer_size: None,
            config_missing: false,
        };
    };

    let mut counts: Vec<u16> = device
        .supported_configs
        .iter()
        .map(|config| config.channel_count)
        .collect();
    counts.sort_unstable();
    counts.dedup();
    let mut channel_counts = vec![choice(
        None,
        format!("{} ({})", default_label(), device.default_channel_count),
    )];
    channel_counts.extend(counts.into_iter().map(|c| choice(Some(c), c.to_string())));
    let channel_count = offered(&channel_counts, saved.channel_count);
    let channels = channel_count.unwrap_or(device.default_channel_count);

    let mut rates: Vec<u32> = device
        .supported_configs
        .iter()
        .filter(|config| config.channel_count == channels)
        .flat_map(|config| config.sample_rates.keys().copied())
        .collect();
    rates.sort_unstable();
    rates.dedup();
    let mut sample_rates = vec![choice(
        None,
        format!("{} ({})", default_label(), kilohertz(device.default_sample_rate)),
    )];
    sample_rates.extend(rates.into_iter().map(|r| choice(Some(r), kilohertz(r))));
    let sample_rate = offered(&sample_rates, saved.sample_rate);
    let rate = sample_rate.unwrap_or(device.default_sample_rate);

    let mut sizes: Vec<u32> = device
        .supported_configs
        .iter()
        .filter(|config| config.channel_count == channels)
        .filter_map(|config| config.sample_rates.get(&rate))
        .flatten()
        .copied()
        .collect();
    sizes.sort_unstable();
    sizes.dedup();
    let mut buffer_sizes = vec![choice(None, default_label())];
    buffer_sizes.extend(sizes.into_iter().map(|s| choice(Some(s), format!("{s} Frames"))));
    let buffer_size = offered(&buffer_sizes, saved.buffer_size);

    // A list with only "Default" in it (nothing is known about the
    // hardware) cannot say that a saved value is missing.
    let missing = |list: usize, saved_value: bool, shown: bool| list > 1 && saved_value && !shown;
    let config_missing = missing(
        channel_counts.len(),
        saved.channel_count.is_some(),
        channel_count.is_some(),
    ) || missing(
        sample_rates.len(),
        saved.sample_rate.is_some(),
        sample_rate.is_some(),
    ) || missing(
        buffer_sizes.len(),
        saved.buffer_size.is_some(),
        buffer_size.is_some(),
    );

    Options {
        device_missing: saved.device_id.is_some() && device_id.is_none(),
        devices,
        channel_counts,
        sample_rates,
        buffer_sizes,
        device: device_id,
        channel_count,
        sample_rate,
        buffer_size,
        config_missing,
    }
}

fn heading<'a>(title: String, flagged: bool, warning: String) -> Element<'a, Message> {
    let mark: Element<'a, Message> = if flagged {
        tooltip(
            icon::toned(Icon::Alert, 18.0, Tone::Warning),
            container(text(warning).size(12))
                .padding(6)
                .style(container::rounded_box),
            tooltip::Position::Right,
        )
        .into()
    } else {
        Space::new().into()
    };
    row![text(title).size(16), mark]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
}

pub fn view<'a>(
    hardware: Option<&Result<SupportedHardware, String>>,
    saved: &AudioHardwareSettings,
) -> Element<'a, Message> {
    let warning = container(text(fl!("dialog-settings-global-audio-hardware-warning")))
        .padding(8)
        .width(Length::Fill)
        .style(container::danger);

    let body: Element<'a, Message> = match hardware {
        None => Space::new().into(),
        Some(Err(error)) => text(error.clone()).into(),
        Some(Ok(hardware)) => {
            let options = options(hardware, saved);
            column![
                heading(
                    fl!("dialog-settings-global-audio-hardware-device"),
                    options.device_missing,
                    fl!("dialog-settings-global-audio-hardware-device-id-fallback-title"),
                ),
                picker(options.devices, &options.device, Message::Device),
                heading(
                    fl!("dialog-settings-global-audio-hardware-device-config"),
                    options.config_missing,
                    fl!("dialog-settings-global-audio-hardware-config-fallback-title"),
                ),
                labeled(
                    Some(fl!("dialog-settings-global-audio-hardware-channel-count").as_str()),
                    picker(options.channel_counts, &options.channel_count, Message::ChannelCount),
                ),
                labeled(
                    Some(fl!("dialog-settings-global-audio-hardware-sample-rate").as_str()),
                    picker(options.sample_rates, &options.sample_rate, Message::SampleRate),
                ),
                labeled(
                    Some(fl!("dialog-settings-global-audio-hardware-buffer-size").as_str()),
                    picker(options.buffer_sizes, &options.buffer_size, Message::BufferSize),
                ),
            ]
            .spacing(12)
            .into()
        }
    };

    column![warning, body].spacing(16).max_width(500).into()
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use indexmap::IndexMap;
    use sbsp_backend::helper::FrameConfig;

    use super::*;

    fn config(channels: u16, rates: &[(u32, &[u32])]) -> FrameConfig {
        FrameConfig {
            channel_count: channels,
            sample_rates: rates
                .iter()
                .map(|(rate, sizes)| (*rate, sizes.iter().copied().collect::<BTreeSet<_>>()))
                .collect::<BTreeMap<_, _>>(),
        }
    }

    fn hardware() -> SupportedHardware {
        let mut devices = IndexMap::new();
        devices.insert(
            "a".to_owned(),
            DeviceInformation {
                name: "Device A".to_owned(),
                supported_configs: vec![
                    config(2, &[(44100, &[256, 512]), (48000, &[256, 512, 1024])]),
                    config(8, &[(48000, &[512])]),
                ],
                default_channel_count: 2,
                default_sample_rate: 48000,
            },
        );
        devices.insert(
            "b".to_owned(),
            DeviceInformation {
                name: "Device B".to_owned(),
                supported_configs: vec![config(1, &[(96000, &[128])])],
                default_channel_count: 1,
                default_sample_rate: 96000,
            },
        );
        SupportedHardware {
            default: "a".to_owned(),
            devices,
        }
    }

    fn values<T: Clone>(list: &[Choice<Option<T>>]) -> Vec<Option<T>> {
        list.iter().map(|option| option.value.clone()).collect()
    }

    #[test]
    fn the_lists_follow_the_default_device_when_none_is_chosen() {
        let options = options(&hardware(), &AudioHardwareSettings::default());

        assert_eq!(
            values(&options.devices),
            [None, Some("a".to_owned()), Some("b".to_owned())]
        );
        assert_eq!(values(&options.channel_counts), [None, Some(2), Some(8)]);
        // The default channel count (2) decides the rates and sizes.
        assert_eq!(values(&options.sample_rates), [None, Some(44100), Some(48000)]);
        assert_eq!(
            values(&options.buffer_sizes),
            [None, Some(256), Some(512), Some(1024)]
        );
        assert!(!options.device_missing && !options.config_missing);
    }

    #[test]
    fn the_lists_follow_the_chosen_device_and_configuration() {
        let saved = AudioHardwareSettings {
            device_id: Some("a".to_owned()),
            channel_count: Some(8),
            sample_rate: Some(48000),
            buffer_size: Some(512),
        };
        let options = options(&hardware(), &saved);

        assert_eq!(values(&options.sample_rates), [None, Some(48000)]);
        assert_eq!(values(&options.buffer_sizes), [None, Some(512)]);
        assert_eq!(options.buffer_size, Some(512));
        assert!(!options.config_missing);

        let other = AudioHardwareSettings {
            device_id: Some("b".to_owned()),
            ..AudioHardwareSettings::default()
        };
        let options = super::options(&hardware(), &other);
        assert_eq!(values(&options.channel_counts), [None, Some(1)]);
        assert_eq!(values(&options.sample_rates), [None, Some(96000)]);
        assert_eq!(values(&options.buffer_sizes), [None, Some(128)]);
    }

    #[test]
    fn a_saved_value_that_is_not_offered_shows_as_default_and_is_flagged() {
        let gone = AudioHardwareSettings {
            device_id: Some("unplugged".to_owned()),
            ..AudioHardwareSettings::default()
        };
        let options = options(&hardware(), &gone);
        assert!(options.device_missing);
        assert_eq!(options.device, None);

        let odd = AudioHardwareSettings {
            channel_count: Some(2),
            sample_rate: Some(22050),
            ..AudioHardwareSettings::default()
        };
        let options = super::options(&hardware(), &odd);
        assert!(options.config_missing);
        assert_eq!(options.sample_rate, None);
        assert_eq!(options.channel_count, Some(2));
    }
}
