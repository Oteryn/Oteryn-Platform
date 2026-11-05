//! Owner-only full projections. Ingestion never certifies custody, delivery or
//! payment. Higher epochs fence old data; watermarks never complete a command.
use super::{
    Error, Outbox,
    game_wire::{decimal, digest, hash, id, ordered, text},
};
use oteryn_internal_api::canonical::{self, Value};
use sqlx::Row;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestResult {
    Applied,
    Duplicate,
    Superseded,
    Quarantined,
}
fn uint(v: &Value, k: &str) -> Result<u64, Error> {
    decimal(text(v, k)?, false, u64::MAX)
}
fn key(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes().enumerate().all(|(i, b)| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || (i > 0 && b"._:/-".contains(&b))
        })
}
fn attributes(v: &Value) -> Result<(), Error> {
    let Value::Array(values) = v else {
        return Err(Error::Invalid);
    };
    if values.len() > 6 {
        return Err(Error::Invalid);
    }
    let mut previous = "";
    for v in values {
        let Value::String(s) = v else {
            return Err(Error::Invalid);
        };
        if s.as_str() <= previous
            || !matches!(
                s.as_str(),
                "tier" | "upgrade_level" | "charges" | "attack" | "defense" | "armor"
            )
        {
            return Err(Error::Invalid);
        }
        previous = s;
    }
    Ok(())
}
fn policy(v: &Value) -> Result<(), Error> {
    id(text(v, "world_id")?)?;
    uint(v, "eligibility_revision")?;
    let Some(Value::Array(definitions)) = v.get("definitions") else {
        return Err(Error::Invalid);
    };
    if definitions.len() > 1024 {
        return Err(Error::Invalid);
    }
    let mut previous: Option<(&str, &str, &str)> = None;
    for entry in definitions {
        ordered(
            entry,
            &[
                "definition",
                "tradeable",
                "max_quantity",
                "allowed_attribute_keys",
            ],
        )?;
        let definition = entry.get("definition").ok_or(Error::Invalid)?;
        ordered(definition, &["family", "production_key", "revision_ref"])?;
        let tuple = (
            text(definition, "family")?,
            text(definition, "production_key")?,
            text(definition, "revision_ref")?,
        );
        if ![tuple.0, tuple.1, tuple.2].into_iter().all(key)
            || previous.is_some_and(|old| old >= tuple)
        {
            return Err(Error::Invalid);
        }
        previous = Some(tuple);
        if !matches!(entry.get("tradeable"), Some(Value::Bool(_)))
            || !matches!(
                entry.get("max_quantity"),
                Some(Value::Unsigned(1..=4_294_967_295))
            )
        {
            return Err(Error::Invalid);
        }
        attributes(entry.get("allowed_attribute_keys").ok_or(Error::Invalid)?)?;
    }
    Ok(())
}
fn status(v: &Value) -> Result<(), Error> {
    for k in [
        "listing_id",
        "seller_account_id",
        "world_id",
        "asset_id",
        "custody_id",
    ] {
        id(text(v, k)?)?;
    }
    uint(v, "custody_revision")?;
    let state = text(v, "state")?;
    if !matches!(
        state,
        "locked" | "ESCROWED" | "DELIVERED" | "RETURNED" | "UNLOCKED" | "INVALID"
    ) {
        return Err(Error::Invalid);
    }
    match text(v, "asset_kind")? {
        "CHARACTER" if state != "ESCROWED" && hash(text(v, "snapshot_hash")?) => {}
        "ITEM"
            if !matches!(state, "locked" | "UNLOCKED")
                && v.get("snapshot_hash") == Some(&Value::Null) => {}
        _ => return Err(Error::Invalid),
    }
    let terminal = matches!(state, "DELIVERED" | "RETURNED" | "UNLOCKED");
    if v.get("last_operation_id") != Some(&Value::Null) {
        id(text(v, "last_operation_id")?)?;
    } else if terminal {
        return Err(Error::Invalid);
    }
    if state == "INVALID" {
        if !matches!(
            text(v, "reason_code")?,
            "POLICY_CHANGED" | "STATE_CHANGED" | "RESTORE_RECONCILING" | "AUTHORITY_UNAVAILABLE"
        ) {
            return Err(Error::Invalid);
        }
    } else if v.get("reason_code") != Some(&Value::Null) {
        return Err(Error::Invalid);
    }
    Ok(())
}
impl Outbox {
    /// `authority` is the configured peer namespace supplied after purpose-bound
    /// TLS authentication, never taken from an HTTP header or request member.
    /// Commit Quarantined to retain the digest alarm; do not log the private body.
    pub async fn ingest(
        &self,
        db: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        authority: &str,
        raw: &[u8],
        now: u64,
    ) -> Result<IngestResult, Error> {
        if !self.settings.preparation_enabled {
            return Err(Error::Unavailable);
        }
        if authority.is_empty()
            || authority.len() > 128
            || !authority
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
        {
            return Err(Error::Denied);
        }
        let v = canonical::decode(raw, 262144, 12).map_err(|_| Error::Invalid)?;
        if raw.contains(&b'\\') || canonical::encode(&v).map_err(|_| Error::Invalid)? != raw {
            return Err(Error::Invalid);
        }
        if v.get("contract_version") != Some(&Value::Unsigned(2)) {
            return Err(Error::Invalid);
        }
        if text(&v, "source_authority")? != authority {
            return Err(Error::Denied);
        }
        let (kind, subject, fields, max) = match text(&v, "operation")? {
            "PublishBazaarTransferabilityV1" => (
                "POLICY",
                id(text(&v, "world_id")?)?,
                vec![
                    "world_id",
                    "projection_epoch",
                    "projection_revision",
                    "source_observed_at",
                    "eligibility_revision",
                    "definitions",
                ],
                262144,
            ),
            "PublishBazaarListingStatusV1" => (
                "STATUS",
                id(text(&v, "listing_id")?)?,
                vec![
                    "listing_id",
                    "seller_account_id",
                    "world_id",
                    "projection_epoch",
                    "projection_revision",
                    "source_observed_at",
                    "asset_kind",
                    "asset_id",
                    "state",
                    "custody_id",
                    "custody_revision",
                    "snapshot_hash",
                    "last_operation_id",
                    "reason_code",
                ],
                2048,
            ),
            "PublishBazaarWatermarkV1" => (
                "WATERMARK",
                Uuid::nil(),
                vec!["projection_epoch", "complete_through", "observed_at"],
                512,
            ),
            _ => return Err(Error::Invalid),
        };
        if raw.len() > max {
            return Err(Error::Invalid);
        }
        let mut all = vec!["contract_version", "operation", "source_authority"];
        all.extend_from_slice(&fields);
        ordered(&v, &all)?;
        let epoch = uint(&v, "projection_epoch")?;
        let watermark = kind == "WATERMARK";
        let revision = uint(
            &v,
            if watermark {
                "complete_through"
            } else {
                "projection_revision"
            },
        )?;
        let observed = uint(
            &v,
            if watermark {
                "observed_at"
            } else {
                "source_observed_at"
            },
        )?;
        if observed > now.saturating_add(1) || (watermark && revision > observed) {
            return Err(Error::Invalid);
        }
        match kind {
            "POLICY" => policy(&v)?,
            "STATUS" => status(&v)?,
            _ => {}
        }
        // The issuer lock is shared with enqueue and restore, so no lower epoch
        // write can race the invalidation of older evidence.
        let floor=sqlx::query("SELECT projection_epoch::text AS epoch,recovery_generation::text AS generation FROM bazaar_game_issuer WHERE singleton FOR UPDATE").fetch_one(&mut **db).await?;
        let floor_epoch = decimal(floor.try_get("epoch")?, true, u64::MAX)?;
        let generation = decimal(floor.try_get("generation")?, true, u64::MAX)?;
        if epoch < floor_epoch {
            return Ok(IngestResult::Superseded);
        }
        if epoch > floor_epoch {
            // An epoch cannot invent a recovery generation. Unknown fence denies.
            if generation == 0 {
                return Err(Error::Denied);
            }
            Outbox::reopen(db, epoch, generation).await?;
        }
        let Value::Object(members) = &v else {
            return Err(Error::Invalid);
        };
        let content = Value::Object(
            members
                .iter()
                .filter(|(k, _)| {
                    !matches!(
                        k.as_str(),
                        "contract_version"
                            | "operation"
                            | "source_authority"
                            | "projection_epoch"
                            | "projection_revision"
                            | "source_observed_at"
                            | "observed_at"
                    )
                })
                .cloned()
                .collect(),
        );
        let content_digest = digest(&canonical::encode(&content).map_err(|_| Error::Invalid)?);
        let old=sqlx::query("SELECT projection_epoch::text AS epoch,projection_revision::text AS revision,source_observed_at::text AS observed,content_digest FROM bazaar_game_projections WHERE source_authority=$1 AND kind=$2 AND subject=$3 FOR UPDATE").bind(authority).bind(kind).bind(subject).fetch_optional(&mut **db).await?;
        if let Some(old) = old {
            let old_epoch = decimal(old.try_get("epoch")?, false, u64::MAX)?;
            let old_revision = decimal(old.try_get("revision")?, false, u64::MAX)?;
            let old_observed = decimal(old.try_get("observed")?, false, u64::MAX)?;
            if (epoch, revision) < (old_epoch, old_revision)
                || (watermark && epoch == old_epoch && observed < old_observed)
            {
                return Ok(IngestResult::Superseded);
            }
            if (epoch, revision) == (old_epoch, old_revision) {
                if old.try_get::<String, _>("content_digest")? == content_digest {
                    return Ok(IngestResult::Duplicate);
                }
                sqlx::query("INSERT INTO bazaar_game_quarantine(source_authority,kind,subject,incoming_digest) VALUES($1,$2,$3,$4)").bind(authority).bind(kind).bind(subject).bind(content_digest).execute(&mut **db).await?;
                return Ok(IngestResult::Quarantined);
            }
        }
        sqlx::query("INSERT INTO bazaar_game_projections(source_authority,kind,subject,projection_epoch,projection_revision,content_digest,canonical_body,source_observed_at) VALUES($1,$2,$3,$4::text::numeric,$5::text::numeric,$6,$7,$8::text::numeric) ON CONFLICT(source_authority,kind,subject) DO UPDATE SET projection_epoch=EXCLUDED.projection_epoch,projection_revision=EXCLUDED.projection_revision,content_digest=EXCLUDED.content_digest,canonical_body=EXCLUDED.canonical_body,source_observed_at=EXCLUDED.source_observed_at,updated_at=now()")
            .bind(authority).bind(kind).bind(subject).bind(epoch.to_string()).bind(revision.to_string()).bind(content_digest).bind(std::str::from_utf8(raw).map_err(|_|Error::Invalid)?).bind(observed.to_string()).execute(&mut **db).await?;
        Ok(IngestResult::Applied)
    }
}
