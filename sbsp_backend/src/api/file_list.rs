// SPDX-License-Identifier: Elastic-2.0
// Copyright (c) 2025 Keinsleif (https://github.com/Keinsleif)

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FileList {
    Dir {
        name: String,
        files: Vec<FileList>,
    },
    File {
        name: String,
        path: PathBuf,
        extension: String,
    },
}
