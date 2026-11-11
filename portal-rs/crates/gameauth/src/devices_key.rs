use super::*;
use ed25519_dalek::{Signature, VerifyingKey};

/// Public enrollment proof. Debug omits the signature and key material.
#[derive(Clone)]
pub struct DeviceKey {
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}
impl std::fmt::Debug for DeviceKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DeviceKey([redacted])")
    }
}
impl DeviceKey {
    pub(super) fn verify(&self, bearer: &str, client: Uuid, remembered: bool) -> Result<()> {
        let bytes: [u8; 32] = self
            .public_key
            .as_slice()
            .try_into()
            .map_err(|_| Failure::Invalid)?;
        let key = VerifyingKey::from_bytes(&bytes).map_err(|_| Failure::Invalid)?;
        let signature = Signature::from_slice(&self.signature).map_err(|_| Failure::Invalid)?;
        let digest: String = hash(bearer).iter().map(|b| format!("{b:02x}")).collect();
        // All values have ASCII alphabets without JSON escaping; order is JCS.
        let message = format!(
            "{{\"access_token_sha256\":\"{digest}\",\"client_id\":\"{client}\",\"persistence\":\"{}\",\"public_key\":\"{}\",\"purpose\":\"oteryn-native-device-enroll-v1\"}}",
            if remembered { "remembered" } else { "session" },
            URL_SAFE_NO_PAD.encode(bytes)
        );
        key.verify_strict(message.as_bytes(), &signature)
            .map_err(|_| Failure::Denied)
    }
}

/// Fresh, single-use possession proof for one device family.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotationProof {
    pub nonce: String,
    pub session_id: Uuid,
    pub timestamp: i64,
    pub device_public_key: String,
    pub signature: String,
}
impl std::fmt::Debug for RotationProof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RotationProof([redacted])")
    }
}
pub(super) fn new_nonce() -> Result<String> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| Failure::Unavailable)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
pub(super) fn decode_bytes(raw: &str, length: usize) -> Result<Vec<u8>> {
    let bytes = URL_SAFE_NO_PAD.decode(raw).map_err(|_| Failure::Invalid)?;
    if bytes.len() != length || URL_SAFE_NO_PAD.encode(&bytes) != raw {
        return Err(Failure::Invalid);
    }
    Ok(bytes)
}
impl RotationProof {
    pub(super) fn check_fresh(&self, family: Uuid, nonce: Option<&str>, now: i64) -> Result<()> {
        if self.session_id != family
            || nonce != Some(self.nonce.as_str())
            || now.abs_diff(self.timestamp) > 60
        {
            return Err(Failure::Denied);
        }
        decode_bytes(&self.nonce, 32)?;
        Ok(())
    }
    pub(super) fn message(&self) -> String {
        wire::RotationMessage {
            nonce: self.nonce.clone(),
            purpose: "oteryn-native-device-rotate-v1".into(),
            session_id: self.session_id,
            timestamp: self.timestamp,
        }
        .canonical()
    }
    pub(super) fn verify(&self, public: &[u8]) -> Result<()> {
        let bytes: [u8; 32] = public.try_into().map_err(|_| Failure::Denied)?;
        let key = VerifyingKey::from_bytes(&bytes).map_err(|_| Failure::Denied)?;
        let signature = decode_bytes(&self.signature, 64)?;
        let signature = Signature::from_slice(&signature).map_err(|_| Failure::Invalid)?;
        key.verify_strict(self.message().as_bytes(), &signature)
            .map_err(|_| Failure::Denied)
    }
}
