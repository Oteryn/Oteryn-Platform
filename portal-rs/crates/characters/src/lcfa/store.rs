use super::{
    AccountId, Availability, CharacterId, CharacterSummary, Error, Outcome, Settings, Snapshot,
    Watermark, WorldId,
};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::{future::Future, pin::Pin, sync::Arc};
use subtle::ConstantTimeEq;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    Active,
    Denied,
    Unknown,
    Ambiguous,
    Unavailable,
}

/// Implement using authoritative identity state. Projection data never grants
/// account permission. Only an explicit Active result permits a read.
pub trait AccountStateLookup: std::fmt::Debug + Send + Sync {
    fn state(&self, account: AccountId) -> Pin<Box<dyn Future<Output = AccountState> + Send + '_>>;
}
#[derive(Debug)]
pub struct DenyAccounts;
impl AccountStateLookup for DenyAccounts {
    fn state(&self, _: AccountId) -> Pin<Box<dyn Future<Output = AccountState> + Send + '_>> {
        Box::pin(async { AccountState::Unknown })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadState {
    Ready,
    Missing,
    Invalid,
    Stale,
    Unavailable,
    Denied,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountView {
    pub state: ReadState,
    pub characters: Vec<CharacterSummary>,
}
impl AccountView {
    fn empty(state: ReadState) -> Self {
        Self {
            state,
            characters: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Projection {
    pub(super) pool: PgPool,
    pub(super) settings: Settings,
    accounts: Arc<dyn AccountStateLookup>,
}
impl Projection {
    pub fn new(pool: PgPool, settings: Settings, accounts: Arc<dyn AccountStateLookup>) -> Self {
        Self {
            pool,
            settings,
            accounts,
        }
    }
    fn publication(&self, authority: &str, observed: i64, now: i64) -> Result<(), Error> {
        if !self.settings.available() {
            return Err(Error::Unavailable);
        }
        if !bool::from(
            authority
                .as_bytes()
                .ct_eq(self.settings.source_authority.as_bytes()),
        ) {
            return Err(Error::Unauthorized);
        }
        if now < 0
            || observed > now.saturating_add(i64::from(self.settings.clock_uncertainty_seconds))
        {
            return Err(Error::BadInput);
        }
        Ok(())
    }
    pub(super) async fn snapshot(&self, snapshot: Snapshot, now: i64) -> Result<Outcome, Error> {
        self.publication(&snapshot.authority, snapshot.observed, now)?;
        let mut tx = self.pool.begin().await?;
        if !advance_epoch(&mut tx, snapshot.epoch).await? {
            return Ok(Outcome::Superseded);
        }
        let current = sqlx::query("SELECT projection_epoch, projection_revision, content_digest, invalid FROM account_character_snapshots WHERE account_id = $1 FOR UPDATE")
            .bind(snapshot.account.0).fetch_optional(&mut *tx).await?;
        if let Some(row) = current {
            let pair = (
                number(row.try_get("projection_epoch")?)?,
                number(row.try_get("projection_revision")?)?,
            );
            match (snapshot.epoch, snapshot.revision).cmp(&pair) {
                std::cmp::Ordering::Less => return Ok(Outcome::Superseded),
                std::cmp::Ordering::Equal => {
                    let digest: Vec<u8> = row.try_get("content_digest")?;
                    if row.try_get::<bool, _>("invalid")?
                        || !bool::from(digest.ct_eq(&snapshot.digest))
                    {
                        sqlx::query("UPDATE account_character_snapshots SET invalid = true, updated_at = now() WHERE account_id = $1")
                            .bind(snapshot.account.0).execute(&mut *tx).await?;
                        tx.commit().await?;
                        return Err(Error::Conflict);
                    }
                    tx.commit().await?;
                    return Ok(Outcome::Accepted);
                }
                std::cmp::Ordering::Greater => {}
            }
        }
        sqlx::query("INSERT INTO account_character_snapshots (account_id, source_authority, projection_epoch, projection_revision, source_observed_at, content_digest) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (account_id) DO UPDATE SET source_authority = EXCLUDED.source_authority, projection_epoch = EXCLUDED.projection_epoch, projection_revision = EXCLUDED.projection_revision, source_observed_at = EXCLUDED.source_observed_at, content_digest = EXCLUDED.content_digest, invalid = false, updated_at = now()")
            .bind(snapshot.account.0).bind(snapshot.authority).bind(snapshot.epoch.to_string())
            .bind(snapshot.revision.to_string()).bind(snapshot.observed).bind(snapshot.digest).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM account_characters WHERE account_id = $1")
            .bind(snapshot.account.0)
            .execute(&mut *tx)
            .await?;
        for character in snapshot.characters {
            sqlx::query("INSERT INTO account_characters (account_id, character_id, world_id, name, availability) VALUES ($1,$2,$3,$4,$5)")
                .bind(snapshot.account.0).bind(character.character_id.0).bind(character.world_id.0)
                .bind(character.name).bind(character.availability.as_str()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(Outcome::Accepted)
    }
    pub(super) async fn watermark(&self, watermark: Watermark, now: i64) -> Result<Outcome, Error> {
        self.publication(&watermark.authority, watermark.observed, now)?;
        let mut tx = self.pool.begin().await?;
        if !advance_epoch(&mut tx, watermark.epoch).await? {
            return Ok(Outcome::Superseded);
        }
        let current: Option<i64> =
            sqlx::query_scalar("SELECT complete_through FROM account_character_feed WHERE id = 1")
                .fetch_one(&mut *tx)
                .await?;
        if current.is_some_and(|complete| watermark.complete <= complete) {
            return Ok(Outcome::Superseded);
        }
        sqlx::query("UPDATE account_character_feed SET watermark_epoch = $1, source_authority = $2, complete_through = $3, observed_at = $4, updated_at = now() WHERE id = 1")
            .bind(watermark.epoch.to_string()).bind(watermark.authority).bind(watermark.complete).bind(watermark.observed).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Outcome::Accepted)
    }
    /// An internal API for an already authenticated caller. The caller must bind
    /// the requested account to its authenticated principal before invoking this.
    pub async fn list_characters(
        &self,
        account: AccountId,
        now: i64,
    ) -> Result<AccountView, Error> {
        if !self.settings.available() || now < 0 {
            return Ok(AccountView::empty(ReadState::Unavailable));
        }
        if self.accounts.state(account).await != AccountState::Active {
            return Ok(AccountView::empty(ReadState::Denied));
        }
        let mut tx = self.pool.begin().await?;
        // A shared feed lock spans metadata and rows, excluding epoch changes,
        // snapshot replacement and watermark updates until the read completes.
        let Some(feed) = sqlx::query("SELECT highest_epoch, watermark_epoch, source_authority, complete_through FROM account_character_feed WHERE id = 1 FOR SHARE")
            .fetch_optional(&mut *tx).await? else { return Ok(AccountView::empty(ReadState::Unavailable)); };
        let highest: Option<String> = feed.try_get("highest_epoch")?;
        let watermark: Option<String> = feed.try_get("watermark_epoch")?;
        let authority: Option<String> = feed.try_get("source_authority")?;
        let complete: Option<i64> = feed.try_get("complete_through")?;
        let live = highest.is_some()
            && highest == watermark
            && authority.as_deref() == Some(&self.settings.source_authority)
            && complete.is_some_and(|complete| {
                complete <= now.saturating_add(i64::from(self.settings.clock_uncertainty_seconds))
                    && now
                        .saturating_sub(complete)
                        .saturating_add(i64::from(self.settings.clock_uncertainty_seconds))
                        <= i64::from(self.settings.freshness_seconds)
            });
        if !live {
            return Ok(AccountView::empty(ReadState::Stale));
        }
        let Some(snapshot) = sqlx::query("SELECT projection_epoch, invalid, source_authority FROM account_character_snapshots WHERE account_id = $1")
            .bind(account.0).fetch_optional(&mut *tx).await? else { return Ok(AccountView::empty(ReadState::Missing)); };
        if snapshot.try_get::<bool, _>("invalid")?
            || Some(snapshot.try_get::<String, _>("projection_epoch")?) != highest
            || snapshot.try_get::<String, _>("source_authority")? != self.settings.source_authority
        {
            return Ok(AccountView::empty(ReadState::Invalid));
        }
        let rows = sqlx::query("SELECT character_id, world_id, name, availability FROM account_characters WHERE account_id = $1 ORDER BY character_id")
            .bind(account.0).fetch_all(&mut *tx).await?;
        let mut characters = Vec::with_capacity(rows.len());
        for row in rows {
            characters.push(CharacterSummary {
                character_id: CharacterId(row.try_get("character_id")?),
                world_id: WorldId(row.try_get("world_id")?),
                name: row.try_get("name")?,
                availability: Availability::parse(row.try_get("availability")?)
                    .map_err(|_| Error::Unavailable)?,
            });
        }
        tx.commit().await?;
        // Recheck current state after potentially waiting on projection locks.
        if self.accounts.state(account).await != AccountState::Active {
            return Ok(AccountView::empty(ReadState::Denied));
        }
        Ok(AccountView {
            state: ReadState::Ready,
            characters,
        })
    }
}
fn number(raw: &str) -> Result<u64, Error> {
    raw.parse().map_err(|_| Error::Unavailable)
}
async fn advance_epoch(tx: &mut Transaction<'_, Postgres>, epoch: u64) -> Result<bool, Error> {
    let current: Option<String> = sqlx::query_scalar(
        "SELECT highest_epoch FROM account_character_feed WHERE id = 1 FOR UPDATE",
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(Error::Unavailable)?;
    if let Some(current) = current {
        match epoch.cmp(&number(&current)?) {
            std::cmp::Ordering::Less => return Ok(false),
            std::cmp::Ordering::Equal => return Ok(true),
            std::cmp::Ordering::Greater => {}
        }
    }
    // Epoch comparison on every read invalidates all old accounts immediately,
    // without rewriting the entire projection during a producer resync.
    sqlx::query("UPDATE account_character_feed SET highest_epoch = $1, watermark_epoch = NULL, source_authority = NULL, complete_through = NULL, observed_at = NULL, updated_at = now() WHERE id = 1")
        .bind(epoch.to_string()).execute(&mut **tx).await?;
    Ok(true)
}
