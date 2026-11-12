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
