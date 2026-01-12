use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::action::CueAction;

#[cfg(feature = "backend")]
use crate::executor::ExecutorCommand;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
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

#[cfg(feature = "backend")]
impl ControllerCommand {
    pub(super) fn into_executor_command(self) -> ExecutorCommand {
        match self {
            Self::Pause(uuid) => ExecutorCommand::Pause(uuid),
            Self::Resume(uuid) => ExecutorCommand::Resume(uuid),
            Self::Stop(uuid) => ExecutorCommand::Stop(uuid),
            Self::Load(uuid) => ExecutorCommand::Load(uuid),
            Self::SeekTo(uuid, position) => ExecutorCommand::SeekTo(uuid, position),
            Self::SeekBy(uuid, amount) => ExecutorCommand::SeekBy(uuid, amount),
            Self::PerformAction(uuid, action) => {
                ExecutorCommand::PerformAction(uuid, action)
            }
            _ => unreachable!(),
        }
    }

    pub(super) fn try_all_into_single_executor_command(
        &self,
        cue_id: Uuid,
    ) -> ExecutorCommand {
        match self {
            Self::PauseAll => ExecutorCommand::Pause(cue_id),
            Self::ResumeAll => ExecutorCommand::Resume(cue_id),
            Self::StopAll => ExecutorCommand::Stop(cue_id),
            _ => unreachable!(),
        }
    }
}
