use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{action::CueAction, executor::ExecutorCommand};

#[derive(Serialize, Deserialize, Debug)]
#[serde(
    tag = "command",
    content = "params",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ControllerCommand {
    Go,
    Load(Uuid),
    Pause(Uuid),
    Resume(Uuid),
    Stop(Uuid),
    SeekTo(Uuid, f64),
    SeekBy(Uuid, f64),
    PauseAll,
    ResumeAll,
    StopAll,
    PerformAction(Uuid, CueAction),
    SetPlaybackCursor { cue_id: Option<Uuid> },
}


impl ControllerCommand {
    pub(super) fn try_into_executor_command(&self) -> Option<ExecutorCommand> {
        match self {
            Self::Pause(uuid) => Some(ExecutorCommand::Pause(*uuid)),
            Self::Resume(uuid) => Some(ExecutorCommand::Resume(*uuid)),
            Self::Stop(uuid) => Some(ExecutorCommand::Stop(*uuid)),
            Self::Load(uuid) => Some(ExecutorCommand::Load(*uuid)),
            Self::SeekTo(uuid, position) => Some(ExecutorCommand::SeekTo(*uuid, *position)),
            Self::SeekBy(uuid, amount) => Some(ExecutorCommand::SeekBy(*uuid, *amount)),
            Self::PerformAction(uuid, action) => Some(ExecutorCommand::PerformAction(*uuid, *action)),
            _ => None,
        }
    }

    pub(super) fn try_all_into_single_executor_command(&self, cue_id: Uuid) -> Option<ExecutorCommand> {
        match self {
            Self::PauseAll => Some(ExecutorCommand::Pause(cue_id)),
            Self::ResumeAll => Some(ExecutorCommand::Resume(cue_id)),
            Self::StopAll => Some(ExecutorCommand::Stop(cue_id)),
            _ => None,
        }
    }
}