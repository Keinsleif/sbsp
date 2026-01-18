use std::{path::PathBuf, sync::RwLock};

use base64::prelude::*;
use ed25519_dalek::{
    SIGNATURE_LENGTH, Signature, Verifier, VerifyingKey, pkcs8::DecodePublicKey as _,
};

use data::{LicenseEdition, LicenseFile};

use crate::data::LicenseInformation;

pub mod data;

pub struct LicenseManager {
    public_key: VerifyingKey,
    current_license: RwLock<Option<LicenseInformation>>,
}

impl LicenseManager {
    pub fn new_from_pem(pem: &str) -> Self {
        let public_key = VerifyingKey::from_public_key_pem(pem).unwrap();

        Self {
            public_key,
            current_license: RwLock::new(None),
        }
    }

    pub fn activate_by_file(&self, path: PathBuf) -> Result<(), anyhow::Error> {
        let content = std::fs::read_to_string(&path)?;
        let license_file: LicenseFile = serde_json::from_str(&content)?;

        self.activate(license_file)
    }

    pub fn activate(&self, license_file: LicenseFile) -> Result<(), anyhow::Error> {
        let payload_bytes = serde_json::to_vec(&license_file.payload).unwrap_or_default();

        let mut signature_bytes = [0u8; SIGNATURE_LENGTH];
        if BASE64_STANDARD
            .decode_slice(&license_file.signature, &mut signature_bytes)
            .is_err()
        {
            anyhow::bail!("signature length error");
        }
        let signature = Signature::from_bytes(&signature_bytes);

        if self.public_key.verify(&payload_bytes, &signature).is_err() {
            anyhow::bail!("invalid signature");
        }

        *self.current_license.write().unwrap() = Some(license_file.payload.clone());

        Ok(())
    }

    pub fn is_pro(&self) -> bool {
        if let Some(payload) = &(*self.current_license.read().unwrap()) {
            payload.edition == LicenseEdition::Pro
        } else {
            false
        }
    }

    pub fn get_license_info(&self) -> Option<LicenseInformation> {
        self.current_license.read().unwrap().clone()
    }
}
