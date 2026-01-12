use crate::{
    asset_processor::AssetProcessorCommand,
    controller::{ControllerCommand, state::ShowState},
    event::UiEvent,
    manager::{ModelCommand, ProjectStatus},
    model::ShowModel,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "apiclient")]
pub mod client;
mod file_list;
#[cfg(feature = "apiserver")]
pub mod server;

#[cfg(feature = "type_export")]
pub mod client {
    mod service_entry;
    pub use service_entry::ServiceEntry;
}

pub use file_list::FileList;

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum WsFeedback {
    Event(Box<UiEvent>),
    State(ShowState),
    AssetList(Vec<FileList>),
    FullShowState(FullShowState)
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WsCommand {
    Controll(ControllerCommand),
    Model(Box<ModelCommand>),
    AssetProcessor(AssetProcessorCommand),
    RequestAssetList,
    RequestFullShowState,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FullShowState {
    pub project_status: ProjectStatus,
    pub show_model: ShowModel,
    pub show_state: ShowState,
}
