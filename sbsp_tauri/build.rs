use sbsp_backend::{
    api::{WsCommand, WsFeedback}, asset_processor::AssetData, controller::state::ShowState, event::UiEvent, model::ShowModel
};
use sbsp_license::data::LicenseInformation;
use ts_rs::TS;

fn main() {
    <ShowModel as TS>::export_all().unwrap();
    <ShowState as TS>::export_all().unwrap();
    <UiEvent as TS>::export_all().unwrap();
    <AssetData as TS>::export_all().unwrap();
    <WsCommand as TS>::export_all().unwrap();
    <WsFeedback as TS>::export_all().unwrap();
    <LicenseInformation as TS>::export_all().unwrap();

    tauri_build::build()
}
