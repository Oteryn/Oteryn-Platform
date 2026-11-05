#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
struct Db {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Db {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error + Send + Sync>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED T64 PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let schema = format!("t64_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .after_connect(move |c, _| {
                let p = path.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(format!("SET search_path TO {p}")))
                        .execute(c)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        sqlx::raw_sql(include_str!(
            "../../../migrations/20261105152830_t57_bazaar_core.sql"
        ))
        .execute(&pool)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../../migrations/20261105230107_t64_moderation.sql"
        ))
        .execute(&pool)
        .await?;
        sqlx::raw_sql("CREATE TABLE test_promotion_debits(id uuid PRIMARY KEY,amount bigint)")
            .execute(&pool)
            .await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    async fn listing(&self, owner: Uuid) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at) VALUES($1,$2,'item','auction','coins',100,false,'active',clock_timestamp()+interval '2 days')").bind(id).bind(owner).execute(&self.pool).await.unwrap();
        id
    }
    fn api(&self, view: View, ledger: Ledger) -> Moderation<Allow, View, Ledger> {
        Moderation::new(
            self.pool.clone(),
            Allow,
            view,
            ledger,
            Flags {
                enabled: true,
                public_profiles: true,
                promotions: true,
            },
        )
    }
    async fn staff(&self, actor: Uuid, command: StaffCommand) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        execute_staff(&mut tx, actor, &command).await?;
        tx.commit().await?;
        Ok(())
    }
    async fn end(self) {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await
        .unwrap();
        self.admin.close().await;
    }
}
#[derive(Clone)]
struct Allow;
impl Accounts for Allow {
    async fn state(
        &self,
        _: &mut PgConnection,
        _: Uuid,
    ) -> Result<Option<crate::AccountState>, Error> {
        Ok(Some(crate::AccountState {
            active: true,
            email_verified: true,
            mfa: true,
            sudo: true,
            bazaar_blocked: false,
        }))
    }
}
#[derive(Clone, Default)]
struct View {
    listings: Vec<PublicListing>,
    hidden: bool,
}
impl PublicViews for View {
    async fn seller(&self, _: &mut PgConnection, _: Uuid) -> Result<Option<SellerProfile>, Error> {
        Ok((!self.hidden).then(|| SellerProfile {
            character_name: "Selected Hero".into(),
            joined: "2026-11-05".into(),
            completed_transactions: 3,
            listings: self.listings.clone(),
        }))
    }
    async fn listing_visible(&self, _: &mut PgConnection, _: Uuid) -> Result<bool, Error> {
        Ok(!self.hidden)
    }
}
#[derive(Clone, Default)]
struct Ledger(Arc<AtomicBool>);
impl PromotionLedger for Ledger {
    async fn debit(
        &self,
        db: &mut PgConnection,
        _: Uuid,
        amount: i64,
        key: Uuid,
    ) -> Result<(), Error> {
        sqlx::query("INSERT INTO test_promotion_debits VALUES($1,$2)")
            .bind(key)
            .bind(amount)
            .execute(db)
            .await?;
        if self.0.load(Ordering::SeqCst) {
            Err(Error::InsufficientFunds)
        } else {
            Ok(())
        }
    }
}
fn report(listing: Uuid) -> ReportInput {
    ReportInput {
        id: Uuid::now_v7(),
        listing,
        category: "fraud".into(),
        detail: "Evidence for this report".into(),
    }
}
fn fees() -> StaffCommand {
    StaffCommand::Fees {
        id: Uuid::now_v7(),
        coin_bps: 1000,
        gold_bps: 250,
        listing_fee: 0,
        promotion_price: 25,
        reason: "Reviewed price configuration".into(),
    }
}
fn suspend(listing: Uuid) -> StaffCommand {
    StaffCommand::Suspend {
        listing,
        reason: "Review requires suspension".into(),
    }
}
#[test]
fn bad_inputs_and_default_flags() {
    assert!(
        !Flags::default().enabled
            && !Flags::default().public_profiles
            && !Flags::default().promotions
    );
    assert!(report(Uuid::nil()).validate().is_err());
    let mut input = report(Uuid::now_v7());
    input.detail = "\0invalid input text".into();
    assert!(input.validate().is_err());
    input.detail = "Valid report detail".into();
    input.category = "unknown".into();
    assert!(input.validate().is_err());
    assert!(!definition_key("A"));
    assert!(!definition_key("../asset"));
    assert!(definition_key("item:rare/one"));
    for lang in ["pl", "en"] {
        for key in ["report", "staff", "promotion-help", "invalidation-help"] {
            assert!(!web::Locale::parse(lang).unwrap().text(key).is_empty());
        }
    }
}
#[tokio::test]
async fn postgres_reports_success_replay_refusal_and_bad_input() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let owner = Uuid::now_v7();
    let actor = Uuid::now_v7();
    let listing = db.listing(owner).await;
    let api = db.api(View::default(), Ledger::default());
    let input = report(listing);
    assert_eq!(api.report(actor, input.clone()).await?, input.id);
    assert_eq!(api.report(actor, input.clone()).await?, input.id);
    let mut changed = input.clone();
    changed.detail = "Different report evidence".into();
    assert!(matches!(
        api.report(actor, changed).await,
        Err(Error::IdempotencyConflict)
    ));
    assert!(matches!(
        api.report(owner, report(listing)).await,
        Err(Error::Denied)
    ));
    let mut invalid = report(listing);
    invalid.detail = "short".into();
    assert!(matches!(
        api.report(actor, invalid).await,
        Err(Error::InvalidInput)
    ));
    assert!(matches!(
        api.report(actor, report(listing)).await,
        Err(Error::Denied)
    ));
    db.end().await;
    Ok(())
}
#[tokio::test]
async fn postgres_hold_fences_every_trading_write() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let owner = Uuid::now_v7();
    let listing = db.listing(owner).await;
    db.staff(Uuid::now_v7(), suspend(listing)).await?;
    assert!(
        sqlx::query("UPDATE bazaar_listings SET state='cancelled' WHERE id=$1")
            .bind(listing)
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert!(sqlx::query("INSERT INTO bazaar_offers(id,listing_id,account,amount,idempotency_key) VALUES($1,$2,$3,100,'test')").bind(Uuid::now_v7()).bind(listing).bind(Uuid::now_v7()).execute(&db.pool).await.is_err());
    assert!(sqlx::query("INSERT INTO bazaar_bids(id,listing_id,account,maximum,idempotency_key,leader,price,ends_at) VALUES($1,$2,$3,100,'test',$3,100,clock_timestamp())").bind(Uuid::now_v7()).bind(listing).bind(Uuid::now_v7()).execute(&db.pool).await.is_err());
    let audit: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM bazaar_audit WHERE relation_name='bazaar_moderation_holds'",
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(audit, 1);
    db.end().await;
    Ok(())
}
#[tokio::test]
async fn postgres_four_eyes_queues_once_and_retains_value() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let first = Uuid::now_v7();
    let second = Uuid::now_v7();
    let listing = db.listing(Uuid::now_v7()).await;
    let id = Uuid::now_v7();
    db.staff(first, suspend(listing)).await?;
    db.staff(
        first,
        StaffCommand::RequestInvalidation {
            id,
            listing,
            reason: "Confirmed policy violation".into(),
        },
    )
    .await?;
    assert!(matches!(
        db.staff(first, StaffCommand::ApproveInvalidation { id })
            .await,
        Err(Error::Denied)
    ));
    db.staff(second, StaffCommand::ApproveInvalidation { id })
        .await?;
    db.staff(second, StaffCommand::ApproveInvalidation { id })
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_moderation_outbox")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    let state: State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1")
        .bind(listing)
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(state, State::Active);
    let mut tx = db.pool.begin().await?;
    assert!(held(&mut tx, listing).await?);
    tx.rollback().await?;
    db.end().await;
    Ok(())
}
#[tokio::test]
async fn postgres_changed_hold_and_started_transfer_refuse_invalidation() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let first = Uuid::now_v7();
    let listing = db.listing(Uuid::now_v7()).await;
    let id = Uuid::now_v7();
    db.staff(first, suspend(listing)).await?;
    db.staff(
        first,
        StaffCommand::RequestInvalidation {
            id,
            listing,
            reason: "Confirmed policy violation".into(),
        },
    )
    .await?;
    db.staff(first, suspend(listing)).await?;
    assert!(matches!(
        db.staff(Uuid::now_v7(), StaffCommand::ApproveInvalidation { id })
            .await,
        Err(Error::Denied)
    ));
    let late = db.listing(Uuid::now_v7()).await;
    sqlx::query("UPDATE bazaar_listings SET state='awaiting_delivery' WHERE id=$1")
        .bind(late)
        .execute(&db.pool)
        .await?;
    db.staff(first, suspend(late)).await?;
    assert!(matches!(
        db.staff(
            first,
            StaffCommand::RequestInvalidation {
                id: Uuid::now_v7(),
                listing: late,
                reason: "Must not reverse delivery".into()
            }
        )
        .await,
        Err(Error::IllegalTransition)
    ));
    db.end().await;
    Ok(())
}
#[tokio::test]
async fn postgres_promotion_debit_is_atomic_and_idempotent() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let owner = Uuid::now_v7();
    let listing = db.listing(owner).await;
    db.staff(Uuid::now_v7(), fees()).await?;
    let ledger = Ledger::default();
    let api = db.api(View::default(), ledger.clone());
    let id = Uuid::now_v7();
    let quote = api.promotion_quote().await?;
    ledger.0.store(true, Ordering::SeqCst);
    assert!(matches!(
        api.promote(owner, listing, id, quote).await,
        Err(Error::InsufficientFunds)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_promotions")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    let wrong_quote = PromotionQuote {
        amount: quote.amount + 1,
        ..quote
    };
    assert!(matches!(
        api.promote(owner, listing, id, wrong_quote).await,
        Err(Error::IdempotencyConflict)
    ));
    ledger.0.store(false, Ordering::SeqCst);
    let (a, b) = tokio::join!(
        api.promote(owner, listing, id, quote),
        api.promote(owner, listing, id, quote)
    );
    a?;
    b?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM test_promotion_debits")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    assert!(matches!(
        api.promote(Uuid::now_v7(), listing, id, quote).await,
        Err(Error::Denied)
    ));
    assert!(
        sqlx::query("DELETE FROM bazaar_promotions")
            .execute(&db.pool)
            .await
            .is_err()
    );
    db.end().await;
    Ok(())
}
#[tokio::test]
async fn postgres_fees_cannot_override_owner_economics() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let mut command = fees();
    if let StaffCommand::Fees { gold_bps, .. } = &mut command {
        *gold_bps = 1000;
    }
    assert!(matches!(
        db.staff(Uuid::now_v7(), command).await,
        Err(Error::InvalidInput)
    ));
    db.staff(Uuid::now_v7(), fees()).await?;
    assert!(
        sqlx::query("UPDATE bazaar_fee_revisions SET coin_bps=1000")
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("TRUNCATE bazaar_item_restrictions")
            .execute(&db.pool)
            .await
            .is_err()
    );
    db.end().await;
    Ok(())
}
#[tokio::test]
async fn postgres_item_policy_only_narrows_and_fences_unknown_bindings() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let listing = db.listing(Uuid::now_v7()).await;
    let world = Uuid::now_v7();
    db.staff(
        Uuid::now_v7(),
        StaffCommand::RestrictItem {
            id: Uuid::now_v7(),
            world,
            family: "item".into(),
            production_key: "rare".into(),
            revision_ref: "v1".into(),
            minimum_value: 100,
            denied: false,
            reason: "Restrict low value trading".into(),
        },
    )
    .await?;
    let mut c = db.pool.acquire().await?;
    assert!(!item_allowed(&mut c, world, ("item", "rare", "v1"), None, 1000).await?);
    assert!(!item_allowed(&mut c, world, ("item", "rare", "v1"), Some(false), 1000).await?);
    assert!(!item_allowed(&mut c, world, ("item", "rare", "v1"), Some(true), 99).await?);
    assert!(item_allowed(&mut c, world, ("item", "rare", "v1"), Some(true), 100).await?);
    drop(c);
    assert!(
        sqlx::query("UPDATE bazaar_listings SET price=101 WHERE id=$1")
            .bind(listing)
            .execute(&db.pool)
            .await
            .is_err()
    );
    let future = Uuid::now_v7();
    sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,'item','auction','coins',100,false,clock_timestamp()+interval '2 days')").bind(future).bind(Uuid::now_v7()).execute(&db.pool).await?;
    assert!(
        sqlx::query("UPDATE bazaar_listings SET state='active' WHERE id=$1")
            .bind(future)
            .execute(&db.pool)
            .await
            .is_err()
    );
    db.end().await;
    Ok(())
}
#[tokio::test]
async fn postgres_seller_profile_filters_holds_foreign_and_hidden_listings() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let owner = Uuid::now_v7();
    let good = db.listing(owner).await;
    let held_listing = db.listing(owner).await;
    let foreign = db.listing(Uuid::now_v7()).await;
    db.staff(Uuid::now_v7(), suspend(held_listing)).await?;
    let view = View {
        hidden: false,
        listings: [good, held_listing, foreign]
            .into_iter()
            .map(|id| PublicListing {
                id,
                title: "Public listing".into(),
                featured: false,
            })
            .collect(),
    };
    let api = db.api(view, Ledger::default());
    let profile = api.seller(owner).await?;
    assert_eq!(profile.listings.len(), 1);
    assert_eq!(profile.listings[0].id, good);
    let api = db.api(
        View {
            hidden: true,
            ..View::default()
        },
        Ledger::default(),
    );
    assert!(matches!(api.seller(owner).await, Err(Error::NotFound)));
    let api = Moderation::new(
        db.pool.clone(),
        PendingIntegration,
        PendingIntegration,
        PendingIntegration,
        Flags {
            enabled: true,
            public_profiles: true,
            promotions: true,
        },
    );
    assert!(matches!(api.seller(owner).await, Err(Error::NotFound)));
    assert!(matches!(
        api.report(owner, report(good)).await,
        Err(Error::Denied)
    ));
    db.end().await;
    Ok(())
}
#[tokio::test]
async fn postgres_report_limit_serializes_concurrent_submissions() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let actor = Uuid::now_v7();
    let listing = db.listing(Uuid::now_v7()).await;
    let api = db.api(View::default(), Ledger::default());
    let (a, b) = tokio::join!(
        api.report(actor, report(listing)),
        api.report(actor, report(listing))
    );
    assert!(a.is_ok() ^ b.is_ok());
    for _ in 0..4 {
        let l = db.listing(Uuid::now_v7()).await;
        api.report(actor, report(l)).await?;
    }
    let l = db.listing(Uuid::now_v7()).await;
    assert!(matches!(
        api.report(actor, report(l)).await,
        Err(Error::Denied)
    ));
    db.end().await;
    Ok(())
}

#[tokio::test]
async fn postgres_report_review_requires_independent_staff_and_pending_state() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let actor = Uuid::now_v7();
    let listing = db.listing(Uuid::now_v7()).await;
    let input = report(listing);
    let id = input.id;
    db.api(View::default(), Ledger::default())
        .report(actor, input)
        .await?;
    assert!(matches!(
        db.staff(actor, StaffCommand::ReviewReport { id, accepted: true })
            .await,
        Err(Error::Denied)
    ));
    db.staff(
        Uuid::now_v7(),
        StaffCommand::ReviewReport { id, accepted: true },
    )
    .await?;
    assert!(matches!(
        db.staff(
            Uuid::now_v7(),
            StaffCommand::ReviewReport {
                id,
                accepted: false
            }
        )
        .await,
        Err(Error::IllegalTransition)
    ));
    db.end().await;
    Ok(())
}
