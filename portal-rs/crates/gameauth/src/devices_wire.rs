//! Public native-device wire types. Secrets have no Debug implementation.
use super::*;
use serde::{Deserialize, Serialize};

/// Protocol 1 enrollment: the original unbound two-member remembered request,
/// or the four-member bound extension (accepted contract, HTTP protocol 1).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollmentRequest {
    pub protocol_version: u8,
    pub remember_device: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enrollment_proof: Option<String>,
}
/// Rotation and revocation carry exactly these two members and no proof.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationRequest {
    pub protocol_version: u8,
    pub client_id: Uuid,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialResponse {
    pub protocol_version: u8,
    pub device_credential: String,
    pub family_id: Uuid,
    pub absolute_expires_at: i64,
    pub idle_expires_at: i64,
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

/// No valid device body contains a JSON escape: the Game client sends only
/// normalized members and values, so any backslash is refused before parsing.
pub(super) fn reject_escapes(body: &[u8]) -> Result<()> {
    if body.contains(&b'\\') {
        return Err(Failure::Invalid);
    }
    Ok(())
}

pub(super) fn decode_operation(body: &[u8], query: bool) -> Result<OperationRequest> {
    if query {
        return Err(Failure::Invalid);
    }
    reject_escapes(body)?;
    oteryn_internal_api::canonical::decode(body, 1024, 2).map_err(|_| Failure::Invalid)?;
    let request: OperationRequest = serde_json::from_slice(body).map_err(|_| Failure::Invalid)?;
    let raw: serde_json::Value = serde_json::from_slice(body).map_err(|_| Failure::Invalid)?;
    if request.protocol_version != 1
        || raw["client_id"] != request.client_id.to_string()
        || !(1..=8).contains(&request.client_id.get_version_num())
        || request.client_id.get_variant() != uuid::Variant::RFC4122
    {
        return Err(Failure::Invalid);
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
