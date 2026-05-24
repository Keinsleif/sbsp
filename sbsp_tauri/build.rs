use sbsp_backend::{
    ts_rs::{TS, Config},
    api::{ApiServerOptions, WsCommand, WsFeedback},
    asset_processor::AssetData,
    controller::state::ShowState,
    event::BackendEvent,
    helper::SupportedHardware,
    model::ShowModel,
};
use sbsp_license::data::LicenseInformation;

fn main() {
    let config = Config::from_env();

    <ShowModel as TS>::export_all(&config).unwrap();
    <ShowState as TS>::export_all(&config).unwrap();
    <BackendEvent as TS>::export_all(&config).unwrap();
    <AssetData as TS>::export_all(&config).unwrap();
    <ApiServerOptions as TS>::export_all(&config).unwrap();
    <WsCommand as TS>::export_all(&config).unwrap();
    <WsFeedback as TS>::export_all(&config).unwrap();
    <LicenseInformation as TS>::export_all(&config).unwrap();
    <SupportedHardware as TS>::export_all(&config).unwrap();

    tauri_build::build()
}
