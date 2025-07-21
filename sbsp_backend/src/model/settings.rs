use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShowSettings {
    pub general: GeneralSettings,
    // TODO Templates, Audio, Network, MIDI, OSC, Video settings
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSettings {}
