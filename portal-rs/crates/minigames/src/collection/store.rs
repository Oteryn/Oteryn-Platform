use super::*;
use oteryn_identity::accounts::Accounts;
use oteryn_playerdata::progress::{Actor, Authority, Payload, ProgressStore, ReadScope};
use oteryn_world::WorldStore;
use sqlx::{PgPool, Row};

#[derive(Debug, Clone)]
pub struct Collection {
    pool: PgPool,
    world: WorldStore,
    progress: ProgressStore,
    flags: Flags,
}
impl Collection {
    pub fn new(pool: PgPool, world: WorldStore, progress: ProgressStore, flags: Flags) -> Self {
        Self {
            pool,
            world,
            progress,
            flags,
        }
    }
    fn enabled(&self) -> Result<()> {
        if self.flags.collection {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }
    async fn eligible(&self, account: Uuid) -> Result<()> {
        self.enabled()?;
        let account = Accounts {
            pool: self.pool.clone(),
        }
        .find_by_id(account)
        .await?;
        if account.is_some_and(|a| a.status == "active" && a.email_verified_at.is_some()) {
            Ok(())
        } else {
            Err(Error::Denied)
        }
    }
    async fn visible(&self, card: &Card) -> Result<()> {
        card.validate()?;
        let entity = self
            .world
            .get_entity(&card.kind, &card.key)
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        if entity.version_id != card.world_version {
            return Err(Error::Unavailable);
        }
        card.matches_entity(&entity)
    }
    /// Trusted worker only: the caller verifies the native creature→race, revision,
    /// threshold, bilingual labels and appearance mapping. Immutable publication
    /// prevents changing a card after it has been used by a deck or battle.
    pub async fn publish(&self, card: Card, sprite: Sprite) -> Result<()> {
        self.enabled()?;
        self.visible(&card).await?;
        let report = self
            .world
            .import_report(card.world_version)
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        if report.source_commit != sprite.commit || card.appearance_id != sprite.appearance {
            return Err(Error::Unavailable);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO collection_cards(id,definition,sprite) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(card.id).bind(sqlx::types::Json(&card)).bind(&sprite.bytes).execute(&mut *tx).await?;
        let row = sqlx::query("SELECT definition,sprite FROM collection_cards WHERE id=$1")
            .bind(card.id)
            .fetch_one(&mut *tx)
            .await?;
        if row.try_get::<sqlx::types::Json<Card>, _>("definition")?.0 != card
            || row.try_get::<Vec<u8>, _>("sprite")? != sprite.bytes
        {
            return Err(Error::Conflict);
        }
        tx.commit().await?;
        Ok(())
    }
    /// Current owner-only G-11 reads, not historical unlocks retained after sale.
    /// Unknown ownership, baseline, revision, freshness or catalog state denies.
    pub async fn cards(
        &self,
        account: Uuid,
        character: Uuid,
        authority: &impl Authority,
        now: u64,
    ) -> Result<Vec<OwnedCard>> {
        self.eligible(account).await?;
        if character.get_version_num() != 7 {
            return Err(Error::BadInput);
        }
        let progress = self
            .progress
            .read(
                Actor::Owner(account),
                &ReadScope {
                    account,
                    character: Some(character),
                    family: "bestiary",
                },
                authority,
                now,
            )
            .await
            .map_err(|e| match e {
                oteryn_playerdata::progress::Error::Denied => Error::Denied,
                _ => Error::Unavailable,
            })?;
        let definitions: Vec<sqlx::types::Json<Card>> =
            sqlx::query_scalar("SELECT definition FROM collection_cards ORDER BY id LIMIT 1001")
                .fetch_all(&self.pool)
                .await?;
        if definitions.len() > 1000 {
            return Err(Error::Unavailable);
        }
        let active = self
            .world
            .active_version_id()
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        let mut cards = Vec::new();
        for sqlx::types::Json(card) in definitions {
            // Retired/hidden entities disappear; a deck referencing one is refused.
            card.validate()?;
            if card.world_version != active {
                continue;
            }
            if self
                .world
                .get_entity(&card.kind, &card.key)
                .await
                .map_err(|_| Error::Unavailable)?
                .is_none()
            {
                continue;
            }
            self.visible(&card).await?;
            let unlocked = progress.iter().any(|p| {
                matches!(p, Payload::Bestiary(b)
                if b.race_key == card.race_key && b.race_definition_revision == card.race_revision
                    && b.final_kill_threshold == card.final_kill_threshold
                    && b.kill_count >= b.final_kill_threshold)
            });
            cards.push(OwnedCard { card, unlocked });
        }
        Ok(cards)
    }
    pub async fn decks(&self, account: Uuid) -> Result<Vec<Deck>> {
        self.eligible(account).await?;
        let rows = sqlx::query("SELECT id,character_id,name,cards FROM collection_decks WHERE account_id=$1 ORDER BY created_at,id").bind(account).fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|r| {
                Ok(Deck {
                    id: r.try_get("id")?,
                    character: r.try_get("character_id")?,
                    name: r.try_get("name")?,
                    cards: r.try_get("cards")?,
                })
            })
            .collect()
    }
    pub async fn save_deck(
        &self,
        account: Uuid,
        deck: Deck,
        authority: &impl Authority,
        now: u64,
    ) -> Result<()> {
        self.enabled()?;
        valid_deck(&deck.name, &deck.cards)?;
        if deck.id.get_version_num() != 7 {
            return Err(Error::BadInput);
        }
        let owned = self.cards(account, deck.character, authority, now).await?;
        require_cards(&owned, &deck.cards)?;
        let mut tx = self.pool.begin().await?;
        // Lock the owning account, serializing quota checks, edits and battle starts.
        sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(account)
            .fetch_one(&mut *tx)
            .await?;
        let owner: Option<Uuid> =
            sqlx::query_scalar("SELECT account_id FROM collection_decks WHERE id=$1")
                .bind(deck.id)
                .fetch_optional(&mut *tx)
                .await?;
        if owner.is_some_and(|o| o != account) {
            return Err(Error::Denied);
        }
        if owner.is_none() {
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM collection_decks WHERE account_id=$1")
                    .bind(account)
                    .fetch_one(&mut *tx)
                    .await?;
            if count >= 5 {
                return Err(Error::Conflict);
            }
        }
        let changed = sqlx::query("INSERT INTO collection_decks(id,account_id,character_id,name,cards) VALUES($1,$2,$3,$4,$5) ON CONFLICT(id) DO UPDATE SET character_id=$3,name=$4,cards=$5,updated_at=now() WHERE collection_decks.account_id=$2")
            .bind(deck.id).bind(account).bind(deck.character).bind(deck.name).bind(deck.cards).execute(&mut *tx).await?;
        if changed.rows_affected() != 1 {
            return Err(Error::Denied);
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn start(
        &self,
        account: Uuid,
        deck_id: Uuid,
        authority: &impl Authority,
        now: u64,
    ) -> Result<Uuid> {
        self.eligible(account).await?;
        let deck = self
            .decks(account)
            .await?
            .into_iter()
            .find(|d| d.id == deck_id)
            .ok_or(Error::Denied)?;
        let cards = self.cards(account, deck.character, authority, now).await?;
        let player = require_cards(&cards, &deck.cards)?;
        // Symmetric mirrored encounter: no purchases, loot, ratings or currency.
        let enemy = player.clone();
        let seed = getrandom::u64().map_err(|_| Error::Unavailable)?;
        let battle = Battle::new(player, enemy, seed)?;
        let id = Uuid::now_v7();
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(account)
            .fetch_one(&mut *tx)
            .await?;
        let current: (Uuid,Vec<Uuid>) = sqlx::query_as("SELECT character_id,cards FROM collection_decks WHERE id=$1 AND account_id=$2 FOR UPDATE").bind(deck_id).bind(account).fetch_one(&mut *tx).await?;
        if current != (deck.character, deck.cards) {
            return Err(Error::Conflict);
        }
        let recent: i64 = sqlx::query_scalar("SELECT count(*) FROM collection_battles WHERE account_id=$1 AND created_at > clock_timestamp() - interval '1 hour'").bind(account).fetch_one(&mut *tx).await?;
        if recent >= 30 {
            return Err(Error::Conflict);
        }
        let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM collection_battles WHERE account_id=$1 AND state->>'outcome'='active')").bind(account).fetch_one(&mut *tx).await?;
        if active {
            return Err(Error::Conflict);
        }
        sqlx::query("INSERT INTO collection_battles(id,account_id,character_id,deck_id,state) VALUES($1,$2,$3,$4,$5)").bind(id).bind(account).bind(deck.character).bind(deck_id).bind(sqlx::types::Json(battle)).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(id)
    }
    pub async fn battle(
        &self,
        account: Uuid,
        id: Uuid,
        authority: &impl Authority,
        now: u64,
    ) -> Result<Battle> {
        self.eligible(account).await?;
        let (character, state): (Uuid, sqlx::types::Json<Battle>) = sqlx::query_as(
            "SELECT character_id,state FROM collection_battles WHERE id=$1 AND account_id=$2",
        )
        .bind(id)
        .bind(account)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(Error::Denied)?;
        self.validate_battle(account, character, &state.0, authority, now)
            .await?;
        Ok(state.0)
    }
    async fn validate_battle(
        &self,
        account: Uuid,
        character: Uuid,
        state: &Battle,
        authority: &impl Authority,
        now: u64,
    ) -> Result<()> {
        let cards = self.cards(account, character, authority, now).await?;
        require_cards(
            &cards,
            &state.player.iter().map(|f| f.card).collect::<Vec<_>>(),
        )?;
        Ok(())
    }
    pub async fn turn(
        &self,
        account: Uuid,
        id: Uuid,
        expected: u32,
        action: Action,
        authority: &impl Authority,
        now: u64,
    ) -> Result<()> {
        // Check ownership and freshness before exposing or changing any private state.
        let checked = self.battle(account, id, authority, now).await?;
        let mut tx = self.pool.begin().await?;
        let sqlx::types::Json(mut state): sqlx::types::Json<Battle> = sqlx::query_scalar(
            "SELECT state FROM collection_battles WHERE id=$1 AND account_id=$2 FOR UPDATE",
        )
        .bind(id)
        .bind(account)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Denied)?;
        if checked.player.iter().map(|f| f.card).collect::<Vec<_>>()
            != state.player.iter().map(|f| f.card).collect::<Vec<_>>()
        {
            return Err(Error::Conflict);
        }
        state.advance(expected, action)?;
        sqlx::query(
            "UPDATE collection_battles SET state=$3,updated_at=now() WHERE id=$1 AND account_id=$2",
        )
        .bind(id)
        .bind(account)
        .bind(sqlx::types::Json(state))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn sprite(
        &self,
        account: Uuid,
        character: Uuid,
        id: Uuid,
        authority: &impl Authority,
        now: u64,
    ) -> Result<Vec<u8>> {
        if !self
            .cards(account, character, authority, now)
            .await?
            .iter()
            .any(|c| c.card.id == id)
        {
            return Err(Error::Denied);
        }
        sqlx::query_scalar("SELECT sprite FROM collection_cards WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(Error::Unavailable)
    }
}
fn require_cards(cards: &[OwnedCard], ids: &[Uuid]) -> Result<Vec<Fighter>> {
    valid_deck("deck", ids)?;
    ids.iter()
        .map(|id| {
            cards
                .iter()
                .find(|c| c.card.id == *id && c.unlocked)
                .map(|c| c.card.fighter())
                .ok_or(Error::Denied)
        })
        .collect()
}
impl Collection {
    /// Private resume link; the actual battle still revalidates its source.
    pub async fn active_battle(&self, account: Uuid, character: Uuid) -> Result<Option<Uuid>> {
        self.eligible(account).await?;
        Ok(sqlx::query_scalar("SELECT id FROM collection_battles WHERE account_id=$1 AND character_id=$2 AND state->>'outcome'='active'").bind(account).bind(character).fetch_optional(&self.pool).await?)
    }
    /// Owner-only cleanup remains possible after a source is retired or a
    /// character is transferred. It exposes no history and grants no rewards.
    pub async fn forfeit(&self, account: Uuid, id: Uuid) -> Result<Uuid> {
        self.eligible(account).await?;
        sqlx::query_scalar("UPDATE collection_battles SET state=jsonb_set(state,'{outcome}','\"defeat\"'),updated_at=now() WHERE id=$1 AND account_id=$2 AND state->>'outcome'='active' RETURNING character_id").bind(id).bind(account).fetch_optional(&self.pool).await?.ok_or(Error::Denied)
    }
}

impl Collection {
    pub async fn delete_deck(&self, account: Uuid, id: Uuid) -> Result<Uuid> {
        self.eligible(account).await?;
        sqlx::query_scalar(
            "DELETE FROM collection_decks WHERE id=$1 AND account_id=$2 RETURNING character_id",
        )
        .bind(id)
        .bind(account)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(Error::Denied)
    }
    pub async fn battle_character(&self, account: Uuid, id: Uuid) -> Result<Uuid> {
        self.eligible(account).await?;
        sqlx::query_scalar(
            "SELECT character_id FROM collection_battles WHERE id=$1 AND account_id=$2",
        )
        .bind(id)
        .bind(account)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(Error::Denied)
    }
}
