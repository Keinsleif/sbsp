use std::{path::PathBuf, sync::Arc};

use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::{
    manager::ModelCommand,
    model::{ShowModel, cue::Cue, settings::ShowSettings},
};

#[derive(Clone)]
pub struct ShowModelHandle {
    pub(crate) model: Arc<RwLock<ShowModel>>,
    pub(crate) command_tx: mpsc::Sender<ModelCommand>,
    pub(crate) show_model_path: Arc<RwLock<Option<PathBuf>>>,
}

impl ShowModelHandle {
    pub async fn send_command(&self, command: ModelCommand) -> anyhow::Result<()> {
        self.command_tx.send(command).await?;
        Ok(())
    }

    pub async fn update_cue(&self, cue: Cue) -> anyhow::Result<()> {
        self.send_command(ModelCommand::UpdateCue(cue)).await?;
        Ok(())
    }

    pub async fn add_cue(&self, cue: Cue, at_index: usize) -> anyhow::Result<()> {
        self.send_command(ModelCommand::AddCue { cue, at_index })
            .await?;
        Ok(())
    }

    pub async fn add_cues(&self, cues: Vec<Cue>, at_index: usize) -> anyhow::Result<()> {
        self.send_command(ModelCommand::AddCues { cues, at_index })
            .await?;
        Ok(())
    }

    pub async fn remove_cue(&self, cue_id: Uuid) -> anyhow::Result<()> {
        self.send_command(ModelCommand::RemoveCue { cue_id })
            .await?;
        Ok(())
    }

    pub async fn move_cue(&self, cue_id: Uuid, to_index: usize) -> anyhow::Result<()> {
        self.send_command(ModelCommand::MoveCue { cue_id, to_index })
            .await?;
        Ok(())
    }

    pub async fn renumber_cues(
        &self,
        cues: Vec<Uuid>,
        start_from: f64,
        increment: f64,
    ) -> anyhow::Result<()> {
        self.send_command(ModelCommand::RenumberCues {
            cues,
            start_from,
            increment,
        })
        .await?;
        Ok(())
    }

    pub async fn update_settings(&self, new_settings: ShowSettings) -> anyhow::Result<()> {
        self.send_command(ModelCommand::UpdateSettings(Box::new(new_settings)))
            .await?;
        Ok(())
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        self.send_command(ModelCommand::Save).await?;
        Ok(())
    }

    pub async fn save_as(&self, path: PathBuf) -> anyhow::Result<()> {
        self.send_command(ModelCommand::SaveToFile(path)).await?;
        Ok(())
    }

    pub async fn load_from_file(&self, path: PathBuf) -> anyhow::Result<()> {
        self.send_command(ModelCommand::LoadFromFile(path)).await?;
        Ok(())
    }

    pub async fn get_cue_by_id(&self, cue_id: &Uuid) -> Option<Cue> {
        self.read()
            .await
            .cues
            .iter()
            .find(|c| c.id.eq(cue_id))
            .cloned()
    }

    pub async fn get_current_file_path(&self) -> Option<PathBuf> {
        self.show_model_path.read().await.clone()
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, ShowModel> {
        self.model.read().await
    }
}
