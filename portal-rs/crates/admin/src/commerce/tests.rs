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
struct Session(Uuid);
impl crate::web::StaffSession for Session {
    async fn resolve(
        &self,
        _: &oteryn_web::session::BrowserSession,
    ) -> Option<crate::web::StaffIdentity> {
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
fn context(account: Uuid) -> rbac::CheckContext {
    rbac::CheckContext {
        account,
        require_staff_mfa: true,
        mfa_verified: true,
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
    let Some(h) = Harness::new().await else {
        return;
    };
    let service = h.service();
    let ctx = context(h.actor);
    let no_mfa = rbac::CheckContext {
        mfa_verified: false,
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
    assert!(!service.submit(ctx, req.clone()).await.unwrap().pending);
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
            .submit(context(h.target), request(h.target, Command::Revoke))
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
    let Some(h) = Harness::new().await else {
        return;
    };
    let s = h.service();
    let ctx = context(h.actor);
    let approver = context(h.second);
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
    let Some(h) = Harness::new().await else {
        return;
    };
    let s = h.service();
    let ctx = context(h.actor);
    let approver = context(h.second);
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
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused@127.0.0.1:1/unused")
        .unwrap();
    let app = router(
        pool.clone(),
        crate::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Session(Uuid::now_v7()),
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
            context(Uuid::now_v7()),
            request(Uuid::now_v7(), Command::Refund)
        )
        .await,
        Err(Error::Disabled)
    ));
    assert!(serde_json::from_str::<Command>(r#"{"kind":"grant","days":2,"extra":true}"#).is_err());
}
#[tokio::test]
async fn postgres_http_forms_localization_csrf_permissions_and_input() {
    let Some(h) = Harness::new().await else {
        return;
    };
    let app = router(
        h.pool.clone(),
        crate::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Session(h.actor),
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
            HttpRequest::get("/admin/commerce")
                .header("host", "staff.test")
                .header("accept-language", "pl")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
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
    let Some(h) = Harness::new().await else {
        return;
    };
    sqlx::raw_sql("CREATE FUNCTION refuse_commerce_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='commerce.requested' THEN RAISE EXCEPTION 'audit unavailable'; END IF; RETURN NEW; END $$; CREATE TRIGGER refuse_commerce_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION refuse_commerce_audit();").execute(&h.pool).await.unwrap();
    assert!(matches!(
        h.service()
            .submit(
                context(h.actor),
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
            let adjustment = service.submit(context(h.actor), request(h.target, Command::Adjust { amount: 50 })).await.unwrap();
            service.approve(context(h.second), adjustment.id).await.unwrap();
            let payment = Uuid::now_v7();
            sqlx::query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES ($1,$2,'test',$3,100,'PLN','succeeded')")
                .bind(payment).bind(h.target).bind(format!("test:{payment}")).execute(&h.pool).await.unwrap();
            service.submit(context(h.actor), request(payment, Command::Reconcile)).await.unwrap();
            let refund = service.submit(context(h.actor), request(payment, Command::Refund)).await.unwrap();
            service.approve(context(h.second), refund.id).await.unwrap();
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
            tasks.spawn(async move {
                barrier.wait().await;
                service
                    .submit(context(actor), request(target, Command::Grant { days: 1 }))
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
            Session(actor),
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
        .oneshot(
            HttpRequest::post(format!("/admin/commerce/approvals/{adjustment_id}"))
                .header("host", "staff.test")
                .header("origin", "https://staff.test")
                .header("cookie", second_cookie)
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
    let started = std::time::Instant::now();
    let h = Harness::new().await.expect("PostgreSQL required");
    let service = h.service();
    let pending = service
        .submit(
            context(h.actor),
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
                context(h.actor),
                request(h.target, Command::Grant { days: 1 })
            )
        ),
        tokio::time::timeout(
            std::time::Duration::from_secs(4),
            service.approve(context(h.second), pending.id)
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
            Session(actor),
            Authority(h.pool.clone()),
            rbac::Permission::Access,
        )
    };
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
