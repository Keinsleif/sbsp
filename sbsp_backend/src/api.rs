use crate::{
    asset_processor::AssetProcessorCommand,
    controller::{ControllerCommand, state::ShowState},
    event::UiEvent,
    manager::ModelCommand,
    model::ShowModel,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "apiclient")]
pub mod client;
mod file_list;
#[cfg(feature = "apiserver")]
pub mod server;

pub use file_list::FileList;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum WsFeedback {
    Event(Box<UiEvent>),
    State(ShowState),
    AssetList(Vec<FileList>),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WsCommand {
    Controll(ControllerCommand),
    Model(Box<ModelCommand>),
    AssetProcessor(AssetProcessorCommand),
    RequestAssetList,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullShowState {
    pub show_model: ShowModel,
    pub show_state: ShowState,
}
