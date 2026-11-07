//! Canonical Game JSON authenticated solely by the dedicated verified mTLS peer.
//! Meta and Decision are local durable adapter records, never Game HTTP headers
//! or native ANL envelopes. Game retains its own native audit provenance.
use super::*;
use prost::Message;
use serde::{Deserialize, Serialize};

pub(super) const EVENT: &str = "/internal/v1/game-public/world-event-outcome";
pub(super) const SEASON: &str = "/internal/v1/game-public/guild-season";
pub(super) const WATERMARK: &str = "/internal/v1/game-public/league-watermark";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Meta {
    pub source_authority: String,
    pub projection_epoch: String,
    pub recovery_generation: String,
    pub request_id: String,
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
pub(super) fn json<T: for<'a> Deserialize<'a> + Serialize>(bytes: &[u8]) -> Result<T> {
    let value = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
    schema::ensure(serde_json::to_vec(&value).map_err(|_| Error::Invalid)? == bytes)?;
    Ok(value)
}
/// Stable local receipt identity for retained retries; not a Game EventId.
fn receipt(path: &str, body: &[u8], generation: &str) -> Uuid {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(path);
    hash.update([0]);
    hash.update(generation);
    hash.update([0]);
    hash.update(body);
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hash.finalize()[..16]);
    bytes[6] = (bytes[6] & 15) | 0x70;
    bytes[8] = (bytes[8] & 63) | 0x80;
    Uuid::from_bytes(bytes)
}
pub(super) fn prepare(c: &config::Config, path: &str, body: &[u8], peer: &str) -> Result<Meta> {
    schema::ensure(peer == c.client_spki_sha256 && c.authority == "oteryn:game:primary")?;
    let (authority, epoch) = match path {
        EVENT | SEASON => {
            let s = schema::Snapshot::decode(body, path == SEASON, runtime::now()?)?;
            (s.authority().to_owned(), s.epoch().to_owned())
        }
        WATERMARK => {
            let w: Watermark = json(body)?;
            (w.source_authority, w.projection_epoch)
        }
        _ => return Err(Error::Invalid),
    };
    schema::ensure(authority == c.authority)?;
    let recovery = c.recovery()?;
    if epoch != recovery.projection_epoch {
        return Err(Error::Conflict);
    }
    Ok(Meta {
        source_authority: authority,
        projection_epoch: epoch,
        request_id: receipt(path, body, &recovery.recovery_generation).to_string(),
        recovery_generation: recovery.recovery_generation,
    })
}
/// Recheck the persisted mTLS admission record against current operator trust.
/// Historical SQL columns named signature contain the SPKI identity, not a signature.
pub(super) fn authenticate(
    c: &config::Config,
    path: &str,
    meta: &[u8],
    body: &[u8],
    identity: &[u8],
) -> Result<Meta> {
    let peer = std::str::from_utf8(identity).map_err(|_| Error::Invalid)?;
    let expected = prepare(c, path, body, peer)?;
    schema::ensure(json::<Meta>(meta)? == expected)?;
    Ok(expected)
}
#[derive(Debug)]
pub(super) struct Decision {
    id: Uuid,
    payload: Vec<u8>,
    observed: u64,
    generation: u64,
}
impl Decision {
    pub fn decode(m: &Meta, s: &schema::Snapshot) -> Result<Self> {
        let generation = schema::number(&m.recovery_generation, false)?;
        let record = s.record()?;
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(&record).to_vec();
        let payload = match s {
            schema::Snapshot::Event(e) => {
                let r = e.event.first().ok_or(Error::Invalid)?;
                WorldEventOutcomeDecidedV1 {
                    record,
                    record_sha256: digest,
                    source_evidence_sha256: config::unhex(
                        r.evidence_sha256.first().ok_or(Error::Invalid)?,
                    )?,
                    recovery_generation: generation,
                    subject_event_id: schema::id(s.subject())?.as_bytes().to_vec(),
                }
                .encode_to_vec()
            }
            schema::Snapshot::Season(_) => GuildSeasonStandingsDecidedV1 {
                record_sha256: digest,
                recovery_generation: generation,
                season_id: schema::id(s.subject())?.as_bytes().to_vec(),
            }
            .encode_to_vec(),
        };
        Ok(Self {
            id: schema::id(&m.request_id)?,
            payload,
            observed: s.observed()?,
            generation,
        })
    }
    pub fn delivery<'a>(
        &'a self,
        wire: &'a [u8],
        identity: &'a [u8],
        s: &schema::Snapshot,
    ) -> Result<Delivery<'a>> {
        Ok(Delivery {
            envelope_id: self.id,
            transaction_id: self.id,
            ordinal: 1,
            count: 1,
            event_type: if matches!(s, schema::Snapshot::Event(_)) {
                10
            } else {
                11
            },
            occurred_at_unix_ms: i64::try_from(
                self.observed.checked_mul(1000).ok_or(Error::Invalid)?,
            )
            .map_err(|_| Error::Invalid)?,
            payload: &self.payload,
            wire,
            signature: identity,
            recovery_generation: self.generation,
            finalized_at: s.final_state().then_some(self.observed),
        })
    }
}
