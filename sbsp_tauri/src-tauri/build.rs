use sbsp_backend::{
    asset_processor::AssetData, controller::state::ShowState, event::UiEvent, model::ShowModel,
};
use ts_rs::TS;

fn main() {
    println!("cargo:rerun-if-changed=../../sbsp_backend/src/model.rs");
    println!("cargo:rerun-if-changed=../../sbsp_backend/src/model/");
    println!("cargo:rerun-if-changed=../../sbsp_backend/src/controller/state.rs");
    println!("cargo:rerun-if-changed=../../sbsp_backend/src/event.rs");

    <ShowModel as TS>::export_all_to("../src/types").unwrap();
    <ShowState as TS>::export_all_to("../src/types").unwrap();
    <UiEvent as TS>::export_all_to("../src/types").unwrap();
    <AssetData as TS>::export_all_to("../src/types").unwrap();

    tauri_build::build()
}
