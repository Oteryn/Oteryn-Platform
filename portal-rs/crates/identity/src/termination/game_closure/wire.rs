use super::*;
use serde::{Deserialize, Serialize};

/// Exact pinned protobuf payload, event type 7/schema 1. Envelope ownership stays
/// with the foundation publisher. Decode is followed by validation, never authority.
#[derive(Clone, PartialEq, Message)]
pub struct AccountClosureDecisionV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub intent_sha256: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub receipt: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub receipt_sha256: Vec<u8>,
    #[prost(uint64, tag = "4")]
    pub recovery_generation: u64,
}
impl AccountClosureDecisionV1 {
    pub fn validate(&self, intent: &[u8]) -> Result<Receipt> {
        if self.intent_sha256.len() != 32
            || self.receipt_sha256.len() != 32
            || self.intent_sha256 != digest(intent)
            || self.receipt_sha256 != digest(&self.receipt)
            || self.recovery_generation == 0
        {
            return Err(Error::InvalidInput);
        }
        let receipt = Receipt::parse(&self.receipt)?;
        receipt.bind(&Intent::parse(intent)?, self.recovery_generation)?;
        Ok(receipt)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    pub contract_version: u8,
    pub command: String,
    pub variant: String,
    pub issuer_authority: String,
    pub issuer_decision_id: String,
    pub source_revision: String,
    pub operation_id: String,
    pub account_id: String,
    pub termination_id: String,
    pub lifecycle_generation: String,
    pub expected_recovery_generation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    pub scheduled_for: String,
    pub issued_at_source: String,
    pub expires_at_source: String,
    pub audience: String,
}
impl Intent {
    pub fn parse(raw: &[u8]) -> Result<Self> {
        let value: Self = canonical(raw, 4096)?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        if self.contract_version != 2
            || self.issuer_authority != "OTERYN_PLATFORM_CHARACTER_AUTHORITY"
            || self.audience != "OTERYN_GAME_CHARACTER_AUTHORITY"
        {
            return Err(Error::InvalidInput);
        }
        for v in [
            &self.issuer_decision_id,
            &self.operation_id,
            &self.account_id,
            &self.termination_id,
        ] {
            id(v)?;
        }
        for v in [
            &self.source_revision,
            &self.lifecycle_generation,
            &self.expected_recovery_generation,
        ] {
            number(v, true)?;
        }
        let issued = number(&self.issued_at_source, true)?;
        let expires = number(&self.expires_at_source, true)?;
        let scheduled = number(&self.scheduled_for, true)?;
        if expires <= issued || expires - issued > 300 {
            return Err(Error::InvalidInput);
        }
        match (
            self.command.as_str(),
            self.variant.as_str(),
            self.close_operation_id.as_deref(),
            self.decision.as_deref(),
        ) {
            ("CloseAccount", "PLATFORM_ACCOUNT_CLOSE", None, None) => {
                if issued < scheduled {
                    return Err(Error::InvalidInput);
                }
            }
            (
                "ResolveAccountClosure",
                "PLATFORM_ACCOUNT_CLOSE_RESOLUTION",
                Some(close),
                Some(decision @ ("CANCEL" | "FINALIZE")),
            ) => {
                id(close)?;
                if close == self.operation_id || (decision == "FINALIZE" && issued < scheduled) {
                    return Err(Error::InvalidInput);
                }
            }
            _ => return Err(Error::InvalidInput),
        }
        Ok(())
    }
    pub fn bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|_| Error::InvalidInput)
    }
    pub(super) fn new(
        t: &Termination,
        operation: Uuid,
        generation: i64,
        recovery: u64,
        now: u64,
        close: Option<Uuid>,
        decision: Option<&str>,
    ) -> Result<Self> {
        let resolving = decision.is_some();
        let value = Self {
            contract_version: 2,
            command: if resolving {
                "ResolveAccountClosure"
            } else {
                "CloseAccount"
            }
            .into(),
            variant: if resolving {
                "PLATFORM_ACCOUNT_CLOSE_RESOLUTION"
            } else {
                "PLATFORM_ACCOUNT_CLOSE"
            }
            .into(),
            issuer_authority: "OTERYN_PLATFORM_CHARACTER_AUTHORITY".into(),
            issuer_decision_id: t.id.to_string(),
            source_revision: generation.to_string(),
            operation_id: operation.to_string(),
            account_id: t.account_id.to_string(),
            termination_id: t.id.to_string(),
            lifecycle_generation: generation.to_string(),
            expected_recovery_generation: recovery.to_string(),
            close_operation_id: if resolving {
                close.map(|v| v.to_string())
            } else {
                None
            },
            decision: decision.map(str::to_owned),
            scheduled_for: t.scheduled_for.unix_timestamp().to_string(),
            issued_at_source: now.to_string(),
            expires_at_source: now.checked_add(300).ok_or(Error::UnknownState)?.to_string(),
            audience: "OTERYN_GAME_CHARACTER_AUTHORITY".into(),
        };
        value.validate()?;
        Ok(value)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub command: String,
    pub operation_id: String,
    pub outcome: String,
    pub result_code: String,
    pub result_data: Option<ClosureResult>,
    pub recovery_generation: String,
    pub decided_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClosureResult {
    pub account_id: String,
    pub termination_id: String,
    pub lifecycle_generation: String,
    pub closure_state: String,
    pub detached_characters: String,
    pub manifest_sha256: String,
    pub projection_epoch: String,
    pub projection_revision: String,
    pub proof_sha256: String,
}
impl Receipt {
    pub fn parse(raw: &[u8]) -> Result<Self> {
        let value: Self = canonical(raw, 2048)?;
        if value.contract_version != 2
            || value.operation != "PublishCharacterCommandReceiptV1"
            || !matches!(
                value.command.as_str(),
                "CloseAccount" | "ResolveAccountClosure"
            )
        {
            return Err(Error::InvalidInput);
        }
        id(&value.operation_id)?;
        number(&value.recovery_generation, true)?;
        time(&value.decided_at)?;
        match (value.outcome.as_str(), value.result_data.as_ref()) {
            ("COMMITTED", Some(result)) if value.result_code == "CHAR_CMD_OK" => {
                id(&result.account_id)?;
                id(&result.termination_id)?;
                for v in [
                    &result.lifecycle_generation,
                    &result.projection_epoch,
                    &result.projection_revision,
                ] {
                    number(v, true)?;
                }
                if number(&result.detached_characters, false)? > 64
                    || !matches!(
                        (value.command.as_str(), result.closure_state.as_str()),
                        ("CloseAccount", "CLOSED_DETACHED")
                            | ("ResolveAccountClosure", "FINALIZED" | "CANCELLED")
                    )
                {
                    return Err(Error::InvalidInput);
                }
                hash(&result.manifest_sha256)?;
                hash(&result.proof_sha256)?;
                let bytes = serde_json::to_vec(result).map_err(|_| Error::InvalidInput)?;
                let marker = b",\"proof_sha256\":";
                let end = bytes
                    .windows(marker.len())
                    .position(|w| w == marker)
                    .ok_or(Error::InvalidInput)?;
                let mut body = bytes[..end].to_vec();
                body.push(b'}');
                if hex(&digest(&body)) != result.proof_sha256 {
                    return Err(Error::InvalidInput);
                }
            }
            ("REJECTED", None)
                if matches!(
                    value.result_code.as_str(),
                    "CHAR_CMD_INTENT_INVALID"
                        | "CHAR_CMD_INTENT_EXPIRED"
                        | "CHAR_CMD_UNSUPPORTED"
                        | "CHAR_CMD_OPERATION_CONFLICT"
                        | "CHAR_CMD_ACCOUNT_CLOSE_STALE_GENERATION"
                        | "CHAR_CMD_ACCOUNT_CLOSE_OBLIGATION"
                        | "CHAR_CMD_ACCOUNT_CLOSE_RESOLUTION_CONFLICT"
                ) => {}
            _ => return Err(Error::InvalidInput),
        }
        Ok(value)
    }
    pub fn bind(&self, intent: &Intent, current_recovery: u64) -> Result<()> {
        if self.command != intent.command
            || self.operation_id != intent.operation_id
            || number(&self.recovery_generation, true)? != current_recovery
            || (self.outcome == "COMMITTED"
                && number(&intent.expected_recovery_generation, true)? != current_recovery)
        {
            return Err(Error::UnknownState);
        }
        let decided = number(&self.decided_at, true)?;
        let issued = number(&intent.issued_at_source, true)?;
        if decided.saturating_add(5) < issued {
            return Err(Error::UnknownState);
        }
        if let Some(result) = &self.result_data
            && (result.account_id != intent.account_id
                || result.termination_id != intent.termination_id
                || result.lifecycle_generation != intent.lifecycle_generation
                || decided > number(&intent.expires_at_source, true)?
                || (intent.command == "ResolveAccountClosure"
                    && result.closure_state
                        != if intent.decision.as_deref() == Some("CANCEL") {
                            "CANCELLED"
                        } else {
                            "FINALIZED"
                        }))
        {
            return Err(Error::UnknownState);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClosureRecord {
    pub termination_id: String,
    pub operation_id: String,
    pub lifecycle_generation: String,
    pub closure_state: String,
    pub receipt_digest: String,
    pub proof_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub account_id: String,
    pub projection_epoch: String,
    pub projection_revision: String,
    pub source_observed_at: String,
    pub closures: Vec<ClosureRecord>,
}
impl Projection {
    pub fn parse(raw: &[u8]) -> Result<Self> {
        let value: Self = canonical(raw, 2048)?;
        if value.contract_version != 2
            || value.operation != "PublishAccountClosureV1"
            || value.closures.len() > 1
        {
            return Err(Error::InvalidInput);
        }
        id(&value.account_id)?;
        number(&value.projection_epoch, true)?;
        number(&value.projection_revision, true)?;
        time(&value.source_observed_at)?;
        for record in &value.closures {
            id(&record.termination_id)?;
            id(&record.operation_id)?;
            number(&record.lifecycle_generation, true)?;
            hash(&record.receipt_digest)?;
            hash(&record.proof_sha256)?;
            if !matches!(
                record.closure_state.as_str(),
                "CLOSED_DETACHED" | "FINALIZED" | "CANCELLED"
            ) {
                return Err(Error::InvalidInput);
            }
        }
        Ok(value)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Watermark {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub family: String,
    pub projection_epoch: String,
    pub complete_through: String,
    pub observed_at: String,
}
impl Watermark {
    pub fn parse(raw: &[u8]) -> Result<Self> {
        let value: Self = canonical(raw, 512)?;
        if value.contract_version != 2
            || value.operation != "PublishPublicProjectionWatermarkV1"
            || value.family != "account_closure"
            || time(&value.observed_at)? < time(&value.complete_through)?
        {
            return Err(Error::InvalidInput);
        }
        number(&value.projection_epoch, true)?;
        Ok(value)
    }
    pub fn fresh(&self, now: u64) -> Result<bool> {
        let complete = number(&self.complete_through, false)?;
        let observed = number(&self.observed_at, false)?;
        Ok(complete <= now.saturating_add(1)
            && observed <= now.saturating_add(1)
            && now.saturating_add(1).saturating_sub(complete) <= 30)
    }
}
fn canonical<T: serde::de::DeserializeOwned + Serialize>(raw: &[u8], max: usize) -> Result<T> {
    if raw.is_empty() || raw.len() > max || raw.contains(&b'\\') {
        return Err(Error::InvalidInput);
    }
    let value: T = serde_json::from_slice(raw).map_err(|_| Error::InvalidInput)?;
    if serde_json::to_vec(&value).map_err(|_| Error::InvalidInput)? != raw {
        return Err(Error::InvalidInput);
    }
    Ok(value)
}
pub(super) fn id(raw: &str) -> Result<Uuid> {
    let id = Uuid::parse_str(raw).map_err(|_| Error::InvalidInput)?;
    if id.get_version_num() != 7
        || id.get_variant() != uuid::Variant::RFC4122
        || id.to_string() != raw
    {
        return Err(Error::InvalidInput);
    }
    Ok(id)
}
pub(super) fn number(raw: &str, positive: bool) -> Result<u64> {
    let value = raw.parse::<u64>().map_err(|_| Error::InvalidInput)?;
    if value.to_string() != raw || (positive && value == 0) {
        return Err(Error::InvalidInput);
    }
    Ok(value)
}
pub(super) fn time(raw: &str) -> Result<i64> {
    i64::try_from(number(raw, false)?).map_err(|_| Error::InvalidInput)
}
fn hash(raw: &str) -> Result<()> {
    if raw.len() != 64
        || !raw
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
pub(super) fn hex(raw: &[u8]) -> String {
    raw.iter().map(|v| format!("{v:02x}")).collect()
}
