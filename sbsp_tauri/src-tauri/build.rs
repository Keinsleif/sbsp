use sbsp_backend::{controller::state::ShowState, model::ShowModel};
use ts_rs::TS;

fn main() {
    println!("cargo:rerun-if-changed=../../sbsp_backend/src/model.rs");
    println!("cargo:rerun-if-changed=../../sbsp_backend/src/model/");
    println!("cargo:rerun-if-changed=../../sbsp_backend/src/controller/state.rs");

    <ShowModel as TS>::export_all_to("../src/types/").unwrap();
    <ShowState as TS>::export_all_to("../src/types/").unwrap();

    tauri_build::build()
}
