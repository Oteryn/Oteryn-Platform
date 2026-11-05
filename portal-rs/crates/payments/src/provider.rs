use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

use crate::{Amount, Error, IntentState, core::identifier};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentEvent {
    pub event_id: String,
    pub intent_id: Uuid,
    pub provider_reference: String,
    pub amount: Amount,
    pub state: IntentState,
}

impl PaymentEvent {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        identifier(&self.event_id)?;
        identifier(&self.provider_reference)?;
        self.amount.validate()?;
        if self.intent_id.is_nil() || self.state == IntentState::Created {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}

/// Provider implementations must use intent_id as the remote idempotency key for creation
/// and a stable refund key for refunds. Signature verification precedes parsing.
#[allow(async_fn_in_trait)]
pub trait PaymentProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn create_intent(&self, intent_id: Uuid, amount: &Amount) -> Result<String, Error>;
    fn verify_webhook_signature(&self, body: &[u8], signature: &[u8]) -> Result<(), Error>;
    fn parse_event(&self, body: &[u8]) -> Result<PaymentEvent, Error>;
    async fn refund(&self, intent_id: Uuid, amount: &Amount) -> Result<String, Error>;
}

/// Local deterministic provider only. Debug output never includes signing material.
#[derive(Clone)]
pub struct TestProvider {
    secret: Vec<u8>,
}

impl std::fmt::Debug for TestProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestProvider").finish_non_exhaustive()
    }
}

impl TestProvider {
    pub fn new(secret: &[u8]) -> Result<Self, Error> {
        if secret.len() < 32 {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            secret: secret.to_vec(),
        })
    }

    fn mac(&self) -> Result<Hmac<Sha256>, Error> {
        Hmac::<Sha256>::new_from_slice(&self.secret).map_err(|_| Error::InvalidInput)
    }

    /// Raw 32-byte HMAC-SHA256 over the exact request body.
    pub fn sign(&self, body: &[u8]) -> Result<Vec<u8>, Error> {
        let mut mac = self.mac()?;
        mac.update(body);
        Ok(mac.finalize().into_bytes().to_vec())
    }
}

impl PaymentProvider for TestProvider {
    fn name(&self) -> &'static str {
        "test"
    }

    async fn create_intent(&self, intent_id: Uuid, amount: &Amount) -> Result<String, Error> {
        amount.validate()?;
        if intent_id.is_nil() {
            return Err(Error::InvalidInput);
        }
        Ok(format!("test:{intent_id}"))
    }

    fn verify_webhook_signature(&self, body: &[u8], signature: &[u8]) -> Result<(), Error> {
        let mut mac = self.mac()?;
        mac.update(body);
        mac.verify_slice(signature).map_err(|_| Error::BadSignature)
    }

    fn parse_event(&self, body: &[u8]) -> Result<PaymentEvent, Error> {
        let event: PaymentEvent = serde_json::from_slice(body).map_err(|_| Error::InvalidInput)?;
        event.validate()?;
        Ok(event)
    }

    async fn refund(&self, intent_id: Uuid, amount: &Amount) -> Result<String, Error> {
        self.create_intent(intent_id, amount).await?;
        Ok(format!("refund:{intent_id}"))
    }
}
