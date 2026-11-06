//! Production composition. One feature flag, real private Game-initiated exchange,
//! browser sessions and a bounded worker; no test producer is linked here.
mod browser;
pub(crate) mod characters;
mod config;
pub(crate) mod gold;
mod gold_worker;
mod tls;
mod transport;
mod worker;
use super::authority_wire::{decode, number, same};
use super::game_wire::{decimal, digest, id, text};
use super::{
    Authority, CoinReservations, Command, Error, Outbox, PlatformAccounts, PreparedIntent, Settings,
};
use crate::{Accounts, Bazaar, Currency, Flags, Form, Kind, Listing, NewListing};
pub use config::{Config, Fence};
use oteryn_internal_api::canonical::{self, Value};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool, Row};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Runtime {
    pub pool: PgPool,
    pub config: Arc<Config>,
}
impl Runtime {
    pub async fn configured(pool: PgPool) -> Result<Option<Self>, Error> {
        let Some(config) = Config::from_env()? else {
            return Ok(None);
        };
        config.current()?;
        let mut tx = pool.begin().await?;
        Outbox::reopen(
            &mut tx,
            config.fence.projection_epoch,
            config.fence.recovery_generation,
        )
        .await?;
        for (kid, key) in &config.authority.config.snapshot_keys {
            let old=sqlx::query("SELECT public_key,revoked FROM bazaar_authority_snapshot_keys WHERE kid=$1 FOR UPDATE").bind(kid).fetch_optional(&mut *tx).await?;
            if let Some(old) = old {
                if old.try_get::<bool, _>("revoked")?
                    || old.try_get::<Vec<u8>, _>("public_key")? != key.as_bytes()
                {
                    return Err(Error::Denied);
                }
            } else {
                sqlx::query(
                    "INSERT INTO bazaar_authority_snapshot_keys(kid,public_key) VALUES($1,$2)",
                )
                .bind(kid)
                .bind(key.as_bytes().as_slice())
                .execute(&mut *tx)
                .await?;
            }
        }
        let kids = config
            .authority
            .config
            .snapshot_keys
            .iter()
            .map(|(kid, _)| kid.clone())
            .collect::<Vec<_>>();
        sqlx::query(
            "UPDATE bazaar_authority_snapshot_keys SET revoked=true WHERE NOT (kid=ANY($1))",
        )
        .bind(kids)
        .execute(&mut *tx)
        .await?;
        if let Some(escrow) = config.escrow_account {
            let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND status='active' AND email_verified_at IS NOT NULL)").bind(escrow).fetch_one(&mut *tx).await?;
            if !active {
                return Err(Error::Unavailable);
            }
        }
        tx.commit().await?;
        Ok(Some(Self {
            pool,
            config: Arc::new(config),
        }))
    }
    pub fn authority(&self) -> &Authority {
        &self.config.authority
    }
    fn accounts(&self, session_hash: Option<[u8; 32]>) -> PlatformAccounts {
        PlatformAccounts {
            enabled: true,
            session_hash,
        }
    }
    fn core(
        &self,
        hash: Option<[u8; 32]>,
    ) -> Bazaar<PlatformAccounts, CoinReservations, Authority> {
        Bazaar::new(
            self.pool.clone(),
            self.accounts(hash),
            CoinReservations {
                authority: self.authority().clone(),
            },
            self.authority().clone(),
            Flags {
                enabled: true,
                gold_enabled: true,
            },
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn issue(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        listing: &Listing,
        actor: Uuid,
        hash: Option<[u8; 32]>,
        command: Command,
        step: Uuid,
        extra: Vec<(&str, Value)>,
    ) -> Result<Uuid, Error> {
        self.authority().lock(tx).await?;
        let key = format!("bazaar:{}:{}:{step}:1", listing.id, command.wire());
        if let Some(raw) = sqlx::query_scalar::<_, String>(
            "SELECT intent FROM bazaar_game_outbox WHERE issuance_key=$1",
        )
        .bind(&key)
        .fetch_optional(&mut **tx)
        .await?
        {
            let intent = PreparedIntent::parse(&raw)?;
            for (k, v) in extra {
                if !k.starts_with("expected_")
                    && k != "eligibility_revision"
                    && intent.value.get(k) != Some(&v)
                {
                    return Err(Error::Conflict);
                }
            }
            return id(text(&intent.value, "operation_id")?);
        }
        let asset = sqlx::query("SELECT world_id FROM bazaar_authority_assets WHERE listing_id=$1")
            .bind(listing.id)
            .fetch_one(&mut **tx)
            .await?;
        let now = now(tx).await?;
        let previous: String = sqlx::query_scalar(
            "SELECT source_revision::text FROM bazaar_game_issuer WHERE singleton",
        )
        .fetch_one(&mut **tx)
        .await?;
        let revision = decimal(&previous, true, u64::MAX)?
            .checked_add(1)
            .ok_or(Error::Invalid)?;
        let operation = oteryn_core::new_id();
        let decision = oteryn_core::new_id();
        let mut members = vec![
            ("contract_version".into(), Value::Unsigned(2)),
            ("command".into(), string(command.wire())),
            ("variant".into(), string("PLATFORM_BAZAAR")),
            (
                "issuer_authority".into(),
                string("OTERYN_PLATFORM_BAZAAR_AUTHORITY"),
            ),
            ("issuer_decision_id".into(), string(decision)),
            ("source_revision".into(), string(revision)),
            ("operation_id".into(), string(operation)),
            ("listing_id".into(), string(listing.id)),
            ("seller_account_id".into(), string(listing.owner)),
            (
                "world_id".into(),
                string(asset.try_get::<Uuid, _>("world_id")?),
            ),
        ];
        members.extend(extra.into_iter().map(|(k, v)| (k.to_owned(), v)));
        members.extend([
            ("issued_at_source".into(), string(now)),
            (
                "expires_at_source".into(),
                string(now.checked_add(300).ok_or(Error::Invalid)?),
            ),
            ("audience".into(), string("OTERYN_GAME_BAZAAR_AUTHORITY")),
        ]);
        let raw = String::from_utf8(
            canonical::encode(&Value::Object(members)).map_err(|_| Error::Invalid)?,
        )
        .map_err(|_| Error::Invalid)?;
        let intent = PreparedIntent::parse(&raw)?;
        let outbox = Outbox {
            settings: Settings {
                preparation_enabled: true,
                game_enabled: true,
            },
        };
        outbox
            .stage(
                tx,
                &self.accounts(hash),
                actor,
                step,
                &intent,
                self.config.fence.projection_epoch,
                self.config.fence.recovery_generation,
            )
            .await?;
        self.authority()
            .publish_intent(tx, &self.accounts(hash), operation)
            .await?;
        Ok(operation)
    }
    pub(super) async fn latest(
        &self,
        db: &mut PgConnection,
        listing: Uuid,
        command: Command,
    ) -> Result<Option<(PreparedIntent, super::Receipt)>, Error> {
        let operation:Option<Uuid>=sqlx::query_scalar("SELECT operation_id FROM bazaar_game_outbox WHERE listing_id=$1 AND command=$2 ORDER BY source_revision DESC LIMIT 1").bind(listing).bind(command.wire()).fetch_optional(&mut *db).await?;
        match operation {
            Some(operation) => self.authority().retained(db, operation).await,
            None => Ok(None),
        }
    }
    pub async fn create(
        &self,
        actor: Uuid,
        session_hash: [u8; 32],
        input: NewAuction,
    ) -> Result<Uuid, Error> {
        self.config.current()?;
        input.validate()?;
        if !self.config.worlds.contains(&input.world_id) {
            return Err(Error::Denied);
        }
        let currency = match input.currency.as_str() {
            "COINS" => Currency::Coins,
            "GOLD" => {
                super::gold_payout(input.starting_bid as u64)?;
                Currency::Gold
            }
            _ => return Err(Error::Invalid),
        };
        let binding = serde_json::to_string(&input).map_err(|_| Error::Invalid)?;
        let mut tx = self.pool.begin().await?;
        let state = self
            .accounts(Some(session_hash))
            .state(&mut tx, actor)
            .await
            .map_err(core_error)?
            .ok_or(Error::Denied)?;
        if !state.allows_listing() {
            return Err(Error::Denied);
        }
        self.authority().lock(&mut tx).await?;
        if let Some(row)=sqlx::query("SELECT a.listing_id,a.input_binding,l.owner FROM bazaar_authority_assets a JOIN bazaar_listings l ON l.id=a.listing_id WHERE a.request_id=$1").bind(input.request_id).fetch_optional(&mut *tx).await? {
            if row.try_get::<Uuid,_>("owner")?!=actor || row.try_get::<String,_>("input_binding")?!=binding{return Err(Error::Conflict);}return Ok(row.try_get("listing_id")?);
        }
        if !self.authority().fresh(&mut tx).await? {
            return Err(Error::Unavailable);
        }
        let world: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM world_status WHERE id=$1)")
                .bind(input.world_id)
                .fetch_one(&mut *tx)
                .await?;
        if !world {
            return Err(Error::Denied);
        }
        let duplicate:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_authority_assets a JOIN bazaar_listings l ON l.id=a.listing_id WHERE a.asset_id=$1 AND l.state NOT IN ('completed','cancelled','expired') AND a.refusal_code IS NULL)").bind(input.asset_id).fetch_one(&mut *tx).await?;
        if duplicate {
            return Err(Error::Conflict);
        }
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM bazaar_listings WHERE owner=$1 AND state NOT IN ('completed','cancelled','expired')").bind(actor).fetch_one(&mut *tx).await?;
        if count >= 24 {
            return Err(Error::Backlog);
        }
        let now = OffsetDateTime::from_unix_timestamp(now(&mut tx).await? as i64)
            .map_err(|_| Error::Invalid)?;
        let kind = if input.kind == "character" {
            Kind::Character
        } else {
            Kind::Item
        };
        let listing = NewListing {
            owner: actor,
            kind,
            form: Form::Auction,
            currency,
            price: input.starting_bid,
            accept_offers: false,
            ends_at: now + Duration::days(i64::from(input.duration_days)),
        };
        listing.validate(now).map_err(core_error)?;
        let listing_id = oteryn_core::new_id();
        sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,$3,'auction',$4,$5,false,$6)").bind(listing_id).bind(actor).bind(kind).bind(currency).bind(input.starting_bid).bind(listing.ends_at).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO bazaar_authority_assets(listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,session_hash,definition_key,definition_revision,quantity,publication_consent,input_binding) VALUES($1,$2,$3,$4,$5,$6::text::numeric,$7,$8,$9,$10,true,$11)")
            .bind(listing_id).bind(input.request_id).bind(input.world_id).bind(input.seller_character_id).bind(input.asset_id).bind(&input.expected_revision).bind(session_hash.to_vec()).bind(&input.definition_key).bind(&input.definition_revision).bind(i64::from(input.quantity)).bind(binding).execute(&mut *tx).await?;
        if kind == Kind::Item {
            sqlx::query("INSERT INTO bazaar_item_details(id,definition_key,definition_revision,world_id,seller_character_id,item_instance_id,quantity,requested_price) VALUES($1,$2,$3,$4,$5,$6,$7,$8)").bind(listing_id).bind(&input.definition_key).bind(&input.definition_revision).bind(input.world_id).bind(input.seller_character_id).bind(input.asset_id).bind(i64::from(input.quantity)).bind(input.starting_bid).execute(&mut *tx).await?;
        }
        let stored: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1")
            .bind(listing_id)
            .fetch_one(&mut *tx)
            .await?;
        self.stage_deposit(&mut tx, &stored, Some(session_hash))
            .await?;
        tx.commit().await?;
        Ok(listing_id)
    }
    pub async fn bid(
        &self,
        actor: Uuid,
        listing: Uuid,
        maximum: i64,
        buyer_character: Uuid,
        key: Uuid,
    ) -> Result<(), Error> {
        self.config.current()?;
        id(&key.to_string())?;
        id(&buyer_character.to_string())?;
        // Destination request, full funding, bid and replay fence share one SQL transaction.
        let mut tx = self.pool.begin().await?;
        let row: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 FOR UPDATE")
            .bind(listing)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
        if actor == row.owner {
            return Err(Error::Denied);
        }
        if row.currency == Currency::Gold {
            self.request_gold_bid(&mut tx, &row, actor, maximum, buyer_character, key)
                .await?;
            tx.commit().await?;
            return Ok(());
        }

        let old=sqlx::query("SELECT listing_id,buyer,character_id,maximum FROM bazaar_authority_bid_requests WHERE request_id=$1").bind(key).fetch_optional(&mut *tx).await?;
        if let Some(old) = old {
            if old.try_get::<Uuid, _>("listing_id")? != listing
                || old.try_get::<Uuid, _>("buyer")? != actor
                || old.try_get::<Uuid, _>("character_id")? != buyer_character
                || old.try_get::<i64, _>("maximum")? != maximum
            {
                return Err(Error::Conflict);
            }
            self.core(None)
                .bid_in(&mut tx, listing, actor, maximum, &key.to_string())
                .await
                .map_err(core_error)?;
            tx.commit().await?;
            return Ok(());
        }
        if !crate::Evidence::escrow_ready(self.authority(), &mut tx, &row)
            .await
            .map_err(core_error)?
        {
            return Err(Error::Unavailable);
        }
        let outcome = self
            .core(None)
            .bid_in(&mut tx, listing, actor, maximum, &key.to_string())
            .await
            .map_err(core_error)?;
        sqlx::query("INSERT INTO bazaar_authority_bid_requests(request_id,listing_id,buyer,character_id,maximum) VALUES($1,$2,$3,$4,$5)").bind(key).bind(listing).bind(actor).bind(buyer_character).bind(maximum).execute(&mut *tx).await?;
        if outcome.leader == actor {
            sqlx::query("INSERT INTO bazaar_authority_destinations(listing_id,buyer,character_id) VALUES($1,$2,$3) ON CONFLICT(listing_id,buyer) DO UPDATE SET character_id=EXCLUDED.character_id").bind(listing).bind(actor).bind(buyer_character).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewAuction {
    pub request_id: Uuid,
    #[serde(default = "coins")]
    pub currency: String,
    pub kind: String,
    pub world_id: Uuid,
    pub seller_character_id: Uuid,
    pub asset_id: Uuid,
    pub expected_revision: String,
    pub quantity: u32,
    pub definition_key: String,
    pub definition_revision: String,
    pub starting_bid: i64,
    pub duration_days: u8,
    pub publish_snapshot: bool,
    #[serde(default, skip_serializing)]
    pub _csrf: Option<String>,
}
impl NewAuction {
    fn validate(&self) -> Result<(), Error> {
        for value in [
            self.request_id,
            self.world_id,
            self.seller_character_id,
            self.asset_id,
        ] {
            id(&value.to_string())?;
        }
        decimal(&self.expected_revision, false, u64::MAX)?;
        if !matches!(self.kind.as_str(), "character" | "item")
            || !(1..=14).contains(&self.duration_days)
            || self.starting_bid <= 0
            || self.quantity == 0
            || !self.publish_snapshot
        {
            return Err(Error::Invalid);
        }
        if self.kind == "character"
            && (self.asset_id != self.seller_character_id || self.quantity != 1)
        {
            return Err(Error::Invalid);
        }
        if self.kind == "item"
            && (!super::authority_wire::key(&self.definition_key)
                || !super::authority_wire::key(&self.definition_revision))
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
pub(super) fn string(v: impl ToString) -> Value {
    Value::String(v.to_string())
}
pub(super) fn core_error(e: crate::Error) -> Error {
    match e {
        crate::Error::Database(e) => Error::Database(e),
        crate::Error::Disabled => Error::Unavailable,
        crate::Error::IdempotencyConflict => Error::Conflict,
        crate::Error::NotFound => Error::NotFound,
        crate::Error::InvalidInput | crate::Error::Overflow => Error::Invalid,
        _ => Error::Denied,
    }
}
pub(super) async fn now(db: &mut PgConnection) -> Result<u64, Error> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(db)
            .await?;
    u64::try_from(now).map_err(|_| Error::Invalid)
}

fn coins() -> String {
    "COINS".into()
}

impl Runtime {
    pub async fn control(
        &self,
        actor: Uuid,
        hash: [u8; 32],
        listing: Uuid,
        request: Uuid,
        action: &str,
    ) -> Result<(), Error> {
        self.config.current()?;
        id(&request.to_string())?;
        if !matches!(action, "resume" | "cancel") {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        let l: Listing = sqlx::query_as("SELECT * FROM bazaar_listings WHERE id=$1 FOR UPDATE")
            .bind(listing)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::NotFound)?;
        if l.owner != actor {
            return Err(Error::Denied);
        }
        self.authority().lock(&mut tx).await?;
        if let Some(row)=sqlx::query("SELECT listing_id,actor,action FROM bazaar_authority_browser_requests WHERE request_id=$1").bind(request).fetch_optional(&mut *tx).await? {
            if row.try_get::<Uuid,_>("listing_id")?!=listing || row.try_get::<Uuid,_>("actor")?!=actor || row.try_get::<String,_>("action")?!=action{return Err(Error::Conflict);}return Ok(());
        }
        if action == "resume" {
            if !self
                .accounts(Some(hash))
                .state(&mut tx, actor)
                .await
                .map_err(core_error)?
                .ok_or(Error::Denied)?
                .allows_listing()
            {
                return Err(Error::Denied);
            }
            sqlx::query("UPDATE bazaar_authority_assets SET session_hash=$2 WHERE listing_id=$1 AND refusal_code IS NULL").bind(listing).bind(hash.to_vec()).execute(&mut *tx).await?;
        } else {
            if !self
                .accounts(None)
                .safe_return(&mut tx, actor)
                .await
                .map_err(core_error)?
                || !matches!(l.state, crate::State::Draft | crate::State::Active)
                || l.leader.is_some()
            {
                return Err(Error::Denied);
            }
            let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_authority_gold_bids WHERE listing_id=$1 AND phase IN ('pending','releasing'))").bind(listing).fetch_one(&mut *tx).await?;
            if pending {
                return Err(Error::Unavailable);
            }
            sqlx::query(
                "UPDATE bazaar_authority_assets SET cancel_requested=true WHERE listing_id=$1",
            )
            .bind(listing)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("INSERT INTO bazaar_authority_browser_requests(request_id,listing_id,actor,action) VALUES($1,$2,$3,$4)").bind(request).bind(listing).bind(actor).bind(action).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}
