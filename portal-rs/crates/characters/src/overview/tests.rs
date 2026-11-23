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
    for lang in ["en", "pl"] {
        for key in [
            "verified-through",
            "profile-observed",
            "roster-empty",
            "game-history",
            "display-settings",
            "display-name",
            "display-locale",
            "display-unset",
            "display-en",
            "display-pl",
            "display-policy",
            "display-invalid",
            "display-denied",
            "display-save",
            "display-saved",
            "display-back",
        ] {
            assert!(!text(lang, key).is_empty(), "{lang} {key}");
        }
        for code in 0..=5 {
            assert!(
                game::vocation_key(&code.to_string())
                    .is_some_and(|key| !text(lang, key).is_empty())
            );
        }
    }
    assert!(game::vocation_key("255").is_none());
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
    for sql in [
        include_str!("../../../../migrations/20261105155256_t03_accounts.sql"),
        include_str!("../../../../migrations/20261105220202_t04_login_guard.sql"),
        include_str!("../../../../migrations/20261105220225_t09b_account_terminations.sql"),
        include_str!("../../../../migrations/20261105144852_t02_web_sessions.sql"),
        include_str!("../../../../migrations/20261105220221_t08_sessions.sql"),
        include_str!("../../../../migrations/20261105145653_t18_account_characters.sql"),
        include_str!("../../../../migrations/20261124100800_gap-11_player_account_edits.sql"),
        include_str!("../../../../migrations/20261105180000_t24_public_snapshots.sql"),
        include_str!("../../../../migrations/20261115120019_fix_a2-l8_projection_conflicts.sql"),
        include_str!("../../../../migrations/20261105231104_t45_profile_preferences.sql"),
        include_str!("../../../../migrations/20261105220226_t09c_privacy.sql"),
        include_str!("../../../../migrations/20261107130001_t45_privacy.up.sql"),
    ] {
        sqlx::raw_sql(sql).execute(&pool).await?;
    }
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1,$2,'test-hash','active','en')").bind(id).bind(format!("overview-{id}@example.invalid")).execute(&pool).await?;
    let other = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1,$2,'test-hash','active','en')")
        .bind(other).bind(format!("overview-{other}@example.invalid")).execute(&pool).await?;
    sqlx::query(
        "INSERT INTO security_events(account_id,kind,created_at) VALUES
                 ($1,'login_succeeded','2026-10-01 12:34:56+00'),
                 ($1,'session_revoked','2026-10-02 12:34:56+00'),
                 ($2,'login_succeeded','2026-10-03 12:34:56+00')",
    )
    .bind(id)
    .bind(other)
    .execute(&pool)
    .await?;
    let response = show(&pool, Some(id), "en", true, Settings::default()).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let body = axum::body::to_bytes(response.into_body(), 65536).await?;
    let html = String::from_utf8(body.to_vec())?;
    assert!(html.contains("Account overview"));
    assert!(html.contains("href=\"/en/account/display\""));
    assert!(html.contains("href=\"/en/shop/history\""));
    assert!(!html.contains("/panel/balance"));
    assert!(html.contains("Unavailable"));
    assert!(!html.contains("test-hash"));
    assert!(html.contains("No closure scheduled"));
    assert!(html.contains("2026-10-01 12:34:56 UTC"));
    assert!(!html.contains("2026-10-02 12:34:56 UTC"));
    assert!(!html.contains("2026-10-03 12:34:56 UTC"));
    display_round_trip(&pool, id, other).await?;
    sqlx::query("INSERT INTO account_terminations(account_id,scheduled_for) VALUES($1,now()+interval '14 days')")
        .bind(id)
        .execute(&pool)
        .await?;
    let pending = show(&pool, Some(id), "en", true, Settings::default()).await;
    assert_eq!(pending.status(), StatusCode::OK);
    let pending = String::from_utf8(
        axum::body::to_bytes(pending.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(pending.contains("Pending account closure"));
    assert!(pending.contains("href=\"/en/account/privacy/delete\""));
    sqlx::query("DELETE FROM account_terminations WHERE account_id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    let authority = "oteryn:character-authority:primary";
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority=$1,complete_through=EXTRACT(EPOCH FROM now())::bigint,observed_at=EXTRACT(EPOCH FROM now())::bigint WHERE id=1").bind(authority).execute(&pool).await?;
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,$2,'1','1',0,$3)").bind(id).bind(authority).bind(vec![0u8;32]).execute(&pool).await?;
    let character = Uuid::now_v7();
    let world = Uuid::now_v7();
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Aldric','AVAILABLE')").bind(id).bind(character).bind(world).execute(&pool).await?;
    sqlx::query("UPDATE public_projection_state SET highest_epoch=7 WHERE id=1")
        .execute(&pool)
        .await?;
    sqlx::query(
        "INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload)
         VALUES ('character_profile','',true,7,1,floor(extract(epoch FROM now())), jsonb_build_object('source_authority',$1::text)),
                ('character_profile',$2,false,7,2,floor(extract(epoch FROM now())),
                 jsonb_build_object('source_authority',$1::text,'character_id',$2::text,'profile',
                   jsonb_build_array(jsonb_build_object('world_id',$3::text,'name','Aldric','level','412','vocation',4))))",
    ).bind(authority).bind(character.to_string()).bind(world.to_string()).execute(&pool).await?;
    let settings = Settings {
        enabled: true,
        environment: oteryn_core::Environment::Testing,
        source_authority: authority.into(),
        ..Settings::default()
    };
    let response = load(
        &pool,
        Some(id),
        "en",
        true,
        settings.clone(),
        Some(authority),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(html.contains("/characters/Aldric"));
    assert!(html.contains("Current game connection"));
    assert!(html.contains("data-roster-verified"));
    assert!(html.contains("<dd>412</dd>"));
    assert!(html.contains("<dd>Paladin</dd>"));
    assert!(html.contains("Profile observed"));
    assert!(
        game::profiles(&pool, other, &settings, authority)
            .await?
            .is_empty()
    );
    assert!(
        game::profiles(&pool, id, &settings, "other-source")
            .await?
            .is_empty()
    );
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Aldric','AVAILABLE')")
        .bind(other).bind(character).bind(world).execute(&pool).await?;
    let ambiguous = load(
        &pool,
        Some(id),
        "en",
        true,
        settings.clone(),
        Some(authority),
    )
    .await;
    let ambiguous = String::from_utf8(
        axum::body::to_bytes(ambiguous.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(ambiguous.contains("Game connection could not be verified"));
    assert!(!ambiguous.contains("Aldric"));
    sqlx::query("DELETE FROM account_characters WHERE account_id=$1")
        .bind(other)
        .execute(&pool)
        .await?;
    sqlx::query("UPDATE public_snapshots SET watermark=0 WHERE NOT is_watermark")
        .execute(&pool)
        .await?;
    assert!(
        game::profiles(&pool, id, &settings, authority)
            .await?
            .is_empty()
    );
    let stale_profile = load(
        &pool,
        Some(id),
        "en",
        true,
        settings.clone(),
        Some(authority),
    )
    .await;
    let stale_profile = String::from_utf8(
        axum::body::to_bytes(stale_profile.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(stale_profile.contains("data-roster-verified"));
    assert!(stale_profile.contains("<dt>Level</dt><dd>Unavailable</dd>"));
    assert!(!stale_profile.contains("Profile observed"));
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
    assert!(!html.contains("data-roster-verified"));
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
    sqlx::query("DELETE FROM security_events WHERE account_id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM account_privacy_events WHERE account_id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
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
        include_str!("../../../../migrations/20261105220202_t04_login_guard.sql"),
        include_str!("../../../../migrations/20261105220225_t09b_account_terminations.sql"),
        include_str!("../../../../migrations/20261105160001_t22_character_commands.sql"),
        include_str!("../../../../migrations/20261105145653_t18_account_characters.sql"),
        include_str!("../../../../migrations/20261105231104_t45_profile_preferences.sql"),
        include_str!("../../../../migrations/20261105220226_t09c_privacy.sql"),
        include_str!("../../../../migrations/20261107130001_t45_privacy.up.sql"),
        include_str!("../../../../migrations/20261124100800_gap-11_player_account_edits.sql"),
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
        sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,$4,'AVAILABLE')").bind(account).bind(id).bind(Uuid::now_v7()).bind(name).execute(&pool).await?;
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

async fn display_round_trip(
    pool: &PgPool,
    owner: Uuid,
    other: Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    use axum::{body::Body, http::Request};
    use sha2::{Digest, Sha256};
    use tower::ServiceExt;

    let token = "a".repeat(64);
    let csrf = "b".repeat(64);
    sqlx::query("UPDATE accounts SET email_verified_at=now() WHERE id=$1")
        .bind(owner)
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES ($1,$2,$3,'test',now()+interval '30 minutes',now()+interval '1 day')")
        .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(owner).bind(&csrf).execute(pool).await?;
    let app = oteryn_web::router(oteryn_web::AppState { db: pool.clone() }, "static");
    let href = "/en/account/display";
    let cookie = format!("__Host-oteryn_session={token}");
    let request = |body| {
        Request::post(href)
            .header(header::COOKIE, &cookie)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(body))
    };
    let anonymous = app
        .clone()
        .oneshot(Request::get(href).body(Body::empty())?)
        .await?;
    assert_eq!(anonymous.status(), StatusCode::SEE_OTHER);
    let initial = app
        .clone()
        .oneshot(
            Request::get(href)
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(initial.status(), StatusCode::OK);
    let saved = app
        .clone()
        .oneshot(request(format!(
            "_csrf={csrf}&display_name=%20%3Cb%3EAlias%3C%2Fb%3E%20&locale=pl"
        ))?)
        .await?;
    assert_eq!(saved.status(), StatusCode::OK);
    let confirmed = String::from_utf8(
        axum::body::to_bytes(saved.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(confirmed.contains("Display settings saved."));
    assert!(confirmed.contains("value=\"pl\" selected"));
    let current = app
        .clone()
        .oneshot(
            Request::get(format!("{href}?saved=1"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    let html = String::from_utf8(
        axum::body::to_bytes(current.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(!html.contains("Display settings saved."));
    assert!(!html.contains("<b>Alias</b>"));
    assert!(html.contains("Alias"));
    assert!(html.contains("value=\"pl\" selected"));
    let read = |id| {
        sqlx::query_as::<_, (String, String, i64)>(
            "SELECT display_name,locale,privacy_revision FROM accounts WHERE id=$1",
        )
        .bind(id)
        .fetch_one(pool)
    };
    assert_eq!(read(owner).await?, ("<b>Alias</b>".into(), "pl".into(), 1));
    assert_eq!(read(other).await?, (String::new(), "en".into(), 0));
    assert_eq!(
        app.clone()
            .oneshot(request("display_name=No&locale=en".into())?)
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    for form in [
        format!("_csrf={csrf}&display_name={}&locale=en", "x".repeat(81)),
        format!("_csrf={csrf}&display_name=Invalid&locale=de"),
        format!("_csrf={csrf}&display_name=Invalid%00&locale=en"),
        format!("_csrf={csrf}&display_name=One&display_name=Two&locale=en"),
        format!("_csrf={csrf}&display_name=Invalid&locale=en&account_id={other}"),
    ] {
        let response = app.clone().oneshot(request(form)?).await?;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let html = String::from_utf8(
            axum::body::to_bytes(response.into_body(), 65536)
                .await?
                .to_vec(),
        )?;
        assert!(html.contains(r#"<p role="alert">Enter a display name"#));
        assert!(html.contains(r#"<form method="post" action="/en/account/display""#));
    }
    sqlx::query("ALTER TABLE account_privacy_events ADD CHECK (revision < 2)")
        .execute(pool)
        .await?;
    let failed = app
        .clone()
        .oneshot(request(format!(
            "_csrf={csrf}&display_name=Changed&locale=en"
        ))?)
        .await?;
    assert_eq!(failed.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(read(owner).await?, ("<b>Alias</b>".into(), "pl".into(), 1));
    sqlx::query("INSERT INTO account_terminations(account_id,scheduled_for) VALUES($1,now()+interval '14 days')")
        .bind(owner).execute(pool).await?;
    let refused = app
        .oneshot(request(format!(
            "_csrf={csrf}&display_name=Refused&locale=en"
        ))?)
        .await?;
    assert_eq!(refused.status(), StatusCode::FORBIDDEN);
    assert_eq!(read(owner).await?, ("<b>Alias</b>".into(), "pl".into(), 1));
    sqlx::query("DELETE FROM account_terminations WHERE account_id=$1")
        .bind(owner)
        .execute(pool)
        .await?;
    Ok(())
}
