// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

mod file_list;
#[cfg(any(feature = "client", feature = "type_export"))]
pub mod client;
#[cfg(feature = "server")]
pub mod server;
#[cfg(any(feature = "server", feature = "client"))]
mod auth;

pub use file_list::FileList;

use serde::{Deserialize, Serialize};
#[cfg(any(feature = "server", feature = "client"))]
use std::str::FromStr;
#[cfg(any(feature = "server", feature = "client"))]
use bitflags::bitflags;

use crate::{
    FullShowState, asset_processor::AssetProcessorCommand, controller::ControllerCommand,
    event::BackendEvent, manager::ModelCommand,
};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
pub struct Permissions(u8);

#[cfg(any(feature = "server", feature = "client"))]
bitflags! {
    impl Permissions: u8 {
        const READ = 0b0001;
        const CONTROL = 0b0010;
        const EDIT = 0b0100;
    }
}

#[cfg(any(feature = "server", feature = "client"))]
impl FromStr for Permissions {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut perms = Self::empty();

        for item in s.split(',') {
            match item.trim().to_lowercase().as_str() {
                "read" | "r" => perms |= Self::READ,
                "control" | "c" => perms |= Self::CONTROL,
                "edit" | "e" => perms |= Self::EDIT,
                other => {
                    if let Ok(num) = other.parse::<u8>() {
                        perms |= Self::from_bits_truncate(num);
                    } else {
                        return Err(format!("Invalid permission specifier: '{other}'"));
                    }
                }
            }
        }

        if perms.is_empty() {
            return Err("No permissions are specified".to_string());
        }

        Ok(perms)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PermissionInfo {
    pub password: String,
    pub permission: Permissions,
}

#[cfg(any(feature = "server", feature = "client"))]
impl FromStr for PermissionInfo {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (password, perm_str) = s.split_once(':').ok_or_else(|| {
            "Format error: permission must be in '<password>:<permissions>' format".to_string()
        })?;

        let permission = perm_str.parse::<Permissions>()?;

        Ok(PermissionInfo {
            password: password.to_string(),
            permission,
        })
    }
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
    FullShowState(Box<FullShowState>),
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
