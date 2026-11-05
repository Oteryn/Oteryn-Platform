use super::*;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Row};
use std::future::Future;
use uuid::Uuid;

/// Installed only by a trusted server adapter. Never derive this identity from a URL/header.
#[derive(Debug, Clone, Copy)]
pub enum Actor {
    /// Internal ingest marker, never a credential; Authority verifies its bound peer.
    Publisher,
    Owner(Uuid),
    Anonymous,
    Staff,
}

/// Calls must lock authority rows until transaction commit, including on reads.
/// `stream` verifies mTLS purpose/partition on ingest, accepted epoch/recovery and
/// current independent ownership freshness. `record` additionally verifies the exact
/// receipt transfer cut (including CharacterRevision), visibility floor and catalog.
/// Skull projection_revision is a source table revision, never an ownership proof.
/// Historical records before an unproven transfer cut must be denied to both parties.
pub trait Authority: Send + Sync {
    fn stream<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        stream: &'a Stream,
        actor: Actor,
    ) -> impl Future<Output = Result<(), Failure>> + Send + 'a;
    fn subject<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        stream: &'a Stream,
        character: Uuid,
        actor: Actor,
    ) -> impl Future<Output = Result<(), Failure>> + Send + 'a;
    fn record<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        record: &'a Record,
        actor: Actor,
    ) -> impl Future<Output = Result<(), Failure>> + Send + 'a;
}
#[derive(Debug, Default)]
pub struct DenyAll;
impl Authority for DenyAll {
    async fn stream(&self, _: &mut PgConnection, _: &Stream, _: Actor) -> Result<(), Failure> {
        Err(Failure::Unavailable)
    }
    async fn subject(
        &self,
        _: &mut PgConnection,
        _: &Stream,
        _: Uuid,
        _: Actor,
    ) -> Result<(), Failure> {
        Err(Failure::Unavailable)
    }
    async fn record(&self, _: &mut PgConnection, _: &Record, _: Actor) -> Result<(), Failure> {
        Err(Failure::Unavailable)
    }
}
#[derive(Debug, Clone, Default)]
pub struct Flags {
    pub progress_ingest_enabled: bool,
    pub progress_read_enabled: bool,
    pub enabled_families: Vec<Family>,
}
impl Flags {
    fn check(&self, family: Family, ingest: bool) -> Result<(), Failure> {
        if (if ingest {
            self.progress_ingest_enabled
        } else {
            self.progress_read_enabled
        }) && self.enabled_families.contains(&family)
        {
            Ok(())
        } else {
            Err(Failure::Unavailable)
        }
    }
}

/// Feature-local adapter. T72 owns listener authentication/rate limits and authority.
/// No routes are activated by constructing this repository.
#[derive(Debug, Clone)]
pub struct Deaths<A = DenyAll> {
    pool: PgPool,
    flags: Flags,
    authority: A,
}
fn db(_: sqlx::Error) -> Failure {
    Failure::Unavailable
}
fn now_valid(now_ms: u64) -> Result<(), Failure> {
    if now_ms > MAX_TIME {
        Err(Failure::Unavailable)
    } else {
        Ok(())
    }
}
impl<A: Authority> Deaths<A> {
    pub(crate) fn ingest_enabled(&self) -> bool {
        self.flags.progress_ingest_enabled
    }
    pub fn new(pool: PgPool, flags: Flags, authority: A) -> Self {
        Self {
            pool,
            flags,
            authority,
        }
    }

    pub async fn ingest(&self, bytes: &[u8], now_ms: u64) -> Result<Vec<u8>, Failure> {
        // Parse only after the global switch: disabled endpoints always return 503.
        if !self.flags.progress_ingest_enabled {
            return Err(Failure::Unavailable);
        }
        now_valid(now_ms)?;
        let record = Record::parse(bytes)?;
        self.flags.check(record.family, true)?;
        let occurred = decimal(&record.occurred_at_unix_ms, true, MAX_TIME)?;
        if occurred > now_ms.saturating_add(5000) {
            return Err(Failure::BadInput);
        }
        let stream = record.stream();
        let actor = Actor::Publisher;
        let mut tx = self.pool.begin().await.map_err(db)?;
        self.authority.stream(&mut tx, &stream, actor).await?;
        self.authority.record(&mut tx, &record, actor).await?;
        lock_stream(&mut tx, &stream).await?;
        let digest = Sha256::digest(bytes).to_vec();
        let old: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT digest FROM pd_t78_records WHERE source_authority=$1 AND record_id=$2",
        )
        .bind(&record.source_authority)
        .bind(id(&record.record_id)?)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(old) = old {
            if old != digest {
                return Err(Failure::Conflict);
            }
            return record_ack(&record);
        }
        let accepted: String = sqlx::query_scalar("SELECT accepted_through::text FROM pd_t78_streams WHERE source_authority=$1 AND account_id=$2 AND projection_epoch=$3::text::numeric AND recovery_generation=$4::text::numeric AND family=$5")
            .bind(&stream.source_authority).bind(id(&stream.account_id)?).bind(&stream.projection_epoch).bind(&stream.recovery_generation).bind(stream.family.key())
            .fetch_one(&mut *tx).await.map_err(db)?;
        let revision = decimal(&record.source_revision, false, u64::MAX)?;
        if decimal(&accepted, true, u64::MAX)?.checked_add(1) != Some(revision) {
            return Err(Failure::Conflict);
        }
        // No correction indicator exists in v1. Fail closed on replacement of an
        // immutable historical occurrence; REMOVE remains available for erasure.
        if record.family != Family::Skull && record.record_kind == RecordKind::Upsert {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pd_t78_records WHERE source_authority=$1 AND account_id=$2 AND projection_epoch=$3::text::numeric AND recovery_generation=$4::text::numeric AND family=$5 AND character_id=$6 AND world_id=$7 AND entity_key=$8)")
                .bind(&stream.source_authority).bind(id(&stream.account_id)?).bind(&stream.projection_epoch).bind(&stream.recovery_generation).bind(stream.family.key())
                .bind(id(&record.character_id)?).bind(id(&record.world_id)?).bind(&record.entity_key).fetch_one(&mut *tx).await.map_err(db)?;
            if exists {
                return Err(Failure::Conflict);
            }
        }
        // The contract permits refusing gaps instead of staging them. Replay fences survive expiry.
        let retained =
            record.family == Family::Skull || now_ms.saturating_sub(occurred) < HISTORY_MS;
        let payload = if retained {
            Some(serde_json::to_value(&record).map_err(|_| Failure::BadInput)?)
        } else {
            None
        };
        let result = sqlx::query("INSERT INTO pd_t78_records (source_authority,account_id,projection_epoch,recovery_generation,family,source_revision,record_id,character_id,world_id,entity_key,digest,occurred_at_ms,payload) VALUES ($1,$2,$3::text::numeric,$4::text::numeric,$5,$6::text::numeric,$7,$8,$9,$10,$11,$12,$13)")
            .bind(&record.source_authority).bind(id(&record.account_id)?).bind(&record.projection_epoch).bind(&record.recovery_generation)
            .bind(record.family.key()).bind(&record.source_revision).bind(id(&record.record_id)?).bind(id(&record.character_id)?).bind(id(&record.world_id)?)
            .bind(&record.entity_key).bind(digest).bind(occurred as i64).bind(payload).execute(&mut *tx).await;
        if let Err(error) = result {
            return Err(
                if error
                    .as_database_error()
                    .is_some_and(|e| e.is_unique_violation())
                {
                    Failure::Conflict
                } else {
                    db(error)
                },
            );
        }
        sqlx::query("UPDATE pd_t78_streams SET accepted_through=$6::text::numeric WHERE source_authority=$1 AND account_id=$2 AND projection_epoch=$3::text::numeric AND recovery_generation=$4::text::numeric AND family=$5")
            .bind(&stream.source_authority).bind(id(&stream.account_id)?).bind(&stream.projection_epoch).bind(&stream.recovery_generation).bind(stream.family.key()).bind(&record.source_revision)
            .execute(&mut *tx).await.map_err(db)?;
        tx.commit().await.map_err(db)?;
        record_ack(&record)
    }

    pub async fn watermark(&self, bytes: &[u8], now_ms: u64) -> Result<Vec<u8>, Failure> {
        if !self.flags.progress_ingest_enabled {
            return Err(Failure::Unavailable);
        }
        now_valid(now_ms)?;
        let mark = Watermark::parse(bytes)?;
        self.flags.check(mark.family, true)?;
        let observed = decimal(&mark.observed_at_unix_ms, true, MAX_TIME)?;
        if observed > now_ms.saturating_add(5000) {
            return Err(Failure::BadInput);
        }
        let stream = mark.stream();
        let mut tx = self.pool.begin().await.map_err(db)?;
        self.authority
            .stream(&mut tx, &stream, Actor::Publisher)
            .await?;
        lock_stream(&mut tx, &stream).await?;
        let row = sqlx::query("SELECT accepted_through::text,complete_through::text,observed_at_ms,baseline_complete FROM pd_t78_streams WHERE source_authority=$1 AND account_id=$2 AND projection_epoch=$3::text::numeric AND recovery_generation=$4::text::numeric AND family=$5")
            .bind(&stream.source_authority).bind(id(&stream.account_id)?).bind(&stream.projection_epoch).bind(&stream.recovery_generation).bind(stream.family.key())
            .fetch_one(&mut *tx).await.map_err(db)?;
        let accepted: String = row.try_get("accepted_through").map_err(db)?;
        let complete: String = row.try_get("complete_through").map_err(db)?;
        let old_time: Option<i64> = row.try_get("observed_at_ms").map_err(db)?;
        let baseline: bool = row.try_get("baseline_complete").map_err(db)?;
        let through = decimal(&mark.complete_through, true, u64::MAX)?;
        if through > decimal(&accepted, true, u64::MAX)?
            || through < decimal(&complete, true, u64::MAX)?
            || old_time.is_some_and(|t| observed < t as u64)
            || (baseline && !mark.baseline_complete)
        {
            return Err(Failure::Conflict);
        }
        sqlx::query("UPDATE pd_t78_streams SET complete_through=$6::text::numeric,observed_at_ms=$7,baseline_complete=$8 WHERE source_authority=$1 AND account_id=$2 AND projection_epoch=$3::text::numeric AND recovery_generation=$4::text::numeric AND family=$5")
            .bind(&stream.source_authority).bind(id(&stream.account_id)?).bind(&stream.projection_epoch).bind(&stream.recovery_generation).bind(stream.family.key())
            .bind(&mark.complete_through).bind(observed as i64).bind(mark.baseline_complete).execute(&mut *tx).await.map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(format!(
            "{{\"contract_version\":1,\"result\":\"accepted\",\"complete_through\":\"{}\"}}",
            mark.complete_through
        )
        .into_bytes())
    }

    /// A bounded owner-only projection, latest entity per character/world and family.
    /// An empty proven baseline differs from an unavailable stream. No staff override.
    pub async fn read(
        &self,
        actor: Actor,
        stream: &Stream,
        character: Uuid,
        now_ms: u64,
    ) -> Result<Projection, Failure> {
        self.flags.check(stream.family, false)?;
        now_valid(now_ms)?;
        let account = id(&stream.account_id)?;
        if !matches!(actor, Actor::Owner(owner) if owner == account) {
            return Err(Failure::Denied);
        }
        let mut tx = self.pool.begin().await.map_err(db)?;
        self.authority.stream(&mut tx, stream, actor).await?;
        self.authority
            .subject(&mut tx, stream, character, actor)
            .await?;
        let row = sqlx::query("SELECT complete_through::text,observed_at_ms,baseline_complete FROM pd_t78_streams WHERE source_authority=$1 AND account_id=$2 AND projection_epoch=$3::text::numeric AND recovery_generation=$4::text::numeric AND family=$5 FOR SHARE")
            .bind(&stream.source_authority).bind(account).bind(&stream.projection_epoch).bind(&stream.recovery_generation).bind(stream.family.key())
            .fetch_optional(&mut *tx).await.map_err(db)?.ok_or(Failure::Unavailable)?;
        let baseline: bool = row.try_get("baseline_complete").map_err(db)?;
        let observed: Option<i64> = row.try_get("observed_at_ms").map_err(db)?;
        let observed = observed.ok_or(Failure::Unavailable)? as u64;
        if !baseline
            || observed > now_ms.saturating_add(5000)
            || now_ms.saturating_sub(observed) > STALE_MS
        {
            return Err(Failure::Unavailable);
        }
        let stale = now_ms.saturating_sub(observed) > 10000;
        // Skull timers require fresh evidence; stale state must never imply elapsed/cleared.
        if stale && stream.family == Family::Skull {
            return Err(Failure::Unavailable);
        }
        let complete: String = row.try_get("complete_through").map_err(db)?;
        let rows = sqlx::query("SELECT payload FROM (SELECT DISTINCT ON (world_id,entity_key) payload,source_revision FROM pd_t78_records WHERE source_authority=$1 AND account_id=$2 AND projection_epoch=$3::text::numeric AND recovery_generation=$4::text::numeric AND family=$5 AND character_id=$6 AND source_revision <= $7::text::numeric ORDER BY world_id,entity_key,source_revision DESC) latest ORDER BY source_revision DESC LIMIT 201")
            .bind(&stream.source_authority).bind(account).bind(&stream.projection_epoch).bind(&stream.recovery_generation).bind(stream.family.key()).bind(character).bind(complete)
            .fetch_all(&mut *tx).await.map_err(db)?;
        if rows.len() > 200 {
            return Err(Failure::Unavailable);
        }
        let mut records = Vec::new();
        for row in rows {
            let payload: Option<serde_json::Value> = row.try_get("payload").map_err(db)?;
            let Some(payload) = payload else {
                continue;
            };
            let record: Record =
                serde_json::from_value(payload).map_err(|_| Failure::Unavailable)?;
            record.validate().map_err(|_| Failure::Unavailable)?;
            self.authority.record(&mut tx, &record, actor).await?;
            if record.record_kind == RecordKind::Remove {
                continue;
            }
            let occurred = decimal(&record.occurred_at_unix_ms, true, MAX_TIME)?;
            if record.family != Family::Skull && now_ms.saturating_sub(occurred) >= HISTORY_MS {
                continue;
            }
            records.push(record);
        }
        tx.commit().await.map_err(db)?;
        Ok(Projection { stale, records })
    }

    /// Payload erasure keeps only replay digests and sequence/fence metadata.
    /// T72 invokes this in its retention worker and deletes streams on account erasure.
    pub async fn purge_expired(&self, now_ms: u64) -> Result<u64, Failure> {
        now_valid(now_ms)?;
        let result = sqlx::query("UPDATE pd_t78_records r SET payload=NULL WHERE payload IS NOT NULL AND ((family IN ('death_recap','pvp') AND occurred_at_ms <= $1) OR (family='skull' AND occurred_at_ms <= $2 AND EXISTS (SELECT 1 FROM pd_t78_records newer WHERE newer.source_authority=r.source_authority AND newer.account_id=r.account_id AND newer.projection_epoch=r.projection_epoch AND newer.recovery_generation=r.recovery_generation AND newer.family=r.family AND newer.character_id=r.character_id AND newer.world_id=r.world_id AND newer.entity_key=r.entity_key AND newer.source_revision > r.source_revision)))")
            .bind(now_ms.saturating_sub(HISTORY_MS) as i64).bind(now_ms.saturating_sub(7*24*60*60*1000) as i64).execute(&self.pool).await.map_err(db)?;
        Ok(result.rows_affected())
    }
    pub async fn erase_account(&self, account: Uuid) -> Result<(), Failure> {
        sqlx::query("DELETE FROM pd_t78_streams WHERE account_id=$1")
            .bind(account)
            .execute(&self.pool)
            .await
            .map_err(db)?;
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct Projection {
    pub stale: bool,
    pub records: Vec<Record>,
}
fn record_ack(record: &Record) -> Result<Vec<u8>, Failure> {
    Ok(format!("{{\"contract_version\":1,\"result\":\"accepted\",\"record_id\":\"{}\",\"accepted_source_revision\":\"{}\"}}",record.record_id,record.source_revision).into_bytes())
}
async fn lock_stream(connection: &mut PgConnection, stream: &Stream) -> Result<(), Failure> {
    sqlx::query("INSERT INTO pd_t78_streams (source_authority,account_id,projection_epoch,recovery_generation,family) VALUES ($1,$2,$3::text::numeric,$4::text::numeric,$5) ON CONFLICT DO NOTHING")
        .bind(&stream.source_authority).bind(id(&stream.account_id)?).bind(&stream.projection_epoch).bind(&stream.recovery_generation).bind(stream.family.key()).execute(&mut *connection).await.map_err(db)?;
    sqlx::query("SELECT 1 FROM pd_t78_streams WHERE source_authority=$1 AND account_id=$2 AND projection_epoch=$3::text::numeric AND recovery_generation=$4::text::numeric AND family=$5 FOR UPDATE")
        .bind(&stream.source_authority).bind(id(&stream.account_id)?).bind(&stream.projection_epoch).bind(&stream.recovery_generation).bind(stream.family.key()).fetch_one(connection).await.map_err(db)?;
    Ok(())
}
