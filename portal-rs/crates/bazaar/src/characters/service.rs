use super::*;
use crate::{
    Accounts, Bazaar, BidOutcome, Currency, Error, Evidence, Flags, Form, Kind, Listing,
    NewListing, Reservations, State,
};
use sqlx::{PgConnection, PgPool};
use std::future::Future;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// T58 must lock durable evidence in this transaction until commit. No network I/O.
/// Return None for unknown/stale/revoked trust, missing publication consent, restore fences,
/// ambiguous escrow/CHAR_CMD receipts or unsupported capabilities. Verify the exact seller,
/// character and listing binding, including the 30-day resale restriction and ownership cut.
/// A fresh projection alone is never proof of custody. No separate Bazaar receipt is assumed.
pub trait CharacterEvidence: Evidence + Clone + Send + Sync + 'static {
    /// Current owner-only selection from durable account projection, never a browser assertion.
    fn choices(
        &self,
        db: &mut PgConnection,
        actor: Uuid,
    ) -> impl Future<Output = Result<Vec<CharacterChoice>, Error>> + Send;
    fn approved(
        &self,
        db: &mut PgConnection,
        listing: &Listing,
    ) -> impl Future<Output = Result<Option<SaleSnapshot>, Error>> + Send;
}

#[derive(Clone, Debug)]
pub struct CharacterMarket<A, R, G> {
    pool: PgPool,
    accounts: A,
    reservations: R,
    game: G,
    flags: CharacterFlags,
}
impl<A: Accounts + Clone, R: Reservations + Clone, G: CharacterEvidence> CharacterMarket<A, R, G> {
    pub fn new(pool: PgPool, accounts: A, reservations: R, game: G, flags: CharacterFlags) -> Self {
        Self {
            pool,
            accounts,
            reservations,
            game,
            flags,
        }
    }
    fn enabled(&self) -> Result<(), Error> {
        if self.flags.enabled {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }
    pub async fn own_requests(&self, actor: Uuid) -> Result<Vec<OwnRequest>, Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let state = self
            .accounts
            .state(&mut tx, actor)
            .await?
            .ok_or(Error::Denied)?;
        if !state.allows_trade() {
            return Err(Error::Denied);
        }
        Ok(sqlx::query_as("SELECT l.id,l.state::text AS state FROM bazaar_listings l JOIN bazaar_character_intents i ON i.listing_id=l.id WHERE l.owner=$1 ORDER BY l.created_at DESC LIMIT 50")
            .bind(actor).fetch_all(&mut *tx).await?)
    }
    pub async fn choices(&self, actor: Uuid) -> Result<Vec<CharacterChoice>, Error> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let state = self
            .accounts
            .state(&mut tx, actor)
            .await?
            .ok_or(Error::Denied)?;
        if !state.allows_listing() {
            return Err(Error::Denied);
        }
        self.game.choices(&mut tx, actor).await
    }
    /// Persist a local intent even while game dispatch is disabled. This is not a wire command.
    /// T58 must translate it only after capability qualification; no worker reads this table yet.
    pub async fn request(&self, actor: Uuid, input: &ListingRequest) -> Result<Uuid, Error> {
        self.enabled()?;
        let (character, key, currency, price) = input.validate()?;
        super::id(&actor.to_string())?;
        if currency == Currency::Gold && !self.flags.gold {
            return Err(Error::Disabled);
        }
        let mut tx = self.pool.begin().await?;
        let state = self
            .accounts
            .state(&mut tx, actor)
            .await?
            .ok_or(Error::Denied)?;
        if !state.allows_listing() {
            return Err(Error::Denied);
        }
        if !self
            .game
            .choices(&mut tx, actor)
            .await?
            .iter()
            .any(|c| c.id == character)
        {
            return Err(Error::Denied);
        }
        // Serialize retries, backlog and duplicate Character requests for this authenticated owner.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,59))")
            .bind(actor.to_string())
            .execute(&mut *tx)
            .await?;
        let old: Option<(Uuid, Uuid, String, i64, i16)> = sqlx::query_as("SELECT listing_id,character_id,currency,starting_bid,duration_days FROM bazaar_character_intents WHERE owner=$1 AND request_id=$2")
            .bind(actor).bind(key).fetch_optional(&mut *tx).await?;
        if let Some((id, c, cur, p, d)) = old {
            if c != character
                || cur != input.currency
                || p != price
                || d != i16::from(input.duration_days)
            {
                return Err(Error::IdempotencyConflict);
            }
            return Ok(id);
        }
        let duplicate: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_character_intents i JOIN bazaar_listings l ON l.id=i.listing_id WHERE i.character_id=$1 AND l.state NOT IN ('completed','cancelled','expired'))")
            .bind(character).fetch_one(&mut *tx).await?;
        if duplicate {
            return Err(Error::Denied);
        }
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_character_intents i JOIN bazaar_listings l ON l.id=i.listing_id WHERE i.owner=$1 AND l.state NOT IN ('completed','cancelled','expired')")
            .bind(actor).fetch_one(&mut *tx).await?;
        let recent: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_character_intents WHERE owner=$1 AND created_at > clock_timestamp()-interval '60 seconds'")
            .bind(actor).fetch_one(&mut *tx).await?;
        if count >= 24 || recent >= 20 {
            return Err(Error::Denied);
        }
        let now: OffsetDateTime = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        let listing = NewListing {
            owner: actor,
            kind: Kind::Character,
            form: Form::Auction,
            currency,
            price,
            accept_offers: false,
            ends_at: now + Duration::days(i64::from(input.duration_days)),
        };
        listing.validate(now)?;
        let id = oteryn_core::new_id();
        sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,'character','auction',$3,$4,false,$5)")
            .bind(id).bind(actor).bind(currency).bind(price).bind(listing.ends_at).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO bazaar_character_intents(listing_id,owner,character_id,request_id,currency,starting_bid,duration_days,publication_consent) VALUES($1,$2,$3,$4,$5,$6,$7,true)")
            .bind(id).bind(actor).bind(character).bind(key).bind(&input.currency).bind(price).bind(i16::from(input.duration_days)).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(id)
    }
    /// Publish only after verified escrow and approved immutable snapshot evidence.
    pub async fn publish(&self, actor: Uuid, id: Uuid) -> Result<(), Error> {
        self.enabled()?;
        if !self.flags.game_commands {
            return Err(Error::Disabled);
        }
        let mut tx = self.pool.begin().await?;
        let l: Listing = sqlx::query_as(
            "SELECT * FROM bazaar_listings WHERE id=$1 AND kind='character' FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?;
        let state = self
            .accounts
            .state(&mut tx, actor)
            .await?
            .ok_or(Error::Denied)?;
        let now: OffsetDateTime = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        if l.owner != actor
            || !state.allows_listing()
            || l.state != State::Draft
            || l.ends_at <= now
        {
            return Err(Error::Denied);
        }
        let consent: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_character_intents WHERE listing_id=$1 AND owner=$2 AND publication_consent)")
            .bind(id).bind(actor).fetch_one(&mut *tx).await?;
        if !consent || !self.game.escrow_ready(&mut tx, &l).await? {
            return Err(Error::Denied);
        }
        let snapshot = self
            .game
            .approved(&mut tx, &l)
            .await?
            .ok_or(Error::Denied)?;
        snapshot.validate()?;
        let body = serde_json::to_string(&snapshot).map_err(|_| Error::Denied)?;
        sqlx::query(
            "INSERT INTO bazaar_character_publications(listing_id,projection) VALUES($1,$2)",
        )
        .bind(id)
        .bind(body)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE bazaar_listings SET state='active',updated_at=clock_timestamp() WHERE id=$1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn detail(&self, id: Uuid) -> Result<CharacterCard, Error> {
        self.enabled()?;
        if !self.flags.game_commands {
            return Err(Error::NotFound);
        }
        let mut tx = self.pool.begin().await?;
        let l: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 AND kind='character' AND state='active' AND ends_at>clock_timestamp() FOR SHARE")
            .bind(id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        let body: String = sqlx::query_scalar(
            "SELECT projection FROM bazaar_character_publications WHERE listing_id=$1",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::NotFound)?;
        let stored: SaleSnapshot = serde_json::from_str(&body).map_err(|_| Error::Denied)?;
        let current = self
            .game
            .approved(&mut tx, &l)
            .await?
            .ok_or(Error::NotFound)?;
        current.validate()?;
        if stored != current {
            return Err(Error::NotFound);
        }
        tx.commit().await?;
        Ok(CharacterCard {
            id,
            snapshot: stored,
            price: l.price,
            currency: l.currency,
            ends_at: l.ends_at.to_string(),
        })
    }
    pub async fn cards(&self) -> Result<Vec<CharacterCard>, Error> {
        self.enabled()?;
        let ids: Vec<Uuid> = sqlx::query_scalar("SELECT l.id FROM bazaar_listings l JOIN bazaar_character_publications p ON p.listing_id=l.id WHERE l.kind='character' AND l.state='active' AND l.ends_at>clock_timestamp() ORDER BY l.ends_at,l.id LIMIT 50")
            .fetch_all(&self.pool).await?;
        let mut cards = Vec::new();
        for id in ids {
            match self.detail(id).await {
                Ok(card) => cards.push(card),
                Err(Error::NotFound | Error::Denied) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(cards)
    }
    pub async fn bid(
        &self,
        actor: Uuid,
        id: Uuid,
        maximum: &str,
        key: &str,
    ) -> Result<BidOutcome, Error> {
        self.enabled()?;
        if !self.flags.game_commands {
            return Err(Error::Disabled);
        }
        super::id(key)?;
        let card = self.detail(id).await?;
        let max = amount(maximum, card.currency)?;
        // The account adapter rechecks evidence under the core's listing lock, so a restore or
        // trust change between page display and submission cannot authorize a stale bid.
        let accounts = BidAccounts {
            inner: self.accounts.clone(),
            game: self.game.clone(),
            id,
        };
        let core = Bazaar::new(
            self.pool.clone(),
            accounts,
            self.reservations.clone(),
            self.game.clone(),
            Flags {
                enabled: true,
                gold_enabled: self.flags.gold,
            },
        );
        core.bid(id, actor, max, key).await
    }
}
struct BidAccounts<A, G> {
    inner: A,
    game: G,
    id: Uuid,
}
impl<A: Accounts, G: CharacterEvidence> Accounts for BidAccounts<A, G> {
    async fn state(
        &self,
        db: &mut PgConnection,
        account: Uuid,
    ) -> Result<Option<crate::AccountState>, Error> {
        let l: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
            .bind(self.id)
            .fetch_one(&mut *db)
            .await?;
        if l.kind != Kind::Character || !self.game.escrow_ready(db, &l).await? {
            return Ok(None);
        }
        let Some(snapshot) = self.game.approved(db, &l).await? else {
            return Ok(None);
        };
        snapshot.validate()?;
        let stored: Option<String> = sqlx::query_scalar(
            "SELECT projection FROM bazaar_character_publications WHERE listing_id=$1",
        )
        .bind(self.id)
        .fetch_optional(&mut *db)
        .await?;
        let stored: SaleSnapshot =
            serde_json::from_str(&stored.ok_or(Error::Denied)?).map_err(|_| Error::Denied)?;
        if stored != snapshot {
            return Ok(None);
        }
        self.inner.state(db, account).await
    }
}
