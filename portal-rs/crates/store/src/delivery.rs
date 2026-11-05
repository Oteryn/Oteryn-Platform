//! Default-off Store delivery exchange. Mount only on the internal TLS listener.
//! Reconcile reports evidence; it never creates a purchase or grants value.
pub use crate::purchase::Error;
use crate::purchase::{Settings, encode, number, object, text};
use oteryn_internal_api::canonical::{self, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;
#[path = "delivery_http.rs"]
pub mod http;
#[path = "delivery_wire.rs"]
mod wire;
use wire::*;

#[derive(Debug, Clone)]
pub struct Delivery {
    pool: PgPool,
    settings: Settings,
}
impl Delivery {
    pub fn new(pool: PgPool, settings: Settings) -> Self {
        Self { pool, settings }
    }
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    pub fn enabled(&self) -> bool {
        let mut settings = self.settings.clone();
        settings.purchase_enabled = true;
        settings.valid()
    }
    fn check(&self) -> Result<(), Error> {
        if self.enabled() {
            Ok(())
        } else {
            Err(Error::Unavailable)
        }
    }
    fn request(&self, raw: &[u8], max: usize, operation: &str) -> Result<Value, Error> {
        self.check()?;
        let v = decode(raw, max)?;
        head(&v, operation)?;
        use subtle::ConstantTimeEq;
        if !bool::from(
            get(&v, "source_authority")?
                .as_bytes()
                .ct_eq(self.settings.source_authority.as_bytes()),
        ) {
            return Err(Error::Unauthorized);
        }
        Ok(v)
    }
    pub(super) async fn peer_limit(&self) -> Result<(), Error> {
        self.check()?;
        let row = sqlx::query("INSERT INTO store_wire_limits(scope,subject,attempts) VALUES('peer',$1,1) ON CONFLICT(scope,subject) DO UPDATE SET window_start=CASE WHEN store_wire_limits.window_start<=clock_timestamp()-interval '60 seconds' THEN clock_timestamp() ELSE store_wire_limits.window_start END, attempts=CASE WHEN store_wire_limits.window_start<=clock_timestamp()-interval '60 seconds' THEN 1 ELSE LEAST(store_wire_limits.attempts+1,$2::integer+1) END, updated_at=clock_timestamp() RETURNING attempts,GREATEST(1,CEIL(EXTRACT(EPOCH FROM window_start+interval '60 seconds'-clock_timestamp())))::integer AS retry")
            .bind(format!("{:x}",Sha256::digest(self.settings.peer_identity.as_bytes()))).bind(self.settings.requests_per_minute as i32).fetch_one(&self.pool).await?;
        if row.try_get::<i32, _>("attempts")? > self.settings.requests_per_minute as i32 {
            return Err(Error::Limited(row.try_get::<i32, _>("retry")?.max(1) as u32));
        }
        Ok(())
    }
    async fn begin(&self) -> Result<Transaction<'_, Postgres>, Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET LOCAL lock_timeout='3s'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SET LOCAL statement_timeout='5s'")
            .execute(&mut *tx)
            .await?;
        Ok(tx)
    }
    pub async fn pending(&self, raw: &[u8]) -> Result<Vec<u8>, Error> {
        self.page(raw, false).await
    }
    pub async fn reconcile(&self, raw: &[u8]) -> Result<Vec<u8>, Error> {
        self.page(raw, true).await
    }
    async fn page(&self, raw: &[u8], reconcile: bool) -> Result<Vec<u8>, Error> {
        let v = self.request(
            raw,
            512,
            if reconcile {
                "ListStoreOperationStatesV1"
            } else {
                "ListPendingStoreOperationsV1"
            },
        )?;
        let (revision, id, limit) = page(&v)?;
        let mut entries = Vec::new();
        let mut tx = self.begin().await?;
        // T29 holds this issuer lock through commit before allocating a revision.
        // A page cannot pass an uncommitted intent and lose it behind its cursor.
        sqlx::query("SELECT pg_advisory_xact_lock(29,1)")
            .execute(&mut *tx)
            .await?;
        if let Ok(revision) = i64::try_from(revision) {
            let rows = sqlx::query("SELECT o.operation_kind,o.operation_id,o.source_revision,o.state,r.receipt_digest,r.recovery_generation::text AS recovery_generation FROM store_operations o LEFT JOIN store_receipts r ON r.operation_id=o.operation_id AND r.issuer_authority=o.issuer_authority AND r.superseded_by_restore_event_id IS NULL WHERE o.issuer_authority='OTERYN_PLATFORM_STORE_AUTHORITY' AND (o.source_revision,o.operation_id)>($1,$2) AND ($4 OR r.receipt_id IS NULL) ORDER BY o.source_revision,o.operation_id LIMIT $3")
                .bind(revision).bind(id).bind(limit).bind(reconcile).fetch_all(&mut *tx).await?;
            for row in rows {
                let state: String = row.try_get("state")?;
                let digest: Option<String> = row.try_get("receipt_digest")?;
                let generation: Option<String> = row.try_get("recovery_generation")?;
                if !matches!(
                    state.as_str(),
                    "PENDING" | "REOPENED" | "COMMITTED" | "REJECTED"
                ) || matches!(state.as_str(), "PENDING" | "REOPENED") != digest.is_none()
                    || digest.is_none() != generation.is_none()
                {
                    return Err(Error::Unavailable);
                }
                let mut fields = vec![
                    (
                        "operation_kind",
                        text(row.try_get::<String, _>("operation_kind")?),
                    ),
                    (
                        "operation_id",
                        text(row.try_get::<Uuid, _>("operation_id")?),
                    ),
                    (
                        "source_revision",
                        text(row.try_get::<i64, _>("source_revision")?),
                    ),
                ];
                if reconcile {
                    fields.extend([
                        ("state", text(state)),
                        ("receipt_digest", digest.map(text).unwrap_or(Value::Null)),
                        (
                            "recovery_generation",
                            generation.map(text).unwrap_or(Value::Null),
                        ),
                    ]);
                }
                entries.push(object(fields));
            }
        }
        let bytes = encode(
            &object([
                ("contract_version", number(1)),
                (
                    if reconcile { "operations" } else { "pending" },
                    Value::Array(entries),
                ),
            ]),
            if reconcile { 12288 } else { 8192 },
        )?;
        tx.commit().await?;
        Ok(bytes)
    }
    pub async fn read(&self, raw: &[u8]) -> Result<Vec<u8>, Error> {
        let v = self.request(raw, 512, "ReadStoreOperationIntentV1")?;
        exact(
            &v,
            &[HEAD, "issuer_authority operation_kind operation_id"].join(" "),
        )?;
        let (kind, id) = key(&v)?;
        sqlx::query_scalar("SELECT intent_bytes FROM store_operations WHERE issuer_authority=$1 AND operation_kind=$2 AND operation_id=$3")
            .bind(ISSUER).bind(kind).bind(id).fetch_optional(&self.pool).await?.ok_or(Error::UnknownAccount)
    }
    pub async fn receipt(&self, raw: &[u8]) -> Result<Vec<u8>, Error> {
        let v = self.request(raw, 1024, "PublishStoreDeliveryReceiptV1")?;
        receipt(&v)?;
        let (kind, id) = key(&v)?;
        let mut tx = self.begin().await?;
        lock_restore(&mut tx, false).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(29,1)")
            .execute(&mut *tx)
            .await?;
        let op = sqlx::query("SELECT intent_bytes,entitlement_id,state FROM store_operations WHERE issuer_authority=$1 AND operation_kind=$2 AND operation_id=$3 FOR UPDATE")
            .bind(ISSUER).bind(kind).bind(id).fetch_optional(&mut *tx).await?.ok_or_else(|| { tracing::warn!("products_entitlements.store_receipt_without_intent"); Error::UnknownAccount })?;
        if let Some(current) = sqlx::query_scalar::<_,Vec<u8>>("SELECT receipt_bytes FROM store_receipts WHERE issuer_authority=$1 AND operation_id=$2 AND superseded_by_restore_event_id IS NULL").bind(ISSUER).bind(id).fetch_optional(&mut *tx).await? {
            if current != raw { tracing::warn!("products_entitlements.store_receipt_conflict"); return Err(Error::Conflict); }
            tx.commit().await?;
            return Ok(ACCEPTED.to_vec());
        }
        let state: String = op.try_get("state")?;
        if !matches!(state.as_str(), "PENDING" | "REOPENED") {
            return Err(Error::Unavailable);
        }
        let stale: bool = sqlx::query_scalar(
            "SELECT new_recovery_generation>$1::text::numeric FROM store_delivery_fence WHERE id=1",
        )
        .bind(get(&v, "recovery_generation")?)
        .fetch_one(&mut *tx)
        .await?;
        if stale {
            tracing::warn!("products_entitlements.store_receipt_lost_to_restore");
            return Err(Error::Conflict);
        }
        let intent_bytes: Vec<u8> = op.try_get("intent_bytes")?;
        if get(&v, "binding_digest")? != format!("{:x}", Sha256::digest(&intent_bytes)) {
            tracing::warn!("products_entitlements.store_receipt_conflict");
            return Err(Error::Conflict);
        }
        let intent = decode(&intent_bytes, 4096).map_err(|_| Error::Unavailable)?;
        let unit: Uuid = op.try_get("entitlement_id")?;
        let quantity: i32 = sqlx::query_scalar(
            "SELECT quantity FROM store_fulfilment_units WHERE entitlement_id=$1 FOR UPDATE",
        )
        .bind(unit)
        .fetch_one(&mut *tx)
        .await?;
        assert_binding(&v, &intent, quantity)?;
        if kind == "RECLAIM" && get(&v, "outcome")? == "COMMITTED" {
            reclaim(&mut tx, &v, &intent).await?;
        }
        let receipt_id = Uuid::now_v7();
        sqlx::query("INSERT INTO store_receipts(receipt_id,issuer_authority,operation_id,operation_kind,receipt_bytes,receipt_digest,binding_digest,outcome,result_code,recovery_generation,decided_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10::text::numeric,$11::text::numeric)")
            .bind(receipt_id).bind(ISSUER).bind(id).bind(kind).bind(raw).bind(format!("{:x}",Sha256::digest(raw))).bind(get(&v,"binding_digest")?).bind(get(&v,"outcome")?).bind(get(&v,"result_code")?).bind(get(&v,"recovery_generation")?).bind(get(&v,"decided_at")?).execute(&mut *tx).await?;
        sqlx::query("UPDATE store_operations SET state=$2,updated_at=clock_timestamp() WHERE operation_id=$1").bind(id).bind(get(&v,"outcome")?).execute(&mut *tx).await?;
        // Only the paid ITEM delivery creates a CLAIM. Re-deciding it after
        // restore reuses that CLAIM; reconcile never creates an operation.
        if kind == "DELIVER"
            && get(&v, "outcome")? == "COMMITTED"
            && get(&intent, "unit_kind")? == "ITEM"
        {
            claim(&mut tx, &intent, id, unit).await?;
        }
        refresh_unit(&mut tx, unit).await?;
        sqlx::query("UPDATE store_reconcile_state SET resolved_receipt_id=$2,updated_at=clock_timestamp() WHERE operation_id=$1 AND resolved_receipt_id IS NULL").bind(id).bind(receipt_id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(ACCEPTED.to_vec())
    }
    /// Called by the restore-notice owner inside its notice transaction, before
    /// locking any intent. The caller must validate and persist the notice and
    /// verify exchange availability before committing. No receipt is discarded.
    pub async fn reopen_in(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        event: Uuid,
        restored: u64,
        generation: u64,
    ) -> Result<u64, Error> {
        self.check()?;
        if event.get_version_num() != 7 || restored == 0 || generation <= restored {
            return Err(Error::Invalid);
        }
        lock_restore(tx, true).await?;
        let old: String = sqlx::query_scalar(
            "SELECT new_recovery_generation::text FROM store_delivery_fence WHERE id=1 FOR UPDATE",
        )
        .fetch_one(&mut **tx)
        .await?;
        if generation <= old.parse::<u64>().map_err(|_| Error::Unavailable)? {
            return Err(Error::Conflict);
        }
        sqlx::query("SELECT pg_advisory_xact_lock(29,1)")
            .execute(&mut **tx)
            .await?;
        let rows = sqlx::query("SELECT o.operation_id,o.entitlement_id,o.operation_kind FROM store_operations o LEFT JOIN store_receipts r ON r.operation_id=o.operation_id AND r.superseded_by_restore_event_id IS NULL WHERE r.recovery_generation >= $1::text::numeric OR o.state='REOPENED' ORDER BY o.source_revision FOR UPDATE OF o")
            .bind(restored.to_string()).fetch_all(&mut **tx).await?;
        for row in &rows {
            let id: Uuid = row.try_get("operation_id")?;
            sqlx::query("UPDATE store_receipts SET superseded_by_restore_event_id=$2 WHERE operation_id=$1 AND superseded_by_restore_event_id IS NULL").bind(id).bind(event).execute(&mut **tx).await?;
            sqlx::query("UPDATE store_operations SET state='REOPENED',updated_at=clock_timestamp() WHERE operation_id=$1").bind(id).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO store_reconcile_state(restore_event_id,operation_id,new_recovery_generation) VALUES($1,$2,$3::text::numeric)").bind(event).bind(id).bind(generation.to_string()).execute(&mut **tx).await?;
        }
        for row in &rows {
            refresh_unit(tx, row.try_get("entitlement_id")?).await?;
        }
        sqlx::query("UPDATE store_delivery_fence SET new_recovery_generation=$1::text::numeric,updated_at=clock_timestamp() WHERE id=1").bind(generation.to_string()).execute(&mut **tx).await?;
        Ok(rows.len() as u64)
    }
}
const ACCEPTED: &[u8] = br#"{"contract_version":1,"result":"accepted"}"#;
async fn lock_restore(tx: &mut Transaction<'_, Postgres>, exclusive: bool) -> Result<(), Error> {
    let found: Option<i32> = sqlx::query_scalar(if exclusive {
        "SELECT id::integer FROM character_restore_locks WHERE id=1 FOR UPDATE"
    } else {
        "SELECT id::integer FROM character_restore_locks WHERE id=1 FOR SHARE"
    })
    .fetch_optional(&mut **tx)
    .await?;
    if found != Some(1) {
        return Err(Error::Unavailable);
    }
    Ok(())
}
async fn refresh_unit(tx: &mut Transaction<'_, Postgres>, unit: Uuid) -> Result<(), Error> {
    // Derive presentation from all current evidence, so a surviving claim or
    // reversal cannot be downgraded by a re-decided delivery after restore.
    let current = sqlx::query("SELECT r.operation_kind,r.outcome,r.result_code FROM store_receipts r JOIN store_operations o USING(operation_id) WHERE o.entitlement_id=$1 AND r.superseded_by_restore_event_id IS NULL")
        .bind(unit).fetch_all(&mut **tx).await?;
    let mut state = "DELIVERY_PENDING";
    let mut priority = 0;
    for row in current {
        let kind: String = row.try_get("operation_kind")?;
        let outcome: String = row.try_get("outcome")?;
        let code: String = row.try_get("result_code")?;
        let (rank, next) = match (kind.as_str(), outcome.as_str(), code.as_str()) {
            ("RECLAIM", "COMMITTED", "STORE_RECLAIM_SHORTFALL") => (7, "REVERSAL_PARTIAL"),
            (_, "REJECTED", "STORE_LINE_REVOKED") => (4, "REVOKED"),
            (_, "REJECTED", _) => (6, "MANUAL_REVIEW"),
            ("FLAG", "COMMITTED", _) => (6, "MANUAL_REVIEW"),
            ("RECLAIM", "COMMITTED", _) => (5, "RECLAIMED"),
            ("REVOKE_UNCLAIMED", "COMMITTED", _) => (4, "REVOKED"),
            ("CLAIM", "COMMITTED", _) => (3, "CLAIMED"),
            ("DELIVER", "COMMITTED", _) => (1, "DELIVERED"),
            _ => return Err(Error::Unavailable),
        };
        if rank > priority {
            priority = rank;
            state = next;
        }
    }
    sqlx::query("UPDATE store_fulfilment_units SET state=$2,updated_at=clock_timestamp() WHERE entitlement_id=$1")
        .bind(unit).bind(state).execute(&mut **tx).await?;
    Ok(())
}
async fn reclaim(
    tx: &mut Transaction<'_, Postgres>,
    v: &Value,
    intent: &Value,
) -> Result<(), Error> {
    let bytes: Vec<u8> = sqlx::query_scalar("SELECT receipt_bytes FROM store_receipts WHERE issuer_authority=$1 AND operation_id=$2 AND receipt_digest=$3 AND operation_kind='CLAIM' AND outcome='COMMITTED'")
        .bind(ISSUER).bind(uuid(get(intent,"claim_operation_id")?)?).bind(get(intent,"claim_receipt_digest")?).fetch_optional(&mut **tx).await?.ok_or(Error::Unavailable)?;
    let claim = decode(&bytes, 1024).map_err(|_| Error::Unavailable)?;
    for field in [
        "account_id",
        "entitlement_id",
        "product_id",
        "product_version",
        "character_id",
        "world_id",
    ] {
        if v.get(field) != claim.get(field) {
            return Err(Error::Invalid);
        }
    }
    let short = integer(v, "shortfall_quantity", 0, 65535)?;
    if integer(v, "reclaimed_quantity", 0, 65535)? + short
        != integer(&claim, "minted_quantity", 1, 65535)?
        || (get(v, "result_code")? == "STORE_RECLAIMED") != (short == 0)
    {
        return Err(Error::Invalid);
    }
    Ok(())
}
async fn claim(
    tx: &mut Transaction<'_, Postgres>,
    intent: &Value,
    delivery: Uuid,
    unit: Uuid,
) -> Result<(), Error> {
    // Re-deciding a restored DELIVER retains its original CLAIM identity.
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM store_operations WHERE entitlement_id=$1 AND operation_kind='CLAIM')").bind(unit).fetch_one(&mut **tx).await?;
    if exists {
        return Ok(());
    }
    let id = Uuid::now_v7();
    let revision: i64 = sqlx::query_scalar(
        "SELECT nextval(pg_get_serial_sequence('store_operations','source_revision'))",
    )
    .fetch_one(&mut **tx)
    .await?;
    let mut fields = vec![
        ("contract_version", number(1)),
        ("operation_kind", text("CLAIM")),
        ("issuer_authority", text(ISSUER)),
        ("source_revision", text(revision)),
        ("operation_id", text(id)),
        ("delivery_operation_id", text(delivery)),
    ];
    for field in [
        "entitlement_id",
        "account_id",
        "product_id",
        "product_version",
        "unit_kind",
        "grant_key",
        "grant_revision",
        "quantity",
    ] {
        fields.push((field, intent.get(field).cloned().ok_or(Error::Unavailable)?));
    }
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    if now <= 0 {
        return Err(Error::Unavailable);
    }
    fields.extend([
        ("issued_at_source", text(now)),
        ("audience", text("OTERYN_GAME_STORE")),
    ]);
    let bytes = encode(&object(fields), 4096)?;
    sqlx::query("INSERT INTO store_operations(source_revision,operation_id,issuer_authority,operation_kind,entitlement_id,intent_bytes,binding_digest) VALUES($1,$2,$3,'CLAIM',$4,$5,$6)").bind(revision).bind(id).bind(ISSUER).bind(unit).bind(&bytes).bind(format!("{:x}",Sha256::digest(&bytes))).execute(&mut **tx).await?;
    Ok(())
}
#[cfg(test)]
#[path = "delivery_tests.rs"]
mod tests;
