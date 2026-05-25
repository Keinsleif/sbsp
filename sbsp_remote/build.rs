use sbsp_backend::{
    api::{FileList, client::ServiceEntry},
    asset_processor::AssetData,
    controller::state::ShowState,
    event::BackendEvent,
    model::ShowModel,
    ts_rs::{Config, TS},
};

fn main() {
    println!("cargo:rerun-if-changed=../../sbsp_backend/src/model.rs");
    println!("cargo:rerun-if-changed=../../sbsp_backend/src/model/");
    println!("cargo:rerun-if-changed=../../sbsp_backend/src/controller/state.rs");
    println!("cargo:rerun-if-changed=../../sbsp_backend/src/event.rs");
    println!("cargo:rerun-if-changed=../../sbsp_backend/src/asset_processor/data.rs");

    let config = Config::from_env();

    <ShowModel as TS>::export_all(&config).unwrap();
    <ShowState as TS>::export_all(&config).unwrap();
    <BackendEvent as TS>::export_all(&config).unwrap();
    <AssetData as TS>::export_all(&config).unwrap();
    <ServiceEntry as TS>::export_all(&config).unwrap();
    <FileList as TS>::export_all(&config).unwrap();

    tauri_build::build()
}
