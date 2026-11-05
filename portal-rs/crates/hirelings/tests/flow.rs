use oteryn_db::outbox::{PgPool, Uuid};
use oteryn_hirelings::commands::{CommandFlags, Commands, DenyCommands, Error, Reject};
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
fn legacy_shape_and_flags_remain_inert() {
    let mut d = destination();
    assert!(d.validate().is_ok());
    d.hours = 0;
    assert!(d.validate().is_err());
    assert!(!Flags::default().enabled && !Flags::default().command_outbox);
}
#[tokio::test]
async fn purchase_and_legacy_mutations_refuse_even_with_all_old_flags() -> Result<()> {
    // Lazy pool proves blocked APIs never consult a database or commercial adapter.
    let pool =
        sqlx::postgres::PgPoolOptions::new().connect_lazy("postgresql://localhost/unused")?;
    let service = Hirelings {
        pool: pool.clone(),
        flags: Flags {
            enabled: true,
            command_outbox: true,
        },
        account_weekly_limit: 1,
        eligibility: Verified,
    };
    let id = Uuid::now_v7();
    assert_eq!(
        service.purchase(id, id, id, Profession::Miner).await,
        Err(Reject::PurchaseBlocked)
    );
    let error = service
        .start(id, id, id, &destination())
        .await
        .err()
        .ok_or("legacy start accepted")?;
    assert_eq!(
        error.downcast_ref::<Reject>(),
        Some(&Reject::LegacyBoundaryUnavailable)
    );
    let error = service
        .claim(id, id)
        .await
        .err()
        .ok_or("legacy claim accepted")?;
    assert_eq!(
        error.downcast_ref::<Reject>(),
        Some(&Reject::LegacyBoundaryUnavailable)
    );
    let command = Commands {
        pool,
        flags: CommandFlags::default(),
        source_authority: String::new(),
        authority: DenyCommands,
    };
    let input: oteryn_hirelings::commands::CommandRequest = serde_json::from_value(
        serde_json::json!({
            "idempotency_key": id, "character_id": id, "world_id": id, "hireling_id": id,
            "expected_character_revision": "12", "expected_hireling_revision": "3",
            "action": {"command": "StartHirelingExpedition", "binding": {"reward_table_key": "home-v1", "reward_table_revision": "7", "reward_row_key": "miner-1-short"}}
        }),
    )?;
    assert!(matches!(
        command.enqueue(id, &input).await,
        Err(Error::Rejected(Reject::Disabled))
    ));
    Ok(())
}
#[tokio::test]
async fn postgres_read_only_page_authentication_and_localization() -> Result<()> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("skipping PostgreSQL hireling page test: PORTAL_TEST_DATABASE_URL unset");
        return Ok(());
    };
    let admin = PgPool::connect(&url).await?;
    let schema = format!("t92_view_{}", Uuid::now_v7().simple());
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
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let account = Uuid::now_v7();
    let flags = Flags {
        enabled: true,
        command_outbox: false,
    };
    let app = web::router(pool.clone(), flags);
    let path = "/pl/panel/hirelings";
    assert_eq!(
        app.clone()
            .oneshot(Request::builder().uri(path).body(Body::empty())?)
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        web::router(pool.clone(), Flags::default())
            .oneshot(Request::builder().uri(path).body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,'helper@example.invalid',now(),'test-only','active','pl')").bind(account).execute(&pool).await?;
    // Existing read-model fixture only; purchase is never used to mint an identity.
    sqlx::query("INSERT INTO hirelings(id,account_id,house_id,entitlement_id,profession,slot) VALUES($1,$2,$3,$4,'miner',1)")
        .bind(Uuid::now_v7()).bind(account).bind(Uuid::now_v7()).bind(Uuid::now_v7()).execute(&pool).await?;
    let token = "a".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,csrf_token,account_id,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES(sha256(convert_to($1,'UTF8')),repeat('b',64),$2,'test-agent',now()+interval '1 hour',now()+interval '1 hour')").bind(&token).bind(account).execute(&pool).await?;
    let request = |uri: &str| {
        Request::builder()
            .uri(uri)
            .header("cookie", format!("__Host-oteryn_session={token}"))
            .body(Body::empty())
    };
    for (path, title) in [
        (path, "Pomocnicy w domu"),
        ("/en/panel/hirelings", "House helpers"),
    ] {
        let response = app.clone().oneshot(request(path)?).await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        let html = String::from_utf8(to_bytes(response.into_body(), 100_000).await?.to_vec())?;
        assert!(html.contains(title));
        assert!(!html.contains("helper@example.invalid"));
    }
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
