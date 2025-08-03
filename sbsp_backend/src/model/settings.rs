use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, Default, TS)]
#[serde(rename_all = "camelCase")]
pub struct ShowSettings {
    pub general: GeneralSettings,
    // TODO Templates, Audio, Network, MIDI, OSC, Video settings
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, TS)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSettings {}
