//! Platform admission profile, separate from the Game snapshot wire. All bytes
//! are authenticated together with purpose, method/path and configured audience.
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use prost::Message;
use serde::{Deserialize, Serialize};

pub(super) const EVENT: &str = "/internal/v1/game-public/world-event-outcome";
pub(super) const SEASON: &str = "/internal/v1/game-public/guild-season";
pub(super) const WATERMARK: &str = "/internal/v1/game-public/league-watermark";
pub(super) const CUT: &str = "/internal/v1/game-public/league-cut";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Meta {
    pub contract_version: u8,
    pub source_authority: String,
    pub projection_epoch: String,
    pub recovery_generation: String,
    pub key_id: String,
    pub request_id: String,
    pub envelope: String,
    pub finalized_at: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Watermark {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub family: String,
    pub projection_epoch: String,
    pub complete_through: String,
    pub observed_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(super) struct Subject {
    pub family: String,
    pub world_id: String,
    pub subject_id: String,
    pub projection_revision: String,
    pub record_sha256: String,
    pub withdrawn: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Cut {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub projection_epoch: String,
    pub recovery_generation: String,
    pub subjects: Vec<Subject>,
}
pub(super) fn json<T: for<'a> Deserialize<'a> + Serialize>(bytes: &[u8]) -> Result<T> {
    let value = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
    schema::ensure(serde_json::to_vec(&value).map_err(|_| Error::Invalid)? == bytes)?;
    Ok(value)
}
pub(super) fn signable(config: &config::Config, path: &str, meta: &[u8], body: &[u8]) -> Vec<u8> {
    format!(
        "oteryn-platform-league-admission-v1\n{}\nPOST\n{}\n{}\n{}",
        config.audience,
        path,
        STANDARD.encode(meta),
        schema::hash(body)
    )
    .into_bytes()
}
pub(super) fn authenticate(
    config: &config::Config,
    path: &str,
    meta: &[u8],
    body: &[u8],
    signature: &[u8],
) -> Result<Meta> {
    schema::ensure(matches!(path, EVENT | SEASON | WATERMARK | CUT) && meta.len() <= 32768)?;
    let m: Meta = json(meta)?;
    schema::ensure(
        m.contract_version == 1
            && m.source_authority == config.authority
            && m.finalized_at.len() <= 1,
    )?;
    schema::id(&m.request_id)?;
    let key = config.keys.get(&m.key_id).ok_or(Error::Unavailable)?;
    verify_ed25519(key, &signable(config, path, meta, body), signature)?;
    let recovery = config.recovery()?;
    let epoch = schema::number(&m.projection_epoch, false)?;
    let generation = schema::number(&m.recovery_generation, false)?;
    let current_epoch = schema::number(&recovery.projection_epoch, false)?;
    let current_generation = schema::number(&recovery.recovery_generation, false)?;
    if epoch <= current_epoch
        && generation <= current_generation
        && (epoch < current_epoch || generation < current_generation)
    {
        return Err(Error::Superseded);
    }
    if epoch != current_epoch || generation != current_generation {
        return Err(Error::Unavailable);
    }
    Ok(m)
}
#[derive(Clone, PartialEq, Message)]
pub(super) struct TransactionRef {
    #[prost(bytes = "vec", tag = "1")]
    pub transaction_id: Vec<u8>,
    #[prost(uint32, tag = "2")]
    pub ordinal: u32,
    #[prost(uint32, tag = "3")]
    pub count: u32,
}
// The profile accepts the closed, non-personal provenance subset below. Other
// optional ANL fields fail closed instead of being silently discarded by prost.
#[derive(Clone, PartialEq, Message)]
pub(super) struct Envelope {
    #[prost(uint32, tag = "1")]
    pub envelope_revision: u32,
    #[prost(bytes = "vec", tag = "2")]
    pub event_id: Vec<u8>,
    #[prost(uint32, tag = "3")]
    pub event_type_id: u32,
    #[prost(uint32, tag = "4")]
    pub event_schema_revision: u32,
    #[prost(int32, tag = "5")]
    pub durability_class: i32,
    #[prost(int32, tag = "6")]
    pub privacy_class: i32,
    #[prost(string, tag = "7")]
    pub retention_profile_id: String,
    #[prost(int64, tag = "8")]
    pub occurred_at_unix_ms: i64,
    #[prost(bytes = "vec", optional, tag = "9")]
    pub world_id: Option<Vec<u8>>,
    #[prost(message, optional, tag = "18")]
    pub transaction_event: Option<TransactionRef>,
    #[prost(string, tag = "25")]
    pub server_build_id: String,
    #[prost(bytes = "vec", tag = "26")]
    pub payload: Vec<u8>,
    #[prost(bytes = "vec", tag = "27")]
    pub payload_sha256: Vec<u8>,
}
#[derive(Debug)]
pub(super) struct Decision {
    pub envelope: Envelope,
    pub finalized: Option<u64>,
    pub generation: u64,
}
impl Decision {
    pub fn decode(m: &Meta, s: &schema::Snapshot, ceiling: usize) -> Result<Self> {
        use sha2::{Digest, Sha256};
        let raw = STANDARD.decode(&m.envelope).map_err(|_| Error::Invalid)?;
        // Conservative admission ceiling; never raises the Game ANL ceiling.
        schema::ensure(
            raw.len() <= ceiling
                && s.authority() == m.source_authority
                && s.epoch() == m.projection_epoch,
        )?;
        let e = Envelope::decode(raw.as_slice()).map_err(|_| Error::Invalid)?;
        schema::ensure(
            e.encode_to_vec() == raw
                && e.envelope_revision == 1
                && e.event_schema_revision == 1
                && e.occurred_at_unix_ms > 0
                && e.event_type_id
                    == if matches!(s, schema::Snapshot::Event(_)) {
                        10
                    } else {
                        11
                    }
                && e.durability_class == 2
                && e.privacy_class == 3
                && e.retention_profile_id == "PLAT3_AUTHORITY_AUDIT_RETENTION_V1"
                && !e.server_build_id.is_empty()
                && e.server_build_id.len() <= 128
                && e.payload_sha256 == Sha256::digest(&e.payload).to_vec(),
        )?;
        let event = Uuid::from_slice(&e.event_id).map_err(|_| Error::Invalid)?;
        schema::ensure(
            event.to_string() == m.request_id
                && e.world_id.as_deref() == Some(schema::id(s.world())?.as_bytes()),
        )?;
        let t = e.transaction_event.as_ref().ok_or(Error::Invalid)?;
        schema::id(
            &Uuid::from_slice(&t.transaction_id)
                .map_err(|_| Error::Invalid)?
                .to_string(),
        )?;
        schema::ensure(t.ordinal > 0 && t.ordinal <= t.count && t.count <= 1024)?;
        if let schema::Snapshot::Event(s) = s {
            for r in &s.event {
                schema::ensure(r.occurrence_id != m.request_id)?;
            }
        }
        let d = Self {
            envelope: e,
            finalized: m
                .finalized_at
                .first()
                .map(|s| schema::number(s, false))
                .transpose()?,
            generation: schema::number(&m.recovery_generation, false)?,
        };
        store::check_payload(&d.delivery(&[], &[], s)?, s)?;
        Ok(d)
    }
    pub fn delivery<'a>(
        &'a self,
        wire: &'a [u8],
        signature: &'a [u8],
        s: &schema::Snapshot,
    ) -> Result<Delivery<'a>> {
        let e = &self.envelope;
        let t = e.transaction_event.as_ref().ok_or(Error::Invalid)?;
        if s.final_state() {
            schema::ensure(
                self.finalized
                    .is_some_and(|f| f <= e.occurred_at_unix_ms as u64 / 1000),
            )?;
        }
        Ok(Delivery {
            envelope_id: Uuid::from_slice(&e.event_id).map_err(|_| Error::Invalid)?,
            transaction_id: Uuid::from_slice(&t.transaction_id).map_err(|_| Error::Invalid)?,
            ordinal: t.ordinal,
            count: t.count,
            event_type: e.event_type_id,
            occurred_at_unix_ms: e.occurred_at_unix_ms,
            payload: &e.payload,
            wire,
            signature,
            recovery_generation: self.generation,
            finalized_at: self.finalized,
        })
    }
}
