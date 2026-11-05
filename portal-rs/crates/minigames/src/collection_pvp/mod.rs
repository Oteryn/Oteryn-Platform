//! Consensual collection battles, round-robin tournaments and fixed-content sets.
//! T90 supplies owned deck snapshots, turn rules and non-transferable card grants.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use std::future::Future;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub pvp: bool,
    pub sets: bool,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("feature unavailable")]
    Unavailable,
    #[error("operation refused")]
    Refused,
    #[error("invalid input")]
    Invalid,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Authoritative T90 adapter. All writes use the caller's transaction.
/// Decks must be owned, legal and pinned to a content version. Unknown state denies.
/// Turn implementations must validate the action and actor against the saved state.
pub trait Collection: Send + Sync {
    fn available(&self) -> bool {
        false
    }
    fn deck(
        &self,
        conn: &mut PgConnection,
        owner: Uuid,
        deck: Uuid,
    ) -> impl Future<Output = Result<Value>> + Send;
    fn begin(&self, left: &Value, right: &Value) -> Result<Value>;
    fn turn(&self, state: &Value, side: Side, action: &Value) -> Result<Turn>;
    /// Redact hidden opponent information before returning a participant view.
    fn view(&self, _state: &Value, _side: Side) -> Result<Value> {
        Err(Error::Unavailable)
    }
    fn grant(
        &self,
        conn: &mut PgConnection,
        owner: Uuid,
        cards: &Value,
    ) -> impl Future<Output = Result<()>> + Send;
}
/// Debit exactly this amount in the same transaction, with no external effects.
/// A successful call must be durable only on commit. Missing wallet integration denies.
pub trait SetPayment: Send + Sync {
    fn debit(
        &self,
        conn: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        owner: Uuid,
        amount: i64,
        request: Uuid,
    ) -> impl Future<Output = Result<()>> + Send;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Left,
    Right,
    Draw,
}
#[derive(Debug)]
pub struct Turn {
    pub state: Value,
    pub outcome: Option<Outcome>,
}
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Battle {
    pub id: Uuid,
    pub left_account: Uuid,
    pub right_account: Uuid,
    pub status: String,
    pub revision: i64,
    pub state: Value,
    pub winner: Option<Uuid>,
    pub tournament_id: Option<Uuid>,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Ranking {
    pub account_id: Uuid,
    pub points: i64,
    pub played: i64,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Tournament {
    pub id: Uuid,
    pub capacity: i32,
    pub entrants: i64,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PublishedSet {
    pub id: Uuid,
    pub version: i64,
    pub price: i64,
    pub cards: Value,
}
#[derive(Debug, Clone)]
pub struct Service<C, P> {
    pool: PgPool,
    flags: Flags,
    collection: C,
    payment: P,
}
impl<C: Collection, P: SetPayment> Service<C, P> {
    pub fn new(pool: PgPool, flags: Flags, collection: C, payment: P) -> Self {
        Self {
            pool,
            flags,
            collection,
            payment,
        }
    }
    fn enabled(&self) -> Result<()> {
        if self.flags.pvp && self.collection.available() {
            Ok(())
        } else {
            Err(Error::Unavailable)
        }
    }
    async fn active(conn: &mut PgConnection, account: Uuid) -> Result<()> {
        let allowed: Option<bool> = sqlx::query_scalar("SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR SHARE")
            .bind(account).fetch_optional(conn).await?;
        if allowed == Some(true) {
            Ok(())
        } else {
            Err(Error::Refused)
        }
    }
    async fn creation_budget(conn: &mut PgConnection, actor: Uuid) -> Result<()> {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,92))")
            .bind(actor.to_string())
            .execute(&mut *conn)
            .await?;
        let count:i64=sqlx::query_scalar("SELECT (SELECT count(*) FROM collection_pvp_battles WHERE left_account=$1 AND tournament_id IS NULL AND created_at>now()-interval '1 minute')+(SELECT count(*) FROM collection_pvp_tournaments WHERE organizer=$1 AND created_at>now()-interval '1 minute')")
            .bind(actor).fetch_one(conn).await?;
        if count >= 10 {
            return Err(Error::Refused);
        }
        Ok(())
    }
    pub async fn challenge(&self, actor: Uuid, opponent: Uuid, deck: Uuid) -> Result<Uuid> {
        self.enabled()?;
        if actor == opponent || deck.is_nil() {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        Self::active(&mut tx, actor).await?;
        Self::active(&mut tx, opponent).await?;
        Self::creation_budget(&mut tx, actor).await?;
        let snapshot = self.collection.deck(&mut tx, actor, deck).await?;
        validate_snapshot(&snapshot)?;
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO collection_pvp_battles(id,left_account,right_account,left_deck) VALUES($1,$2,$3,$4)")
            .bind(id).bind(actor).bind(opponent).bind(snapshot).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(id)
    }
    pub async fn accept(&self, actor: Uuid, id: Uuid, deck: Uuid) -> Result<()> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        Self::active(&mut tx, actor).await?;
        let row: (Uuid, Value, String) = sqlx::query_as("SELECT right_account,left_deck,status FROM collection_pvp_battles WHERE id=$1 FOR UPDATE")
            .bind(id).fetch_optional(&mut *tx).await?.ok_or(Error::Refused)?;
        if row.0 != actor || row.2 != "pending" {
            return Err(Error::Refused);
        }
        let left: Uuid =
            sqlx::query_scalar("SELECT left_account FROM collection_pvp_battles WHERE id=$1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        Self::active(&mut tx, left).await?;
        let right = self.collection.deck(&mut tx, actor, deck).await?;
        validate_snapshot(&right)?;
        let state = self.collection.begin(&row.1, &right)?;
        validate_snapshot(&state)?;
        sqlx::query("UPDATE collection_pvp_battles SET right_deck=$2,state=$3,status='active',revision=revision+1,updated_at=now() WHERE id=$1")
            .bind(id).bind(right).bind(state).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn decline(&self, actor: Uuid, id: Uuid) -> Result<()> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        Self::active(&mut tx, actor).await?;
        let changed=sqlx::query("UPDATE collection_pvp_battles SET status='declined',updated_at=now() WHERE id=$1 AND right_account=$2 AND status='pending'")
            .bind(id).bind(actor).execute(&mut *tx).await?.rows_affected();
        if changed != 1 {
            return Err(Error::Refused);
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn battle(&self, actor: Uuid, id: Uuid) -> Result<Battle> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        Self::active(&mut tx, actor).await?;
        let mut battle: Battle=sqlx::query_as("SELECT id,left_account,right_account,status,revision,state,winner,tournament_id FROM collection_pvp_battles WHERE id=$1 AND $2 IN(left_account,right_account)")
            .bind(id).bind(actor).fetch_optional(&mut *tx).await?.ok_or(Error::Refused)?;
        battle.state = self.collection.view(
            &battle.state,
            if actor == battle.left_account {
                Side::Left
            } else {
                Side::Right
            },
        )?;
        tx.commit().await?;
        Ok(battle)
    }
    /// Optimistic revision and row lock prevent duplicate actions and double scoring.
    pub async fn play(&self, actor: Uuid, id: Uuid, revision: i64, action: Value) -> Result<()> {
        self.enabled()?;
        if revision < 0 || action.to_string().len() > 4096 {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        Self::active(&mut tx, actor).await?;
        let tournament: Option<Option<Uuid>> =
            sqlx::query_scalar("SELECT tournament_id FROM collection_pvp_battles WHERE id=$1")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        if let Some(Some(tournament)) = tournament {
            sqlx::query("SELECT id FROM collection_pvp_tournaments WHERE id=$1 FOR UPDATE")
                .bind(tournament)
                .execute(&mut *tx)
                .await?;
        }
        let battle: Battle = sqlx::query_as("SELECT id,left_account,right_account,status,revision,state,winner,tournament_id FROM collection_pvp_battles WHERE id=$1 FOR UPDATE")
            .bind(id).fetch_optional(&mut *tx).await?.ok_or(Error::Refused)?;
        if battle.status != "active" || battle.revision != revision {
            return Err(Error::Refused);
        }
        let side = if actor == battle.left_account {
            Side::Left
        } else if actor == battle.right_account {
            Side::Right
        } else {
            return Err(Error::Refused);
        };
        Self::active(&mut tx, battle.left_account).await?;
        Self::active(&mut tx, battle.right_account).await?;
        if let Some(tournament) = battle.tournament_id {
            sqlx::query("SELECT id FROM collection_pvp_tournaments WHERE id=$1 FOR UPDATE")
                .bind(tournament)
                .execute(&mut *tx)
                .await?;
        }
        let next = self.collection.turn(&battle.state, side, &action)?;
        validate_snapshot(&next.state)?;
        let winner = match next.outcome {
            Some(Outcome::Left) => Some(battle.left_account),
            Some(Outcome::Right) => Some(battle.right_account),
            _ => None,
        };
        sqlx::query("UPDATE collection_pvp_battles SET state=$2,winner=$3,status=$4,revision=revision+1,updated_at=now() WHERE id=$1")
            .bind(id).bind(next.state).bind(winner).bind(if next.outcome.is_some() { "completed" } else { "active" }).execute(&mut *tx).await?;
        if let Some(tournament) = battle.tournament_id {
            sqlx::query("UPDATE collection_pvp_tournaments SET status='completed',updated_at=now() WHERE id=$1 AND status='active' AND NOT EXISTS(SELECT 1 FROM collection_pvp_battles WHERE tournament_id=$1 AND status <> 'completed')")
                .bind(tournament).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    /// Stable ties share a score; UUID order is only presentation order.
    pub async fn rankings(&self, tournament: Option<Uuid>) -> Result<Vec<Ranking>> {
        self.enabled()?;
        Ok(sqlx::query_as("SELECT account_id,sum(points)::bigint AS points,count(*)::bigint AS played FROM (SELECT left_account AS account_id,CASE WHEN winner=left_account THEN 3 WHEN winner IS NULL THEN 1 ELSE 0 END AS points FROM collection_pvp_battles WHERE status='completed' AND ($1::uuid IS NULL OR tournament_id=$1) UNION ALL SELECT right_account,CASE WHEN winner=right_account THEN 3 WHEN winner IS NULL THEN 1 ELSE 0 END FROM collection_pvp_battles WHERE status='completed' AND ($1::uuid IS NULL OR tournament_id=$1)) scores GROUP BY account_id ORDER BY points DESC,account_id LIMIT 100")
            .bind(tournament).fetch_all(&self.pool).await?)
    }
    pub async fn tournaments(&self, actor: Uuid) -> Result<Vec<Tournament>> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        Self::active(&mut tx, actor).await?;
        let rows=sqlx::query_as("SELECT t.id,t.capacity,t.status,(SELECT count(*) FROM collection_pvp_entries e WHERE e.tournament_id=t.id) AS entrants FROM collection_pvp_tournaments t WHERE t.status='registration' OR t.organizer=$1 OR EXISTS(SELECT 1 FROM collection_pvp_entries e WHERE e.tournament_id=t.id AND e.account_id=$1) ORDER BY t.created_at DESC LIMIT 50").bind(actor).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(rows)
    }
    pub async fn create_tournament(&self, organizer: Uuid, capacity: i32) -> Result<Uuid> {
        self.enabled()?;
        if !(2..=16).contains(&capacity) {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        Self::active(&mut tx, organizer).await?;
        Self::creation_budget(&mut tx, organizer).await?;
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO collection_pvp_tournaments(id,organizer,capacity) VALUES($1,$2,$3)",
        )
        .bind(id)
        .bind(organizer)
        .bind(capacity)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(id)
    }
    /// Joining constitutes consent to every round-robin fixture with this pinned deck.
    pub async fn join(&self, actor: Uuid, tournament: Uuid, deck: Uuid) -> Result<()> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        Self::active(&mut tx, actor).await?;
        let (status, capacity): (String, i32) = sqlx::query_as(
            "SELECT status,capacity FROM collection_pvp_tournaments WHERE id=$1 FOR UPDATE",
        )
        .bind(tournament)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Refused)?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM collection_pvp_entries WHERE tournament_id=$1",
        )
        .bind(tournament)
        .fetch_one(&mut *tx)
        .await?;
        if status != "registration" || count >= i64::from(capacity) {
            return Err(Error::Refused);
        }
        let snapshot = self.collection.deck(&mut tx, actor, deck).await?;
        validate_snapshot(&snapshot)?;
        let added = sqlx::query("INSERT INTO collection_pvp_entries(tournament_id,account_id,deck) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(tournament).bind(actor).bind(snapshot).execute(&mut *tx).await?.rows_affected();
        if added != 1 {
            return Err(Error::Refused);
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn start(&self, actor: Uuid, tournament: Uuid) -> Result<()> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        Self::active(&mut tx, actor).await?;
        let (organizer, status): (Uuid, String) = sqlx::query_as(
            "SELECT organizer,status FROM collection_pvp_tournaments WHERE id=$1 FOR UPDATE",
        )
        .bind(tournament)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Refused)?;
        if organizer != actor || status != "registration" {
            return Err(Error::Refused);
        }
        let entries: Vec<(Uuid,Value)> = sqlx::query_as("SELECT account_id,deck FROM collection_pvp_entries WHERE tournament_id=$1 ORDER BY account_id").bind(tournament).fetch_all(&mut *tx).await?;
        if entries.len() < 2 {
            return Err(Error::Refused);
        }
        for (i, (left, ld)) in entries.iter().enumerate() {
            Self::active(&mut tx, *left).await?;
            for (right, rd) in entries.iter().skip(i + 1) {
                let state = self.collection.begin(ld, rd)?;
                validate_snapshot(&state)?;
                sqlx::query("INSERT INTO collection_pvp_battles(id,left_account,right_account,left_deck,right_deck,state,status,tournament_id) VALUES($1,$2,$3,$4,$5,$6,'active',$7)")
                    .bind(Uuid::now_v7()).bind(left).bind(right).bind(ld).bind(rd).bind(state).bind(tournament).execute(&mut *tx).await?;
            }
        }
        sqlx::query(
            "UPDATE collection_pvp_tournaments SET status='active',updated_at=now() WHERE id=$1",
        )
        .bind(tournament)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn sets(&self) -> Result<Vec<PublishedSet>> {
        if !self.flags.sets {
            return Err(Error::Unavailable);
        }
        let sets: Vec<PublishedSet>=sqlx::query_as("SELECT id,version,price,cards FROM collection_pvp_sets WHERE published ORDER BY id LIMIT 100").fetch_all(&self.pool).await?;
        for set in &sets {
            validate_cards(&set.cards)?;
        }
        Ok(sets)
    }
    /// The previewed version/price are mandatory. Payment, exact grants and receipt
    /// commit together. Replaying a request never debits or grants twice.
    pub async fn buy_set(
        &self,
        actor: Uuid,
        set: Uuid,
        version: i64,
        price: i64,
        request: Uuid,
    ) -> Result<()> {
        if !self.flags.sets {
            return Err(Error::Unavailable);
        }
        if request.get_version_num() != 7 || price <= 0 || version <= 0 {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        Self::active(&mut tx, actor).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,91))")
            .bind(request.to_string())
            .execute(&mut *tx)
            .await?;
        let prior: Option<(Uuid, Uuid, i64, i64)> = sqlx::query_as(
            "SELECT account_id,set_id,version,price FROM collection_pvp_purchases WHERE id=$1",
        )
        .bind(request)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(prior) = prior {
            return if prior == (actor, set, version, price) {
                Ok(())
            } else {
                Err(Error::Refused)
            };
        }
        let offer: PublishedSet=sqlx::query_as("SELECT id,version,price,cards FROM collection_pvp_sets WHERE id=$1 AND published FOR SHARE").bind(set).fetch_optional(&mut *tx).await?.ok_or(Error::Refused)?;
        if offer.version != version || offer.price != price {
            return Err(Error::Refused);
        }
        validate_cards(&offer.cards)?;
        self.payment.debit(&mut tx, actor, price, request).await?;
        self.collection.grant(&mut tx, actor, &offer.cards).await?;
        sqlx::query("INSERT INTO collection_pvp_purchases(id,account_id,set_id,version,price,cards) VALUES($1,$2,$3,$4,$5,$6)")
            .bind(request).bind(actor).bind(set).bind(version).bind(price).bind(offer.cards).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}
/// Default production seam until T90 and transactional wallet adapters are installed.
#[derive(Debug, Clone, Copy)]
pub struct Unavailable;
impl Collection for Unavailable {
    fn available(&self) -> bool {
        false
    }
    async fn deck(&self, _: &mut PgConnection, _: Uuid, _: Uuid) -> Result<Value> {
        Err(Error::Unavailable)
    }
    fn begin(&self, _: &Value, _: &Value) -> Result<Value> {
        Err(Error::Unavailable)
    }
    fn turn(&self, _: &Value, _: Side, _: &Value) -> Result<Turn> {
        Err(Error::Unavailable)
    }
    async fn grant(&self, _: &mut PgConnection, _: Uuid, _: &Value) -> Result<()> {
        Err(Error::Unavailable)
    }
}
impl SetPayment for Unavailable {
    async fn debit(
        &self,
        _: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        _: Uuid,
        _: i64,
        _: Uuid,
    ) -> Result<()> {
        Err(Error::Unavailable)
    }
}
pub mod http;
#[cfg(test)]
mod tests;

/// Fixed sets have bounded, positive integral quantities and explicit card keys.
fn validate_cards(cards: &Value) -> Result<()> {
    let cards = cards.as_object().ok_or(Error::Invalid)?;
    if cards.is_empty()
        || cards.len() > 100
        || cards.iter().any(|(key, count)| {
            key.is_empty()
                || key.len() > 128
                || !key
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
                || !count.as_u64().is_some_and(|n| (1..=100).contains(&n))
        })
    {
        return Err(Error::Invalid);
    }
    Ok(())
}

/// Transactional bridge to the wallet's public purchase API. The caller supplies
/// the wallet's account checks, enabled flag and audit policy; none are bypassed.
#[derive(Debug)]
pub struct WalletPayment<A, B, H>(oteryn_wallet::Ledger<A, B, H>);
impl<
    A: oteryn_wallet::AccountStateReader,
    B: oteryn_wallet::TransferBlockCheck,
    H: oteryn_wallet::AuditHook,
> WalletPayment<A, B, H>
{
    pub fn new(ledger: oteryn_wallet::Ledger<A, B, H>) -> Self {
        Self(ledger.with_adjustment_kind(oteryn_wallet::AdjustmentKind::StorePurchase))
    }
}
impl<
    A: oteryn_wallet::AccountStateReader,
    B: oteryn_wallet::TransferBlockCheck,
    H: oteryn_wallet::AuditHook,
> SetPayment for WalletPayment<A, B, H>
{
    async fn debit(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        owner: Uuid,
        amount: i64,
        request: Uuid,
    ) -> Result<()> {
        if amount <= 0 {
            return Err(Error::Invalid);
        }
        self.0
            .adjust_in(
                tx,
                request,
                owner,
                -amount,
                owner,
                &format!("store-purchase:{request}"),
            )
            .await
            .map_err(|_| Error::Refused)?;
        Ok(())
    }
}

fn validate_snapshot(value: &Value) -> Result<()> {
    if !value.is_object() || value.to_string().len() > 65536 {
        return Err(Error::Invalid);
    }
    Ok(())
}
