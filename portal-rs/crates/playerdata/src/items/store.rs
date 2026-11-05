use super::model::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub ingest: bool,
    pub items: bool,
    pub history: bool,
    pub bazaar_shortcut: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub source_authority: String,
    pub account_id: String,
    pub world_id: String,
    pub character_id: Option<String>,
    pub cut: Option<Cut>,
    pub family: String,
    pub recovery_generation: String,
}
impl Scope {
    pub fn from_record(r: &Record) -> Self {
        Self {
            source_authority: r.source_authority.clone(),
            account_id: r.account_id.clone(),
            world_id: r.world_id.clone(),
            character_id: r.character_id.clone(),
            cut: r.cut(),
            family: r.family.clone(),
            recovery_generation: r.recovery_generation.clone(),
        }
    }
    pub fn stream_key(&self) -> Result<Vec<u8>> {
        let s = serde_json::to_string(&(
            &self.source_authority,
            &self.account_id,
            &self.world_id,
            &self.character_id,
            &self.cut,
            &self.family,
            &self.recovery_generation,
        ))
        .map_err(|_| Error::Invalid)?;
        Ok(Sha256::digest(s.as_bytes()).to_vec())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Access {
    Ingest,
    Read(Uuid),
}

/// A trusted, transactional proof from T72, not a browser-provided ownership claim.
#[derive(Debug, Clone)]
pub struct Proof {
    /// Latest plausible trusted UTC second (clock uncertainty at most 30 seconds).
    pub now: u64,
    /// Accepted baseline coverage; silence must never create an empty ledger.
    pub coverage_start: u64,
    /// Last proven contiguous source coverage, never inferred from the clock.
    pub coverage_through: u64,
}
#[derive(Debug, Clone)]
pub struct Definition {
    pub maximum_quantity: u32,
    pub container: bool,
    pub tradeable: bool,
    /// An explicitly estimated T74 unit price; None means unknown, never zero.
    pub estimated_gold: Option<u64>,
    pub price_source: Option<String>,
}

/// Implementations must lock current ownership/privacy/restore evidence in `db`
/// until the caller's transaction ends. Reject pending transfers, stale cuts,
/// unregistered generations, erased accounts and incomplete baselines. Reads
/// require actor == current account, with no staff override. BANK is account/world
/// scoped. Resolve scopes server-side, never from a historical character name.
#[async_trait]
pub trait Evidence: std::fmt::Debug + Send + Sync {
    async fn authorize(
        &self,
        db: &mut PgConnection,
        scope: &Scope,
        access: Access,
    ) -> Result<Proof>;
    async fn definition(&self, db: &mut PgConnection, item: &Item) -> Result<Definition>;
    async fn resolve(
        &self,
        db: &mut PgConnection,
        actor: Uuid,
        world: Uuid,
        character: Option<Uuid>,
        family: &str,
    ) -> Result<Scope>;
}

/// Until T72 supplies accepted baseline and transfer fences, configuration denies.
#[derive(Debug)]
pub struct UnavailableEvidence;
#[async_trait]
impl Evidence for UnavailableEvidence {
    async fn authorize(&self, _: &mut PgConnection, _: &Scope, _: Access) -> Result<Proof> {
        Err(Error::Unavailable)
    }
    async fn definition(&self, _: &mut PgConnection, _: &Item) -> Result<Definition> {
        Err(Error::Unavailable)
    }
    async fn resolve(
        &self,
        _: &mut PgConnection,
        _: Uuid,
        _: Uuid,
        _: Option<Uuid>,
        _: &str,
    ) -> Result<Scope> {
        Err(Error::Unavailable)
    }
}

#[derive(Debug, Clone)]
pub struct Projection {
    pub pool: PgPool,
    pub flags: Flags,
    pub evidence: Arc<dyn Evidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub at_logout: u64,
    pub snapshot_id: String,
    pub depots: Vec<u16>,
    pub items: Vec<Item>,
}
#[derive(Debug)]
pub struct ValuedItem {
    pub item: Item,
    pub estimate: Option<u64>,
    pub price_source: Option<String>,
    pub bazaar_shortcut: bool,
}
#[derive(Debug)]
pub struct Inventory {
    pub at_logout: u64,
    pub snapshot_id: String,
    pub depots: Vec<u16>,
    pub items: Vec<ValuedItem>,
    /// Absent unless every row has a checked, source-labelled price.
    pub estimated_total: Option<u64>,
    pub unpriced: usize,
}
#[derive(Debug)]
pub struct History {
    pub coverage_start: u64,
    pub through: u64,
    pub records: Vec<Record>,
}

fn uuid(s: &str) -> Result<Uuid> {
    id(s)?;
    Uuid::parse_str(s).map_err(|_| Error::Invalid)
}
fn integer(n: u64) -> Result<i64> {
    i64::try_from(n).map_err(|_| Error::Invalid)
}
fn time_check(r: &Record, p: &Proof) -> Result<()> {
    let at = counter(&r.occurred_at, false)?;
    if at > p.now.saturating_add(30)
        || at.saturating_add(RETENTION) <= p.now
        || (r.family != "ITEMS" && counter(&r.coverage_start_at, false)? != p.coverage_start)
    {
        return Err(Error::Unavailable);
    }
    Ok(())
}
fn event_identity(p: &Payload) -> Option<(&str, &str)> {
    match p {
        Payload::Bank(b) => Some((&b.event_id, &b.operation_occurrence_id)),
        Payload::Market(m) => Some((&m.event_id, &m.event_id)),
        Payload::Trade(t) => Some((&t.event_id, &t.occurrence_id)),
        _ => None,
    }
}

impl Projection {
    /// T72 must authenticate the dedicated peer before calling this method.
    /// ACK is returned only after staging, deduplication and cursor commit together.
    pub async fn ingest(&self, bytes: &[u8], peer_authority: &str) -> Result<()> {
        if !self.flags.ingest {
            return Err(Error::Unavailable);
        }
        let r = Record::parse(bytes)?;
        if r.source_authority != peer_authority {
            return Err(Error::Denied);
        }
        let scope = Scope::from_record(&r);
        let sk = scope.stream_key()?;
        let mut tx = self.pool.begin().await?;
        let proof = self
            .evidence
            .authorize(&mut tx, &scope, Access::Ingest)
            .await?;
        time_check(&r, &proof)?;
        let actor = uuid(&r.account_id)?;
        let world = uuid(&r.world_id)?;
        let character = r.character_id.as_deref().map(uuid).transpose()?;
        let coverage = integer(proof.coverage_start)?;
        sqlx::query("INSERT INTO pd_t76_streams(stream_key,source_authority,account_id,world_id,character_id,family,generation,coverage_start) VALUES($1,$2,$3,$4,$5,$6,$7::text::numeric,$8) ON CONFLICT DO NOTHING")
            .bind(&sk).bind(&r.source_authority).bind(actor).bind(world).bind(character).bind(&r.family).bind(&r.recovery_generation).bind(coverage).execute(&mut *tx).await?;
        let stream=sqlx::query("SELECT fenced,cursor::text,retention_floor::text,coverage_start,staging_start::text,staging_at,last_bank_entry,last_bank_balance FROM pd_t76_streams WHERE stream_key=$1 FOR UPDATE")
            .bind(&sk).fetch_one(&mut *tx).await?;
        if stream.try_get::<bool, _>("fenced")? {
            return Err(Error::Conflict);
        }
        let revision = counter(&r.source_revision, false)?;
        let cursor = counter(stream.try_get::<&str, _>("cursor")?, true)?;
        let floor = counter(stream.try_get::<&str, _>("retention_floor")?, true)?;
        if revision <= floor || stream.try_get::<i64, _>("coverage_start")? != coverage {
            return Err(Error::Conflict);
        }
        let digest = Sha256::digest(bytes).to_vec();
        let existing=sqlx::query("SELECT stream_key,source_revision::text,digest FROM pd_t76_records WHERE (stream_key=$1 AND source_revision=$2::text::numeric) OR (source_authority=$3 AND generation=$4::text::numeric AND record_id=$5)")
            .bind(&sk).bind(&r.source_revision).bind(&r.source_authority).bind(&r.recovery_generation).bind(uuid(&r.record_id)?).fetch_all(&mut *tx).await?;
        if !existing.is_empty() {
            if existing.len() != 1
                || existing[0].try_get::<Vec<u8>, _>("stream_key")? != sk
                || existing[0].try_get::<String, _>("source_revision")? != r.source_revision
                || existing[0].try_get::<Vec<u8>, _>("digest")? != digest
            {
                return Err(Error::Conflict);
            }
            tx.commit().await?;
            return Ok(());
        }
        if cursor.checked_add(1) != Some(revision) {
            sqlx::query("UPDATE pd_t76_streams SET fenced=true WHERE stream_key=$1")
                .bind(&sk)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Err(Error::Conflict);
        }
        let staging: Option<String> = stream.try_get("staging_start")?;
        let staging_at: Option<i64> = stream.try_get("staging_at")?;
        if staging_at.is_some_and(|t| t.saturating_add(86400) <= proof.now as i64) {
            return Err(Error::Unavailable);
        }
        match &r.payload {
            Payload::Begin(_) => {
                if staging.is_some() {
                    return Err(Error::Conflict);
                }
                sqlx::query("UPDATE pd_t76_streams SET staging_start=$2::text::numeric,staging_at=$3 WHERE stream_key=$1")
                    .bind(&sk).bind(&r.source_revision).bind(integer(proof.now)?).execute(&mut *tx).await?;
            }
            Payload::Page(page) => {
                let start = staging.ok_or(Error::Conflict)?;
                // Read only the BEGIN and preceding page, rather than replaying
                // every staged page on every publication. END validates the graph.
                let stored: Vec<Vec<u8>> = sqlx::query_scalar("SELECT canonical_bytes FROM pd_t76_records WHERE stream_key=$1 AND (source_revision=$2::text::numeric OR source_revision=$3::text::numeric) ORDER BY source_revision")
                    .bind(&sk).bind(&start).bind(cursor.to_string()).fetch_all(&mut *tx).await?;
                let first = Record::parse(stored.first().ok_or(Error::Conflict)?)?;
                let Payload::Begin(begin) = &first.payload else {
                    return Err(Error::Conflict);
                };
                if r.occurred_at != first.occurred_at
                    || r.coverage_start_at != first.coverage_start_at
                    || page.snapshot_id != begin.snapshot_id
                    || page.page_index >= begin.page_count
                    || revision
                        .checked_sub(counter(&start, false)?)
                        .and_then(|v| v.checked_sub(1))
                        != Some(u64::from(page.page_index))
                {
                    return Err(Error::Conflict);
                }
                if page.page_index > 0 {
                    let previous = Record::parse(stored.last().ok_or(Error::Conflict)?)?;
                    let Payload::Page(previous) = previous.payload else {
                        return Err(Error::Conflict);
                    };
                    if previous
                        .items
                        .last()
                        .zip(page.items.first())
                        .is_none_or(|(a, b)| a.item_instance_id >= b.item_instance_id)
                    {
                        return Err(Error::Conflict);
                    }
                }
            }
            Payload::End(_) => {
                let start = staging.ok_or(Error::Conflict)?;
                let stored:Vec<Vec<u8>>=sqlx::query_scalar("SELECT canonical_bytes FROM pd_t76_records WHERE stream_key=$1 AND source_revision >= $2::text::numeric ORDER BY source_revision")
                    .bind(&sk).bind(&start).fetch_all(&mut *tx).await?;
                let mut records = stored
                    .iter()
                    .map(|b| Record::parse(b))
                    .collect::<Result<Vec<_>>>()?;
                records.push(r.clone());
                let snapshot = assemble(&records)?;
                if let Some(snapshot) = snapshot {
                    let parents: std::collections::HashSet<_> = snapshot
                        .items
                        .iter()
                        .filter_map(|i| i.parent_item_instance_id.as_ref())
                        .collect();
                    for i in &snapshot.items {
                        let d = self.evidence.definition(&mut tx, i).await?;
                        if d.maximum_quantity < i.quantity {
                            return Err(Error::Invalid);
                        }
                        if parents.contains(&i.item_instance_id) && !d.container {
                            return Err(Error::Invalid);
                        }
                    }
                    let body = serde_json::to_vec(&snapshot).map_err(|_| Error::Invalid)?;
                    sqlx::query("UPDATE pd_t76_streams SET snapshot=$2,snapshot_at=$3,staging_start=NULL,staging_at=NULL WHERE stream_key=$1")
                        .bind(&sk).bind(body).bind(integer(snapshot.at_logout)?).execute(&mut *tx).await?;
                    // Obsolete snapshot payloads are erased immediately; replay hashes remain.
                    sqlx::query("UPDATE pd_t76_records SET canonical_bytes=NULL WHERE stream_key=$1 AND source_revision < $2::text::numeric")
                        .bind(&sk).bind(&start).execute(&mut *tx).await?;
                }
            }
            _ => {
                let (event, business) = event_identity(&r.payload).ok_or(Error::Invalid)?;
                let payload = serde_json::to_vec(&r.payload).map_err(|_| Error::Invalid)?;
                let pd = Sha256::digest(&payload).to_vec();
                let old:Option<Vec<u8>>=sqlx::query_scalar("SELECT payload_digest FROM pd_t76_events WHERE stream_key=$1 AND (event_id=$2 OR business_id=$3)")
                    .bind(&sk).bind(uuid(event)?).bind(uuid(business)?).fetch_optional(&mut *tx).await?;
                // An event with a new stream revision is not an exact wire retry.
                if old.is_some() {
                    return Err(Error::Conflict);
                }
                if let Payload::Bank(b) = &r.payload {
                    let previous: Option<String> = stream.try_get("last_bank_entry")?;
                    let balance: Option<String> = stream.try_get("last_bank_balance")?;
                    // V1 has no accepted partial-ledger baseline schema. Require source origin.
                    if previous != b.previous_entry_id
                        || balance.as_ref().is_some_and(|v| v != &b.balance_before)
                    {
                        return Err(Error::Conflict);
                    }
                    sqlx::query("UPDATE pd_t76_streams SET last_bank_entry=$2,last_bank_balance=$3 WHERE stream_key=$1")
                        .bind(&sk).bind(&b.entry_id).bind(&b.balance_after).execute(&mut *tx).await?;
                }
                sqlx::query("INSERT INTO pd_t76_events(stream_key,event_id,business_id,payload_digest,canonical_bytes,occurred_at) VALUES($1,$2,$3,$4,$5,$6)")
                    .bind(&sk).bind(uuid(event)?).bind(uuid(business)?).bind(pd).bind(bytes).bind(integer(counter(&r.occurred_at,false)?)?).execute(&mut *tx).await?;
            }
        }
        // Unique record IDs fence collisions across streams as well as cursor retries.
        let inserted=sqlx::query("INSERT INTO pd_t76_records(stream_key,source_revision,source_authority,generation,record_id,digest,canonical_bytes,occurred_at) VALUES($1,$2::text::numeric,$3,$4::text::numeric,$5,$6,$7,$8) ON CONFLICT DO NOTHING")
            .bind(&sk).bind(&r.source_revision).bind(&r.source_authority).bind(&r.recovery_generation).bind(uuid(&r.record_id)?).bind(digest).bind(bytes).bind(integer(counter(&r.occurred_at,false)?)?).execute(&mut *tx).await?;
        if inserted.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        sqlx::query("UPDATE pd_t76_streams SET cursor=$2::text::numeric,updated_at=now() WHERE stream_key=$1")
            .bind(&sk).bind(&r.source_revision).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn inventory(
        &self,
        actor: Uuid,
        world: Uuid,
        character: Uuid,
        query: &str,
    ) -> Result<Inventory> {
        if !self.flags.items {
            return Err(Error::Unavailable);
        }
        if query.len() > 128 || query.chars().any(char::is_control) {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        let scope = self
            .evidence
            .resolve(&mut tx, actor, world, Some(character), "ITEMS")
            .await?;
        check_resolved(&scope, actor, world, Some(character), "ITEMS")?;
        let proof = self
            .evidence
            .authorize(&mut tx, &scope, Access::Read(actor))
            .await?;
        let sk = scope.stream_key()?;
        let raw: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT snapshot FROM pd_t76_streams WHERE stream_key=$1 AND NOT fenced AND staging_start IS NULL",
        )
        .bind(&sk)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
        let snapshot: Snapshot = serde_json::from_slice(&raw.ok_or(Error::Unavailable)?)
            .map_err(|_| Error::Unavailable)?;
        if snapshot.at_logout.saturating_add(RETENTION) <= proof.now
            || snapshot.at_logout > proof.now.saturating_add(30)
        {
            return Err(Error::Unavailable);
        }
        let mut items = Vec::new();
        let mut total = Some(0u64);
        let mut unpriced = 0;
        for i in snapshot.items {
            let d = self.evidence.definition(&mut tx, &i).await?;
            let estimate = d
                .estimated_gold
                .filter(|_| d.price_source.as_ref().is_some_and(|s| !s.is_empty()))
                .filter(|v| *v <= GOLD_MAX)
                .and_then(|v| v.checked_mul(u64::from(i.quantity)));
            if estimate.is_none() {
                unpriced += 1;
            }
            total = total.zip(estimate).and_then(|(a, b)| a.checked_add(b));
            if i.definition_production_key
                .to_lowercase()
                .contains(&query.to_lowercase())
            {
                items.push(ValuedItem {
                    item: i,
                    estimate,
                    price_source: d.price_source,
                    bazaar_shortcut: self.flags.bazaar_shortcut && d.tradeable,
                });
            }
        }
        tx.commit().await?;
        Ok(Inventory {
            at_logout: snapshot.at_logout,
            snapshot_id: snapshot.snapshot_id,
            depots: snapshot.depots,
            items,
            estimated_total: total,
            unpriced,
        })
    }

    pub async fn history(
        &self,
        actor: Uuid,
        world: Uuid,
        character: Option<Uuid>,
        family: &str,
    ) -> Result<History> {
        if !self.flags.history {
            return Err(Error::Unavailable);
        }
        if !matches!(family, "BANK" | "MARKET" | "PLAYER_TRADE")
            || ((family == "BANK") != character.is_none())
        {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        let scope = self
            .evidence
            .resolve(&mut tx, actor, world, character, family)
            .await?;
        check_resolved(&scope, actor, world, character, family)?;
        let proof = self
            .evidence
            .authorize(&mut tx, &scope, Access::Read(actor))
            .await?;
        let fenced: Option<bool> =
            sqlx::query_scalar("SELECT fenced FROM pd_t76_streams WHERE stream_key=$1")
                .bind(scope.stream_key()?)
                .fetch_optional(&mut *tx)
                .await?;
        if fenced == Some(true) {
            return Err(Error::Unavailable);
        }
        if proof.coverage_through < proof.coverage_start
            || proof.coverage_through <= proof.now.saturating_sub(RETENTION)
        {
            return Err(Error::Unavailable);
        }
        // Returning a partial page as complete history would be misleading. Refuse
        // oversized results until a coverage-preserving pagination contract exists.
        let rows:Vec<Vec<u8>>=sqlx::query_scalar("SELECT canonical_bytes FROM pd_t76_records WHERE stream_key=$1 AND occurred_at>$2 AND occurred_at<=$3 AND canonical_bytes IS NOT NULL ORDER BY source_revision LIMIT 1001")
            .bind(scope.stream_key()?).bind(integer(proof.now.saturating_sub(RETENTION))?).bind(integer(proof.coverage_through.min(proof.now))?).fetch_all(&mut *tx).await?;
        if rows.len() > 1000 {
            return Err(Error::Unavailable);
        }
        let records = rows
            .iter()
            .map(|b| Record::parse(b))
            .collect::<Result<Vec<_>>>()?;
        // Empty history requires separately verified baseline in Evidence.
        tx.commit().await?;
        Ok(History {
            coverage_start: proof
                .coverage_start
                .max(proof.now.saturating_sub(RETENTION)),
            through: proof.coverage_through.min(proof.now),
            records,
        })
    }

    /// Apply a trusted T72 erasure/transfer event. Character erasure never touches
    /// BANK. This fences first and erases payloads in the same transaction; replay
    /// metadata survives until cleanup advances the durable floor.
    pub async fn erase(&self, account: Uuid, character: Option<Uuid>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE pd_t76_streams SET fenced=true,snapshot=NULL,snapshot_at=NULL,staging_start=NULL,staging_at=NULL,updated_at=now() WHERE account_id=$1 AND ($2::uuid IS NULL OR character_id=$2)")
            .bind(account).bind(character).execute(&mut *tx).await?;
        sqlx::query("UPDATE pd_t76_records SET canonical_bytes=NULL WHERE stream_key IN (SELECT stream_key FROM pd_t76_streams WHERE account_id=$1 AND ($2::uuid IS NULL OR character_id=$2))")
            .bind(account).bind(character).execute(&mut *tx).await?;
        sqlx::query("UPDATE pd_t76_events SET canonical_bytes=NULL WHERE stream_key IN (SELECT stream_key FROM pd_t76_streams WHERE account_id=$1 AND ($2::uuid IS NULL OR character_id=$2))")
            .bind(account).bind(character).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Called with T72's latest plausible trusted UTC; no wall-clock fallback.
    /// Purges payloads first and advances durable floors before dropping replay hashes.
    pub async fn cleanup(&self, now: u64) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE pd_t76_streams SET snapshot=NULL,snapshot_at=NULL WHERE snapshot_at<=$1",
        )
        .bind(integer(now.saturating_sub(RETENTION))?)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE pd_t76_records SET canonical_bytes=NULL WHERE occurred_at<=$1 OR stream_key IN (SELECT stream_key FROM pd_t76_streams WHERE staging_at<=$2)")
            .bind(integer(now.saturating_sub(RETENTION))?).bind(integer(now.saturating_sub(86400))?).execute(&mut *tx).await?;
        sqlx::query("UPDATE pd_t76_events SET canonical_bytes=NULL WHERE occurred_at<=$1")
            .bind(integer(now.saturating_sub(RETENTION))?)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE pd_t76_streams s SET retention_floor=GREATEST(retention_floor,(SELECT COALESCE(MAX(source_revision),0) FROM pd_t76_records r WHERE r.stream_key=s.stream_key AND occurred_at<=$1))")
            .bind(integer(now.saturating_sub(61*86400))?).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM pd_t76_records WHERE occurred_at<=$1")
            .bind(integer(now.saturating_sub(61 * 86400))?)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM pd_t76_events WHERE occurred_at<=$1")
            .bind(integer(now.saturating_sub(61 * 86400))?)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

fn check_resolved(
    s: &Scope,
    actor: Uuid,
    world: Uuid,
    character: Option<Uuid>,
    family: &str,
) -> Result<()> {
    if s.account_id != actor.to_string()
        || s.world_id != world.to_string()
        || s.character_id != character.map(|c| c.to_string())
        || s.family != family
    {
        return Err(Error::Denied);
    }
    Ok(())
}

/// Replays bounded staging, preserving the exact canonical page-payload digest.
pub fn assemble(records: &[Record]) -> Result<Option<Snapshot>> {
    let first = records.first().ok_or(Error::Conflict)?;
    let Payload::Begin(begin) = &first.payload else {
        return Err(Error::Conflict);
    };
    let mut items = Vec::new();
    let mut hash = Sha256::new();
    let mut pages = 0;
    for (offset, r) in records.iter().enumerate().skip(1) {
        if r.stream()? != first.stream()?
            || r.occurred_at != first.occurred_at
            || r.coverage_start_at != first.coverage_start_at
            || counter(&first.source_revision, false)?.checked_add(offset as u64)
                != Some(counter(&r.source_revision, false)?)
        {
            return Err(Error::Conflict);
        }
        match &r.payload {
            Payload::Page(p)
                if p.snapshot_id == begin.snapshot_id
                    && p.page_index == pages
                    && pages < begin.page_count =>
            {
                hash.update(serde_json::to_vec(p).map_err(|_| Error::Invalid)?);
                items.extend(p.items.clone());
                pages += 1;
            }
            Payload::End(e)
                if offset == records.len() - 1
                    && e.snapshot_id == begin.snapshot_id
                    && e.page_count == begin.page_count
                    && e.item_count == begin.item_count
                    && pages == e.page_count =>
            {
                if format!("{:x}", hash.finalize()) != e.pages_sha256 {
                    return Err(Error::Conflict);
                }
                graph(begin, &items)?;
                return Ok(Some(Snapshot {
                    at_logout: counter(&r.occurred_at, false)?,
                    snapshot_id: begin.snapshot_id.clone(),
                    depots: begin.depots.clone(),
                    items,
                }));
            }
            _ => return Err(Error::Conflict),
        }
    }
    Ok(None)
}
