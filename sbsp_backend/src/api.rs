// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use crate::{
    FullShowState, asset_processor::AssetProcessorCommand, controller::ControllerCommand,
    event::BackendEvent, manager::ModelCommand,
};

#[cfg(any(feature = "apiserver", feature = "apiclient"))]
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

#[cfg(feature = "apiclient")]
pub mod client;
mod file_list;
#[cfg(feature = "apiserver")]
pub mod server;

#[cfg(any(feature = "apiserver", feature = "apiclient"))]
mod auth;

#[cfg(feature = "type_export")]
pub mod client {
    mod service_entry;
    pub use service_entry::ServiceEntry;
}

pub use file_list::FileList;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
pub struct Permissions(u8);

#[cfg(any(feature = "apiserver", feature = "apiclient"))]
bitflags! {
    impl Permissions: u8 {
        const READ = 0b0001;
        const CONTROL = 0b0010;
        const EDIT = 0b0100;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PermissionInfo {
    pub password: String,
    pub permission: Permissions,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ApiServerOptions {
    pub port: u16,
    pub discoverry: Option<String>,
    pub auth_map: Vec<PermissionInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AuthInfo {
    pub challenge: String,
    pub salt: String,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum WsFeedback {
    Hello { auth: AuthInfo },
    Authenticated { perm: Permissions },
    Event(Box<BackendEvent>),
    AssetList(Vec<FileList>),
    FullShowState(FullShowState),
    Error(WsError),
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WsCommand {
    Authenticate { response: Option<String> },
    Control(ControllerCommand),
    Model(Box<ModelCommand>),
    AssetProcessor(AssetProcessorCommand),
    RequestAssetList,
    RequestFullShowState,
    RequestSyncState,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WsError {
    AuthenticationFailed,
    PermissionDenied,
}
