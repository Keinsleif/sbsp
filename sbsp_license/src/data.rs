// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum LicenseEdition {
    Free,
    Pro,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LicenseInformation {
    pub owner: String,
    pub edition: LicenseEdition,
    pub id: Uuid,
    pub issue_time: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LicenseFile {
    pub payload: LicenseInformation,
    pub signature: String,
}
