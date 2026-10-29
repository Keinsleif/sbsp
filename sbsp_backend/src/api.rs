// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

#[cfg(feature = "protocol")]
pub mod auth;
#[cfg(any(feature = "protocol", feature = "client"))]
pub mod client;
mod file_list;
#[cfg(feature = "server")]
pub mod server;

pub use file_list::FileList;

#[cfg(feature = "protocol")]
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
#[cfg(feature = "protocol")]
use std::str::FromStr;

use crate::{
    FullShowState, asset_processor::AssetProcessorCommand, controller::ControllerCommand,
    event::BackendEvent, manager::ModelCommand,
};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Permissions(u8);

#[cfg(feature = "protocol")]
bitflags! {
    impl Permissions: u8 {
        const READ = 0b0001;
        const CONTROL = 0b0010;
        const EDIT = 0b0100;
    }
}

#[cfg(feature = "protocol")]
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
#[serde(rename_all = "camelCase")]
pub struct PermissionInfo {
    pub password: String,
    pub permission: Permissions,
}

#[cfg(feature = "protocol")]
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
#[serde(rename_all = "camelCase")]
pub struct ApiServerOptions {
    pub port: u16,
    pub discoverry: Option<String>,
    pub auth_map: Vec<PermissionInfo>,
    pub trust_ip_headers: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuthInfo {
    pub challenge: String,
    pub salt: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum WsFeedback {
    Hello { auth: AuthInfo },
    Authenticated { perm: Permissions },
    Event(Box<BackendEvent>),
    AssetList(Vec<FileList>),
    FullShowState(Box<FullShowState>),
    /// The peak output level of the host's audio engine, per channel. Sent
    /// only while the client has asked for it (`WsCommand::SubscribeLevel`).
    Level { left: f32, right: f32 },
    Error(WsError),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WsCommand {
    Authenticate { response: Option<String> },
    Control(ControllerCommand),
    Model(Box<ModelCommand>),
    AssetProcessor(AssetProcessorCommand),
    RequestAssetList,
    RequestFullShowState,
    RequestSyncState,
    /// Starts (or stops) the host sending `WsFeedback::Level`. Clients that
    /// never ask are never sent one.
    SubscribeLevel { enabled: bool },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_level_messages_have_a_stable_wire_form() {
        let level = serde_json::to_value(WsFeedback::Level {
            left: 0.5,
            right: 0.25,
        })
        .unwrap();
        assert_eq!(
            level,
            serde_json::json!({ "type": "level", "data": { "left": 0.5, "right": 0.25 } })
        );

        let command: WsCommand =
            serde_json::from_str(r#"{"type":"subscribeLevel","enabled":true}"#).unwrap();
        assert!(matches!(command, WsCommand::SubscribeLevel { enabled: true }));
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WsError {
    AuthenticationFailed,
    AuthenticationRateLimitExceeded,
    PermissionDenied,
}
