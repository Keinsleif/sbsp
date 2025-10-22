use std::path::PathBuf;

use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "type_export", derive(ts_rs::TS))]
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
