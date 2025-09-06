use tokio::sync::mpsc;
use uuid::Uuid;

use crate::action::CueAction;

use super::ControllerCommand;

#[derive(Clone)]
pub struct CueControllerHandle {
    pub(super) command_tx: mpsc::Sender<ControllerCommand>,
}

impl CueControllerHandle {
    pub async fn go(&self) -> anyhow::Result<()> {
        self.command_tx.send(ControllerCommand::Go).await?;
        Ok(())
    }

    pub async fn load(&self, uuid: Uuid) -> anyhow::Result<()> {
        self.command_tx.send(ControllerCommand::Load(uuid)).await?;
        Ok(())
    }

    pub async fn pause(&self, uuid: Uuid) -> anyhow::Result<()> {
        self.command_tx.send(ControllerCommand::Pause(uuid)).await?;
        Ok(())
    }

    pub async fn resume(&self, uuid: Uuid) -> anyhow::Result<()> {
        self.command_tx
            .send(ControllerCommand::Resume(uuid))
            .await?;
        Ok(())
    }

    pub async fn stop(&self, uuid: Uuid) -> anyhow::Result<()> {
        self.command_tx.send(ControllerCommand::Stop(uuid)).await?;
        Ok(())
    }

    pub async fn seek_to(&self, uuid: Uuid, position: f64) -> anyhow::Result<()> {
        self.command_tx
            .send(ControllerCommand::SeekTo(uuid, position))
            .await?;
        Ok(())
    }

    pub async fn seek_by(&self, uuid: Uuid, amount: f64) -> anyhow::Result<()> {
        self.command_tx
            .send(ControllerCommand::SeekBy(uuid, amount))
            .await?;
        Ok(())
    }

    pub async fn pause_all(&self) -> anyhow::Result<()> {
        self.command_tx.send(ControllerCommand::PauseAll).await?;
        Ok(())
    }

    pub async fn resume_all(&self) -> anyhow::Result<()> {
        self.command_tx.send(ControllerCommand::ResumeAll).await?;
        Ok(())
    }

    pub async fn stop_all(&self) -> anyhow::Result<()> {
        self.command_tx.send(ControllerCommand::StopAll).await?;
        Ok(())
    }

    pub async fn perform_action(&self, uuid: Uuid, action: CueAction) -> anyhow::Result<()> {
        self.command_tx
            .send(ControllerCommand::PerformAction(uuid, action))
            .await?;
        Ok(())
    }

    pub async fn set_playback_cursor(&self, uuid: Option<Uuid>) -> anyhow::Result<()> {
        self.command_tx
            .send(ControllerCommand::SetPlaybackCursor { cue_id: uuid })
            .await?;
        Ok(())
    }
}
