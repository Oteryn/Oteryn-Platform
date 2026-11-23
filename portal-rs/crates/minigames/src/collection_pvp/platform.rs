use super::*;
use crate::collection::{self, Action, Card};
use oteryn_wallet::{AdjustmentAudit, AuditHook, Ledger, WalletFlags, wire::DatabasePolicy};
use serde_json::json;
use sqlx::{Postgres, Transaction};

#[derive(Debug, Clone)]
pub struct PlatformCollection {
    catalogue: collection::Collection,
    world_enabled: bool,
}

impl PlatformCollection {
    pub fn new(catalogue: collection::Collection, world_enabled: bool) -> Self {
        Self {
            catalogue,
            world_enabled,
        }
    }

    /// Install the same qualified authority used by the T90 collection router.
    pub fn with_journal(mut self, authority: collection::Ownership) -> Self {
        self.catalogue = self.catalogue.with_ownership(authority);
        self
    }

    async fn pin(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner: Uuid,
        character: Option<Uuid>,
        ids: &[Uuid],
    ) -> Result<DeckSnapshot> {
        collection::valid_deck("PvP", ids).map_err(collection_error)?;
        if let Some(character) = character {
            let now =
                u64::try_from(time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000)
                    .map_err(|_| Error::Unavailable)?;
            self.catalogue
                .require_locked(tx, owner, character, ids, now)
                .await
                .map_err(collection_error)?;
        } else {
            let owned: i64 = sqlx::query_scalar(
                "SELECT count(DISTINCT card.key) FROM collection_pvp_purchases p
                 CROSS JOIN LATERAL jsonb_each(p.cards) card
                 WHERE p.account_id=$1 AND card.key=ANY($2)",
            )
            .bind(owner)
            .bind(ids.iter().map(Uuid::to_string).collect::<Vec<_>>())
            .fetch_one(&mut **tx)
            .await?;
            if usize::try_from(owned).ok() != Some(ids.len()) {
                return Err(Error::Refused);
            }
        }
        let mut cards = Vec::with_capacity(ids.len());
        for id in ids {
            cards.push(self.card(tx, *id).await?);
        }
        Ok(DeckSnapshot {
            character,
            world: cards.first().ok_or(Error::Invalid)?.world_version,
            cards: ids.to_vec(),
        })
    }

    async fn card(&self, tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<Card> {
        self.catalogue
            .locked_card(tx, id)
            .await
            .map_err(collection_error)
    }
}

fn collection_error(error: collection::Error) -> Error {
    match error {
        collection::Error::Denied | collection::Error::Conflict => Error::Refused,
        collection::Error::BadInput => Error::Invalid,
        collection::Error::Database(error) => error.into(),
        _ => Error::Unavailable,
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct DeckSnapshot {
    character: Option<Uuid>,
    world: Uuid,
    cards: Vec<Uuid>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MatchState {
    battle: collection::Battle,
    left: Option<Action>,
    right: Option<Action>,
}

fn read_snapshot<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T> {
    serde_json::from_value(value.clone()).map_err(|_| Error::Unavailable)
}

fn stored(value: impl Serialize) -> Result<Value> {
    serde_json::to_value(value).map_err(|_| Error::Unavailable)
}

impl Collection for PlatformCollection {
    fn available(&self) -> bool {
        self.world_enabled
    }

    fn catalogue_required(&self) -> bool {
        true
    }

    fn journal_available(&self) -> bool {
        self.catalogue.has_bestiary_source()
    }

    async fn deck(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner: Uuid,
        deck: Uuid,
    ) -> Result<Value> {
        let row: Option<(Option<Uuid>, Vec<Uuid>)> = sqlx::query_as(
            "SELECT NULL::uuid, cards FROM collection_pvp_owned_decks WHERE id=$1 AND account_id=$2
             UNION ALL SELECT character_id, cards FROM collection_decks WHERE id=$1 AND account_id=$2",
        )
        .bind(deck)
        .bind(owner)
        .fetch_optional(&mut **tx)
        .await?;
        let (character, ids) = row.ok_or(Error::Refused)?;
        stored(self.pin(tx, owner, character, &ids).await?)
    }

    async fn revalidate(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner: Uuid,
        snapshot: &Value,
    ) -> Result<()> {
        let snapshot: DeckSnapshot = read_snapshot(snapshot)?;
        if self
            .pin(tx, owner, snapshot.character, &snapshot.cards)
            .await?
            != snapshot
        {
            return Err(Error::Unavailable);
        }
        Ok(())
    }

    async fn save_deck(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        owner: Uuid,
        name: &str,
        cards: &[Uuid],
    ) -> Result<()> {
        collection::valid_deck(name, cards).map_err(collection_error)?;
        self.pin(tx, owner, None, cards).await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM collection_pvp_owned_decks WHERE account_id=$1",
        )
        .bind(owner)
        .fetch_one(&mut **tx)
        .await?;
        if count >= 5 {
            return Err(Error::Refused);
        }
        sqlx::query(
            "INSERT INTO collection_pvp_owned_decks(id,account_id,name,cards) VALUES($1,$2,$3,$4)",
        )
        .bind(Uuid::now_v7())
        .bind(owner)
        .bind(name)
        .bind(cards)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    fn begin(&self, left: &Value, right: &Value) -> Result<Value> {
        let left: DeckSnapshot = read_snapshot(left)?;
        let right: DeckSnapshot = read_snapshot(right)?;
        if left.world != right.world {
            return Err(Error::Unavailable);
        }
        let fighters = |deck: DeckSnapshot| {
            deck.cards
                .into_iter()
                .map(|card| collection::Fighter { card, health: 30 })
                .collect()
        };
        let seed = getrandom::u64().map_err(|_| Error::Unavailable)?;
        let battle = collection::Battle::new(fighters(left), fighters(right), seed)
            .map_err(collection_error)?;
        stored(MatchState {
            battle,
            left: None,
            right: None,
        })
    }

    fn turn(&self, state: &Value, side: Side, action: &Value) -> Result<Turn> {
        let action: Action = serde_json::from_value(action.clone()).map_err(|_| Error::Invalid)?;
        let mut state: MatchState = read_snapshot(state)?;
        let slot = match side {
            Side::Left => &mut state.left,
            Side::Right => &mut state.right,
        };
        if slot.is_some() || state.battle.outcome != collection::Outcome::Active {
            return Err(Error::Refused);
        }
        *slot = Some(action);
        if let (Some(left), Some(right)) = (state.left, state.right) {
            state
                .battle
                .resolve_round(left, right)
                .map_err(collection_error)?;
            state.left = None;
            state.right = None;
        }
        let outcome = match state.battle.outcome {
            collection::Outcome::Active => None,
            collection::Outcome::Victory => Some(Outcome::Left),
            collection::Outcome::Defeat => Some(Outcome::Right),
            collection::Outcome::Draw => Some(Outcome::Draw),
        };
        Ok(Turn {
            state: stored(state)?,
            outcome,
        })
    }

    fn view(&self, state: &Value, side: Side) -> Result<Value> {
        let state: MatchState = read_snapshot(state)?;
        let (yours, theirs, waiting) = match side {
            Side::Left => (
                &state.battle.player,
                &state.battle.enemy,
                state.left.is_some(),
            ),
            Side::Right => (
                &state.battle.enemy,
                &state.battle.player,
                state.right.is_some(),
            ),
        };
        let health = |fighters: &[collection::Fighter]| {
            fighters.iter().map(|fighter| fighter.health).sum::<u16>()
        };
        Ok(json!({
            "round": state.battle.turn,
            "your_health": health(yours),
            "opponent_health": health(theirs),
            "waiting": waiting
        }))
    }

    async fn grant(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        _: Uuid,
        cards: &Value,
    ) -> Result<()> {
        // The immutable purchase receipt is the entitlement. Resolve every key
        // before the caller commits that receipt together with the wallet debit.
        for key in cards.as_object().ok_or(Error::Invalid)?.keys() {
            let id: Uuid = key.parse().map_err(|_| Error::Invalid)?;
            if id.to_string() != *key {
                return Err(Error::Invalid);
            }
            self.card(tx, id).await?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PurchaseAudit;

impl AuditHook for PurchaseAudit {
    async fn adjustment(&self, db: &mut PgConnection, event: &AdjustmentAudit) -> bool {
        if event.amount >= 0
            || event.actor_id != event.account_id
            || event.reason != format!("store-purchase:{}", event.operation_id)
        {
            return false;
        }
        sqlx::query("INSERT INTO audit_log(actor,action,target,before,after,request_id) VALUES(NULL,'collection.set.debited',$1,$2,$3,$4)")
            .bind(event.account_id.to_string())
            .bind(json!({"available": event.before.available, "reserved": event.before.reserved}))
            .bind(json!({"available": event.after.available, "reserved": event.after.reserved, "coins": event.amount}))
            .bind(event.operation_id)
            .execute(db)
            .await
            .is_ok()
    }
}

pub(super) fn payment(
    pool: PgPool,
    enabled: bool,
) -> WalletPayment<DatabasePolicy, DatabasePolicy, PurchaseAudit> {
    WalletPayment::new(Ledger::new(
        pool,
        WalletFlags { enabled },
        DatabasePolicy,
        DatabasePolicy,
        PurchaseAudit,
    ))
}
