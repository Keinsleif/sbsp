use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::model::cue::{AudioCueLevels, Cue, CueParam, CueSequence};

#[derive(Serialize, Deserialize, Debug, Clone, Default, TS)]
#[serde(rename_all = "camelCase")]
pub struct ShowSettings {
    pub general: GeneralSettings,
    pub template: TemplateSettings,
    // TODO Templates, Audio, Network, MIDI, OSC, Video settings
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, TS)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSettings {}

#[derive(Serialize, Deserialize, Debug, Clone, TS)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSettings {
    pub audio: Option<Cue>,
    pub wait: Option<Cue>,
}

impl Default for TemplateSettings {
    fn default() -> Self {
        Self {
            audio: Some(Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: "".to_string(),
                notes: "".to_string(),
                pre_wait: 0.0,
                post_wait: 0.0,
                sequence: CueSequence::DoNotContinue,
                params: CueParam::Audio { target: PathBuf::new(), start_time: None, fade_in_param: None, end_time: None, fade_out_param: None, levels: AudioCueLevels::default(), loop_region: None },
            }),
            wait: Some(Cue {
                id: Uuid::nil(),
                number: "".to_string(),
                name: "".to_string(),
                notes: "".to_string(),
                pre_wait: 0.0,
                post_wait: 0.0,
                sequence: CueSequence::DoNotContinue,
                params: CueParam::Wait { duration: 5.0 },
            }),
        }
    }
}
