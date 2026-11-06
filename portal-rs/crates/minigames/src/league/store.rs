use super::{
    schema::{self, Snapshot},
    *,
};
use prost::Message;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

#[derive(Debug)]
pub struct League<A> {
    pub(super) pool: PgPool,
    pub(super) flags: Flags,
    pub(super) authority: A,
}
impl<A: Authority> League<A> {
    pub fn new(pool: PgPool, flags: Flags, authority: A) -> Self {
        Self {
            pool,
            flags,
            authority,
        }
    }
    pub async fn ingest(&self, d: Delivery<'_>, now: u64) -> Result<()> {
        if !self.flags.ready() || !self.flags.ingest {
            return Err(Error::Unavailable);
        }
        schema::id(&d.envelope_id.to_string())?;
        schema::id(&d.transaction_id.to_string())?;
        schema::ensure(
            d.ordinal > 0
                && d.ordinal <= d.count
                && d.count <= i32::MAX as u32
                && !d.signature.is_empty()
                && d.recovery_generation > 0
                && matches!(d.event_type, 10 | 11),
        )?;
        schema::ensure(
            d.occurred_at_unix_ms > 0
                && u64::try_from(d.occurred_at_unix_ms)
                    .is_ok_and(|t| t <= now.saturating_add(1).saturating_mul(1000)),
        )?;
        if (d.occurred_at_unix_ms as u64 / 1000).saturating_add(90 * 86400) <= now {
            return Err(Error::Unavailable);
        }
        let s = Snapshot::decode(d.wire, d.event_type == 11, now)?;
        check_payload(&d, &s)?;
        schema::ensure(
            !s.final_state()
                || d.finalized_at
                    .is_some_and(|t| t > 0 && t <= now.saturating_add(1)),
        )?;
        let record = s.record()?;
        let record_hash = schema::hash(&record);
        let content_hash = schema::hash(&s.content()?);
        // Receipt identity includes all immutable metadata and exact payload/wire.
        let mut h = Sha256::new();
        h.update(d.transaction_id.as_bytes());
        h.update(d.ordinal.to_be_bytes());
        h.update(d.count.to_be_bytes());
        h.update(d.event_type.to_be_bytes());
        h.update(d.occurred_at_unix_ms.to_be_bytes());
        h.update(d.recovery_generation.to_be_bytes());
        h.update(d.finalized_at.unwrap_or(0).to_be_bytes());
        h.update(Sha256::digest(d.payload));
        h.update(Sha256::digest(d.wire));
        let fingerprint = schema::hash(&h.finalize());
        let mut tx = self.pool.begin().await?;
        // One lock order for all league writes, receipt identities and final controls.
        sqlx::query("SELECT pg_advisory_xact_lock(94000003)")
            .execute(&mut *tx)
            .await?;
        let fence = self.authority.fence(&mut tx, now).await?;
        if !fence.valid(now)
            || fence.epoch != schema::number(s.epoch(), false)?
            || fence.recovery_generation != d.recovery_generation
        {
            return Err(Error::Unavailable);
        }
        self.authority.verify(&mut tx, &d, &s).await?;
        let receipt: Option<(Uuid,String)> = sqlx::query_as("SELECT envelope_id,fingerprint FROM league_receipts WHERE envelope_id=$1 OR (transaction_id=$2 AND ordinal=$3)")
            .bind(d.envelope_id).bind(d.transaction_id).bind(d.ordinal as i32).fetch_optional(&mut *tx).await?;
        if let Some((id, old)) = receipt {
            if id == d.envelope_id && old == fingerprint {
                return Ok(());
            }
            // Global receipt collision invalidates the affected feed, not just this request.
            sqlx::query("UPDATE league_subjects SET quarantined=true")
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Err(Error::Conflict);
        }
        let world = schema::id(s.world())?;
        let subject = schema::id(s.subject())?;
        let old = sqlx::query("SELECT wire,epoch::text,revision::text,recovery_generation::text,record_hash,content_hash,final_state,finalized_at,quarantined FROM league_subjects WHERE authority=$1 AND family=$2 AND world_id=$3 AND subject_id=$4 FOR UPDATE")
            .bind(s.authority()).bind(s.family()).bind(world).bind(subject).fetch_optional(&mut *tx).await?;
        if let Some(old) = &old {
            if old
                .try_get::<Option<i64>, _>("finalized_at")?
                .is_some_and(|t| {
                    t <= i64::try_from(now.saturating_sub(90 * 86400)).unwrap_or(i64::MAX)
                })
            {
                return Err(Error::Unavailable);
            }
            if old.try_get::<bool, _>("quarantined")? {
                return Err(Error::Unavailable);
            }
            let epoch: String = old.try_get("epoch")?;
            let revision: String = old.try_get("revision")?;
            let generation: String = old.try_get("recovery_generation")?;
            let old_epoch = schema::number(&epoch, false)?;
            let old_revision = schema::number(&revision, false)?;
            let next_epoch = schema::number(s.epoch(), false)?;
            let next_revision = schema::number(s.revision(), false)?;
            // Epoch installation/resync requires a separate accepted adapter protocol.
            if next_epoch != old_epoch || generation != d.recovery_generation.to_string() {
                return Err(Error::Unavailable);
            }
            if next_revision < old_revision {
                return Err(Error::Conflict);
            }
            let conflict = if next_revision == old_revision {
                content_hash != old.try_get::<String, _>("content_hash")?
            } else if next_revision != old_revision.saturating_add(1) {
                true
            } else if old.try_get::<bool, _>("final_state")? {
                record_hash != old.try_get::<String, _>("record_hash")?
            } else {
                let bytes: Vec<u8> = old.try_get("wire")?;
                let prior = Snapshot::decode(&bytes, d.event_type == 11, now)?;
                transition(&prior, &s).is_err()
            };
            if conflict {
                sqlx::query("UPDATE league_subjects SET quarantined=true WHERE authority=$1 AND family=$2 AND world_id=$3 AND subject_id=$4")
                    .bind(s.authority()).bind(s.family()).bind(world).bind(subject).execute(&mut *tx).await?;
                tx.commit().await?;
                return Err(Error::Conflict);
            }
        } else if !s.correction().is_empty() {
            return Err(Error::Unavailable);
        }
        // Native occurrence cannot acquire a second question identity, including after expiry.
        if let Snapshot::Event(e) = &s
            && let Some(r) = e.event.first()
        {
            let existing: Option<Uuid> = sqlx::query_scalar("SELECT subject_id FROM league_occurrences WHERE authority=$1 AND world_id=$2 AND occurrence_id=$3")
                    .bind(s.authority()).bind(world).bind(schema::id(&r.occurrence_id)?).fetch_optional(&mut *tx).await?;
            if existing.is_some_and(|id| id != subject) {
                return Err(Error::Conflict);
            }
            sqlx::query("INSERT INTO league_occurrences(authority,world_id,occurrence_id,subject_id) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING")
                    .bind(s.authority()).bind(world).bind(schema::id(&r.occurrence_id)?).bind(subject).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO league_receipts(envelope_id,transaction_id,ordinal,event_count,fingerprint) VALUES($1,$2,$3,$4,$5)")
            .bind(d.envelope_id).bind(d.transaction_id).bind(d.ordinal as i32).bind(d.count as i32).bind(fingerprint).execute(&mut *tx).await?;
        // Same pair/content may have a new audit envelope; preserve first immutable audit.
        sqlx::query("INSERT INTO league_audit(envelope_id,authority,family,world_id,subject_id,epoch,revision,recovery_generation,wire,payload,signature,received_at,occurred_at_unix_ms) VALUES($1,$2,$3,$4,$5,$6::numeric,$7::numeric,$8::numeric,$9,$10,$11,$12,$13) ON CONFLICT(authority,family,world_id,subject_id,epoch,revision) DO NOTHING")
            .bind(d.envelope_id).bind(s.authority()).bind(s.family()).bind(world).bind(subject).bind(s.epoch()).bind(s.revision()).bind(d.recovery_generation.to_string()).bind(d.wire).bind(d.payload).bind(d.signature).bind(i64::try_from(now).map_err(|_| Error::Invalid)?).bind(d.occurred_at_unix_ms).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO league_subjects(authority,family,world_id,subject_id,epoch,revision,recovery_generation,wire,content_hash,record_hash,final_state,finalized_at) VALUES($1,$2,$3,$4,$5::numeric,$6::numeric,$7::numeric,$8,$9,$10,$11,$12) ON CONFLICT(authority,family,world_id,subject_id) DO UPDATE SET revision=EXCLUDED.revision,wire=EXCLUDED.wire,content_hash=EXCLUDED.content_hash,record_hash=EXCLUDED.record_hash,final_state=EXCLUDED.final_state,finalized_at=COALESCE(league_subjects.finalized_at,EXCLUDED.finalized_at)")
            .bind(s.authority()).bind(s.family()).bind(world).bind(subject).bind(s.epoch()).bind(s.revision()).bind(d.recovery_generation.to_string()).bind(d.wire).bind(content_hash).bind(record_hash).bind(s.final_state()).bind(if s.final_state() { Some(i64::try_from(d.finalized_at.ok_or(Error::Unavailable)?).map_err(|_| Error::Invalid)?) } else { None }).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn read(&self, now: u64) -> Result<Vec<Snapshot>> {
        if !self.flags.ready() || !self.flags.public {
            return Err(Error::Unavailable);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock_shared(94000003)")
            .execute(&mut *tx)
            .await?;
        let fence = self.authority.fence(&mut tx, now).await?;
        if !fence.valid(now) {
            return Err(Error::Unavailable);
        }
        let rows = sqlx::query("SELECT family,wire,quarantined,epoch::text,recovery_generation::text,content_hash,record_hash,EXISTS(SELECT 1 FROM league_audit a WHERE a.authority=league_subjects.authority AND a.family=league_subjects.family AND a.world_id=league_subjects.world_id AND a.subject_id=league_subjects.subject_id AND a.epoch=league_subjects.epoch AND a.revision=league_subjects.revision AND octet_length(a.signature)>0 AND a.occurred_at_unix_ms / 1000 > $1) AS audit_available FROM league_subjects WHERE finalized_at IS NULL OR finalized_at > $1")
            .bind(i64::try_from(now.saturating_sub(90*86400)).map_err(|_| Error::Invalid)?).fetch_all(&mut *tx).await?;
        let mut out = vec![];
        for row in rows {
            if !row.try_get::<bool, _>("audit_available")?
                || row.try_get::<bool, _>("quarantined")?
                || row.try_get::<String, _>("epoch")? != fence.epoch.to_string()
                || row.try_get::<String, _>("recovery_generation")?
                    != fence.recovery_generation.to_string()
            {
                return Err(Error::Unavailable);
            }
            let wire: Vec<u8> = row.try_get("wire")?;
            let s = Snapshot::decode(
                &wire,
                row.try_get::<String, _>("family")? == "guild_season",
                now,
            )?;
            if schema::hash(&s.content()?) != row.try_get::<String, _>("content_hash")?
                || schema::hash(&s.record()?) != row.try_get::<String, _>("record_hash")?
            {
                return Err(Error::Unavailable);
            }
            if s.observed()? > fence.complete_through {
                return Err(Error::Unavailable);
            }
            if match &s {
                Snapshot::Event(e) => e
                    .event
                    .first()
                    .is_some_and(|r| matches!(r.state.as_str(), "SETTLED" | "VOID" | "CLOSED")),
                Snapshot::Season(e) => !e.season.is_empty(),
            } {
                out.push(s);
            }
        }
        tx.commit().await?;
        Ok(out)
    }
    /// Scheduler-owned cleanup; never removes replay/final-state controls.
    pub async fn expire(&self, now: u64) -> Result<()> {
        if !self.flags.ready() {
            return Err(Error::Unavailable);
        }
        let cutoff = i64::try_from(now.saturating_sub(90 * 86400)).map_err(|_| Error::Invalid)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(94000003)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM league_audit WHERE occurred_at_unix_ms / 1000 <= $1")
            .bind(cutoff)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE league_subjects SET wire=''::bytea WHERE finalized_at <= $1")
            .bind(cutoff)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
fn check_payload(d: &Delivery<'_>, s: &Snapshot) -> Result<()> {
    let record = s.record()?;
    let hash = Sha256::digest(&record).to_vec();
    match s {
        Snapshot::Event(e) => {
            let p = WorldEventOutcomeDecidedV1::decode(d.payload).map_err(|_| Error::Invalid)?;
            schema::ensure(
                p.encode_to_vec() == d.payload
                    && p.record == record
                    && p.record_sha256 == hash
                    && p.subject_event_id == schema::id(s.subject())?.as_bytes()
                    && p.recovery_generation == d.recovery_generation,
            )?;
            let r = e.event.first().ok_or(Error::Unavailable)?;
            schema::ensure(
                r.evidence_sha256.len() == 1
                    && schema::hash_bytes_equal(&p.source_evidence_sha256, &r.evidence_sha256[0]),
            )
        }
        Snapshot::Season(e) => {
            let p = GuildSeasonStandingsDecidedV1::decode(d.payload).map_err(|_| Error::Invalid)?;
            schema::ensure(
                !e.season.is_empty()
                    && p.encode_to_vec() == d.payload
                    && p.record_sha256 == hash
                    && p.season_id == schema::id(s.subject())?.as_bytes()
                    && p.recovery_generation == d.recovery_generation,
            )
        }
    }
}
pub(super) fn transition(old: &Snapshot, new: &Snapshot) -> Result<()> {
    match (old, new) {
        (Snapshot::Event(a), Snapshot::Event(b)) => {
            let a = a.event.first().ok_or(Error::Unavailable)?;
            let b = b.event.first().ok_or(Error::Unavailable)?;
            let mut x = a.clone();
            let mut y = b.clone();
            for e in [&mut x, &mut y] {
                e.state.clear();
                e.outcome.clear();
                e.decided_at.clear();
                e.correction_of.clear();
                e.reason_code.clear();
                e.evidence_sha256.clear();
            }
            schema::ensure(x == y)?;
            let terminal = matches!(a.state.as_str(), "SETTLED" | "VOID");
            if b.state == "CLOSED" {
                schema::ensure(
                    terminal
                        && a.outcome == b.outcome
                        && a.decided_at == b.decided_at
                        && a.evidence_sha256 == b.evidence_sha256
                        && a.correction_of == b.correction_of,
                )
            } else if terminal {
                schema::ensure(
                    matches!(b.state.as_str(), "SETTLED" | "VOID")
                        && b.reason_code == "CORRECTION"
                        && b.correction_of == [schema::hash(&old.record()?)]
                        && a.evidence_sha256 != b.evidence_sha256,
                )
            } else {
                schema::ensure(
                    matches!(
                        (a.state.as_str(), b.state.as_str()),
                        ("SCHEDULED", "LOCKED" | "VOID") | ("LOCKED", "SETTLED" | "VOID")
                    ) && b.correction_of.is_empty(),
                )
            }
        }
        (Snapshot::Season(a), Snapshot::Season(b)) => {
            let a = a.season.first().ok_or(Error::Unavailable)?;
            let b = b.season.first().ok_or(Error::Unavailable)?;
            schema::ensure(
                a.state == "OPEN"
                    && a.ruleset_revision == b.ruleset_revision
                    && a.starts_at == b.starts_at
                    && a.ends_at == b.ends_at
                    && a.freeze_at == b.freeze_at
                    && a.challenges == b.challenges,
            )?;
            schema::ensure(
                (a.standings == b.standings && a.correction_of == b.correction_of)
                    || b.correction_of == [schema::hash(&old.record()?)],
            )
        }
        _ => Err(Error::Invalid),
    }
}
