use oteryn_db::outbox::{PgPool, Uuid};
use oteryn_hirelings::*;

#[derive(Debug)]
struct Verified;
impl Eligibility for Verified {
    fn active_account(&self, _: Uuid) -> bool {
        true
    }
    fn allowed(&self, _: Uuid, _: Uuid, _: Uuid, _: Profession) -> bool {
        true
    }
}
#[derive(Debug)]
struct Accounts;
impl oteryn_notify::AccountState for Accounts {
    fn active_verified(
        &self,
        _: Uuid,
    ) -> oteryn_notify::NotifyFuture<'_, Option<oteryn_notify::Account>> {
        Box::pin(async {
            Ok(Some(oteryn_notify::Account {
                email: "helper@example.invalid".into(),
                locale: oteryn_notify::Locale::Pl,
            }))
        })
    }
}
#[derive(Debug)]
struct VerifiedReceipt;
impl ReceiptVerifier for VerifiedReceipt {
    fn returned(&self, _: Uuid, _: Uuid, _: Uuid) -> bool {
        true
    }
}
fn destination() -> Destination {
    Destination {
        key: "test-mine".into(),
        profession: Profession::Miner,
        hours: 2,
        weekly_limit: 1,
        rewards: vec![Reward {
            item: "test-ore".into(),
            minimum: 1,
            maximum: 2,
            chance_ppm: 500_000,
        }],
    }
}
#[test]
fn validates_explicit_rewards_and_duration() {
    let mut d = destination();
    assert!(d.validate().is_ok());
    d.hours = 0;
    assert!(d.validate().is_err());
    d.hours = 12;
    d.rewards[0].chance_ppm = 1_000_001;
    assert!(d.validate().is_err());
    assert!(!Flags::default().enabled);
    assert!(!Flags::default().command_outbox);
}
#[tokio::test]
async fn postgres_entitlement_limits_isolation_replay_and_outbox() -> Result<()> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("skipping PostgreSQL hireling test: PORTAL_TEST_DATABASE_URL unset");
        return Ok(());
    };
    let admin = PgPool::connect(&url).await?;
    let schema = format!("t92_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let schema_copy = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .after_connect(move |connection, _| {
            let sql = format!("SET search_path TO {schema_copy}");
            Box::pin(async move {
                sqlx::query(sqlx::AssertSqlSafe(sql))
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105220251_t92_hirelings.sql"
    ))
    .execute(&pool)
    .await?;
    // Shared outbox migration, exercised without the unrelated account tables.
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105120000_t37_outbox.sql"
    ))
    .execute(&pool)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105130000_t36_notifications.sql"
    ))
    .execute(&pool)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105144852_t02_web_sessions.sql"
    ))
    .execute(&pool)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105155256_t03_accounts.sql"
    ))
    .execute(&pool)
    .await?;
    let mut service = Hirelings {
        pool: pool.clone(),
        flags: Flags::default(),
        account_weekly_limit: 1,
        eligibility: Verified,
    };
    let account = Uuid::now_v7();
    let house = Uuid::now_v7();
    let entitlement = Uuid::now_v7();
    assert!(
        service
            .purchase(account, house, entitlement, Profession::Miner)
            .await
            .is_err()
    );
    service.flags.enabled = true;
    assert!(
        service
            .purchase(account, house, Uuid::nil(), Profession::Miner)
            .await
            .is_err()
    );
    let id = service
        .purchase(account, house, entitlement, Profession::Miner)
        .await?;
    assert_eq!(
        id,
        service
            .purchase(account, house, entitlement, Profession::Miner)
            .await?
    );
    assert!(
        service
            .purchase(Uuid::now_v7(), house, entitlement, Profession::Miner)
            .await
            .is_err()
    );
    let second = service
        .purchase(account, house, Uuid::now_v7(), Profession::Miner)
        .await?;
    service
        .purchase(account, house, Uuid::now_v7(), Profession::Fisher)
        .await?;
    assert!(
        service
            .purchase(account, house, Uuid::now_v7(), Profession::Miner)
            .await
            .is_err()
    );
    let request = Uuid::now_v7();
    let d = destination();
    assert!(service.start(account, id, request, &d).await.is_err());
    service.flags.command_outbox = true;
    let mut wrong_cap = d.clone();
    wrong_cap.weekly_limit = 2;
    assert!(
        service
            .start(account, id, request, &wrong_cap)
            .await
            .is_err()
    );
    assert!(
        service
            .start(Uuid::now_v7(), id, request, &d)
            .await
            .is_err()
    );
    service.start(account, id, request, &d).await?;
    assert_eq!(request, service.start(account, id, request, &d).await?);
    assert!(
        service
            .start(account, second, Uuid::now_v7(), &d)
            .await
            .is_err()
    );
    let mut changed = d.clone();
    changed.hours = 3;
    assert!(service.start(account, id, request, &changed).await.is_err());
    assert_eq!(service.state(account).await?.len(), 3);
    assert!(service.history(Uuid::now_v7()).await?.is_empty());
    let history = service.history(account).await?;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].state, "pending");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM outbox")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);
    let notifications = oteryn_notify::Notifications {
        pool: pool.clone(),
        accounts: std::sync::Arc::new(Accounts),
        unsubscribe: oteryn_notify::Unsubscribe::new(
            vec![42; 32],
            "https://example.invalid/unsubscribe".into(),
        )?,
    };
    assert!(service.claim(account, request).await.is_err());
    let receipt = Uuid::now_v7();
    assert!(
        service
            .record_return(account, request, receipt, &DenyReceipts, &notifications)
            .await
            .is_err()
    );
    service
        .record_return(account, request, receipt, &VerifiedReceipt, &notifications)
        .await?;
    service
        .record_return(account, request, receipt, &VerifiedReceipt, &notifications)
        .await?;
    assert!(
        service
            .record_return(
                account,
                request,
                Uuid::now_v7(),
                &VerifiedReceipt,
                &notifications
            )
            .await
            .is_err()
    );
    let notices: i64 = sqlx::query_scalar("SELECT count(*) FROM notifications WHERE account_id=$1")
        .bind(account)
        .fetch_one(&pool)
        .await?;
    assert_eq!(notices, 1);
    let claim = service.claim(account, request).await?;
    assert_eq!(claim, service.claim(account, request).await?);
    assert!(service.claim(Uuid::now_v7(), request).await.is_err());
    let denied = Hirelings {
        pool: pool.clone(),
        flags: service.flags,
        account_weekly_limit: 1,
        eligibility: DenyEligibility,
    };
    assert!(
        denied
            .purchase(account, house, entitlement, Profession::Miner)
            .await
            .is_err()
    );
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let path = "/pl/panel/hirelings";
    let app = oteryn_hirelings::web::router(pool.clone(), service.flags);
    let response = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let off = oteryn_hirelings::web::router(pool.clone(), Flags::default());
    assert_eq!(
        off.oneshot(Request::builder().uri(path).body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,'helper@example.invalid',now(),'test-only','active','pl')").bind(account).execute(&pool).await?;
    let token = "a".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,csrf_token,account_id,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES(sha256(convert_to($1,'UTF8')),repeat('b',64),$2,'test-agent',now()+interval '1 hour',now()+interval '1 hour')").bind(&token).bind(account).execute(&pool).await?;
    let request = |uri: &str| {
        Request::builder()
            .uri(uri)
            .header("cookie", format!("__Host-oteryn_session={token}"))
            .body(Body::empty())
    };
    let response = app.clone().oneshot(request(path)?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let html = String::from_utf8(to_bytes(response.into_body(), 100_000).await?.to_vec())?;
    assert!(html.contains("Pomocnicy w domu"));
    assert!(html.contains("Górnik"));
    assert!(html.contains("Powrócił"));
    assert!(!html.contains("helper@example.invalid"));
    assert_eq!(
        app.clone()
            .oneshot(request("/invalid/panel/hirelings")?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(account)
        .execute(&pool)
        .await?;
    assert_eq!(
        app.oneshot(request(path)?).await?.status(),
        StatusCode::FORBIDDEN
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    Ok(())
}
