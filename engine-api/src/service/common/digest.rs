use hex::encode as hex_encode;
use serde::Serialize;
use sha2::{Digest, Sha256};

pub fn generate_digest<T: Serialize>(value: &T) -> Result<String, String> {
    let serialized =
        serde_json::to_vec(value).map_err(|err| err.to_string())?;
    Ok(hex_encode(Sha256::digest(serialized)))
}
