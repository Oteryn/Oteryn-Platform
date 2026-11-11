//! Public native-device wire types. Secrets have no Debug implementation.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollmentRequest {
    pub protocol_version: u8,
    pub remember_device: bool,
    pub device_public_key: String,
    pub enrollment_proof: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationRequest {
    pub protocol_version: u8,
    pub client_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_proof: Option<RotationProof>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialResponse {
    pub protocol_version: u8,
    pub device_credential: String,
    pub family_id: Uuid,
    pub absolute_expires_at: i64,
    pub idle_expires_at: i64,
    pub rotation_nonce: String,
}
#[derive(Serialize, Deserialize)]
pub struct CharactersResponse {
    #[serde(flatten)]
    pub credential: CredentialResponse,
    pub characters: Vec<Character>,
}
#[derive(Serialize, Deserialize)]
pub struct TicketResponse {
    #[serde(flatten)]
    pub credential: CredentialResponse,
    pub ticket: String,
    pub expires_in: i64,
    pub expires_at: i64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeResponse {
    pub protocol_version: u8,
    pub revoked: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorResponse {
    pub error: String,
}

pub(super) fn decode_operation(body: &[u8], query: bool) -> Result<OperationRequest> {
    if query {
        return Err(Failure::Invalid);
    }
    oteryn_internal_api::canonical::decode(body, 1024, 3).map_err(|_| Failure::Invalid)?;
    let request: OperationRequest = serde_json::from_slice(body).map_err(|_| Failure::Invalid)?;
    let raw: serde_json::Value = serde_json::from_slice(body).map_err(|_| Failure::Invalid)?;
    if request.protocol_version != 1
        || raw["client_id"] != request.client_id.to_string()
        || !(1..=8).contains(&request.client_id.get_version_num())
        || request.client_id.get_variant() != uuid::Variant::RFC4122
        || raw
            .get("rotation_proof")
            .is_some_and(serde_json::Value::is_null)
    {
        return Err(Failure::Invalid);
    }
    if let Some(proof) = &request.rotation_proof {
        key::decode_bytes(&proof.nonce, 32)?;
        key::decode_bytes(&proof.device_public_key, 32)?;
        key::decode_bytes(&proof.signature, 64)?;
        if raw["rotation_proof"]["session_id"] != proof.session_id.to_string() {
            return Err(Failure::Invalid);
        }
    }
    Ok(request)
}
pub(super) fn operation_response(
    value: serde_json::Value,
    purpose: Purpose,
) -> Result<serde_json::Value> {
    match purpose {
        Purpose::Characters => {
            serde_json::from_value::<CharactersResponse>(value).and_then(serde_json::to_value)
        }
        Purpose::Ticket => {
            serde_json::from_value::<TicketResponse>(value).and_then(serde_json::to_value)
        }
    }
    .map_err(|_| Failure::Unavailable)
}

impl std::fmt::Debug for EnrollmentRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EnrollmentRequest([redacted])")
    }
}
impl std::fmt::Debug for OperationRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OperationRequest([redacted])")
    }
}
impl std::fmt::Debug for CredentialResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CredentialResponse([redacted])")
    }
}
impl std::fmt::Debug for CharactersResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CharactersResponse([redacted])")
    }
}
impl std::fmt::Debug for TicketResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TicketResponse([redacted])")
    }
}
impl std::fmt::Debug for RevokeResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RevokeResponse([redacted])")
    }
}
impl std::fmt::Debug for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ErrorResponse([redacted])")
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationMessage {
    pub nonce: String,
    pub purpose: String,
    pub session_id: Uuid,
    pub timestamp: i64,
}
impl RotationMessage {
    pub(super) fn canonical(&self) -> String {
        format!(
            "{{\"nonce\":\"{}\",\"purpose\":\"{}\",\"session_id\":\"{}\",\"timestamp\":{}}}",
            self.nonce, self.purpose, self.session_id, self.timestamp
        )
    }
}
