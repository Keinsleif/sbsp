use ts_rs::TS;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, TS)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GroupMode {
    Playlist,
    Concurrency,
}