#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request as HttpRequest, StatusCode},
};
use oteryn_store::premium;
use oteryn_wallet as wallet;
use sqlx::{AssertSqlSafe, PgConnection, postgres::PgPoolOptions};
use std::sync::Arc;
use tower::ServiceExt;

#[derive(Debug, Clone)]
struct Authority(PgPool);
impl rbac::AccountStateReader for Authority {
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

    async fn state_in(
        &self,
        connection: &mut PgConnection,
        id: Uuid,
    ) -> Result<rbac::AccountState, rbac::AccountStateUnavailable> {
        rbac::AccountStateReader::state_in(
            &crate::web::PortalIdentity::configured(self.0.clone()),
            connection,
            id,
        )
        .await
    }
    async fn state(&self, id: Uuid) -> Result<rbac::AccountState, rbac::AccountStateUnavailable> {
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1 AND status='active')",
        )
        .bind(id)
        .fetch_one(&self.0)
        .await
        .map_err(|_| rbac::AccountStateUnavailable)?;
        Ok(if active {
            rbac::AccountState::Active
        } else {
            rbac::AccountState::Unknown
        })
    }
}
impl wallet::AccountStateReader for Authority {
    async fn state(&self, conn: &mut PgConnection, id: Uuid) -> wallet::AccountState {
        match sqlx::query_scalar::<_, String>("SELECT status FROM accounts WHERE id=$1 FOR SHARE")
            .bind(id)
            .fetch_optional(conn)
            .await
        {
            Ok(Some(s)) if s == "active" => wallet::AccountState::Active,
            _ => wallet::AccountState::Unknown,
        }
    }
}
impl wallet::TransferBlockCheck for Authority {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> wallet::TransferPermission {
        wallet::TransferPermission::Unknown
    }
}
impl wallet::AuditHook for Authority {
    async fn adjustment(&self, conn: &mut PgConnection, event: &wallet::AdjustmentAudit) -> bool {
        // Commerce owns the outer atomic audit; this hook independently checks the
        // exact actor permission in the caller transaction.
        sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM account_roles ar JOIN role_permissions rp ON ar.role_id=rp.role_id WHERE ar.account_id=$1 AND rp.permission='marketplace.manage')")
            .bind(event.actor_id).fetch_one(conn).await.unwrap_or(false)
    }
}
impl premium::Accounts for Authority {
    fn state<'a>(
        &'a self,
        id: Uuid,
        conn: &'a mut PgConnection,
    ) -> std::pin::Pin<
        Box<dyn Future<Output = Result<premium::AccountState, premium::Error>> + Send + 'a>,
    > {
        Box::pin(async move {
            Ok(
                match wallet::AccountStateReader::state(self, conn, id).await {
                    wallet::AccountState::Active => premium::AccountState::Known,
                    _ => premium::AccountState::Unknown,
                },
            )
        })
    }
}
#[derive(Clone)]
struct Session(Uuid, PgPool);
impl crate::web::StaffSession for Session {
    async fn resolve(
        &self,
        session: &oteryn_web::session::BrowserSession,
    ) -> Option<crate::web::StaffIdentity> {
        crate::test_staff_authenticate(&self.1, session, self.0)
            .await
            .ok()?;
        Some(crate::web::StaffIdentity {
            account: self.0,
            mfa_satisfied: true,
        })
    }
}
type RealEffects = LedgerEffects<Authority, Authority, Authority, TestPaymentAdapter>;
type Service = Commerce<Authority, RealEffects>;
struct Harness {
    root: PgPool,
    pool: PgPool,
    schema: String,
    actor: Uuid,
    second: Uuid,
    target: Uuid,
}
fn flags() -> Flags {
    Flags {
        enabled: true,
        premium: true,
        wallet: true,
        payments: true,
        refunds: true,
    }
}
async fn context(pool: &PgPool, account: Uuid) -> rbac::CheckContext {
    rbac::CheckContext {
        account,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: crate::test_staff::proof(pool, account).await.ok(),
    }
}
fn request(target: Uuid, command: Command) -> Request {
    Request {
        request_id: Uuid::now_v7(),
        target,
        reason: "Reviewed support request".into(),
        command,
    }
}
impl Harness {
    async fn new() -> Option<Self> {
        Self::with_connections(8).await
    }
    async fn with_connections(maximum: u32) -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping PostgreSQL commerce test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let root = PgPool::connect(&url).await.unwrap();
        let schema = format!("t50_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await
            .unwrap();
        let path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(maximum)
            .after_connect(move |c, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(path)).execute(c).await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        let actor = Uuid::now_v7();
        let second = Uuid::now_v7();
        let target = Uuid::now_v7();
        for (id, name) in [(actor, "staff1"), (second, "staff2"), (target, "player")] {
            sqlx::query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale) VALUES ($1,$2,'test-only','active',now(),'en')").bind(id).bind(format!("{name}@example.test")).execute(&pool).await.unwrap();
        }
        for id in [actor, second] {
            sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(id).execute(&pool).await.unwrap();
        }
        Some(Self {
            root,
            pool,
            schema,
            actor,
            second,
            target,
        })
    }
    fn effects(&self) -> RealEffects {
        let authority = Authority(self.pool.clone());
        LedgerEffects {
            wallet: wallet::Ledger::new(
                self.pool.clone(),
                wallet::WalletFlags { enabled: true },
                authority.clone(),
                authority.clone(),
                authority.clone(),
            ),
            premium: premium::repository::Ledger::new(self.pool.clone(), Arc::new(authority), true),
            payments: TestPaymentAdapter::new(
                self.pool.clone(),
                oteryn_payments::TestProvider::new(&[7; 32]).unwrap(),
                oteryn_payments::PaymentFlags { enabled: true },
            ),
        }
    }
    fn service(&self) -> Service {
        Commerce::new(
            self.pool.clone(),
            Authority(self.pool.clone()),
            self.effects(),
            flags(),
        )
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await
        .unwrap();
        self.root.close().await;
    }
    async fn count(&self, table: &str) -> i64 {
        sqlx::query_scalar(AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}
#[tokio::test]
async fn postgres_premium_success_replay_refusal_and_bad_input() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let service = h.service();
    let ctx = context(&h.pool, h.actor).await;
    let no_mfa = rbac::CheckContext {
        mfa_verified: false,
        session_proof: None,
        require_staff_mfa: false,
        ..ctx
    };
    assert!(matches!(
        service
            .submit(no_mfa, request(h.target, Command::Grant { days: 1 }))
            .await,
        Err(Error::Denied)
    ));
    let req = request(h.target, Command::Grant { days: 30 });
    // Four eyes: the grant waits for a second staff member before any ledger row.
    assert!(service.submit(ctx, req.clone()).await.unwrap().pending);
    let replay = service.submit(ctx, req.clone()).await.unwrap();
    assert!(replay.replayed && replay.pending);
    assert_eq!(h.count("premium_ledger").await, 0);
    assert!(
        !service
            .approve(context(&h.pool, h.second).await, req.request_id)
            .await
            .unwrap()
            .pending
    );
    assert!(service.submit(ctx, req.clone()).await.unwrap().replayed);
    assert_eq!(h.count("premium_ledger").await, 1);
    let mut conflict = req.clone();
    conflict.reason = "Different support request".into();
    assert!(matches!(
        service.submit(ctx, conflict).await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        service
            .submit(ctx, request(h.target, Command::Grant { days: 0 }))
            .await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        service
            .submit(
                context(&h.pool, h.target).await,
                request(h.target, Command::Revoke)
            )
            .await,
        Err(Error::Denied)
    ));
    service
        .submit(ctx, request(h.target, Command::Revoke))
        .await
        .unwrap();
    assert!(matches!(
        service
            .submit(ctx, request(h.target, Command::Revoke))
            .await,
        Err(Error::Conflict)
    ));
    let mut short = request(h.target, Command::Grant { days: 1 });
    short.reason = "short".into();
    assert!(matches!(
        service.submit(ctx, short).await,
        Err(Error::Invalid)
    ));
    let unknown = request(Uuid::now_v7(), Command::Grant { days: 1 });
    assert!(matches!(
        service.submit(ctx, unknown).await,
        Err(Error::Denied)
    ));
    assert_eq!(h.count("premium_ledger").await, 2);
    let plain:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM audit_log WHERE after::text LIKE '%Reviewed support request%')").fetch_one(&h.pool).await.unwrap();
    assert!(!plain);
    h.close().await;
}
#[tokio::test]
async fn postgres_wallet_four_eyes_atomicity_concurrency_and_revocation() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let s = h.service();
    let ctx = context(&h.pool, h.actor).await;
    let approver = context(&h.pool, h.second).await;
    assert!(matches!(
        s.submit(ctx, request(Uuid::now_v7(), Command::Adjust { amount: 1 }))
            .await,
        Err(Error::Denied)
    ));
    let req = request(h.target, Command::Adjust { amount: 50 });
    assert!(s.submit(ctx, req.clone()).await.unwrap().pending);
    assert_eq!(h.count("wallet_entries").await, 0);
    assert!(matches!(
        s.approve(ctx, req.request_id).await,
        Err(Error::Denied)
    ));
    let (a, b) = tokio::join!(
        s.approve(approver, req.request_id),
        s.approve(approver, req.request_id)
    );
    assert!(!a.unwrap().pending);
    assert!(!b.unwrap().pending);
    assert_eq!(h.count("wallet_entries").await, 1);
    let debit = request(h.target, Command::Adjust { amount: -51 });
    assert!(matches!(
        s.submit(ctx, debit.clone()).await,
        Err(Error::Conflict)
    ));
    // A request accepted before a competing debit must still recheck at approval.
    let debit = request(h.target, Command::Adjust { amount: -50 });
    s.submit(ctx, debit.clone()).await.unwrap();
    let other = request(h.target, Command::Adjust { amount: -1 });
    s.submit(ctx, other.clone()).await.unwrap();
    s.approve(approver, other.request_id).await.unwrap();
    assert!(matches!(
        s.approve(approver, debit.request_id).await,
        Err(Error::Conflict)
    ));
    assert_eq!(h.count("wallet_entries").await, 2);
    assert!(matches!(
        s.submit(ctx, request(h.target, Command::Adjust { amount: i64::MIN }))
            .await,
        Err(Error::Invalid)
    ));
    let req = request(h.target, Command::Adjust { amount: 1 });
    s.submit(ctx, req.clone()).await.unwrap();
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(h.actor)
        .execute(&h.pool)
        .await
        .unwrap();
    assert!(matches!(
        s.approve(approver, req.request_id).await,
        Err(Error::Denied)
    ));
    assert_eq!(h.count("wallet_entries").await, 2);
    h.close().await;
}
#[tokio::test]
async fn postgres_payments_review_and_refund_queue_without_forged_settlement() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let s = h.service();
    let ctx = context(&h.pool, h.actor).await;
    let approver = context(&h.pool, h.second).await;
    let payment = Uuid::now_v7();
    sqlx::query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES ($1,$2,'test',$3,100,'PLN','succeeded')").bind(payment).bind(h.target).bind(format!("test:{payment}")).execute(&h.pool).await.unwrap();
    let review = request(payment, Command::Reconcile);
    s.submit(ctx, review.clone()).await.unwrap();
    assert!(s.submit(ctx, review).await.unwrap().replayed);
    let refund = request(payment, Command::Refund);
    s.submit(ctx, refund.clone()).await.unwrap();
    assert_eq!(h.count("commerce_refunds").await, 0);
    assert!(matches!(
        s.approve(ctx, refund.request_id).await,
        Err(Error::Denied)
    ));
    s.approve(approver, refund.request_id).await.unwrap();
    assert_eq!(h.count("commerce_refunds").await, 1);
    assert_eq!(h.count("outbox").await, 1);
    let jobs = oteryn_db::outbox::claim(&h.pool, 10, 60).await.unwrap();
    assert_eq!(jobs.len(), 1);
    let delivery = RefundDelivery {
        pool: h.pool.clone(),
        adapter: h.effects().payments,
    };
    delivery.run(&jobs[0]).await.unwrap();
    delivery.run(&jobs[0]).await.unwrap();
    let mut invalid = jobs[0].clone();
    invalid.payload = "{}".into();
    assert!(matches!(delivery.run(&invalid).await, Err(Error::Invalid)));
    oteryn_db::outbox::finish(&h.pool, &jobs[0], None, false)
        .await
        .unwrap();
    let state: String = sqlx::query_scalar("SELECT state FROM payment_intents WHERE id=$1")
        .bind(payment)
        .fetch_one(&h.pool)
        .await
        .unwrap();
    assert_eq!(state, "succeeded");
    let duplicate = request(payment, Command::Refund);
    s.submit(ctx, duplicate.clone()).await.unwrap();
    assert!(matches!(
        s.approve(approver, duplicate.request_id).await,
        Err(Error::Conflict)
    ));
    assert_eq!(h.count("outbox").await, 1);
    assert!(matches!(
        s.submit(ctx, request(Uuid::now_v7(), Command::Reconcile))
            .await,
        Err(Error::Invalid)
    ));
    sqlx::query("UPDATE payment_intents SET state='pending' WHERE id=$1")
        .bind(payment)
        .execute(&h.pool)
        .await
        .unwrap();
    let pending = request(payment, Command::Refund);
    assert!(matches!(s.submit(ctx, pending).await, Err(Error::Conflict)));
    h.close().await;
}
#[tokio::test]
async fn default_flags_and_host_deny_without_database() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused@127.0.0.1:1/unused")
        .unwrap();
    let app = router(
        pool.clone(),
        crate::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Session(Uuid::now_v7(), pool.clone()),
        Authority(pool.clone()),
        PendingEffects,
        Flags::default(),
    );
    let response = app
        .oneshot(
            HttpRequest::get("/admin/commerce")
                .header("host", "staff.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let s = Commerce::new(
        pool.clone(),
        Authority(pool),
        PendingEffects,
        Flags::default(),
    );
    assert!(matches!(
        s.submit(
            rbac::CheckContext {
                mfa_verified: true,
                ..rbac::CheckContext::new(Uuid::now_v7())
            },
            request(Uuid::now_v7(), Command::Refund)
        )
        .await,
        Err(Error::Disabled)
    ));
    assert!(serde_json::from_str::<Command>(r#"{"kind":"grant","days":2,"extra":true}"#).is_err());
}
#[tokio::test]
async fn postgres_http_forms_localization_csrf_permissions_and_input() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let app = router(
        h.pool.clone(),
        crate::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Session(h.actor, h.pool.clone()),
        Authority(h.pool.clone()),
        h.effects(),
        flags(),
    );
    let response = app
        .clone()
        .oneshot(
            HttpRequest::get("/admin/commerce")
                .header("host", "staff.test")
                .header("accept-language", "en")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let html = String::from_utf8(
        to_bytes(response.into_body(), 200000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    if let Ok(path) = std::env::var("PORTAL_COMMERCE_RENDER_PATH") {
        std::fs::write(path, &html).unwrap();
    }
    let polish = app
        .clone()
        .oneshot(
            HttpRequest::get("/admin/commerce?lang=pl")
                .header("host", "staff.test")
                .header("accept-language", "pl")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Admin navigation links carry the site-wide locale parameter.
    assert_eq!(polish.status(), StatusCode::OK);
    let polish =
        String::from_utf8(to_bytes(polish.into_body(), 200000).await.unwrap().to_vec()).unwrap();
    assert!(polish.contains("Nadaj premium"));
    assert!(html.contains("Grant premium"));
    assert!(html.contains("op-input"));
    let token = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let body = format!(
        "_csrf={token}&request_id={}&target={}&days=30&reason=Reviewed+support+request",
        Uuid::now_v7(),
        h.target
    );
    for (input, status) in [
        (body.clone(), StatusCode::SEE_OTHER),
        (
            body.replace("days=30", "days=0"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (body.replace(token, "bad"), StatusCode::FORBIDDEN),
    ] {
        let response = app
            .clone()
            .oneshot(
                HttpRequest::post("/admin/commerce/grant")
                    .header("host", "staff.test")
                    .header("origin", "https://staff.test")
                    .header("cookie", &cookie)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(input))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    let response = app
        .oneshot(
            HttpRequest::get("/admin/commerce")
                .header("host", "wrong.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    h.close().await;
}

#[tokio::test]
async fn postgres_audit_failure_rolls_back_domain_effect_and_request() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    sqlx::raw_sql("CREATE FUNCTION refuse_commerce_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='commerce.requested' THEN RAISE EXCEPTION 'audit unavailable'; END IF; RETURN NEW; END $$; CREATE TRIGGER refuse_commerce_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION refuse_commerce_audit();").execute(&h.pool).await.unwrap();
    assert!(matches!(
        h.service()
            .submit(
                context(&h.pool, h.actor).await,
                request(h.target, Command::Grant { days: 1 })
            )
            .await,
        Err(Error::Storage(_))
    ));
    assert_eq!(h.count("premium_ledger").await, 0);
    assert_eq!(h.count("premium_audit").await, 0);
    assert_eq!(h.count("commerce_operations").await, 0);
    h.close().await;
}

#[tokio::test]
async fn lk_commerce_single_connection_and_sixteen_mutations() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    for maximum in [1, 16] {
        let h = Harness::with_connections(maximum)
            .await
            .expect("PORTAL_TEST_DATABASE_URL required");
        let service = Arc::new(Commerce::new(
            h.pool.clone(),
            crate::web::PortalIdentity::configured(h.pool.clone()),
            h.effects(),
            flags(),
        ));
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let adjustment = service.submit(context(&h.pool, h.actor).await, request(h.target, Command::Adjust { amount: 50 })).await.unwrap();
            service.approve(context(&h.pool, h.second).await, adjustment.id).await.unwrap();
            let payment = Uuid::now_v7();
            sqlx::query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES ($1,$2,'test',$3,100,'PLN','succeeded')")
                .bind(payment).bind(h.target).bind(format!("test:{payment}")).execute(&h.pool).await.unwrap();
            service.submit(context(&h.pool, h.actor).await, request(payment, Command::Reconcile)).await.unwrap();
            let refund = service.submit(context(&h.pool, h.actor).await, request(payment, Command::Refund)).await.unwrap();
            service.approve(context(&h.pool, h.second).await, refund.id).await.unwrap();
            let delivery = RefundDelivery { pool: h.pool.clone(), adapter: TestPaymentAdapter::new(h.pool.clone(), oteryn_payments::TestProvider::new(&[7;32]).unwrap(), oteryn_payments::PaymentFlags { enabled: true }) };
            delivery.adapter.deliver_refund(&h.pool, refund.id).await.unwrap();
        }).await.unwrap();
        let barrier = Arc::new(tokio::sync::Barrier::new(16));
        let mut tasks = tokio::task::JoinSet::new();
        for _ in 0..16 {
            let service = service.clone();
            let barrier = barrier.clone();
            let actor = h.actor;
            let target = h.target;
            let context = context(&h.pool, actor).await;
            tasks.spawn(async move {
                barrier.wait().await;
                service
                    .submit(context, request(target, Command::Grant { days: 1 }))
                    .await
            });
        }
        // Poll all requests concurrently without needing a second pool for auth.
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while let Some(result) = tasks.join_next().await {
                result.unwrap().unwrap();
            }
        })
        .await
        .unwrap();
        let barrier = Arc::new(tokio::sync::Barrier::new(16));
        let mut logins = tokio::task::JoinSet::new();
        for index in 0..16 {
            let hash = Sha256::digest(format!("lk-browser-{}-{index}", h.target)).to_vec();
            sqlx::query("INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,$4,now()+interval '1 hour',now()+interval '2 hours')")
                .bind(&hash).bind(h.target).bind(format!("Browser {index}")).bind("c".repeat(64)).execute(&h.pool).await.unwrap();
            let sessions = oteryn_identity::sessions::Sessions {
                pool: h.pool.clone(),
            };
            let actor = oteryn_identity::sessions::Actor {
                account: h.target,
                hash,
            };
            let barrier = barrier.clone();
            logins.spawn(async move {
                barrier.wait().await;
                sessions.record_login(&actor).await
            });
        }
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while let Some(result) = logins.join_next().await {
                result.unwrap().unwrap();
            }
        })
        .await
        .unwrap();
        assert_eq!(h.count("email_outbox").await, 16);
        h.close().await;
    }
    eprintln!(
        "lk_commerce_single_connection_and_sixteen_mutations duration: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a2_configured_commerce_domain_adapters_commit_ledgers() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let h = Harness::with_connections(1)
        .await
        .expect("PostgreSQL required");
    let app_for = |actor| {
        crate::web::guarded(
            http::configured_routes(h.pool.clone(), flags(), None),
            h.pool.clone(),
            crate::web::PanelConfig {
                admin_panel: true,
                host: "staff.test".into(),
            },
            Session(actor, h.pool.clone()),
            Authority(h.pool.clone()),
            rbac::Permission::Access,
        )
    };
    let app = app_for(h.actor);
    let response = app
        .clone()
        .oneshot(
            HttpRequest::get("/admin/commerce")
                .header("host", "staff.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let html = String::from_utf8(
        to_bytes(response.into_body(), 200000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let grant_id = Uuid::now_v7();
    let adjustment_id = Uuid::now_v7();
    for (action, id, argument) in [
        ("grant", grant_id, "days=2"),
        ("adjust", adjustment_id, "amount=77"),
    ] {
        let response = app.clone().oneshot(HttpRequest::post(format!("/admin/commerce/{action}")).header("host","staff.test").header("origin","https://staff.test").header("cookie",&cookie).header("content-type","application/x-www-form-urlencoded")
            .body(Body::from(format!("_csrf={csrf}&request_id={id}&target={}&reason=Reviewed+support+request&{argument}",h.target))).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{action}");
    }
    let second_app = app_for(h.second);
    let response = second_app
        .clone()
        .oneshot(
            HttpRequest::get("/admin/commerce")
                .header("host", "staff.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let second_cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let html = String::from_utf8(
        to_bytes(response.into_body(), 200000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let response = second_app
        .clone()
        .oneshot(
            HttpRequest::post(format!("/admin/commerce/approvals/{adjustment_id}"))
                .header("host", "staff.test")
                .header("origin", "https://staff.test")
                .header("cookie", &second_cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM premium_ledger WHERE request_id=$1")
        .bind(grant_id)
        .fetch_one(&h.pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "a staff grant waits for a second staff member");
    assert!(html.contains(&format!("/admin/commerce/approvals/{grant_id}")));
    let response = second_app
        .clone()
        .oneshot(
            HttpRequest::post(format!("/admin/commerce/approvals/{grant_id}"))
                .header("host", "staff.test")
                .header("origin", "https://staff.test")
                .header("cookie", &second_cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM premium_ledger WHERE request_id=$1")
        .bind(grant_id)
        .fetch_one(&h.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let balance: i64 = sqlx::query_scalar(
        "SELECT sum(available_delta)::bigint FROM wallet_entries WHERE account_id=$1",
    )
    .bind(h.target)
    .fetch_one(&h.pool)
    .await
    .unwrap();
    assert_eq!(balance, 77);
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE request_id=$1 AND action='commerce.wallet.adjusted'",
    )
    .bind(adjustment_id)
    .fetch_one(&h.pool)
    .await
    .unwrap();
    assert_eq!(audits, 1);
    h.close().await;
    eprintln!(
        "a2_configured_commerce_domain_adapters_commit_ledgers duration: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a2_commerce_role_writer_waits_are_bounded() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let h = Harness::new().await.expect("PostgreSQL required");
    let service = h.service();
    let pending = service
        .submit(
            context(&h.pool, h.actor).await,
            request(h.target, Command::Adjust { amount: 1 }),
        )
        .await
        .unwrap();
    let mut blocker = h.pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(71010)")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let (submit, approve) = tokio::join!(
        tokio::time::timeout(
            std::time::Duration::from_secs(4),
            service.submit(
                context(&h.pool, h.actor).await,
                request(h.target, Command::Grant { days: 1 })
            )
        ),
        tokio::time::timeout(
            std::time::Duration::from_secs(4),
            service.approve(context(&h.pool, h.second).await, pending.id)
        )
    );
    assert!(matches!(submit.unwrap(), Err(Error::Storage(_))));
    assert!(matches!(approve.unwrap(), Err(Error::Storage(_))));
    blocker.rollback().await.unwrap();
    h.close().await;
    eprintln!(
        "a2_commerce_role_writer_waits_are_bounded duration: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_a3_commerce_locks_payment_owner_before_role_fence() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let h = Harness::new().await.expect("PostgreSQL required");
    let payment = Uuid::now_v7();
    sqlx::query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES($1,$2,'test',$3,100,'PLN','succeeded')").bind(payment).bind(h.target).bind(payment.to_string()).execute(&h.pool).await.unwrap();
    let service = Commerce::new(
        h.pool.clone(),
        crate::web::PortalIdentity::configured(h.pool.clone()),
        h.effects(),
        flags(),
    );
    let actor = context(&h.pool, h.actor).await;
    crate::test_staff::race_account_before_fence(&h.pool, h.target, async move {
        service
            .submit(actor, request(payment, Command::Reconcile))
            .await
    })
    .await
    .unwrap()
    .unwrap();
    h.close().await;
    eprintln!(
        "postgres_a3_commerce_locks_payment_owner_before_role_fence duration: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a3_commerce_queue_pagination_and_terminal_disposition() {
    let started = std::time::Instant::now();
    let h = Harness::new().await.expect("PostgreSQL required");
    let service = h.service();
    let mut blocked = Vec::new();
    for _ in 0..100 {
        blocked.push(
            service
                .submit(
                    context(&h.pool, h.actor).await,
                    request(h.target, Command::Adjust { amount: 1 }),
                )
                .await
                .unwrap()
                .id,
        );
    }
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(h.actor)
        .execute(&h.pool)
        .await
        .unwrap();
    assert!(matches!(
        service
            .approve(context(&h.pool, h.second).await, blocked[0])
            .await,
        Err(Error::Denied)
    ));
    sqlx::query("UPDATE commerce_operations SET created_at='2026-01-01T00:00:00Z'")
        .execute(&h.pool)
        .await
        .unwrap();
    let valid = service
        .submit(
            context(&h.pool, h.second).await,
            request(h.target, Command::Adjust { amount: 50 }),
        )
        .await
        .unwrap()
        .id;
    // A third currently authorized staff member reviews the newer request.
    let reviewer = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale) VALUES($1,'reviewer@example.test','test-only','active',now(),'en')").bind(reviewer).execute(&h.pool).await.unwrap();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(reviewer).execute(&h.pool).await.unwrap();
    let app = router(
        h.pool.clone(),
        crate::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Session(reviewer, h.pool.clone()),
        Authority(h.pool.clone()),
        h.effects(),
        flags(),
    );
    let response = app
        .clone()
        .oneshot(
            HttpRequest::get("/admin/commerce")
                .header("host", "staff.test")
                .header("accept-language", "en")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let html = String::from_utf8(
        to_bytes(response.into_body(), 200000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(!html.contains(&valid.to_string()));
    let next = html
        .split("rel=\"next\" href=\"")
        .nth(1)
        .expect("next page must be reachable")
        .split('"')
        .next()
        .unwrap()
        .replace("&amp;", "&");
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let response = app
        .clone()
        .oneshot(
            HttpRequest::get(&next)
                .header("host", "staff.test")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let second = String::from_utf8(
        to_bytes(response.into_body(), 200000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(second.contains(&valid.to_string()));
    // An audit failure must leave the intent pending and produce no money effect.
    sqlx::raw_sql("CREATE FUNCTION reject_cancellation() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='commerce.cancelled' THEN RAISE EXCEPTION 'test audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_cancellation BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION reject_cancellation()")
        .execute(&h.pool).await.unwrap();
    let response = app
        .clone()
        .oneshot(
            HttpRequest::post(format!("/admin/commerce/cancellations/{}", blocked[0]))
                .header("host", "staff.test")
                .header("origin", "https://staff.test")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let pending: bool =
        sqlx::query_scalar("SELECT result IS NULL FROM commerce_operations WHERE id=$1")
            .bind(blocked[0])
            .fetch_one(&h.pool)
            .await
            .unwrap();
    assert!(pending);
    sqlx::query("DROP TRIGGER reject_cancellation ON audit_log")
        .execute(&h.pool)
        .await
        .unwrap();
    for (path, token, expected) in [
        (
            format!("/admin/commerce/cancellations/{}", blocked[0]),
            "bad",
            StatusCode::FORBIDDEN,
        ),
        (
            format!("/admin/commerce/cancellations/{}", blocked[0]),
            csrf,
            StatusCode::SEE_OTHER,
        ),
        (
            format!("/admin/commerce/cancellations/{}", blocked[0]),
            csrf,
            StatusCode::SEE_OTHER,
        ),
        (
            format!("/admin/commerce/approvals/{valid}"),
            csrf,
            StatusCode::SEE_OTHER,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                HttpRequest::post(path)
                    .header("host", "staff.test")
                    .header("origin", "https://staff.test")
                    .header("cookie", &cookie)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!("_csrf={token}")))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
    let response = app
        .clone()
        .oneshot(
            HttpRequest::post(format!("/admin/commerce/cancellations/{valid}"))
                .header("host", "staff.test")
                .header("origin", "https://staff.test")
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let cancelled: Value = sqlx::query_scalar("SELECT result FROM commerce_operations WHERE id=$1")
        .bind(blocked[0])
        .fetch_one(&h.pool)
        .await
        .unwrap();
    assert_eq!(cancelled, json!({"status":"cancelled"}));
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE request_id=$1 AND actor=$2 AND action='commerce.cancelled'").bind(blocked[0]).bind(reviewer).fetch_one(&h.pool).await.unwrap();
    assert_eq!(audits, 1);
    assert_eq!(h.count("wallet_operations").await, 1);
    h.close().await;
    eprintln!(
        "a3_commerce_queue_pagination_and_terminal_disposition duration: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a3_l6_02_staff_routes_refuse_restricted_acquisitions() {
    let started = std::time::Instant::now();
    let h = Harness::new().await.expect("PostgreSQL required");
    let app_for = |actor| {
        crate::web::guarded(
            http::configured_routes(h.pool.clone(), flags(), None),
            h.pool.clone(),
            crate::web::PanelConfig {
                admin_panel: true,
                host: "staff.test".into(),
            },
            Session(actor, h.pool.clone()),
            Authority(h.pool.clone()),
            rbac::Permission::Access,
        )
    };
    let app = app_for(h.actor);
    let second = app_for(h.second);
    let (cookie, csrf) = login(app.clone()).await;
    let (second_cookie, second_csrf) = login(second.clone()).await;
    for restriction in ["restriction", "suspension", "termination", "cooldown"] {
        let adjustment_id = Uuid::now_v7();
        let response = app.clone().oneshot(HttpRequest::post("/admin/commerce/adjust").header("host","staff.test").header("origin","https://staff.test").header("cookie",&cookie).header("content-type","application/x-www-form-urlencoded").body(Body::from(format!("_csrf={csrf}&request_id={adjustment_id}&target={}&reason=Reviewed+support+request&amount=77",h.target))).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        match restriction {
            "restriction" | "suspension" => {
                sqlx::query("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES($1,$2,'active','test',now()-interval '1 hour',now()+interval '1 day',$1)").bind(h.target).bind(restriction).execute(&h.pool).await.unwrap();
            }
            "termination" => {
                sqlx::query("INSERT INTO account_terminations(account_id,scheduled_for) VALUES($1,now()+interval '14 days')").bind(h.target).execute(&h.pool).await.unwrap();
            }
            _ => {
                sqlx::query("UPDATE accounts SET email_change_blocked_until=now()+interval '1 day' WHERE id=$1").bind(h.target).execute(&h.pool).await.unwrap();
            }
        }
        let response = app.clone().oneshot(HttpRequest::post("/admin/commerce/grant").header("host","staff.test").header("origin","https://staff.test").header("cookie",&cookie).header("content-type","application/x-www-form-urlencoded").body(Body::from(format!("_csrf={csrf}&request_id={}&target={}&reason=Reviewed+support+request&days=2",Uuid::now_v7(),h.target))).unwrap()).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "grant {restriction}"
        );
        let response = second
            .clone()
            .oneshot(
                HttpRequest::post(format!("/admin/commerce/approvals/{adjustment_id}"))
                    .header("host", "staff.test")
                    .header("origin", "https://staff.test")
                    .header("cookie", &second_cookie)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!("_csrf={second_csrf}")))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "approval {restriction}"
        );
        for table in ["premium_ledger", "wallet_entries", "premium_authority"] {
            assert_eq!(h.count(table).await, 0, "{restriction} {table}");
        }
        sqlx::query("UPDATE sanctions SET status='revoked' WHERE account_id=$1")
            .bind(h.target)
            .execute(&h.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE account_terminations SET cancelled_at=now() WHERE account_id=$1")
            .bind(h.target)
            .execute(&h.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE accounts SET email_change_blocked_until='-infinity' WHERE id=$1")
            .bind(h.target)
            .execute(&h.pool)
            .await
            .unwrap();
    }
    h.close().await;
    eprintln!(
        "PG a3_l6_02_staff_routes_refuse_restricted_acquisitions {:.3}s",
        started.elapsed().as_secs_f64()
    );
}

/// Provider double for refund dispatch: records every idempotency key it is
/// given, can fail the next calls and can hold a call open until released.
#[derive(Debug, Clone, Default)]
struct RefundStub {
    keys: Arc<std::sync::Mutex<Vec<Uuid>>>,
    failures: Arc<std::sync::atomic::AtomicUsize>,
    malformed: Arc<std::sync::atomic::AtomicUsize>,
    hold: bool,
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}
impl RefundStub {
    fn holding() -> Self {
        Self {
            hold: true,
            ..Self::default()
        }
    }
    fn keys(&self) -> Vec<Uuid> {
        self.keys.lock().unwrap().clone()
    }
}
impl oteryn_payments::PaymentProvider for RefundStub {
    fn name(&self) -> &'static str {
        "test"
    }
    async fn create_intent(
        &self,
        _: Uuid,
        _: &oteryn_payments::Amount,
    ) -> Result<String, oteryn_payments::Error> {
        Err(oteryn_payments::Error::InvalidInput)
    }
    fn verify_webhook_signature(&self, _: &[u8], _: &[u8]) -> Result<(), oteryn_payments::Error> {
        Err(oteryn_payments::Error::BadSignature)
    }
    fn parse_event(
        &self,
        _: &[u8],
    ) -> Result<oteryn_payments::PaymentEvent, oteryn_payments::Error> {
        Err(oteryn_payments::Error::InvalidInput)
    }
    async fn refund(
        &self,
        intent_id: Uuid,
        _: &oteryn_payments::Amount,
    ) -> Result<String, oteryn_payments::Error> {
        self.keys.lock().unwrap().push(intent_id);
        if self.hold {
            self.entered.notify_one();
            self.release.notified().await;
        }
        let failing = self
            .failures
            .fetch_update(
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
                |n| n.checked_sub(1),
            )
            .is_ok();
        if failing {
            return Err(oteryn_payments::Error::Database(sqlx::Error::PoolTimedOut));
        }
        if self
            .malformed
            .fetch_update(
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
                |n| n.checked_sub(1),
            )
            .is_ok()
        {
            return Ok("refund:\nforged".into());
        }
        Ok(format!("refund:{intent_id}"))
    }
}
/// An approved refund queued for dispatch, as the outbox job sees it.
async fn queued_refund(h: &Harness) -> (Uuid, Uuid) {
    let s = h.service();
    let payment = Uuid::now_v7();
    sqlx::query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES ($1,$2,'test',$3,100,'PLN','succeeded')").bind(payment).bind(h.target).bind(format!("test:{payment}")).execute(&h.pool).await.unwrap();
    let refund = request(payment, Command::Refund);
    s.submit(context(&h.pool, h.actor).await, refund.clone())
        .await
        .unwrap();
    s.approve(context(&h.pool, h.second).await, refund.request_id)
        .await
        .unwrap();
    (refund.request_id, payment)
}
async fn dispatched_audits(h: &Harness) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='commerce.refund.dispatched'")
        .fetch_one(&h.pool)
        .await
        .unwrap()
}
async fn refund_row(h: &Harness, operation: Uuid) -> (String, Option<Uuid>, Option<String>) {
    sqlx::query_as(
        "SELECT state,attempt_id,provider_reference FROM commerce_refunds WHERE operation_id=$1",
    )
    .bind(operation)
    .fetch_one(&h.pool)
    .await
    .unwrap()
}
/// Another session can lock the refund row with NOWAIT while the provider call
/// is in flight: the external call holds no row lock and no open transaction.
#[tokio::test]
async fn postgres_refund_provider_call_holds_no_refund_row_lock() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let (operation, payment) = queued_refund(&h).await;
    let adapter = h.effects().payments;
    let stub = RefundStub::holding();
    let probe = async {
        stub.entered.notified().await;
        let mut tx = h.pool.begin().await.unwrap();
        sqlx::query("SET LOCAL lock_timeout='1s'")
            .execute(&mut *tx)
            .await
            .unwrap();
        let locked =
            sqlx::query("SELECT 1 FROM commerce_refunds WHERE operation_id=$1 FOR UPDATE NOWAIT")
                .bind(operation)
                .fetch_optional(&mut *tx)
                .await;
        tx.rollback().await.unwrap();
        stub.release.notify_one();
        locked
    };
    let (delivered, locked) = tokio::join!(
        adapter.deliver_refund_with(&stub, &h.pool, operation),
        probe
    );
    assert!(
        locked.unwrap().is_some(),
        "refund row was locked during the provider call"
    );
    delivered.unwrap();
    assert_eq!(stub.keys(), vec![payment]);
    assert_eq!(dispatched_audits(&h).await, 1);
    h.close().await;
}
/// While one attempt holds a fresh claim, a concurrent delivery is told to
/// retry later and the provider is called exactly once.
#[tokio::test]
async fn postgres_refund_fresh_claim_defers_concurrent_delivery() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let (operation, payment) = queued_refund(&h).await;
    let adapter = h.effects().payments;
    let stub = RefundStub::holding();
    let second = async {
        stub.entered.notified().await;
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            adapter.deliver_refund_with(&stub, &h.pool, operation),
        )
        .await;
        stub.release.notify_one();
        result
    };
    let (first, second) = tokio::join!(
        adapter.deliver_refund_with(&stub, &h.pool, operation),
        second
    );
    assert!(matches!(second, Ok(Err(Error::Unavailable))), "{second:?}");
    first.unwrap();
    assert_eq!(stub.keys(), vec![payment]);
    let (state, _, reference) = refund_row(&h, operation).await;
    assert_eq!(state, "done");
    assert_eq!(reference, Some(format!("refund:{payment}")));
    assert_eq!(dispatched_audits(&h).await, 1);
    h.close().await;
}
/// A claim older than the stale window is re-claimed under a new attempt id;
/// the provider sees the same idempotency key and the stale attempt's late
/// finalize writes nothing.
#[tokio::test]
async fn postgres_refund_stale_claim_is_reclaimed_with_same_key() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let (operation, payment) = queued_refund(&h).await;
    let adapter = h.effects().payments;
    let stale = RefundStub::holding();
    let fresh = RefundStub::default();
    let reclaim = async {
        stale.entered.notified().await;
        let (_, stale_attempt, _) = refund_row(&h, operation).await;
        sqlx::query("UPDATE commerce_refunds SET claimed_at=claimed_at - make_interval(secs => $2) - interval '1 second' WHERE operation_id=$1")
            .bind(operation)
            .bind(REFUND_CLAIM_STALE_AFTER.as_secs_f64())
            .execute(&h.pool)
            .await
            .unwrap();
        let result = adapter
            .deliver_refund_with(&fresh, &h.pool, operation)
            .await;
        let (state, attempt, reference) = refund_row(&h, operation).await;
        stale.release.notify_one();
        (result, stale_attempt, state, attempt, reference)
    };
    let (late, (reclaimed, stale_attempt, state, attempt, reference)) = tokio::join!(
        adapter.deliver_refund_with(&stale, &h.pool, operation),
        reclaim
    );
    reclaimed.unwrap();
    assert_eq!(state, "done");
    assert!(stale_attempt.is_some());
    assert_ne!(attempt, stale_attempt);
    assert_eq!(reference, Some(format!("refund:{payment}")));
    assert_eq!(dispatched_audits(&h).await, 1);
    late.unwrap();
    assert_eq!(stale.keys(), vec![payment]);
    assert_eq!(fresh.keys(), vec![payment]);
    let (state, final_attempt, reference) = refund_row(&h, operation).await;
    assert_eq!(
        (state.as_str(), final_attempt, reference),
        ("done", attempt, Some(format!("refund:{payment}")))
    );
    assert_eq!(dispatched_audits(&h).await, 1);
    h.close().await;
}
/// A provider error releases the claim back to pending so the outbox retry
/// can dispatch at once; the retry succeeds with one dispatch audit.
#[tokio::test]
async fn postgres_refund_provider_error_releases_claim_for_retry() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let (operation, payment) = queued_refund(&h).await;
    let adapter = h.effects().payments;
    let stub = RefundStub::default();
    stub.failures.store(1, std::sync::atomic::Ordering::SeqCst);
    assert!(matches!(
        adapter.deliver_refund_with(&stub, &h.pool, operation).await,
        Err(Error::Unavailable)
    ));
    assert_eq!(
        refund_row(&h, operation).await,
        ("pending".into(), None, None)
    );
    assert_eq!(dispatched_audits(&h).await, 0);
    adapter
        .deliver_refund_with(&stub, &h.pool, operation)
        .await
        .unwrap();
    assert_eq!(stub.keys(), vec![payment, payment]);
    let (state, _, reference) = refund_row(&h, operation).await;
    assert_eq!(state, "done");
    assert_eq!(reference, Some(format!("refund:{payment}")));
    assert_eq!(dispatched_audits(&h).await, 1);
    h.close().await;
}
/// Runs a stale attempt and a re-claimed attempt at once and finishes the
/// stale one first, while the re-claimed attempt still holds the claim. The
/// stale attempt either records a reference or fails, by `stale_fails`.
async fn stale_attempt_finishes_under_a_live_reclaim(stale_fails: bool) {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let (operation, payment) = queued_refund(&h).await;
    let adapter = h.effects().payments;
    let stale = RefundStub::holding();
    if stale_fails {
        stale.failures.store(1, std::sync::atomic::Ordering::SeqCst);
    }
    let fresh = RefundStub::holding();
    let fresh_go = tokio::sync::Notify::new();
    let stale_done = tokio::sync::Notify::new();
    let stale_run = async {
        let result = adapter
            .deliver_refund_with(&stale, &h.pool, operation)
            .await;
        stale_done.notify_one();
        result
    };
    let fresh_run = async {
        fresh_go.notified().await;
        adapter
            .deliver_refund_with(&fresh, &h.pool, operation)
            .await
    };
    let driver = async {
        stale.entered.notified().await;
        let (_, stale_attempt, _) = refund_row(&h, operation).await;
        sqlx::query("UPDATE commerce_refunds SET claimed_at=claimed_at - make_interval(secs => $2) - interval '1 second' WHERE operation_id=$1")
            .bind(operation)
            .bind(REFUND_CLAIM_STALE_AFTER.as_secs_f64())
            .execute(&h.pool)
            .await
            .unwrap();
        fresh_go.notify_one();
        fresh.entered.notified().await;
        let (_, fresh_attempt, _) = refund_row(&h, operation).await;
        assert!(stale_attempt.is_some());
        assert_ne!(fresh_attempt, stale_attempt);
        stale.release.notify_one();
        stale_done.notified().await;
        // The stale attempt finished, but the re-claimed attempt still owns
        // the row: nothing was recorded and the claim was not released.
        assert_eq!(
            refund_row(&h, operation).await,
            ("refunding".into(), fresh_attempt, None)
        );
        assert_eq!(dispatched_audits(&h).await, 0);
        fresh.release.notify_one();
        fresh_attempt
    };
    let (late, settled, fresh_attempt) = tokio::join!(stale_run, fresh_run, driver);
    assert!(matches!(late, Err(Error::Unavailable)));
    settled.unwrap();
    assert_eq!(
        refund_row(&h, operation).await,
        (
            "done".into(),
            fresh_attempt,
            Some(format!("refund:{payment}"))
        )
    );
    assert_eq!(dispatched_audits(&h).await, 1);
    assert_eq!(stale.keys(), vec![payment]);
    assert_eq!(fresh.keys(), vec![payment]);
    h.close().await;
}
/// A stale attempt whose provider call succeeds after a re-claim cannot
/// record its reference over the live claim (fenced by attempt id).
#[tokio::test]
async fn postgres_refund_stale_finalize_cannot_overwrite_a_live_reclaim() {
    stale_attempt_finishes_under_a_live_reclaim(false).await;
}
/// A stale attempt whose provider call fails after a re-claim cannot
/// release the live claim back to pending (fenced by attempt id).
#[tokio::test]
async fn postgres_refund_stale_failure_cannot_release_a_live_reclaim() {
    stale_attempt_finishes_under_a_live_reclaim(true).await;
}
/// Only the last outbox attempt of an unsettled refund is reported for
/// operator reconciliation, with the row's state and attempt id.
#[tokio::test]
async fn postgres_refund_exhaustion_reports_only_the_last_unsettled_attempt() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let (operation, _) = queued_refund(&h).await;
    assert_eq!(
        effects::exhausted_refund(&h.pool, REFUND_MAX_ATTEMPTS - 1, operation).await,
        None
    );
    assert_eq!(
        effects::exhausted_refund(&h.pool, REFUND_MAX_ATTEMPTS, operation).await,
        Some(("pending".into(), None))
    );
    let attempt = Uuid::now_v7();
    sqlx::query("UPDATE commerce_refunds SET state='refunding',attempt_id=$2,claimed_at=now() WHERE operation_id=$1")
        .bind(operation)
        .bind(attempt)
        .execute(&h.pool)
        .await
        .unwrap();
    assert_eq!(
        effects::exhausted_refund(&h.pool, REFUND_MAX_ATTEMPTS, operation).await,
        Some(("refunding".into(), Some(attempt)))
    );
    let adapter = h.effects().payments;
    sqlx::query("UPDATE commerce_refunds SET claimed_at=claimed_at - make_interval(secs => $2) - interval '1 second' WHERE operation_id=$1")
        .bind(operation)
        .bind(REFUND_CLAIM_STALE_AFTER.as_secs_f64())
        .execute(&h.pool)
        .await
        .unwrap();
    adapter
        .deliver_refund_with(&RefundStub::default(), &h.pool, operation)
        .await
        .unwrap();
    assert_eq!(
        effects::exhausted_refund(&h.pool, REFUND_MAX_ATTEMPTS, operation).await,
        None
    );
    h.close().await;
}
/// A malformed provider reference is not recorded: the claim is released and
/// the retry records the well-formed one with a single dispatch audit.
#[tokio::test]
async fn postgres_refund_malformed_reference_releases_claim_for_retry() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let (operation, payment) = queued_refund(&h).await;
    let adapter = h.effects().payments;
    let stub = RefundStub::default();
    stub.malformed.store(1, std::sync::atomic::Ordering::SeqCst);
    assert!(matches!(
        adapter.deliver_refund_with(&stub, &h.pool, operation).await,
        Err(Error::Unavailable)
    ));
    assert_eq!(
        refund_row(&h, operation).await,
        ("pending".into(), None, None)
    );
    assert_eq!(dispatched_audits(&h).await, 0);
    adapter
        .deliver_refund_with(&stub, &h.pool, operation)
        .await
        .unwrap();
    assert_eq!(stub.keys(), vec![payment, payment]);
    let (state, _, reference) = refund_row(&h, operation).await;
    assert_eq!(state, "done");
    assert_eq!(reference, Some(format!("refund:{payment}")));
    assert_eq!(dispatched_audits(&h).await, 1);
    h.close().await;
}
/// A recorded provider reference or a ledger-refunded intent settles the job
/// without calling the provider again.
#[tokio::test]
async fn postgres_refund_settled_rows_skip_the_provider() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let adapter = h.effects().payments;
    let (dispatched, payment) = queued_refund(&h).await;
    let stub = RefundStub::default();
    adapter
        .deliver_refund_with(&stub, &h.pool, dispatched)
        .await
        .unwrap();
    adapter
        .deliver_refund_with(&stub, &h.pool, dispatched)
        .await
        .unwrap();
    assert_eq!(stub.keys(), vec![payment]);
    assert_eq!(dispatched_audits(&h).await, 1);
    let (ledger, refunded) = queued_refund(&h).await;
    sqlx::query("UPDATE payment_intents SET state='refunded' WHERE id=$1")
        .bind(refunded)
        .execute(&h.pool)
        .await
        .unwrap();
    let untouched = RefundStub::default();
    for _ in 0..2 {
        adapter
            .deliver_refund_with(&untouched, &h.pool, ledger)
            .await
            .unwrap();
    }
    assert!(untouched.keys().is_empty());
    let (state, _, reference) = refund_row(&h, ledger).await;
    assert_eq!(state, "done");
    assert_eq!(reference.as_deref(), Some("verified-by-payment-ledger"));
    assert_eq!(dispatched_audits(&h).await, 2);
    h.close().await;
}
/// Marks the refund's `commerce.refund` job dead after its last attempt.
async fn kill_refund_job(h: &Harness, payment: Uuid) {
    let killed = sqlx::query("UPDATE outbox SET state='dead',attempts=max_attempts,leased_until=NULL WHERE kind='commerce.refund' AND idempotency_key=$1")
        .bind(payment.to_string())
        .execute(&h.pool)
        .await
        .expect("kill refund job")
        .rows_affected();
    assert_eq!(killed, 1);
}
/// Moves the row to `refunding` under an attempt claimed `age` seconds ago.
async fn claim_refund_row(h: &Harness, operation: Uuid, age: f64) -> Uuid {
    let attempt = Uuid::now_v7();
    sqlx::query("UPDATE commerce_refunds SET state='refunding',attempt_id=$2,claimed_at=now() - make_interval(secs => $3) WHERE operation_id=$1")
        .bind(operation)
        .bind(attempt)
        .bind(age)
        .execute(&h.pool)
        .await
        .expect("claim refund row");
    attempt
}
async fn refund_job(h: &Harness, payment: Uuid) -> (String, i32) {
    sqlx::query_as(
        "SELECT state,attempts FROM outbox WHERE kind='commerce.refund' AND idempotency_key=$1",
    )
    .bind(payment.to_string())
    .fetch_one(&h.pool)
    .await
    .expect("refund job")
}
async fn reconciled_audits(h: &Harness) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='commerce.refund.reconciled'")
        .fetch_one(&h.pool)
        .await
        .expect("reconciled audits")
}
/// Claims the revived job from the outbox and runs the delivery path on it.
async fn deliver_revived(h: &Harness, stub: &RefundStub, operation: Uuid) {
    let jobs = oteryn_db::outbox::claim(&h.pool, 10, 60)
        .await
        .expect("claim outbox");
    let job = jobs
        .iter()
        .find(|job| job.kind == "commerce.refund")
        .expect("revived job is claimable");
    assert_eq!(job.attempts, 1);
    assert_eq!(
        serde_json::from_str::<Value>(&job.payload).expect("payload"),
        json!({"operation":operation})
    );
    h.effects()
        .payments
        .deliver_refund_with(stub, &h.pool, operation)
        .await
        .expect("deliver revived refund");
    assert!(
        oteryn_db::outbox::finish(&h.pool, job, None, false)
            .await
            .expect("finish job")
    );
}
/// A dead job over a `pending` row is revived; delivery calls the provider
/// with the intent id and settles the row with one dispatch audit.
#[tokio::test]
async fn postgres_refund_reconcile_revives_dead_pending_row() {
    let _database_guard = oteryn_db::test_support::global_lock_guard()
        .await
        .expect("database guard");
    let Some(h) = Harness::new().await else {
        return;
    };
    let (operation, payment) = queued_refund(&h).await;
    kill_refund_job(&h, payment).await;
    let revived = reconcile_dead_refunds(&h.pool, h.actor, false)
        .await
        .expect("reconcile");
    assert_eq!(revived.len(), 1);
    assert_eq!(
        (
            revived[0].operation,
            revived[0].state.as_str(),
            revived[0].attempt
        ),
        (operation, "pending", None)
    );
    assert_eq!(refund_job(&h, payment).await, ("pending".into(), 0));
    assert_eq!(reconciled_audits(&h).await, 1);
    let stub = RefundStub::default();
    deliver_revived(&h, &stub, operation).await;
    assert_eq!(stub.keys(), vec![payment]);
    let (state, _, reference) = refund_row(&h, operation).await;
    assert_eq!(state, "done");
    assert_eq!(reference, Some(format!("refund:{payment}")));
    assert_eq!(dispatched_audits(&h).await, 1);
    assert_eq!(refund_job(&h, payment).await, ("done".into(), 1));
    h.close().await;
}
/// A dead job over a stale `refunding` claim is revived and settles; a fresh
/// claim is left alone with its job still dead.
#[tokio::test]
async fn postgres_refund_reconcile_revives_stale_refunding_row() {
    let _database_guard = oteryn_db::test_support::global_lock_guard()
        .await
        .expect("database guard");
    let Some(h) = Harness::new().await else {
        return;
    };
    let (fresh_op, fresh_pay) = queued_refund(&h).await;
    kill_refund_job(&h, fresh_pay).await;
    let fresh_attempt = claim_refund_row(&h, fresh_op, 0.0).await;
    let (operation, payment) = queued_refund(&h).await;
    kill_refund_job(&h, payment).await;
    let stale_attempt =
        claim_refund_row(&h, operation, REFUND_CLAIM_STALE_AFTER.as_secs_f64() + 1.0).await;
    let revived = reconcile_dead_refunds(&h.pool, h.actor, false)
        .await
        .expect("reconcile");
    assert_eq!(revived.len(), 1);
    assert_eq!(
        (
            revived[0].operation,
            revived[0].state.as_str(),
            revived[0].attempt
        ),
        (operation, "refunding", Some(stale_attempt))
    );
    assert_eq!(reconciled_audits(&h).await, 1);
    assert_eq!(refund_job(&h, fresh_pay).await.0, "dead");
    assert_eq!(
        refund_row(&h, fresh_op).await,
        ("refunding".into(), Some(fresh_attempt), None)
    );
    let stub = RefundStub::default();
    deliver_revived(&h, &stub, operation).await;
    assert_eq!(stub.keys(), vec![payment]);
    let (state, attempt, reference) = refund_row(&h, operation).await;
    assert_eq!(state, "done");
    assert_ne!(attempt, Some(stale_attempt));
    assert_eq!(reference, Some(format!("refund:{payment}")));
    assert_eq!(dispatched_audits(&h).await, 1);
    h.close().await;
}
/// A settled row, or one whose intent the ledger shows refunded, with a dead
/// job is neither revived nor audited and the provider is not called again.
#[tokio::test]
async fn postgres_refund_reconcile_leaves_settled_rows() {
    let _database_guard = oteryn_db::test_support::global_lock_guard()
        .await
        .expect("database guard");
    let Some(h) = Harness::new().await else {
        return;
    };
    let (operation, payment) = queued_refund(&h).await;
    let stub = RefundStub::default();
    h.effects()
        .payments
        .deliver_refund_with(&stub, &h.pool, operation)
        .await
        .expect("deliver");
    kill_refund_job(&h, payment).await;
    let before = refund_row(&h, operation).await;
    let revived = reconcile_dead_refunds(&h.pool, h.actor, false)
        .await
        .expect("reconcile");
    assert!(revived.is_empty());
    assert_eq!(refund_row(&h, operation).await, before);
    assert_eq!(refund_job(&h, payment).await.0, "dead");
    assert_eq!(reconciled_audits(&h).await, 0);
    assert_eq!(stub.keys(), vec![payment]);
    assert_eq!(dispatched_audits(&h).await, 1);
    let (ledger, refunded) = queued_refund(&h).await;
    kill_refund_job(&h, refunded).await;
    sqlx::query("UPDATE payment_intents SET state='refunded' WHERE id=$1")
        .bind(refunded)
        .execute(&h.pool)
        .await
        .expect("refund intent");
    let revived = reconcile_dead_refunds(&h.pool, h.actor, false)
        .await
        .expect("reconcile ledger-refunded");
    assert!(revived.is_empty());
    assert_eq!(refund_row(&h, ledger).await, ("pending".into(), None, None));
    assert_eq!(refund_job(&h, refunded).await.0, "dead");
    assert_eq!(reconciled_audits(&h).await, 0);
    h.close().await;
}
/// A dry run reports the dead refund and writes nothing; a non-staff actor
/// is refused.
#[tokio::test]
async fn postgres_refund_reconcile_dry_run_changes_nothing() {
    let _database_guard = oteryn_db::test_support::global_lock_guard()
        .await
        .expect("database guard");
    let Some(h) = Harness::new().await else {
        return;
    };
    let (operation, payment) = queued_refund(&h).await;
    kill_refund_job(&h, payment).await;
    let listed = reconcile_dead_refunds(&h.pool, h.actor, true)
        .await
        .expect("dry run");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].operation, operation);
    let (job_state, job_attempts) = refund_job(&h, payment).await;
    assert_eq!(job_state, "dead");
    assert_eq!(job_attempts, REFUND_MAX_ATTEMPTS);
    assert_eq!(reconciled_audits(&h).await, 0);
    assert_eq!(
        refund_row(&h, operation).await,
        ("pending".into(), None, None)
    );
    assert!(matches!(
        reconcile_dead_refunds(&h.pool, h.target, true).await,
        Err(Error::Denied)
    ));
    h.close().await;
}
/// A second run finds the revived job no longer dead and revives nothing.
#[tokio::test]
async fn postgres_refund_reconcile_is_idempotent() {
    let _database_guard = oteryn_db::test_support::global_lock_guard()
        .await
        .expect("database guard");
    let Some(h) = Harness::new().await else {
        return;
    };
    let (_, payment) = queued_refund(&h).await;
    kill_refund_job(&h, payment).await;
    let first = reconcile_dead_refunds(&h.pool, h.actor, false)
        .await
        .expect("first reconcile");
    let second = reconcile_dead_refunds(&h.pool, h.actor, false)
        .await
        .expect("second reconcile");
    assert_eq!((first.len(), second.len()), (1, 0));
    assert_eq!(reconciled_audits(&h).await, 1);
    assert_eq!(refund_job(&h, payment).await, ("pending".into(), 0));
    h.close().await;
}
#[tokio::test]
async fn postgres_staff_never_grant_premium_or_balance_to_themselves() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let service = h.service();
    let ctx = context(&h.pool, h.actor).await;
    let second = context(&h.pool, h.second).await;
    for command in [Command::Grant { days: 30 }, Command::Adjust { amount: 100 }] {
        assert!(matches!(
            service.submit(ctx, request(h.actor, command)).await,
            Err(Error::Denied)
        ));
    }
    // Nor does the approver approve a correction whose beneficiary they are.
    let req = request(h.second, Command::Adjust { amount: 100 });
    assert!(service.submit(ctx, req.clone()).await.unwrap().pending);
    assert!(matches!(
        service.approve(second, req.request_id).await,
        Err(Error::Denied)
    ));
    assert_eq!(h.count("premium_ledger").await, 0);
    assert_eq!(h.count("wallet_entries").await, 0);
    let denied: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='commerce.denied'")
            .fetch_one(&h.pool)
            .await
            .unwrap();
    assert_eq!(denied, 2);
    h.close().await;
}
async fn grant_entitlements(h: &Harness, request: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM premium_ledger WHERE request_id=$1 AND event_type='GRANT'",
    )
    .bind(request)
    .fetch_one(&h.pool)
    .await
    .unwrap()
}
#[tokio::test]
async fn postgres_grant_submit_is_pending_without_entitlement() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let service = h.service();
    let req = request(h.target, Command::Grant { days: 30 });
    let outcome = service
        .submit(context(&h.pool, h.actor).await, req.clone())
        .await
        .unwrap();
    assert!(outcome.pending);
    assert!(!outcome.replayed);
    assert_eq!(grant_entitlements(&h, req.request_id).await, 0);
    assert_eq!(h.count("premium_ledger").await, 0);
    assert_eq!(h.count("premium_authority").await, 0);
    h.close().await;
}
#[tokio::test]
async fn postgres_grant_proposer_cannot_approve_own_grant() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let service = h.service();
    let ctx = context(&h.pool, h.actor).await;
    let req = request(h.target, Command::Grant { days: 30 });
    assert!(service.submit(ctx, req.clone()).await.unwrap().pending);
    assert!(matches!(
        service.approve(ctx, req.request_id).await,
        Err(Error::Denied)
    ));
    assert_eq!(h.count("premium_ledger").await, 0);
    let open: bool =
        sqlx::query_scalar("SELECT result IS NULL FROM commerce_operations WHERE id=$1")
            .bind(req.request_id)
            .fetch_one(&h.pool)
            .await
            .unwrap();
    assert!(open);
    h.close().await;
}
#[tokio::test]
async fn postgres_grant_second_staff_approval_records_reason_digest() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let service = h.service();
    let req = request(h.target, Command::Grant { days: 30 });
    assert!(
        service
            .submit(context(&h.pool, h.actor).await, req.clone())
            .await
            .unwrap()
            .pending
    );
    let outcome = service
        .approve(context(&h.pool, h.second).await, req.request_id)
        .await
        .unwrap();
    assert!(!outcome.pending);
    assert!(!outcome.replayed);
    assert_eq!(grant_entitlements(&h, req.request_id).await, 1);
    let (actor, days, digest): (Uuid, Option<i32>, Vec<u8>) = sqlx::query_as(
        "SELECT actor_id,duration_days,reason_sha256 FROM premium_ledger WHERE request_id=$1",
    )
    .bind(req.request_id)
    .fetch_one(&h.pool)
    .await
    .unwrap();
    assert_eq!(actor, h.actor);
    assert_eq!(days, Some(30));
    assert_eq!(digest, Sha256::digest(req.reason.as_bytes()).to_vec());
    let approved_by: Option<Uuid> =
        sqlx::query_scalar("SELECT approved_by FROM commerce_operations WHERE id=$1")
            .bind(req.request_id)
            .fetch_one(&h.pool)
            .await
            .unwrap();
    assert_eq!(approved_by, Some(h.second));
    h.close().await;
}
#[tokio::test]
async fn postgres_grant_approval_replay_is_idempotent() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let service = h.service();
    let approver = context(&h.pool, h.second).await;
    let req = request(h.target, Command::Grant { days: 7 });
    assert!(
        service
            .submit(context(&h.pool, h.actor).await, req.clone())
            .await
            .unwrap()
            .pending
    );
    assert!(
        !service
            .approve(approver, req.request_id)
            .await
            .unwrap()
            .replayed
    );
    let replay = service.approve(approver, req.request_id).await.unwrap();
    assert!(replay.replayed);
    assert!(!replay.pending);
    assert_eq!(grant_entitlements(&h, req.request_id).await, 1);
    assert_eq!(h.count("premium_ledger").await, 1);
    h.close().await;
}
#[tokio::test]
async fn postgres_grant_to_value_ineligible_account_refused_at_submit() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    sqlx::query(
        "UPDATE accounts SET email_change_blocked_until=now()+interval '1 day' WHERE id=$1",
    )
    .bind(h.target)
    .execute(&h.pool)
    .await
    .unwrap();
    let req = request(h.target, Command::Grant { days: 30 });
    assert!(matches!(
        h.service()
            .submit(context(&h.pool, h.actor).await, req.clone())
            .await,
        Err(Error::Denied)
    ));
    assert_eq!(h.count("commerce_operations").await, 0);
    assert_eq!(h.count("premium_ledger").await, 0);
    let denied: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action='commerce.denied' AND request_id=$1",
    )
    .bind(req.request_id)
    .fetch_one(&h.pool)
    .await
    .unwrap();
    assert_eq!(denied, 1);
    h.close().await;
}
async fn denied_audits(h: &Harness, request: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action='commerce.denied' AND request_id=$1",
    )
    .bind(request)
    .fetch_one(&h.pool)
    .await
    .unwrap()
}
#[tokio::test]
async fn postgres_grant_replay_after_target_turns_ineligible_reports_recorded_state() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let service = h.service();
    let req = request(h.target, Command::Grant { days: 30 });
    assert!(
        service
            .submit(context(&h.pool, h.actor).await, req.clone())
            .await
            .unwrap()
            .pending
    );
    sqlx::query(
        "UPDATE accounts SET email_change_blocked_until=now()+interval '1 day' WHERE id=$1",
    )
    .bind(h.target)
    .execute(&h.pool)
    .await
    .unwrap();
    for _ in 0..2 {
        let replay = service
            .submit(context(&h.pool, h.actor).await, req.clone())
            .await
            .unwrap();
        assert!(replay.replayed);
        assert!(replay.pending);
    }
    assert_eq!(denied_audits(&h, req.request_id).await, 0);
    assert_eq!(h.count("commerce_operations").await, 1);
    assert_eq!(h.count("premium_ledger").await, 0);
    h.close().await;
}
#[tokio::test]
async fn postgres_grant_approval_audit_failure_rolls_back_entitlement() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let service = h.service();
    let approver = context(&h.pool, h.second).await;
    let req = request(h.target, Command::Grant { days: 30 });
    assert!(
        service
            .submit(context(&h.pool, h.actor).await, req.clone())
            .await
            .unwrap()
            .pending
    );
    sqlx::raw_sql("CREATE FUNCTION refuse_grant_approval_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='commerce.approved' THEN RAISE EXCEPTION 'audit unavailable'; END IF; RETURN NEW; END $$; CREATE TRIGGER refuse_grant_approval_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION refuse_grant_approval_audit();").execute(&h.pool).await.unwrap();
    assert!(matches!(
        service.approve(approver, req.request_id).await,
        Err(Error::Storage(_))
    ));
    assert_eq!(h.count("premium_ledger").await, 0);
    assert_eq!(h.count("premium_audit").await, 0);
    let (open, approved_by): (bool, Option<Uuid>) =
        sqlx::query_as("SELECT result IS NULL,approved_by FROM commerce_operations WHERE id=$1")
            .bind(req.request_id)
            .fetch_one(&h.pool)
            .await
            .unwrap();
    assert!(open);
    assert_eq!(approved_by, None);
    sqlx::raw_sql("DROP TRIGGER refuse_grant_approval_audit ON audit_log; DROP FUNCTION refuse_grant_approval_audit();").execute(&h.pool).await.unwrap();
    let outcome = service.approve(approver, req.request_id).await.unwrap();
    assert!(!outcome.replayed);
    assert_eq!(grant_entitlements(&h, req.request_id).await, 1);
    h.close().await;
}
#[test]
fn reason_digest_decodes_only_a_full_hex_sha256() {
    let hash = format!("{:x}", Sha256::digest(b"Reviewed support request"));
    assert_eq!(
        effects::reason_digest(&hash).ok(),
        Some(<[u8; 32]>::from(Sha256::digest(
            b"Reviewed support request"
        )))
    );
    for bad in [
        String::new(),
        hash[..62].to_owned(),
        format!("{hash}00"),
        format!("+{}", &hash[1..]),
        format!("{}g", &hash[..63]),
        format!("{}é", &hash[..62]),
    ] {
        assert!(
            matches!(effects::reason_digest(&bad), Err(Error::Invalid)),
            "{bad}"
        );
    }
}

async fn login(app: axum::Router) -> (String, String) {
    let response = app
        .oneshot(
            HttpRequest::get("/admin/commerce")
                .header("host", "staff.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let html = String::from_utf8(
        to_bytes(response.into_body(), 200000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned();
    (cookie, csrf)
}

#[tokio::test]
async fn commerce_review_owner_cannot_cancel_refund() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let h = Harness::new().await.expect("PostgreSQL required");
    let payment = Uuid::now_v7();
    sqlx::query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES ($1,$2,'test',$3,250,'PLN','succeeded')")
        .bind(payment).bind(h.second).bind(payment.to_string()).execute(&h.pool).await.unwrap();
    let refund = h
        .service()
        .submit(
            context(&h.pool, h.actor).await,
            request(payment, Command::Refund),
        )
        .await
        .unwrap();
    let app = router(
        h.pool.clone(),
        crate::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Session(h.second, h.pool.clone()),
        Authority(h.pool.clone()),
        h.effects(),
        flags(),
    );
    let id = refund.id;
    let (cookie, csrf) = login(app.clone()).await;
    let response = app
        .oneshot(
            HttpRequest::post(format!("/admin/commerce/cancellations/{id}"))
                .header("host", "staff.test")
                .header("origin", "https://staff.test")
                .header("cookie", cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    h.close().await;
}

#[tokio::test]
async fn commerce_review_displays_refund_amount() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let h = Harness::new().await.expect("PostgreSQL required");
    let actor = context(&h.pool, h.actor).await;
    for currency in ["PLN", "JPY", "KWD"] {
        let payment = Uuid::now_v7();
        sqlx::query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES ($1,$2,'test',$3,250,$4,'succeeded')")
            .bind(payment).bind(h.target).bind(payment.to_string()).bind(currency)
            .execute(&h.pool).await.unwrap();
        h.service()
            .submit(actor, request(payment, Command::Refund))
            .await
            .unwrap();
    }
    let app = router(
        h.pool.clone(),
        crate::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Session(h.second, h.pool.clone()),
        Authority(h.pool.clone()),
        h.effects(),
        flags(),
    );
    for (language, label) in [("en", "Refund amount"), ("pl", "Kwota zwrotu")] {
        let response = app
            .clone()
            .oneshot(
                HttpRequest::get("/admin/commerce")
                    .header("host", "staff.test")
                    .header("accept-language", language)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(
            to_bytes(response.into_body(), 200000)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        for amount in ["2.50 PLN", "250 JPY", "0.250 KWD"] {
            assert!(
                html.contains(&format!("<p>{label}: {amount}</p>")),
                "missing {label}: {amount}"
            );
        }
    }

#[tokio::test]
async fn credited_topup_blocks_refund() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(h) = Harness::new().await else {
        return;
    };
    let (operation, payment) = queued_refund(&h).await;
    sqlx::query("INSERT INTO wallet_accounts(account_id) VALUES($1) ON CONFLICT DO NOTHING")
        .bind(h.target)
        .execute(&h.pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'top_up',$2,100)",
    )
    .bind(payment)
    .bind(h.target)
    .execute(&h.pool)
    .await
    .unwrap();
    let adapter = h.effects().payments;
    let stub = RefundStub::default();
    assert!(
        matches!(
            adapter.deliver_refund_with(&stub, &h.pool, operation).await,
            Err(Error::Conflict)
        ),
        "worker accepted credited intent"
    );
    assert!(stub.keys().is_empty());
    let mut tx = h.pool.begin().await.unwrap();
    let refund = request(payment, Command::Refund);
    assert!(matches!(
        adapter.inspect(&mut tx, &refund).await,
        Err(Error::Conflict)
    ));
    tx.rollback().await.unwrap();
    h.close().await;
}
