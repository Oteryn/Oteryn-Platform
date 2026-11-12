//! T64 restrictive policy. G-7 revision 2, §§1, 5, 7–9 governs compensation.
//! T59/T60 are not available: approved public facts and paid promotion use narrow
//! transaction-local ports. Missing adapters deny, without manufacturing evidence.
use crate::{Accounts, Error, State};
use serde::Deserialize;
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use std::future::Future;
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(test)]
#[path = "moderation_tests.rs"]
mod tests;
#[path = "moderation_web.rs"]
pub mod web;

#[derive(Debug, Clone, Copy, Default)]
pub struct Flags {
    pub enabled: bool,
    pub public_profiles: bool,
    pub promotions: bool,
}

/// Contains only currently approved public facts, never account email or bid maxima.
#[derive(Debug, Clone)]
pub struct SellerProfile {
    pub character_name: String,
    pub joined: String,
    pub completed_transactions: i64,
    pub listings: Vec<PublicListing>,
}
#[derive(Debug, Clone)]
pub struct PublicListing {
    pub id: Uuid,
    pub title: String,
    pub featured: bool,
}
/// Recheck current ownership, privacy, epoch, recovery and freshness before returning
/// public facts. Unproven facts return None, including a hidden selected Character.
pub trait PublicViews: Send + Sync {
    fn seller(
        &self,
        db: &mut PgConnection,
        seller: Uuid,
    ) -> impl Future<Output = Result<Option<SellerProfile>, Error>> + Send;
    fn listing_visible(
        &self,
        db: &mut PgConnection,
        listing: Uuid,
    ) -> impl Future<Output = Result<bool, Error>> + Send;
}
/// Debit coins and write the immutable Wallet ledger in this transaction.
/// The key is the promotion ID; errors roll back the promotion and debit together.
pub trait PromotionLedger: Send + Sync {
    fn debit(
        &self,
        db: &mut PgConnection,
        owner: Uuid,
        amount: i64,
        key: Uuid,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}
#[derive(Debug, Clone, Copy)]
pub struct PendingIntegration;
impl PublicViews for PendingIntegration {
    async fn seller(&self, _: &mut PgConnection, _: Uuid) -> Result<Option<SellerProfile>, Error> {
        Ok(None)
    }
    async fn listing_visible(&self, _: &mut PgConnection, _: Uuid) -> Result<bool, Error> {
        Ok(false)
    }
}
impl Accounts for PendingIntegration {
    async fn state(
        &self,
        _: &mut PgConnection,
        _: Uuid,
    ) -> Result<Option<crate::AccountState>, Error> {
        Ok(None)
    }
}
impl PromotionLedger for PendingIntegration {
    async fn debit(&self, _: &mut PgConnection, _: Uuid, _: i64, _: Uuid) -> Result<(), Error> {
        Err(Error::Denied)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct PromotionQuote {
    pub revision: Uuid,
    pub amount: i64,
}

#[derive(Debug, Clone)]
pub struct Moderation<A, V, L> {
    pool: PgPool,
    accounts: A,
    views: V,
    ledger: L,
    flags: Flags,
}
impl<A: Accounts, V: PublicViews, L: PromotionLedger> Moderation<A, V, L> {
    pub fn new(pool: PgPool, accounts: A, views: V, ledger: L, flags: Flags) -> Self {
        Self {
            pool,
            accounts,
            views,
            ledger,
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
    async fn authorize(&self, db: &mut PgConnection, actor: Uuid) -> Result<(), Error> {
        if actor.is_nil()
            || !self
                .accounts
                .state(db, actor)
                .await?
                .is_some_and(|s| s.allows_trade())
        {
            return Err(Error::Denied);
        }
        Ok(())
    }
    pub async fn seller(&self, seller: Uuid) -> Result<SellerProfile, Error> {
        self.enabled()?;
        if !self.flags.public_profiles {
            return Err(Error::Disabled);
        }
        valid_id(seller)?;
        let mut tx = self.pool.begin().await?;
        if !self
            .accounts
            .state(&mut tx, seller)
            .await?
            .is_some_and(|s| s.active)
        {
            return Err(Error::NotFound);
        }
        let mut profile = self
            .views
            .seller(&mut tx, seller)
            .await?
            .ok_or(Error::NotFound)?;
        if profile.character_name.is_empty()
            || profile.joined.is_empty()
            || profile.completed_transactions < 0
            || profile.listings.len() > 100
        {
            return Err(Error::Denied);
        }
        let mut visible = Vec::new();
        for mut listing in profile.listings {
            let allowed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_listings l WHERE l.id=$1 AND l.owner=$2 AND l.state='active' AND l.ends_at>clock_timestamp() AND NOT EXISTS(SELECT 1 FROM bazaar_moderation_holds h WHERE h.listing_id=l.id))")
                .bind(listing.id).bind(seller).fetch_one(&mut *tx).await?;
            if allowed && self.views.listing_visible(&mut tx, listing.id).await? {
                listing.featured=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_promotions WHERE listing_id=$1 AND owner=$2)").bind(listing.id).bind(seller).fetch_one(&mut *tx).await?;
                visible.push(listing);
            }
        }
        profile.completed_transactions = sqlx::query_scalar("SELECT count(*) FROM bazaar_settlements s JOIN bazaar_listings l ON l.id=s.listing_id WHERE s.seller=$1 AND s.completed_at IS NOT NULL AND l.state='completed'")
            .bind(seller).fetch_one(&mut *tx).await?;
        profile.listings = visible;
        tx.commit().await?;
        Ok(profile)
    }
    /// Idempotency is bound to the reporter, listing, category and exact detail.
    pub async fn report(&self, actor: Uuid, input: ReportInput) -> Result<Uuid, Error> {
        self.enabled()?;
        input.validate()?;
        let mut tx = self.pool.begin().await?;
        self.authorize(&mut tx, actor).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,64))")
            .bind(actor.to_string())
            .execute(&mut *tx)
            .await?;
        let replay: Option<(Uuid, Uuid, String, String)> = sqlx::query_as(
            "SELECT reporter, listing_id, category, detail FROM bazaar_reports WHERE id=$1",
        )
        .bind(input.id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(old) = replay {
            return if old == (actor, input.listing, input.category, input.detail) {
                Ok(input.id)
            } else {
                Err(Error::IdempotencyConflict)
            };
        }
        let (owner, state) = lock_listing(&mut tx, input.listing).await?;
        if actor == owner
            || state != State::Active
            || !self.views.listing_visible(&mut tx, input.listing).await?
        {
            return Err(Error::Denied);
        }
        let (pending, daily): (i64,i64) = sqlx::query_as("SELECT count(*) FILTER(WHERE state='pending'), count(*) FILTER(WHERE created_at>clock_timestamp()-interval '1 day') FROM bazaar_reports WHERE reporter=$1")
            .bind(actor).fetch_one(&mut *tx).await?;
        let duplicate: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_reports WHERE reporter=$1 AND listing_id=$2 AND state='pending')").bind(actor).bind(input.listing).fetch_one(&mut *tx).await?;
        if pending >= 5 || daily >= 20 || duplicate {
            return Err(Error::Denied);
        }
        sqlx::query("INSERT INTO bazaar_reports(id,listing_id,reporter,category,detail) VALUES($1,$2,$3,$4,$5)")
            .bind(input.id).bind(input.listing).bind(actor).bind(input.category).bind(input.detail).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(input.id)
    }
    pub async fn promotion_quote(&self) -> Result<PromotionQuote, Error> {
        self.enabled()?;
        if !self.flags.promotions {
            return Err(Error::Disabled);
        }
        sqlx::query_as("SELECT id AS revision,promotion_price AS amount FROM bazaar_fee_revisions ORDER BY created_at DESC,id DESC LIMIT 1")
            .fetch_optional(&self.pool).await?.ok_or(Error::Denied)
    }
    pub async fn promote(
        &self,
        actor: Uuid,
        listing: Uuid,
        id: Uuid,
        quote: PromotionQuote,
    ) -> Result<(), Error> {
        self.enabled()?;
        if !self.flags.promotions {
            return Err(Error::Disabled);
        }
        valid_id(id)?;
        let mut tx = self.pool.begin().await?;
        self.authorize(&mut tx, actor).await?;
        if !self
            .accounts
            .state(&mut tx, actor)
            .await?
            .is_some_and(|s| s.allows_listing())
        {
            return Err(Error::Denied);
        }
        let (owner, state) = lock_listing(&mut tx, listing).await?;
        if actor != owner {
            return Err(Error::Denied);
        }
        let old: Option<(Uuid, Uuid, Uuid, i64)> = sqlx::query_as(
            "SELECT id,owner,fee_revision,amount FROM bazaar_promotions WHERE listing_id=$1",
        )
        .bind(listing)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(old) = old {
            return if old == (id, actor, quote.revision, quote.amount) {
                Ok(())
            } else {
                Err(Error::IdempotencyConflict)
            };
        }
        if state != State::Active
            || held(&mut tx, listing).await?
            || !self.views.listing_visible(&mut tx, listing).await?
        {
            return Err(Error::Denied);
        }
        let (revision, amount): (Uuid,i64) = sqlx::query_as("SELECT id,promotion_price FROM bazaar_fee_revisions ORDER BY created_at DESC,id DESC LIMIT 1").fetch_optional(&mut *tx).await?.ok_or(Error::Denied)?;
        if quote != (PromotionQuote { revision, amount }) {
            return Err(Error::IdempotencyConflict);
        }
        let live: bool =
            sqlx::query_scalar("SELECT ends_at>clock_timestamp() FROM bazaar_listings WHERE id=$1")
                .bind(listing)
                .fetch_one(&mut *tx)
                .await?;
        if !live {
            return Err(Error::Denied);
        }
        sqlx::query("INSERT INTO bazaar_promotions(id,listing_id,owner,fee_revision,amount) VALUES($1,$2,$3,$4,$5)")
            .bind(id).bind(listing).bind(actor).bind(revision).bind(amount).execute(&mut *tx).await?;
        self.ledger.debit(&mut tx, actor, amount, id).await?;
        tx.commit().await?;
        Ok(())
    }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportInput {
    pub id: Uuid,
    pub listing: Uuid,
    pub category: String,
    pub detail: String,
}
impl ReportInput {
    fn validate(&self) -> Result<(), Error> {
        valid_id(self.id)?;
        valid_id(self.listing)?;
        if !["fraud", "inappropriate", "other"].contains(&self.category.as_str())
            || !text_valid(&self.detail, 10, 1000)
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}
fn valid_id(id: Uuid) -> Result<(), Error> {
    if id.is_nil() || id.get_version_num() != 7 {
        Err(Error::InvalidInput)
    } else {
        Ok(())
    }
}
fn text_valid(s: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&s.trim().chars().count()) && !s.chars().any(char::is_control)
}
async fn lock_listing(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<(Uuid, State), Error> {
    valid_id(id)?;
    sqlx::query_as("SELECT owner,state FROM bazaar_listings WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(Error::NotFound)
}
async fn held(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<bool, Error> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_moderation_holds WHERE listing_id=$1)",
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await?)
}

/// Only a server-authorized staff adapter may execute these commands. Call within
/// the same transaction as the staff audit entry; never accept actor from a form.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum StaffCommand {
    Suspend {
        listing: Uuid,
        reason: String,
    },
    RequestInvalidation {
        id: Uuid,
        listing: Uuid,
        reason: String,
    },
    ApproveInvalidation {
        id: Uuid,
    },
    ReviewReport {
        id: Uuid,
        accepted: bool,
    },
    Fees {
        id: Uuid,
        coin_bps: i32,
        gold_bps: i32,
        listing_fee: i64,
        promotion_price: i64,
        reason: String,
    },
    RestrictItem {
        id: Uuid,
        world: Uuid,
        family: String,
        production_key: String,
        revision_ref: String,
        minimum_value: i64,
        denied: bool,
        reason: String,
    },
}
impl StaffCommand {
    pub fn target(&self) -> Uuid {
        match self {
            Self::Suspend { listing, .. } | Self::RequestInvalidation { listing, .. } => *listing,
            Self::ApproveInvalidation { id }
            | Self::ReviewReport { id, .. }
            | Self::Fees { id, .. }
            | Self::RestrictItem { id, .. } => *id,
        }
    }
    pub fn action(&self) -> &'static str {
        match self {
            Self::Suspend { .. } => "bazaar.suspend",
            Self::RequestInvalidation { .. } => "bazaar.invalidation.request",
            Self::ApproveInvalidation { .. } => "bazaar.invalidation.approve",
            Self::ReviewReport { .. } => "bazaar.report.review",
            Self::Fees { .. } => "bazaar.fees.revise",
            Self::RestrictItem { .. } => "bazaar.items.restrict",
        }
    }
}
/// Retains all holds. Approval queues reconciliation only, with no automatic
/// dispatch, state cancellation, balance credit, fabricated receipt or asset reversal.
pub async fn lock_staff_target(
    tx: &mut Transaction<'_, Postgres>,
    command: &StaffCommand,
) -> Result<(), Error> {
    match command {
        StaffCommand::Suspend { listing, .. }
        | StaffCommand::RequestInvalidation { listing, .. } => {
            lock_listing(tx, *listing).await?;
        }
        StaffCommand::ApproveInvalidation { id } => {
            let listing: Uuid = sqlx::query_scalar(
                "SELECT listing_id FROM bazaar_invalidation_requests WHERE id=$1",
            )
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(Error::NotFound)?;
            lock_listing(tx, listing).await?;
            sqlx::query("SELECT id FROM bazaar_invalidation_requests WHERE id=$1 FOR UPDATE")
                .bind(id)
                .execute(&mut **tx)
                .await?;
        }
        StaffCommand::ReviewReport { id, .. } => {
            sqlx::query("SELECT id FROM bazaar_reports WHERE id=$1 FOR UPDATE")
                .bind(id)
                .execute(&mut **tx)
                .await?;
        }
        StaffCommand::Fees { .. } => {
            sqlx::query("SELECT pg_advisory_xact_lock(64001)")
                .execute(&mut **tx)
                .await?;
        }
        StaffCommand::RestrictItem { .. } => {
            sqlx::query("SELECT pg_advisory_xact_lock(64002)")
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}

/// Execute inside a transaction that also records the authenticated staff audit.
pub async fn execute_staff(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    command: &StaffCommand,
) -> Result<(), Error> {
    valid_id(actor)?;
    valid_id(command.target())?;
    lock_staff_target(tx, command).await?;
    match command {
        StaffCommand::Suspend { listing, reason } => {
            reason_valid(reason)?;
            let (_, state) = lock_listing(tx, *listing).await?;
            if !matches!(
                state,
                State::Draft | State::Active | State::Matched | State::AwaitingDelivery
            ) {
                return Err(Error::IllegalTransition);
            }
            sqlx::query("INSERT INTO bazaar_moderation_holds(listing_id,revision,actor,reason) VALUES($1,1,$2,$3) ON CONFLICT(listing_id) DO UPDATE SET revision=bazaar_moderation_holds.revision+1,actor=$2,reason=$3,updated_at=clock_timestamp()")
                .bind(listing).bind(actor).bind(reason).execute(&mut **tx).await?;
        }
        StaffCommand::RequestInvalidation {
            id,
            listing,
            reason,
        } => {
            reason_valid(reason)?;
            valid_id(*id)?;
            cancellable(tx, *listing).await?;
            let revision: i64 = sqlx::query_scalar(
                "SELECT revision FROM bazaar_moderation_holds WHERE listing_id=$1",
            )
            .bind(listing)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(Error::Denied)?;
            sqlx::query("INSERT INTO bazaar_invalidation_requests(id,listing_id,hold_revision,requested_by,reason) VALUES($1,$2,$3,$4,$5)")
                .bind(id).bind(listing).bind(revision).bind(actor).bind(reason).execute(&mut **tx).await?;
        }
        StaffCommand::ApproveInvalidation { id } => {
            // Discover listing first; lock listing before request, as the requester does.
            let listing: Uuid = sqlx::query_scalar(
                "SELECT listing_id FROM bazaar_invalidation_requests WHERE id=$1",
            )
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(Error::NotFound)?;
            cancellable(tx, listing).await?;
            let (requester,revision,state,reviewer):(Uuid,i64,String,Option<Uuid>)=sqlx::query_as("SELECT requested_by,hold_revision,state,approved_by FROM bazaar_invalidation_requests WHERE id=$1 FOR UPDATE").bind(id).fetch_one(&mut **tx).await?;
            if requester == actor {
                return Err(Error::Denied);
            }
            let current: Option<i64> = sqlx::query_scalar(
                "SELECT revision FROM bazaar_moderation_holds WHERE listing_id=$1",
            )
            .bind(listing)
            .fetch_optional(&mut **tx)
            .await?;
            if current != Some(revision) {
                return Err(Error::Denied);
            }
            if state == "approved" {
                return if reviewer == Some(actor) {
                    Ok(())
                } else {
                    Err(Error::Denied)
                };
            }
            if state != "pending" {
                return Err(Error::Denied);
            }
            sqlx::query("UPDATE bazaar_invalidation_requests SET approved_by=$2,state='approved',updated_at=clock_timestamp() WHERE id=$1").bind(id).bind(actor).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO bazaar_moderation_outbox(id,listing_id,kind) VALUES($1,$2,'reconcile_invalidation')").bind(id).bind(listing).execute(&mut **tx).await?;
        }
        StaffCommand::ReviewReport { id, accepted } => {
            let reporter: Uuid =
                sqlx::query_scalar("SELECT reporter FROM bazaar_reports WHERE id=$1 FOR UPDATE")
                    .bind(id)
                    .fetch_optional(&mut **tx)
                    .await?
                    .ok_or(Error::NotFound)?;
            if actor == reporter {
                return Err(Error::Denied);
            }
            let result=sqlx::query("UPDATE bazaar_reports SET state=$2,reviewed_by=$3,updated_at=clock_timestamp() WHERE id=$1 AND state='pending'")
                .bind(id).bind(if *accepted {"accepted"} else {"dismissed"}).bind(actor).execute(&mut **tx).await?;
            if result.rows_affected() != 1 {
                return Err(Error::IllegalTransition);
            }
        }
        StaffCommand::Fees {
            id,
            coin_bps,
            gold_bps,
            listing_fee,
            promotion_price,
            reason,
        } => {
            reason_valid(reason)?;
            if (*coin_bps, *gold_bps, *listing_fee) != (1000, 250, 0) || *promotion_price <= 0 {
                return Err(Error::InvalidInput);
            }
            sqlx::query("SELECT pg_advisory_xact_lock(64001)")
                .execute(&mut **tx)
                .await?;
            sqlx::query("INSERT INTO bazaar_fee_revisions(id,actor,coin_bps,gold_bps,listing_fee,promotion_price,reason) VALUES($1,$2,$3,$4,$5,$6,$7)")
                .bind(id).bind(actor).bind(coin_bps).bind(gold_bps).bind(listing_fee).bind(promotion_price).bind(reason).execute(&mut **tx).await?;
        }
        StaffCommand::RestrictItem {
            id,
            world,
            family,
            production_key,
            revision_ref,
            minimum_value,
            denied,
            reason,
        } => {
            reason_valid(reason)?;
            valid_id(*world)?;
            if *minimum_value <= 0
                || ![family, production_key, revision_ref]
                    .iter()
                    .all(|s| definition_key(s))
            {
                return Err(Error::InvalidInput);
            }
            sqlx::query("SELECT pg_advisory_xact_lock(64002)")
                .execute(&mut **tx)
                .await?;
            // No admitted T58 catalog/instance binding exists yet. Fail closed by
            // suspending every active item listing before publishing a restriction.
            sqlx::query("SELECT id FROM bazaar_listings WHERE kind='item' AND state='active' ORDER BY id FOR UPDATE").execute(&mut **tx).await?;
            sqlx::query("INSERT INTO bazaar_moderation_holds(listing_id,revision,actor,reason) SELECT id,1,$1,$2 FROM bazaar_listings WHERE kind='item' AND state='active' ON CONFLICT(listing_id) DO UPDATE SET revision=bazaar_moderation_holds.revision+1,actor=$1,reason=$2,updated_at=clock_timestamp()")
                .bind(actor).bind(reason).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO bazaar_item_restrictions(id,world_id,family,production_key,revision_ref,minimum_value,denied,actor,reason) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
                .bind(id).bind(world).bind(family).bind(production_key).bind(revision_ref).bind(minimum_value).bind(denied).bind(actor).bind(reason).execute(&mut **tx).await?;
        }
    }
    Ok(())
}
fn reason_valid(reason: &str) -> Result<(), Error> {
    if text_valid(reason, 10, 500) {
        Ok(())
    } else {
        Err(Error::InvalidInput)
    }
}
fn definition_key(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes().enumerate().all(|(i, c)| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || (i > 0 && b"._:/-".contains(&c))
        })
}
async fn cancellable(tx: &mut Transaction<'_, Postgres>, listing: Uuid) -> Result<(), Error> {
    let (_, state) = lock_listing(tx, listing).await?;
    let settlement: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_settlements WHERE listing_id=$1)")
            .bind(listing)
            .fetch_one(&mut **tx)
            .await?;
    if !matches!(state, State::Draft | State::Active) || settlement {
        return Err(Error::IllegalTransition);
    }
    Ok(())
}

/// Apply only to a fresh, current-generation admitted Game catalog decision.
/// Unknown, absent and stale Game decisions must be passed as None, never true.
pub async fn item_allowed(
    db: &mut PgConnection,
    world: Uuid,
    definition: (&str, &str, &str),
    game_tradeable: Option<bool>,
    value: i64,
) -> Result<bool, Error> {
    if game_tradeable != Some(true)
        || world.is_nil()
        || value <= 0
        || ![definition.0, definition.1, definition.2]
            .iter()
            .all(|s| definition_key(s))
    {
        return Ok(false);
    }
    let restriction:Option<(bool,i64)>=sqlx::query_as("SELECT denied,minimum_value FROM bazaar_item_restrictions WHERE world_id=$1 AND family=$2 AND production_key=$3 AND revision_ref=$4 ORDER BY created_at DESC,id DESC LIMIT 1")
        .bind(world).bind(definition.0).bind(definition.1).bind(definition.2).fetch_optional(db).await?;
    Ok(restriction.is_none_or(|(denied, minimum)| !denied && value >= minimum))
}
#[derive(Debug, sqlx::FromRow)]
pub struct PendingReport {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub category: String,
    pub detail: String,
    pub created_at: OffsetDateTime,
}
pub async fn pending_reports(db: &mut PgConnection) -> Result<Vec<PendingReport>, Error> {
    Ok(sqlx::query_as("SELECT id,listing_id,category,detail,created_at FROM bazaar_reports WHERE state='pending' ORDER BY created_at,id LIMIT 100").fetch_all(db).await?)
}
