use oteryn_db::outbox::{self, OffsetDateTime, PgPool, Uuid};
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn refused() -> Box<dyn std::error::Error + Send + Sync> {
    "hireling operation refused or invalid".into()
}
/// Both switches default off. No worker for this command kind is registered.
#[derive(Debug, Default, Clone, Copy)]
pub struct Flags {
    pub enabled: bool,
    pub command_outbox: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Profession {
    Miner,
    Herbalist,
    Fisher,
    Merchant,
}
impl Profession {
    fn key(self) -> &'static str {
        match self {
            Self::Miner => "miner",
            Self::Herbalist => "herbalist",
            Self::Fisher => "fisher",
            Self::Merchant => "merchant",
        }
    }
}
/// Supplied by a reviewed catalogue; never inferred from existing house services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reward {
    pub item: String,
    pub minimum: u32,
    pub maximum: u32,
    pub chance_ppm: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Destination {
    pub key: String,
    pub profession: Profession,
    pub hours: u8,
    pub weekly_limit: i32,
    pub rewards: Vec<Reward>,
}
impl Destination {
    pub fn validate(&self) -> Result<()> {
        if self.key.is_empty()
            || self.key.len() > 100
            || !(1..=12).contains(&self.hours)
            || self.weekly_limit <= 0
            || self.rewards.is_empty()
            || self.rewards.len() > 100
            || self.rewards.iter().any(|r| {
                r.item.is_empty()
                    || r.item.len() > 100
                    || r.minimum > r.maximum
                    || r.maximum == 0
                    || !(1..=1_000_000).contains(&r.chance_ppm)
            })
        {
            return Err(refused());
        }
        Ok(())
    }
}
/// Adapter must verify active account, current house ownership and committed store
/// entitlement, including the profession. Unknown or stale evidence returns false.
pub trait Eligibility: std::fmt::Debug + Send + Sync {
    fn active_account(&self, _account: Uuid) -> bool {
        false
    }
    fn allowed(
        &self,
        account: Uuid,
        house: Uuid,
        entitlement: Uuid,
        profession: Profession,
    ) -> bool;
}
#[derive(Debug)]
pub struct DenyEligibility;
impl Eligibility for DenyEligibility {
    fn allowed(&self, _: Uuid, _: Uuid, _: Uuid, _: Profession) -> bool {
        false
    }
}
#[derive(Debug)]
pub struct Hirelings<E> {
    pub pool: PgPool,
    pub flags: Flags,
    /// Reviewed account-wide cap; zero denies expedition creation.
    pub account_weekly_limit: i32,
    pub eligibility: E,
}
#[derive(Debug, sqlx::FromRow)]
pub struct Hireling {
    pub id: Uuid,
    pub house_id: Uuid,
    pub profession: String,
    pub level: i32,
}
#[derive(Debug, sqlx::FromRow)]
pub struct Expedition {
    pub id: Uuid,
    pub hireling_id: Uuid,
    pub destination: String,
    pub state: String,
    pub reward_table: String,
}
impl<E: Eligibility> Hirelings<E> {
    /// Projects a committed store entitlement, never debits a wallet locally.
    pub async fn purchase(
        &self,
        account: Uuid,
        house: Uuid,
        entitlement: Uuid,
        profession: Profession,
    ) -> Result<Uuid> {
        if !self.flags.enabled
            || !self.eligibility.active_account(account)
            || [account, house, entitlement].iter().any(Uuid::is_nil)
            || !self
                .eligibility
                .allowed(account, house, entitlement, profession)
        {
            return Err(refused());
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,92))")
            .bind(account.to_string())
            .execute(&mut *tx)
            .await?;
        let old: Option<(Uuid, Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT id,account_id,house_id,profession FROM hirelings WHERE entitlement_id=$1",
        )
        .bind(entitlement)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((id, owner, old_house, old_profession)) = old {
            if owner != account || house != old_house || old_profession != profession.key() {
                return Err(refused());
            }
            return Ok(id);
        }
        let slot: Option<i16> = sqlx::query_scalar("SELECT s::smallint FROM generate_series(1,3) s WHERE NOT EXISTS(SELECT 1 FROM hirelings WHERE account_id=$1 AND slot=s) ORDER BY s LIMIT 1").bind(account).fetch_optional(&mut *tx).await?;
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO hirelings(id,account_id,house_id,entitlement_id,profession,slot) VALUES($1,$2,$3,$4,$5,$6)").bind(id).bind(account).bind(house).bind(entitlement).bind(profession.key()).bind(slot.ok_or_else(refused)?).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(id)
    }
    pub async fn state(&self, account: Uuid) -> Result<Vec<Hireling>> {
        if !self.flags.enabled || !self.eligibility.active_account(account) || account.is_nil() {
            return Err(refused());
        }
        Ok(sqlx::query_as(
            "SELECT id,house_id,profession,level FROM hirelings WHERE account_id=$1 ORDER BY slot",
        )
        .bind(account)
        .fetch_all(&self.pool)
        .await?)
    }
    pub async fn history(&self, account: Uuid) -> Result<Vec<Expedition>> {
        if !self.flags.enabled || !self.eligibility.active_account(account) || account.is_nil() {
            return Err(refused());
        }
        Ok(sqlx::query_as("SELECT id,hireling_id,destination,state,reward_table::text AS reward_table FROM hireling_expeditions WHERE account_id=$1 ORDER BY created_at DESC,id DESC LIMIT 100").bind(account).fetch_all(&self.pool).await?)
    }
    /// Caller supplies a trusted catalogue entry, never client-controlled rewards.
    /// Reserves one account-wide weekly slot; game receipts will govern actual loot.
    pub async fn start(
        &self,
        account: Uuid,
        hireling: Uuid,
        request: Uuid,
        destination: &Destination,
    ) -> Result<Uuid> {
        destination.validate()?;
        if self.account_weekly_limit <= 0 || destination.weekly_limit != self.account_weekly_limit {
            return Err(refused());
        }
        if !self.flags.enabled
            || !self.eligibility.active_account(account)
            || !self.flags.command_outbox
            || [account, hireling, request].iter().any(Uuid::is_nil)
        {
            return Err(refused());
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,92))")
            .bind(account.to_string())
            .execute(&mut *tx)
            .await?;
        let (house, entitlement, profession): (Uuid,Uuid,String) = sqlx::query_as("SELECT house_id,entitlement_id,profession FROM hirelings WHERE id=$1 AND account_id=$2 FOR UPDATE").bind(hireling).bind(account).fetch_optional(&mut *tx).await?.ok_or_else(refused)?;
        if profession != destination.profession.key()
            || !self
                .eligibility
                .allowed(account, house, entitlement, destination.profession)
        {
            return Err(refused());
        }
        let table = serde_json::to_string(destination)?;
        let old: Option<(Uuid,Uuid,String)> = sqlx::query_as("SELECT account_id,hireling_id,reward_table::text FROM hireling_expeditions WHERE id=$1").bind(request).fetch_optional(&mut *tx).await?;
        if let Some((owner, target, rewards)) = old {
            if owner != account
                || target != hireling
                || serde_json::from_str::<serde_json::Value>(&rewards)?
                    != serde_json::from_str::<serde_json::Value>(&table)?
            {
                return Err(refused());
            }
            return Ok(request);
        }
        let reserved: Option<i32> = sqlx::query_scalar("INSERT INTO hireling_weekly_limits(account_id,week,reserved) VALUES($1,date_trunc('week',now() AT TIME ZONE 'UTC')::date,1) ON CONFLICT(account_id,week) DO UPDATE SET reserved=hireling_weekly_limits.reserved+1,updated_at=now() WHERE hireling_weekly_limits.reserved<$2 RETURNING reserved").bind(account).bind(destination.weekly_limit).fetch_optional(&mut *tx).await?;
        reserved.ok_or_else(refused)?;
        sqlx::query("INSERT INTO hireling_expeditions(id,account_id,hireling_id,destination,reward_table,duration_hours,state) VALUES($1,$2,$3,$4,$5::text::jsonb,$6,'pending')").bind(request).bind(account).bind(hireling).bind(&destination.key).bind(table).bind(i32::from(destination.hours)).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO hireling_commands(id,account_id,expedition_id,kind) VALUES($1,$2,$1,'start')").bind(request).bind(account).execute(&mut *tx).await?;
        outbox::enqueue(
            &mut tx,
            "hirelings.command",
            &serde_json::json!({"command_id":request,"kind":"start"}).to_string(),
            &request.to_string(),
            OffsetDateTime::now_utc(),
            1,
        )
        .await?;
        tx.commit().await?;
        Ok(request)
    }
}

/// The future private G-14 adapter must authenticate the producer and validate the
/// complete receipt before returning true. No network endpoint accepts this type.
pub trait ReceiptVerifier: std::fmt::Debug + Send + Sync {
    fn returned(&self, account: Uuid, expedition: Uuid, receipt: Uuid) -> bool;
}
#[derive(Debug)]
pub struct DenyReceipts;
impl ReceiptVerifier for DenyReceipts {
    fn returned(&self, _: Uuid, _: Uuid, _: Uuid) -> bool {
        false
    }
}
impl<E: Eligibility> Hirelings<E> {
    /// No portal timer can mark an expedition returned or award an item.
    pub async fn record_return(
        &self,
        account: Uuid,
        expedition: Uuid,
        receipt: Uuid,
        verifier: &impl ReceiptVerifier,
        notifications: &oteryn_notify::Notifications,
    ) -> Result<()> {
        if !self.flags.enabled
            || !self.flags.command_outbox
            || !self.eligibility.active_account(account)
            || receipt.is_nil()
            || !verifier.returned(account, expedition, receipt)
        {
            return Err(refused());
        }
        let mut tx = self.pool.begin().await?;
        let old: Option<(String,Option<Uuid>)> = sqlx::query_as("SELECT state,receipt_id FROM hireling_expeditions WHERE id=$1 AND account_id=$2 FOR UPDATE")
            .bind(expedition).bind(account).fetch_optional(&mut *tx).await?;
        let (state, previous) = old.ok_or_else(refused)?;
        if previous == Some(receipt) && state == "returned" {
            return Ok(());
        }
        if previous.is_some() || !matches!(state.as_str(), "pending" | "running") {
            return Err(refused());
        }
        sqlx::query("UPDATE hireling_expeditions SET state='returned',receipt_id=$2,updated_at=now() WHERE id=$1").bind(expedition).bind(receipt).execute(&mut *tx).await?;
        // Existing T36 preferences use the optional News category for this notice.
        let locale = notifications
            .accounts
            .active_verified(account)
            .await?
            .ok_or_else(refused)?
            .locale;
        let lang = if matches!(locale, oteryn_notify::Locale::Pl) {
            "pl"
        } else {
            "en"
        };
        notifications
            .enqueue(
                &mut tx,
                account,
                &oteryn_notify::EmailTemplate::News {
                    title: crate::text(lang, "returned-title")?,
                    body: crate::text(lang, "returned-body")?,
                },
            )
            .await?;
        tx.commit().await?;
        Ok(())
    }
    /// Claim remains an intent; only the game may place loot in a house chest.
    pub async fn claim(&self, account: Uuid, expedition: Uuid) -> Result<Uuid> {
        if !self.flags.enabled
            || !self.flags.command_outbox
            || !self.eligibility.active_account(account)
        {
            return Err(refused());
        }
        let mut tx = self.pool.begin().await?;
        let row: Option<(String,Uuid,Uuid,String)> = sqlx::query_as("SELECT e.state,h.house_id,h.entitlement_id,h.profession FROM hireling_expeditions e JOIN hirelings h ON h.id=e.hireling_id WHERE e.id=$1 AND e.account_id=$2 AND h.account_id=$2 FOR UPDATE OF e,h").bind(expedition).bind(account).fetch_optional(&mut *tx).await?;
        let (state, house, entitlement, profession) = row.ok_or_else(refused)?;
        let profession = match profession.as_str() {
            "miner" => Profession::Miner,
            "herbalist" => Profession::Herbalist,
            "fisher" => Profession::Fisher,
            "merchant" => Profession::Merchant,
            _ => return Err(refused()),
        };
        if state != "returned"
            || !self
                .eligibility
                .allowed(account, house, entitlement, profession)
        {
            return Err(refused());
        }
        let id:Uuid=sqlx::query_scalar("INSERT INTO hireling_commands(id,account_id,expedition_id,kind) VALUES($1,$2,$3,'claim') ON CONFLICT(expedition_id,kind) DO UPDATE SET updated_at=hireling_commands.updated_at RETURNING id").bind(Uuid::now_v7()).bind(account).bind(expedition).fetch_one(&mut *tx).await?;
        outbox::enqueue(
            &mut tx,
            "hirelings.command",
            &serde_json::json!({"command_id":id,"kind":"claim"}).to_string(),
            &id.to_string(),
            OffsetDateTime::now_utc(),
            1,
        )
        .await?;
        tx.commit().await?;
        Ok(id)
    }
}
