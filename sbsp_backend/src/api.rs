use serde::{Deserialize, Serialize};
use crate::{
    asset_processor::AssetProcessorCommand, controller::{state::ShowState, ControllerCommand}, event::UiEvent, manager::ModelCommand, model::ShowModel
};

#[cfg(feature = "apiserver")]
pub mod server;
#[cfg(feature = "apiclient")]
pub mod client;
mod file_list;

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
