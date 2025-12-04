use std::time::{SystemTime, UNIX_EPOCH};
use ed25519_dalek::{SigningKey, ed25519::signature::SignerMut, pkcs8::DecodePrivateKey};
use base64::prelude::*;
use clap::Parser;
use uuid::Uuid;

use sbsp_license::data::{LicenseFile, LicenseInformation, LicenseEdition};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    privkey: String,

    #[arg(short, long)]
    owner: String,

    #[arg(short, long)]
    edition: Option<String>,

    #[arg(short = 'O', long)]
    output: Option<String>,
}

fn main() {
    let args = Args::parse();

    let license_payload = LicenseInformation {
        owner: args.owner,
        edition: if let Some(edition) = args.edition && edition != "pro" {
            LicenseEdition::Free
        } else {
            LicenseEdition::Pro
        },
        id: Uuid::new_v4(),
        issue_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    };

    let key_content = std::fs::read_to_string(&args.privkey).unwrap();

    let mut signing_key = SigningKey::from_pkcs8_pem(&key_content).unwrap();

    let payload_bytes = serde_json::to_vec(&license_payload).unwrap();

    let signature = signing_key.sign(&payload_bytes);

    let license_file = LicenseFile {
        payload: license_payload,
        signature: BASE64_STANDARD.encode(signature.to_bytes()),
    };

    let content = serde_json::to_string_pretty(&license_file).unwrap();
    let out_path = args.output.unwrap_or("./license.json".to_string());
    std::fs::write(out_path, content).unwrap();
}