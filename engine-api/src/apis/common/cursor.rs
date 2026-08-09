use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;

use super::APIError;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const CURSOR_VERSION: u8 = 1;

#[derive(Serialize, Deserialize)]
struct CursorEnvelope<C> {
    version: u8,
    scope: String,
    tenant_id: Uuid,
    cursor: C,
}

pub struct CursorSigner {
    key: Vec<u8>,
}

impl CursorSigner {
    pub fn new(key: Vec<u8>) -> Self {
        return Self { key };
    }

    pub fn encode<C: Serialize>(
        &self,
        scope: &str,
        tenant_id: Uuid,
        cursor: Option<C>,
    ) -> Result<Option<String>, APIError> {
        let Some(cursor) = cursor else {
            return Ok(None);
        };

        let envelope = CursorEnvelope {
            version: CURSOR_VERSION,
            scope: scope.to_owned(),
            tenant_id: tenant_id,
            cursor: cursor,
        };

        let payload = serde_json::to_vec(&envelope)
            .map_err(|err| APIError::Internal(err.to_string()))?;

        let mut mac = HmacSha256::new_from_slice(&self.key)
            .map_err(|err| APIError::Internal(err.to_string()))?;

        mac.update(&payload);

        let signature = mac.finalize().into_bytes();

        Ok(Some(format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(payload),
            URL_SAFE_NO_PAD.encode(signature)
        )))
    }

    pub fn decode<C: DeserializeOwned>(
        &self,
        token: Option<&str>,
        expected_scope: &str,
        expected_tenant_id: Uuid,
    ) -> Result<Option<C>, APIError> {
        let Some(token) = token else {
            return Ok(None);
        };

        let (payload, signature) = token
            .split_once(".")
            .ok_or_else(|| APIError::BadRequest("invalid token".to_owned()))?;

        let payload = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|err| APIError::BadRequest(err.to_string()))?;

        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|err| APIError::BadRequest(err.to_string()))?;

        let mut mac = HmacSha256::new_from_slice(&self.key)
            .map_err(|err| APIError::Internal(err.to_string()))?;

        mac.update(&payload);

        mac.verify_slice(&signature)
            .map_err(|_| APIError::BadRequest("invalid token".to_owned()))?;

        let envelope: CursorEnvelope<C> = serde_json::from_slice(&payload)
            .map_err(|err| APIError::BadRequest(err.to_string()))?;

        if envelope.version != CURSOR_VERSION
            || envelope.scope != expected_scope
            || envelope.tenant_id != expected_tenant_id
        {
            return Err(APIError::BadRequest(
                "cursor is not valid for this request".to_owned(),
            ));
        }

        Ok(Some(envelope.cursor))
    }
}
