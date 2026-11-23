use super::*;

fn pool() -> Result<PgPool, sqlx::Error> {
    sqlx::postgres::PgPoolOptions::new().connect_lazy("postgres://portal@127.0.0.1:1/unreachable")
}
#[tokio::test]
async fn disabled_anonymous_and_bad_locale() -> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    assert_eq!(
        show(&pool()?, None, "en", false, Settings::default())
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        show(&pool()?, None, "en", true, Settings::default())
            .await
            .status(),
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        show(&pool()?, None, "de", true, Settings::default())
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        show(&pool()?, Some(Uuid::nil()), "en", true, Settings::default())
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    Ok(())
}
#[test]
fn translations_and_profile_segments() {
    for lang in ["en", "pl"] {
        assert!(!text(lang, "overview").is_empty());
    }
    assert_eq!(parse_limits("8", "16"), Some((8, 16)));
    assert_eq!(parse_limits("0", "16"), None);
    assert_eq!(parse_limits("8", "7"), None);
    assert_eq!(parse_limits("x", "16"), None);
    assert_eq!(segment("A /?<&"), "A%20%2F%3F%3C%26");
}
#[tokio::test]
async fn postgres_overview_success_and_denial() -> Result<(), Box<dyn std::error::Error>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping PostgreSQL overview test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = sqlx::postgres::PgPoolOptions::new().connect(&url).await?;
    let schema = format!("overview_test_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .after_connect(move |connection, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105155256_t03_accounts.sql"
    ))
    .execute(&pool)
    .await?;
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1,$2,'test-hash','active','en')").bind(id).bind(format!("overview-{id}@example.invalid")).execute(&pool).await?;
    let response = show(&pool, Some(id), "en", true, Settings::default()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let body = axum::body::to_bytes(response.into_body(), 65536).await?;
    let html = String::from_utf8(body.to_vec())?;
    assert!(html.contains("Account overview"));
    assert!(html.contains("href=\"/en/shop/history\""));
    assert!(!html.contains("/panel/balance"));
    assert!(html.contains("Unavailable"));
    assert!(!html.contains("test-hash"));
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105145653_t18_account_characters.sql"
    ))
    .execute(&pool)
    .await?;
    let authority = "oteryn:character-authority:primary";
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority=$1,complete_through=EXTRACT(EPOCH FROM now())::bigint,observed_at=EXTRACT(EPOCH FROM now())::bigint WHERE id=1").bind(authority).execute(&pool).await?;
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,$2,'1','1',0,$3)").bind(id).bind(authority).bind(vec![0u8;32]).execute(&pool).await?;
    let character = Uuid::now_v7();
    let world = Uuid::now_v7();
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Aldric','AVAILABLE')").bind(id).bind(character).bind(world).execute(&pool).await?;
    for migration in [
        include_str!("../../../../migrations/20261105160001_t22_character_commands.sql"),
        include_str!("../../../../migrations/20261105160020_t23_restore_reconciliation.sql"),
        include_str!("../../../../migrations/20261105180000_t24_public_snapshots.sql"),
        include_str!("../../../../migrations/20261105230108_t72_private_journal.sql"),
        include_str!("../../../../migrations/20261112100006_t72_journal_admission.sql"),
        include_str!("../../../../migrations/20261115120019_fix_a2-l8_projection_conflicts.sql"),
    ] {
        sqlx::raw_sql(migration).execute(&pool).await?;
    }
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1 WHERE id=1")
        .execute(&pool)
        .await?;
    seed_creation(&pool, id, character, world).await?;
    let settings = Settings {
        enabled: true,
        environment: oteryn_core::Environment::Testing,
        source_authority: authority.into(),
        ..Settings::default()
    };
    for lang in ["en", "pl"] {
        let response = show(&pool, Some(id), lang, true, settings.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(
            axum::body::to_bytes(response.into_body(), 65536)
                .await?
                .to_vec(),
        )?;
        assert!(html.contains(&format!("href=\"/{lang}/characters/Aldric\"")));
        if lang == "en" {
            assert!(html.contains("Current game connection"));
        }
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
    let candidates = Projection::new(
        pool.clone(),
        settings.clone(),
        Arc::new(Accounts(pool.clone())),
    )
    .list_characters(AccountId::parse(&id.to_string())?, now)
    .await?
    .characters;
    let before = show(&pool, Some(id), "en", true, settings.clone()).await;
    let before = String::from_utf8(
        axum::body::to_bytes(before.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(before.contains("/characters/Aldric"));
    assert!(before.contains("/en/account/characters/Aldric/profile"));
    let profile = serde_json::json!({"profile":[{"name":"Aldric","world_id":world,"level":"412","vocation":4,"guild":[],"guild_visible":false}]});
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES ('character_profile',$1,false,1,1,0,$2), ('character_profile','',true,1,1,floor(extract(epoch FROM now()))::bigint,'{}')")
        .bind(character.to_string()).bind(profile).execute(&pool).await?;
    let facts = progression::load(&pool, id, &candidates, true).await;
    let facts = facts
        .rows
        .get(&character)
        .and_then(Option::as_ref)
        .ok_or("fresh progression missing")?;
    assert_eq!((facts.level, facts.vocation), (412, 4));
    let mut writer = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(22005160000)")
        .execute(&mut *writer)
        .await?;
    let during_write = tokio::time::timeout(
        std::time::Duration::from_secs(6),
        progression::load(&pool, id, &candidates, true),
    )
    .await?;
    assert!(during_write.unproven && during_write.rows.is_empty());
    writer.rollback().await?;
    assert!(
        progression::load(&pool, id, &candidates, true)
            .await
            .rows
            .contains_key(&character)
    );
    assert!(
        progression::load(&pool, Uuid::now_v7(), &candidates, true)
            .await
            .rows
            .is_empty()
    );
    sqlx::query("UPDATE public_snapshots SET watermark=0 WHERE is_watermark")
        .execute(&pool)
        .await?;
    assert!(matches!(
        progression::load(&pool, id, &candidates, true)
            .await
            .rows
            .get(&character),
        Some(None)
    ));
    sqlx::query("UPDATE public_snapshots SET watermark=floor(extract(epoch FROM now()))::bigint WHERE is_watermark")
        .execute(&pool)
        .await?;
    let legacy = Uuid::now_v7();
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Legacy','AVAILABLE')")
        .bind(id).bind(legacy).bind(world).execute(&pool).await?;
    let render = |lang| show(&pool, Some(id), lang, true, settings.clone());
    let before = axum::body::to_bytes(render("en").await.into_body(), 65536).await?;
    let before = std::str::from_utf8(&before)?;
    assert!(before.contains("Some character ownership could not be confirmed"));
    assert!(!before.contains("/characters/Legacy"));
    assert!(!before.contains("Your character roster is empty."));
    assert!(before.contains("/characters/Aldric"));

    sqlx::query("INSERT INTO pd_ownership_baselines(character_id,account_id,world_id,character_revision,from_projection_revision,to_projection_revision,projection_epoch,recovery_generation) VALUES($1,$2,$3,1,1,1,1,1)")
        .bind(legacy).bind(id).bind(world).execute(&pool).await?;
    let verified = axum::body::to_bytes(render("en").await.into_body(), 65536).await?;
    let verified = std::str::from_utf8(&verified)?;
    assert!(verified.contains("/characters/Legacy"));
    assert!(!verified.contains("Some character ownership could not be confirmed"));

    let buyer = Uuid::now_v7();
    let transfer = Uuid::now_v7();
    let mut committed: serde_json::Value =
        serde_json::from_str(include_str!("../commands/fixtures/transfer-receipt.json"))?;
    committed["operation_id"] = serde_json::json!(transfer);
    committed["character_id"] = serde_json::json!(character);
    committed["from_account_id"] = serde_json::json!(id);
    committed["to_account_id"] = serde_json::json!(buyer);
    sqlx::query("INSERT INTO character_commands(id,issuer_decision_id,request_key,command,account_id,target_account_id,binding_json,intent_json,receipt_json,state) VALUES($1,$1,$2,'TransferCharacterOwnership',$3,$4,json_build_object('character_id',$5::text)::text,'',$6,'TERMINAL')")
        .bind(transfer).bind(transfer.to_string()).bind(id).bind(buyer).bind(character.to_string())
        .bind(committed.to_string()).execute(&pool).await?;

    // The committed receipt changes authority before the seller portfolio is republished.
    let projection_owner: Uuid =
        sqlx::query_scalar("SELECT account_id FROM account_characters WHERE character_id=$1")
            .bind(character)
            .fetch_one(&pool)
            .await?;
    assert_eq!(projection_owner, id);
    assert!(
        progression::load(&pool, id, &candidates, true)
            .await
            .rows
            .is_empty()
    );

    assert!(
        progression::load(&pool, id, &candidates, false)
            .await
            .rows
            .is_empty()
    );
    let response = show(&pool, Some(id), "en", true, settings.clone()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(!html.contains("Aldric"));
    assert!(html.contains("Current game connection"));
    assert!(html.contains("1970-01-01 00:00:00 UTC"));
    assert!(html.contains("Ownership transfer"));
    assert!(!html.contains("/services"));
    sqlx::query("UPDATE account_character_feed SET complete_through=0 WHERE id=1")
        .execute(&pool)
        .await?;
    let response = show(&pool, Some(id), "pl", true, settings).await;
    let html = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(!html.contains("Aldric"));
    assert!(html.contains("nieaktualne"));
    assert!(html.contains("1970-01-01 00:00:00 UTC"));
    assert!(html.contains("Przywrócenie postaci"));
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    assert_eq!(
        show(&pool, Some(id), "en", true, Settings::default())
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("DELETE FROM accounts WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    assert_eq!(
        show(&pool, Some(id), "en", true, Settings::default())
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    Ok(())
}

#[tokio::test]
async fn audit_a210_postgres_overview_main_selection_and_removed_ownership()
-> Result<(), Box<dyn std::error::Error>> {
    // Isolate the feature flag from other tests without mutating process globals.
    if std::env::var("OTERYN_CHARACTER_PROFILES_ENABLED").as_deref() != Ok("true") {
        let status = std::process::Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "overview::tests::audit_a210_postgres_overview_main_selection_and_removed_ownership",
                "--nocapture",
            ])
            .env("OTERYN_CHARACTER_PROFILES_ENABLED", "true")
            .env("OTERYN_ACCOUNT_OVERVIEW_ENABLED", "true")
            .env("OTERYN_LCFA_ENABLED", "true")
            .env("OTERYN_ENV", "testing")
            .env("OTERYN_LCFA_SOURCE_AUTHORITY", "oteryn:character-authority:primary")
            .status()?;
        assert!(status.success());
        return Ok(());
    }
    // The re-exec launcher must not retain the child fixture's guard.
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let started = std::time::Instant::now();
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping PostgreSQL profile test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = sqlx::postgres::PgPoolOptions::new().connect(&url).await?;
    let schema = format!("overview_main_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
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
        .connect(&url)
        .await?;
    for sql in [
        include_str!("../../../../migrations/20261105155256_t03_accounts.sql"),
        include_str!("../../../../migrations/20261105160001_t22_character_commands.sql"),
        include_str!("../../../../migrations/20261105160020_t23_restore_reconciliation.sql"),
        include_str!("../../../../migrations/20261105145653_t18_account_characters.sql"),
        include_str!("../../../../migrations/20261105231104_t45_profile_preferences.sql"),
        include_str!("../../../../migrations/20261105220226_t09c_privacy.sql"),
        include_str!("../../../../migrations/20261107130001_t45_privacy.up.sql"),
    ] {
        sqlx::raw_sql(sql).execute(&pool).await?;
    }
    let account = Uuid::now_v7();
    let other = Uuid::now_v7();
    let one = Uuid::now_v7();
    let two = Uuid::now_v7();
    for id in [account, other] {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1,$2,'test-hash','active','en')").bind(id).bind(format!("profile-{id}@example.invalid")).execute(&pool).await?;
        sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'oteryn:character-authority:primary','1','1',1000,$2)").bind(id).bind(vec![0u8;32]).execute(&pool).await?;
    }
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='oteryn:character-authority:primary',complete_through=1000,observed_at=1000 WHERE id=1").execute(&pool).await?;
    for (id, name) in [(one, "Aldric"), (two, "Belric")] {
        let world = Uuid::now_v7();
        seed_creation(&pool, account, id, world).await?;
        sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,$4,'AVAILABLE')").bind(account).bind(id).bind(world).bind(name).execute(&pool).await?;
    }
    sqlx::query("UPDATE accounts SET email_verified_at=now()")
        .execute(&pool)
        .await?;
    let settings = Settings {
        enabled: true,
        environment: oteryn_core::Environment::Testing,
        source_authority: "oteryn:character-authority:primary".into(),
        ..Settings::default()
    };

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
    sqlx::query("UPDATE account_character_feed SET complete_through=$1,observed_at=$1 WHERE id=1")
        .bind(now)
        .execute(&pool)
        .await?;
    use sha2::{Digest, Sha256};
    use tower::ServiceExt;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105144852_t02_web_sessions.sql"
    ))
    .execute(&pool)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105220221_t08_sessions.sql"
    ))
    .execute(&pool)
    .await?;
    let token = "d".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES ($1,$2,$3,'test',now()+interval '30 minutes',now()+interval '1 day')")
        .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(account).bind("e".repeat(64)).execute(&pool).await?;
    let app = oteryn_web::router(oteryn_web::AppState { db: pool.clone() }, "static");
    let service = crate::profile::Service::new(pool.clone(), settings.clone(), true);
    service
        .update(
            account,
            "Aldric",
            crate::profile::Preferences {
                is_main_character: true,
                ..Default::default()
            },
            now,
        )
        .await
        .map_err(|_| "selection failed")?;
    for removed in [false, true] {
        if removed {
            sqlx::query("DELETE FROM account_characters WHERE account_id=$1 AND character_id=$2")
                .bind(account)
                .bind(one)
                .execute(&pool)
                .await?;
        }
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::get("/account")
                    .header("accept-language", "en")
                    .header("cookie", format!("__Host-oteryn_session={token}"))
                    .body(axum::body::Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(
            axum::body::to_bytes(response.into_body(), 65536)
                .await?
                .to_vec(),
        )?;
        assert_eq!(
            html.matches("data-main-character").count(),
            usize::from(!removed)
        );
        assert!(html.contains("/en/account/characters/Belric/profile"));
        assert_eq!(
            html.contains("/en/account/characters/Aldric/profile"),
            !removed
        );
    }
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    eprintln!(
        "audit_a210_postgres_overview_main_selection_and_removed_ownership duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn seed_creation(
    pool: &PgPool,
    id: Uuid,
    character: Uuid,
    world: Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    let create = Uuid::now_v7();
    let mut created: serde_json::Value =
        serde_json::from_str(include_str!("../commands/fixtures/create-receipt.json"))?;
    created["operation_id"] = serde_json::json!(create);
    created["account_id"] = serde_json::json!(id);
    created["character_id"] = serde_json::json!(character);
    created["world_id"] = serde_json::json!(world);
    created["projection_revision"] = serde_json::json!("1");
    sqlx::query("INSERT INTO character_commands(id,issuer_decision_id,request_key,command,account_id,binding_json,intent_json,receipt_json,state) VALUES($1,$1,$2,'CreateCharacter',$3,'{}','',$4,'TERMINAL')")
        .bind(create).bind(create.to_string()).bind(id).bind(created.to_string()).execute(pool).await?;
    Ok(())
}
