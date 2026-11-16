#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::rbac::{AccountState, AccountStateUnavailable};
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use tower::ServiceExt;
#[derive(Debug, Clone)]
struct Identity(Uuid);
impl AccountStateReader for Identity {
    // Synthetic session authority for this fixture; production denies missing proofs.
    async fn authenticated_in(
        &self,
        connection: &mut sqlx::PgConnection,
        context: oteryn_admin::rbac::CheckContext,
    ) -> std::result::Result<bool, oteryn_admin::rbac::AccountStateUnavailable> {
        if let Some(proof) = context.session_proof {
            oteryn_admin::rbac::durable_session_in(connection, context.account, proof).await
        } else {
            Ok(!context.require_staff_mfa || context.mfa_verified)
        }
    }

    async fn state(&self, _: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        Ok(AccountState::Active)
    }
}
impl StaffSession for Identity {
    async fn resolve(&self, _: &oteryn_web::session::BrowserSession) -> Option<web::StaffIdentity> {
        Some(web::StaffIdentity {
            account: self.0,
            mfa_satisfied: true,
        })
    }
}
struct Db {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    actor: Uuid,
}
impl Db {
    async fn new() -> Option<Self> {
        Self::with_connections(8).await
    }
    async fn with_connections(maximum: u32) -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED T64 staff PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("t64_staff_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(maximum)
            .after_connect(move |c, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(format!("SET search_path TO {path}")))
                        .execute(c)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        let actor = Uuid::now_v7();
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'unused','active','en',now())").bind(actor).bind(format!("{actor}@example.invalid")).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).await.unwrap();
        Some(Self {
            pool,
            admin,
            schema,
            actor,
        })
    }
    fn service(&self) -> StaffBazaar<Identity> {
        StaffBazaar::new(self.pool.clone(), Identity(self.actor), true)
    }
    fn context(&self) -> CheckContext {
        CheckContext {
            account: self.actor,
            require_staff_mfa: true,
            mfa_verified: true,
            session_proof: None,
        }
    }
    async fn listing(&self) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at) VALUES($1,$2,'item','auction','coins',100,false,'active',clock_timestamp()+interval '2 days')").bind(id).bind(Uuid::now_v7()).execute(&self.pool).await.unwrap();
        id
    }
    fn app(&self) -> Router {
        router(
            self.pool.clone(),
            PanelConfig {
                admin_panel: true,
                host: "staff.test".into(),
            },
            Identity(self.actor),
            Identity(self.actor),
            true,
        )
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
fn suspend(listing: Uuid) -> StaffCommand {
    StaffCommand::Suspend {
        listing,
        reason: "Verified moderation reason".into(),
    }
}
#[test]
fn unknown_action_and_bad_numbers_deny() {
    assert!(
        Input {
            action: "unknown".into(),
            ..Input::default()
        }
        .command()
        .is_err()
    );
    assert!(
        Input {
            action: "fees".into(),
            id: Uuid::now_v7().to_string(),
            coin_bps: "no".into(),
            ..Input::default()
        }
        .command()
        .is_err()
    );
}
#[tokio::test]
async fn postgres_staff_authorization_mfa_and_audit() {
    let Some(db) = Db::new().await else { return };
    let listing = db.listing().await;
    let api = db.service();
    let mut context = db.context();
    context.mfa_verified = false;
    assert!(matches!(
        api.execute(context, suspend(listing)).await,
        Err(Error::Authorization(_))
    ));
    context = db.context();
    context.account = Uuid::now_v7();
    assert!(matches!(
        api.execute(context, suspend(listing)).await,
        Err(Error::Authorization(_))
    ));
    api.execute(db.context(), suspend(listing)).await.unwrap();
    let (actor, before, after): (Uuid, Value, Value) =
        sqlx::query_as("SELECT actor,before,after FROM audit_log WHERE action='bazaar.suspend'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(actor, db.actor);
    assert!(before["hold"].is_null());
    assert_eq!(after["hold"]["revision"], 1);
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(db.actor)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(api.execute(db.context(), suspend(listing)).await.is_err());
    db.end().await;
}
#[tokio::test]
async fn postgres_audit_failure_rolls_back_hold() {
    let Some(db) = Db::new().await else { return };
    let listing = db.listing().await;
    sqlx::raw_sql("CREATE FUNCTION reject_fixture_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture audit unavailable'; END; $$; CREATE TRIGGER fixture_audit_fail BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION reject_fixture_audit();").execute(&db.pool).await.unwrap();
    assert!(
        db.service()
            .execute(db.context(), suspend(listing))
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_moderation_holds")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    db.end().await;
}
#[tokio::test]
async fn postgres_staff_http_actor_to_result_csrf_and_localization() {
    let Some(db) = Db::new().await else { return };
    let listing = db.listing().await;
    let app = db.app();
    for lang in ["pl", "en"] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/admin/{lang}/bazaar"))
                    .header("host", "staff.test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response
            .headers()
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let html = String::from_utf8(
            to_bytes(response.into_body(), 100000)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(html.contains("width=device-width"));
        assert!(html.contains("for=\"suspend-listing\""));
        assert!(html.contains(&Locale::parse(lang).unwrap().text("staff")));
        std::fs::write(format!("/tmp/t64-staff-{lang}.html"), &html).unwrap();
        let token = html
            .split("name=\"_csrf\" value=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        let body = format!(
            "action=suspend&listing={listing}&reason=Verified+moderation+reason&_csrf={token}"
        );
        let bad = app
            .clone()
            .oneshot(
                Request::post(format!("/admin/{lang}/bazaar"))
                    .header("host", "staff.test")
                    .header("cookie", &cookie)
                    .header("origin", "https://staff.test")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!("action=suspend&listing={listing}")))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad.status(), StatusCode::FORBIDDEN);
        let result = app
            .clone()
            .oneshot(
                Request::post(format!("/admin/{lang}/bazaar"))
                    .header("host", "staff.test")
                    .header("cookie", &cookie)
                    .header("origin", "https://staff.test")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::SEE_OTHER);
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='bazaar.suspend'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(count, 2);
    let response = app
        .oneshot(
            Request::get("/admin/en/bazaar")
                .header("host", "wrong.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    db.end().await;
}
#[tokio::test]
async fn postgres_staff_four_eyes_requires_second_live_permission() {
    let Some(db) = Db::new().await else { return };
    let listing = db.listing().await;
    let api = db.service();
    api.execute(db.context(), suspend(listing)).await.unwrap();
    let id = Uuid::now_v7();
    api.execute(
        db.context(),
        StaffCommand::RequestInvalidation {
            id,
            listing,
            reason: "Confirmed invalid listing".into(),
        },
    )
    .await
    .unwrap();
    assert!(
        api.execute(db.context(), StaffCommand::ApproveInvalidation { id })
            .await
            .is_err()
    );
    let second = Uuid::now_v7();
    let context = CheckContext {
        account: second,
        ..db.context()
    };
    assert!(
        api.execute(context, StaffCommand::ApproveInvalidation { id })
            .await
            .is_err()
    );
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(second).execute(&db.pool).await.unwrap();
    api.execute(context, StaffCommand::ApproveInvalidation { id })
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_moderation_outbox")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    db.end().await;
}

#[tokio::test]
async fn a2_bazaar_single_connection_and_sixteen_mutations() {
    let started = std::time::Instant::now();
    for maximum in [1, 16] {
        let db = Db::with_connections(maximum)
            .await
            .expect("PostgreSQL required");
        let listing = db.listing().await;
        let api = Arc::new(db.service());
        let mut tasks = tokio::task::JoinSet::new();
        let barrier = Arc::new(tokio::sync::Barrier::new(16));
        for _ in 0..16 {
            let api = api.clone();
            let barrier = barrier.clone();
            let context = db.context();
            tasks.spawn(async move {
                barrier.wait().await;
                api.execute(context, suspend(listing)).await
            });
        }
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while let Some(result) = tasks.join_next().await {
                result.unwrap().unwrap();
            }
        })
        .await
        .unwrap();
        drop(api);
        db.end().await;
    }
    eprintln!(
        "a2_bazaar_single_connection_and_sixteen_mutations duration: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_bazaar_single_connection_mutation_and_audit() {
    let started = std::time::Instant::now();
    let Some(db) = Db::new().await else {
        panic!("PostgreSQL required")
    };
    let path = db.schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .after_connect(move |c, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&std::env::var("PORTAL_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let listing = db.listing().await;
    let api = StaffBazaar::new(pool.clone(), Identity(db.actor), true);
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        api.execute(db.context(), suspend(listing)),
    )
    .await
    .unwrap()
    .unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='bazaar.suspend'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
    pool.close().await;
    db.end().await;
    eprintln!(
        "postgres_bazaar_single_connection_mutation_and_audit: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_a3_bazaar_audit_excludes_lifetime_invalidation_history() {
    let started = std::time::Instant::now();
    let db = Db::new().await.expect("PostgreSQL required");
    let listing = db.listing().await;
    db.service()
        .execute(db.context(), suspend(listing))
        .await
        .unwrap();
    sqlx::query("INSERT INTO bazaar_invalidation_requests(id,listing_id,hold_revision,requested_by,reason) SELECT uuidv7(),$1,n,$2,repeat('historical reason ',25) FROM generate_series(2,1001) n")
        .bind(listing).bind(db.actor).execute(&db.pool).await.unwrap();
    db.service()
        .execute(db.context(), suspend(listing))
        .await
        .unwrap();
    let (before,after): (Value,Value) = sqlx::query_as("SELECT before,after FROM audit_log WHERE action='bazaar.suspend' AND after->'hold'->>'revision'='2'")
        .fetch_one(&db.pool).await.unwrap();
    assert_eq!(before["invalidation"].as_array().unwrap().len(), 0);
    assert_eq!(after["invalidation"].as_array().unwrap().len(), 1);
    assert_eq!(after["invalidation"][0]["hold_revision"], 2);
    assert!(before.to_string().len() + after.to_string().len() < 5000);
    db.end().await;
    eprintln!(
        "postgres_a3_bazaar_audit_excludes_lifetime_invalidation_history duration: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_a3_bazaar_target_and_gate_waits_share_deadline_and_release_pool() {
    let started = std::time::Instant::now();
    let db = Db::with_connections(1).await.expect("PostgreSQL required");
    let listing = db.listing().await;
    let report = Uuid::now_v7();
    sqlx::query("INSERT INTO bazaar_reports(id,listing_id,reporter,category,detail) VALUES($1,$2,$3,'other','Review this listing')").bind(report).bind(listing).bind(db.actor).execute(&db.pool).await.unwrap();
    let commands = [
        suspend(listing),
        StaffCommand::ReviewReport {
            id: report,
            accepted: false,
        },
        StaffCommand::Fees {
            id: Uuid::now_v7(),
            coin_bps: 1000,
            gold_bps: 250,
            listing_fee: 0,
            promotion_price: 100,
            reason: "Reviewed fee schedule".into(),
        },
        StaffCommand::RestrictItem {
            id: Uuid::now_v7(),
            world: Uuid::now_v7(),
            family: "item".into(),
            production_key: "rare".into(),
            revision_ref: "v1".into(),
            minimum_value: 100,
            denied: true,
            reason: "Reviewed item restriction".into(),
        },
    ];
    for (index, command) in commands.into_iter().enumerate() {
        let mut blocker = db.admin.begin().await.unwrap();
        sqlx::query("SELECT set_config('search_path',$1,true)")
            .bind(&db.schema)
            .execute(&mut *blocker)
            .await
            .unwrap();
        match index {
            0 => {
                sqlx::query("SELECT id FROM bazaar_listings WHERE id=$1 FOR UPDATE")
                    .bind(listing)
                    .execute(&mut *blocker)
                    .await
                    .unwrap();
            }
            1 => {
                sqlx::query("SELECT id FROM bazaar_reports WHERE id=$1 FOR UPDATE")
                    .bind(report)
                    .execute(&mut *blocker)
                    .await
                    .unwrap();
            }
            2 => {
                sqlx::query("SELECT pg_advisory_xact_lock(64001)")
                    .execute(&mut *blocker)
                    .await
                    .unwrap();
            }
            _ => {
                sqlx::query("SELECT pg_advisory_xact_lock(64002)")
                    .execute(&mut *blocker)
                    .await
                    .unwrap();
            }
        }
        let wait = std::time::Instant::now();
        let api = db.service();
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_secs(4),
                api.execute(db.context(), command)
            )
            .await
            .unwrap()
            .is_err()
        );
        assert!(wait.elapsed() >= std::time::Duration::from_millis(2800));
        tokio::time::timeout(
            std::time::Duration::from_millis(500),
            sqlx::query("SELECT 1").execute(&db.pool),
        )
        .await
        .unwrap()
        .unwrap();
        eprintln!(
            "postgres_a3_bazaar_target_and_gate_waits_share_deadline_and_release_pool case={index} duration: {:?}",
            wait.elapsed()
        );
        blocker.rollback().await.unwrap();
    }
    db.end().await;
    eprintln!(
        "postgres_a3_bazaar_target_and_gate_waits_share_deadline_and_release_pool duration: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_a3_bazaar_orders_actor_before_role_fence() {
    let started = std::time::Instant::now();
    let db = Db::new().await.expect("PostgreSQL required");
    let listing = db.listing().await;
    let api = StaffBazaar::new(
        db.pool.clone(),
        web::PortalIdentity::configured(db.pool.clone()),
        true,
    );
    let mut context = db.context();
    context.session_proof = Some(crate::test_staff::proof(&db.pool, db.actor).await.unwrap());
    crate::test_staff::race_account_before_fence(&db.pool, db.actor, async move {
        api.execute(context, suspend(listing)).await
    })
    .await
    .unwrap()
    .unwrap();
    db.end().await;
    eprintln!(
        "postgres_a3_bazaar_orders_actor_before_role_fence duration: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn delta_bazaar_listing_before_staff_account_avoids_deadlock() {
    let started = std::time::Instant::now();
    let db = Db::new().await.unwrap();
    let listing = db.listing().await;
    sqlx::query("UPDATE bazaar_listings SET owner=$2 WHERE id=$1")
        .bind(listing)
        .bind(db.actor)
        .execute(&db.pool)
        .await
        .unwrap();
    let mut bid = db.pool.begin().await.unwrap();
    // Model the real bid entry protocol before deliberately retaining its row.
    sqlx::query("SELECT pg_advisory_xact_lock_shared(64002)")
        .execute(&mut *bid)
        .await
        .unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *bid)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM bazaar_listings WHERE id=$1 FOR UPDATE")
        .bind(listing)
        .execute(&mut *bid)
        .await
        .unwrap();
    let service = db.service();
    let context = db.context();
    let staff = tokio::spawn(async move { service.execute(context, suspend(listing)).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(pid)
            .fetch_one(&db.admin)
            .await
            .unwrap();
            if waiting {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let authority = oteryn_bazaar::game::Authority {
        config: oteryn_bazaar::game::AuthorityConfig::default(),
        fence: oteryn_bazaar::game::RecoveryFence {
            projection_epoch: 1,
            recovery_generation: 1,
        },
    };
    let bazaar = oteryn_bazaar::Bazaar::new(
        db.pool.clone(),
        oteryn_bazaar::game::PlatformAccounts {
            enabled: true,
            session_hash: None,
        },
        oteryn_bazaar::game::CoinReservations {
            authority: authority.clone(),
        },
        authority,
        oteryn_bazaar::Flags {
            enabled: true,
            gold_enabled: false,
        },
    );
    // Self-bidding is denied, but only after the real bid path reads its owner.
    let owner_read = bazaar
        .bid_in(&mut bid, listing, db.actor, 200, "delta-bid")
        .await;
    bid.rollback().await.unwrap();
    let moderation = staff.await.unwrap();
    db.end().await;
    eprintln!(
        "delta_bazaar_listing_before_staff_account_avoids_deadlock duration={:?}",
        started.elapsed()
    );
    assert!(
        matches!(owner_read, Err(oteryn_bazaar::Error::IllegalTransition)),
        "bid failed before self-bid refusal: {owner_read:?}"
    );
    assert!(
        moderation.is_ok(),
        "staff moderation failed: {moderation:?}"
    );
}
