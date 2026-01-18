use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};

#[cfg(feature = "apiserver")]
use rand::{RngCore, rng};

#[cfg(feature = "apiserver")]
pub fn generate_salt() -> String {
    let mut salt = [0u8; 32];
    rng().fill_bytes(&mut salt);
    STANDARD.encode(salt)
}

pub fn generate_secret(password: &String, salt: &String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password);
    hasher.update(salt);

    STANDARD.encode(hasher.finalize())
}

pub fn generate_authentication_string(secret: &String, challenge: &String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret);
    hasher.update(challenge);

    STANDARD.encode(hasher.finalize())
}

#[cfg(feature = "apiserver")]
pub fn check_authentication_string(
    secret: &String,
    challenge: &String,
    authentication_string: &String,
) -> bool {
    generate_authentication_string(secret, challenge) == *authentication_string
}
