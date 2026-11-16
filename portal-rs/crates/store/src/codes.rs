//! Gift code issuance and atomic redemption. Item codes remain unavailable on the browser surface.
use crate::premium;
use oteryn_wallet::{AccountStateReader, AuditHook, TransferBlockCheck};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use subtle::ConstantTimeEq;
use time::OffsetDateTime;
use uuid::Uuid;

const ALPHABET: &[u8; 32] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

#[derive(Debug, Default, Clone, Copy)]
pub struct CodeFlags {
    pub enabled: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Disabled,
    Denied,
    Invalid,
    Refused,
    Conflict,
    Unavailable,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "code operation refused: {self:?}")
    }
}
impl std::error::Error for Error {}
impl From<sqlx::Error> for Error {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}
/// Application-owned authorization boundary; implementations acquire the shared
/// staff account/fence protocol and revalidate the live session in this connection.
pub trait StaffAuthorization: Send + Sync {
    fn authorize(
        &self,
        connection: &mut sqlx::PgConnection,
        actor: Uuid,
    ) -> impl std::future::Future<Output = Result<(), Error>> + Send;
}
/// Internal issuance capability. Production issuance uses `generate_staff`,
/// which checks current permissions in the issuance transaction.
#[derive(Debug)]
pub struct AuthorizedStaff {
    actor: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Coins,
    PremiumDays,
    ItemGrant,
}
impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Coins => "coins",
            Self::PremiumDays => "premium_days",
            Self::ItemGrant => "item_grant",
        }
    }
}
#[derive(Debug, Clone)]
pub struct Batch {
    pub kind: Kind,
    pub amount: i64,
    pub max_per_code: i32,
    /// Account limit applies across every code in this batch.
    pub max_per_account: i32,
    pub valid_from: OffsetDateTime,
    pub valid_until: OffsetDateTime,
    pub note: String,
}
/// Debug output exposes only the batch identifier and count.
pub struct Generated {
    pub batch_id: Uuid,
    pub codes: Vec<String>,
}
impl std::fmt::Debug for Generated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Generated")
            .field("batch_id", &self.batch_id)
            .field("count", &self.codes.len())
            .finish()
    }
}
#[derive(Clone)]
pub struct Redemption {
    pub code: String,
    pub account: Uuid,
    pub character_ref: Option<Uuid>,
    pub key: Uuid,
}
impl std::fmt::Debug for Redemption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Redemption")
            .field("account", &self.account)
            .field("character_ref", &self.character_ref)
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Receipt {
    pub redemption_id: Uuid,
    pub kind: Kind,
    pub amount: i64,
}
/// T04a supplies the adapter. Failure denies the attempt before lookup; this
/// hook must also account for malformed inputs. No limiter lives in this core.
pub trait AttemptHook: Send + Sync {
    fn check(&self, account: Uuid) -> impl std::future::Future<Output = Result<(), Error>> + Send;
}
#[derive(Debug)]
pub struct Codes<A, B, H, R> {
    pool: PgPool,
    flags: CodeFlags,
    wallet: oteryn_wallet::Ledger<A, B, H>,
    premium: premium::repository::Ledger,
    attempts: R,
}
impl<A: AccountStateReader, B: TransferBlockCheck, H: AuditHook, R: AttemptHook> Codes<A, B, H, R> {
    pub fn new(
        pool: PgPool,
        flags: CodeFlags,
        wallet: oteryn_wallet::Ledger<A, B, H>,
        premium: premium::repository::Ledger,
        attempts: R,
    ) -> Self {
        Self {
            pool,
            flags,
            wallet,
            premium,
            attempts,
        }
    }
    fn enabled(&self) -> Result<(), Error> {
        if self.flags.enabled {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }
    /// Called only after the browser staff MFA guard. Recheck current role grants
    /// under the role-writer fence and retain it through issuance and audit.
    pub async fn generate_staff(
        &self,
        actor: Uuid,
        authorization: &impl StaffAuthorization,
        batch: &Batch,
        count: usize,
    ) -> Result<Generated, Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        authorization.authorize(&mut tx, actor).await?;
        if batch.kind == Kind::ItemGrant {
            return Err(Error::Disabled);
        }
        let result = self
            .generate_in(&mut tx, &AuthorizedStaff { actor }, batch, count)
            .await?;
        sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES($1,'store.codes.generated',$2,'{}'::jsonb,jsonb_build_object('count',$3::bigint),$4)")
            .bind(actor).bind(result.batch_id.to_string()).bind(count as i64).bind(result.batch_id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(result)
    }
    pub async fn generate(
        &self,
        staff: &AuthorizedStaff,
        batch: &Batch,
        count: usize,
    ) -> Result<Generated, Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let result = self.generate_in(&mut tx, staff, batch, count).await?;
        tx.commit().await?;
        Ok(result)
    }
    async fn generate_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        staff: &AuthorizedStaff,
        batch: &Batch,
        count: usize,
    ) -> Result<Generated, Error> {
        self.enabled()?;
        if !valid_id(staff.actor)
            || !(1..=10000).contains(&count)
            || batch.amount <= 0
            || (batch.kind == Kind::PremiumDays && batch.amount > 366)
            || batch.max_per_code <= 0
            || batch.max_per_account <= 0
            || batch.valid_from >= batch.valid_until
            || !(1..=500).contains(&batch.note.chars().count())
        {
            return Err(Error::Invalid);
        }
        let batch_id = Uuid::now_v7();
        sqlx::query("INSERT INTO code_batches(id,kind,amount,max_per_code,max_per_account,valid_from,valid_until,created_by,note) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)")
            .bind(batch_id).bind(batch.kind.name()).bind(batch.amount).bind(batch.max_per_code).bind(batch.max_per_account)
            .bind(batch.valid_from).bind(batch.valid_until).bind(staff.actor).bind(&batch.note).execute(&mut **tx).await?;
        let mut codes = Vec::with_capacity(count);
        for _ in 0..count {
            let mut random = [0u8; 16];
            getrandom::fill(&mut random).map_err(|_| Error::Unavailable)?;
            // A power-of-two alphabet avoids modulo bias.
            let plain: String = random
                .iter()
                .map(|b| char::from(ALPHABET[usize::from(b & 31)]))
                .collect();
            let hash = Sha256::digest(plain.as_bytes()).to_vec();
            sqlx::query(
                "INSERT INTO gift_codes(id,batch_id,code_hash,support_prefix) VALUES ($1,$2,$3,$4)",
            )
            .bind(Uuid::now_v7())
            .bind(batch_id)
            .bind(hash)
            .bind(&plain[..4])
            .execute(&mut **tx)
            .await?;
            codes.push(format!(
                "{}-{}-{}-{}",
                &plain[..4],
                &plain[4..8],
                &plain[8..12],
                &plain[12..]
            ));
        }
        Ok(Generated { batch_id, codes })
    }
    /// Browser adapter entry: the caller must charge the authenticated attempt
    /// budget before decoding the body, including requests that never reach here.
    /// Only account rewards are exposed; no character authority is inferred.
    pub async fn redeem_player(&self, request: &Redemption) -> Result<Receipt, Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let active: Option<bool> = sqlx::query_scalar(
            "SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1",
        )
        .bind(request.account)
        .fetch_optional(&mut *tx)
        .await?;
        if active != Some(true) {
            return Err(Error::Denied);
        }
        // Item batches have no item/product binding or qualified consumer yet.
        // Never expose an unfulfillable queued command through production.
        let hash = normalized_hash(&request.code)?;
        let item: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM gift_codes c JOIN code_batches b ON b.id=c.batch_id WHERE c.code_hash=$1 AND b.kind='item_grant')")
            .bind(hash.as_slice()).fetch_one(&mut *tx).await?;
        if item || request.character_ref.is_some() {
            return Err(Error::Disabled);
        }
        let result = self.redeem_in(&mut tx, request, true).await?;
        tx.commit().await?;
        Ok(result)
    }
    pub async fn redeem(&self, request: &Redemption) -> Result<Receipt, Error> {
        self.redeem_impl(request, true).await
    }
    async fn redeem_impl(&self, request: &Redemption, charge: bool) -> Result<Receipt, Error> {
        self.enabled()?;
        if charge {
            self.attempts.check(request.account).await?;
        }
        let mut tx = self.pool.begin().await?;
        let result = self.redeem_in(&mut tx, request, false).await?;
        tx.commit().await?;
        Ok(result)
    }
    async fn redeem_in(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        request: &Redemption,
        browser: bool,
    ) -> Result<Receipt, Error> {
        if !valid_id(request.account)
            || !valid_id(request.key)
            || request.character_ref.is_some_and(|id| !valid_id(id))
        {
            return Err(Error::Invalid);
        }
        let hash = normalized_hash(&request.code)?;
        // Global key lock handles concurrent reuse across unrelated code rows.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,55))")
            .bind(request.key.to_string())
            .execute(&mut **tx)
            .await?;
        let row = sqlx::query("SELECT c.id,c.code_hash,c.batch_id,b.kind,b.amount,b.max_per_code,b.max_per_account,b.created_by,b.valid_from,b.valid_until FROM gift_codes c JOIN code_batches b ON b.id=c.batch_id WHERE c.code_hash=$1 FOR UPDATE OF c")
            .bind(hash.as_slice()).fetch_optional(&mut **tx).await?;
        // Index lookup is bounded; hash equality is additionally checked in constant
        // time. PostgreSQL index traversal itself has no timing guarantee.
        let stored = row
            .as_ref()
            .map(|r| r.try_get::<Vec<u8>, _>("code_hash"))
            .transpose()?
            .unwrap_or_else(|| vec![0; 32]);
        if !bool::from(hash.as_slice().ct_eq(&stored)) {
            return Err(Error::Refused);
        }
        let row = row.ok_or(Error::Refused)?;
        let code_id: Uuid = row.try_get("id")?;
        let batch_id: Uuid = row.try_get("batch_id")?;
        let kind = match row.try_get::<&str, _>("kind")? {
            "coins" => Kind::Coins,
            "premium_days" => Kind::PremiumDays,
            "item_grant" => Kind::ItemGrant,
            _ => return Err(Error::Unavailable),
        };
        let amount: i64 = row.try_get("amount")?;
        // Request, code and batch locks precede identity locks on every path.
        // Serialize all codes of the same batch for this account before counting.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,56))")
            .bind(format!("{batch_id}:{}", request.account))
            .execute(&mut **tx)
            .await?;
        if browser {
            // The admission read may have preceded a code/batch wait. Recheck
            // current eligibility under the retained account lock before replay
            // or effects; no account lock may be retained while waiting above.
            let active: Option<bool> = sqlx::query_scalar("SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(request.account).fetch_optional(&mut **tx).await?;
            if active != Some(true) {
                return Err(Error::Denied);
            }
        }
        if let Some(previous) = sqlx::query("SELECT id,code_id,account_id,character_ref FROM code_redemptions WHERE idempotency_key=$1").bind(request.key).fetch_optional(&mut **tx).await? {
            if previous.try_get::<Uuid,_>("code_id")? != code_id || previous.try_get::<Uuid,_>("account_id")? != request.account || previous.try_get::<Option<Uuid>,_>("character_ref")? != request.character_ref { return Err(Error::Conflict); }
            return Ok(Receipt { redemption_id: previous.try_get("id")?, kind, amount });
        }
        if browser {
            // Exact replays above remain valid; new value requires the shared gate.
            let eligible: bool = sqlx::query_scalar("SELECT account_value_eligible($1)")
                .bind(request.account)
                .fetch_one(&mut **tx)
                .await?;
            if !eligible {
                return Err(Error::Denied);
            }
        }
        let now: OffsetDateTime = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut **tx)
            .await?;
        if now < row.try_get::<OffsetDateTime, _>("valid_from")?
            || now >= row.try_get::<OffsetDateTime, _>("valid_until")?
        {
            return Err(Error::Refused);
        }
        let used: i64 =
            sqlx::query_scalar("SELECT count(*) FROM code_redemptions WHERE code_id=$1")
                .bind(code_id)
                .fetch_one(&mut **tx)
                .await?;
        let by_account: i64 = sqlx::query_scalar("SELECT count(*) FROM code_redemptions r JOIN gift_codes c ON c.id=r.code_id WHERE c.batch_id=$1 AND r.account_id=$2").bind(batch_id).bind(request.account).fetch_one(&mut **tx).await?;
        if used >= i64::from(row.try_get::<i32, _>("max_per_code")?)
            || by_account >= i64::from(row.try_get::<i32, _>("max_per_account")?)
        {
            return Err(Error::Refused);
        }
        if kind == Kind::ItemGrant && request.character_ref.is_none() {
            return Err(Error::Invalid);
        }
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO code_redemptions(id,code_id,account_id,character_ref,idempotency_key) VALUES ($1,$2,$3,$4,$5)").bind(id).bind(code_id).bind(request.account).bind(request.character_ref).bind(request.key).execute(&mut **tx).await?;
        let actor = row.try_get("created_by")?;
        match kind {
            Kind::Coins => {
                self.wallet
                    .adjust_in(
                        tx,
                        request.key,
                        request.account,
                        amount,
                        actor,
                        "Gift code redemption",
                    )
                    .await
                    .map_err(|_| Error::Refused)?;
            }
            Kind::PremiumDays => {
                self.premium
                    .mutate_in(
                        tx,
                        &premium::Operator {
                            actor,
                            confirmed_mfa: true,
                            permission: "products.premium.manage".into(),
                        },
                        &premium::Mutation {
                            account: request.account,
                            request: request.key,
                            change: premium::Change::Grant(
                                u16::try_from(amount).map_err(|_| Error::Unavailable)?,
                            ),
                            reason: "Gift code redemption".into(),
                        },
                    )
                    .await
                    .map_err(|_| Error::Refused)?;
            }
            Kind::ItemGrant => {
                sqlx::query("INSERT INTO code_game_commands(redemption_id,character_ref,amount) VALUES ($1,$2,$3)").bind(id).bind(request.character_ref).bind(amount).execute(&mut **tx).await?;
            }
        }
        Ok(Receipt {
            redemption_id: id,
            kind,
            amount,
        })
    }
}
fn valid_id(id: Uuid) -> bool {
    id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122
}
fn normalized_hash(input: &str) -> Result<[u8; 32], Error> {
    if input.len() > 64 {
        return Err(Error::Invalid);
    }
    let normalized: String = input
        .trim()
        .bytes()
        .filter(|b| *b != b'-')
        .map(|b| char::from(b.to_ascii_uppercase()))
        .collect();
    if normalized.len() != 16 || !normalized.bytes().all(|b| ALPHABET.contains(&b)) {
        return Err(Error::Invalid);
    }
    Ok(Sha256::digest(normalized.as_bytes()).into())
}
#[cfg(test)]
#[path = "codes_tests.rs"]
pub(crate) mod tests;
