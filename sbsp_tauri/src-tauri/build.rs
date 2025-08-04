use sbsp_backend::model::ShowModel;
use ts_rs::TS;

fn main() {
    println!("cargo:rerun-if-changed=../../sbsp_backend/src/model.rs");
    println!("cargo:rerun-if-changed=../../sbsp_backend/src/model/");

    <ShowModel as TS>::export_all_to("../src/types/").unwrap();

    tauri_build::build()
}
