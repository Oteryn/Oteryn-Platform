use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

#[derive(Clone)]
struct PausedCodeSession {
    identity: oteryn_admin::web::PortalIdentity,
    authenticated: Arc<tokio::sync::Notify>,
    resume: Arc<tokio::sync::Notify>,
}
impl oteryn_admin::web::StaffSession for PausedCodeSession {
    async fn resolve(&self, session: &BrowserSession) -> Option<oteryn_admin::web::StaffIdentity> {
        let identity = oteryn_admin::web::StaffSession::resolve(&self.identity, session).await?;
        self.authenticated.notify_one();
        self.resume.notified().await;
        Some(identity)
    }
}

#[tokio::test]
async fn db_codes_production_dispatch_issues_redeems_and_refuses_unfulfillable_items()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let admin = PgPool::connect(&url).await?;
    let schema = format!("code_dispatch_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let search = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(12)
        .after_connect(move |c, _| {
            let search = search.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1 || ',pg_catalog',false)")
                    .bind(search)
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    crate::rate_clock::install(&pool).await?;
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,'code-dispatch@example.test',now(),'test-credential-hash','active','en')").bind(actor).execute(&pool).await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).await?;
    // Persist a valid current MFA grant and browser session. Dispatch still runs
    // the real host, session, CSRF, enrollment, generation and permission checks.
    let generation = Uuid::now_v7();
    let grant = oteryn_identity::mfa::random_token()?;
    let cookie = oteryn_identity::mfa::random_token()?;
    let csrf = oteryn_identity::mfa::random_token()?;
    sqlx::query("INSERT INTO account_mfa_totp(account_id,secret,confirmed,generation) VALUES($1,$2,true,$3)").bind(actor).bind(Sha256::digest(grant.as_bytes()).to_vec()).bind(generation).execute(&pool).await?;
    sqlx::query("INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) VALUES($1,$2,$3,$4,'grant',now()+interval '10 minutes')").bind(oteryn_identity::mfa::digest(&grant)).bind(actor).bind(generation).bind(oteryn_identity::mfa::digest("test-credential-hash")).execute(&pool).await?;
    sqlx::query("INSERT INTO web_sessions(id_hash,csrf_token,account_id,data,user_agent_summary,idle_expires_at,absolute_expires_at,authorization_generation) VALUES($1,$2,$3,$4,'code-dispatch-test',now()+interval '10 minutes',now()+interval '1 hour',(SELECT native_security_generation FROM accounts WHERE id=$3))").bind(Sha256::digest(cookie.as_bytes()).to_vec()).bind(&csrf).bind(actor).bind(serde_json::json!({"mfa_grant":grant})).execute(&pool).await?;
    let config = |key: &str| match key {
        "OTERYN_STORE_CODES_ENABLED"
        | "OTERYN_ADMIN_PANEL"
        | "OTERYN_WALLET_ENABLED"
        | "OTERYN_PREMIUM_ENABLED" => Some("true".into()),
        "OTERYN_ADMIN_HOST" => Some("admin.example.test".into()),
        "OTERYN_MFA_KEY" => Some("19".repeat(32)),
        _ => None,
    };
    let app = configured_from(pool.clone(), &config)?;
    let send = |path: &str, body: serde_json::Value| {
        let req = Request::post(path)
            .header("host", "admin.example.test")
            .header("cookie", format!("__Host-oteryn_session={cookie}"))
            .header("x-csrf-token", &csrf)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))?;
        Ok::<_, Box<dyn std::error::Error>>(app.clone().oneshot(req))
    };
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let form = app
        .clone()
        .oneshot(
            Request::get("/en/admin/store/codes")
                .header("host", "admin.example.test")
                .header("cookie", format!("__Host-oteryn_session={cookie}"))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(form.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(form.into_body(), 8192).await?.to_vec())?;
    let request_id = html
        .split_once("name=\"request_id\" value=\"")
        .and_then(|(_, rest)| rest.split_once('"'))
        .ok_or("missing form request ID")?
        .0;
    assert_eq!(Uuid::parse_str(request_id)?.get_version_num(), 7);
    let batch = serde_json::json!({"request_id":request_id,"kind":"coins","amount":7,"count":1,"max_per_code":1,"max_per_account":1,"valid_from":now-60,"valid_until":now+3600,"note":"Dispatch regression"});
    for (lang, message) in [
        ("en", "Check the request and try again."),
        ("pl", "Sprawdź żądanie i spróbuj ponownie."),
    ] {
        let path = format!("/{lang}/admin/store/codes");
        for (field, invalid) in [("note", ""), ("amount", "not a number")] {
            let mut input = batch.clone();
            input[field] = invalid.into();
            let response = send(&path, input)?.await?;
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(response.headers()[header::CONTENT_LANGUAGE], lang);
            let body = to_bytes(response.into_body(), 8192).await?;
            assert_eq!(&body[..], message.as_bytes());
        }
    }
    let identity = oteryn_admin::web::PortalIdentity::configured_from(pool.clone(), &config);
    let authenticated = Arc::new(tokio::sync::Notify::new());
    let resume = Arc::new(tokio::sync::Notify::new());
    let paused = oteryn_admin::web::guarded(
        Router::new()
            .route(GENERATE, post(generate))
            .with_state(context(pool.clone(), true, true)),
        pool.clone(),
        oteryn_admin::web::PanelConfig {
            admin_panel: true,
            host: "admin.example.test".into(),
        },
        PausedCodeSession {
            identity: identity.clone(),
            authenticated: authenticated.clone(),
            resume: resume.clone(),
        },
        identity,
        oteryn_admin::rbac::Permission::GenerateStoreCodes,
    );
    let mut form_data = batch.clone();
    form_data["_csrf"] = csrf.clone().into();
    let issue = Request::post("/en/admin/store/codes")
        .header("host", "admin.example.test")
        .header("origin", "https://admin.example.test")
        .header("cookie", format!("__Host-oteryn_session={cookie}"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(serde_urlencoded::to_string(&form_data)?))?;
    let issue = tokio::spawn(async move { paused.oneshot(issue).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), authenticated.notified()).await?;
    let response = crate::test_staff::race_account_before_fence(&pool, actor, async move {
        resume.notify_one();
        issue.await
    })
    .await
    .map_err(|e| e as Box<dyn std::error::Error>)???;
    assert_eq!(response.status(), StatusCode::OK);
    let result: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await?)?;
    let replay = send("/en/admin/store/codes", batch.clone())?.await?;
    assert_eq!(replay.status(), StatusCode::OK);
    let replay: serde_json::Value =
        serde_json::from_slice(&to_bytes(replay.into_body(), 8192).await?)?;
    assert_eq!(replay["batch_id"], result["batch_id"]);
    assert_eq!(replay["codes"], serde_json::json!([]));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM audit_log WHERE action='store.codes.generated'"
        )
        .fetch_one(&pool)
        .await?,
        1
    );
    let redemption = serde_json::json!({"code":result["codes"][0],"operation":Uuid::now_v7()});
    for _ in 0..2 {
        assert_eq!(
            send("/en/shop/codes/redeem", redemption.clone())?
                .await?
                .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT sum(available_delta)::bigint FROM wallet_entries WHERE account_id=$1"
        )
        .bind(actor)
        .fetch_one(&pool)
        .await?,
        7
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM code_redemptions")
            .fetch_one(&pool)
            .await?,
        1
    );
    let mut item = batch.clone();
    item["kind"] = "item_grant".into();
    assert_eq!(
        send("/en/admin/store/codes", item)?.await?.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM code_game_commands")
            .fetch_one(&pool)
            .await?,
        0
    );
    let mut premium_batch = batch.clone();
    premium_batch["request_id"] = Uuid::now_v7().to_string().into();
    premium_batch["kind"] = "premium_days".into();
    premium_batch["max_per_code"] = 4.into();
    let response = send("/en/admin/store/codes", premium_batch)?.await?;
    assert_eq!(response.status(), StatusCode::OK);
    let premium_code: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await?)?;
    for restriction in [
        "UPDATE accounts SET email_change_blocked_until=now()+interval '1 hour' WHERE id=$1",
        "INSERT INTO account_terminations(account_id,scheduled_for) VALUES($1,now()+interval '2 days')",
        "INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES($1,'restriction','active','Restricted code acquisition',now()-interval '1 hour',now()+interval '1 hour',$1)",
    ] {
        let player = Uuid::now_v7();
        sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,$2,now(),'player-test-credential','active','en')")
            .bind(player).bind(format!("{player}@example.invalid")).execute(&pool).await?;
        let bearer = oteryn_identity::mfa::random_token()?;
        let player_csrf = oteryn_identity::mfa::random_token()?;
        sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at,authorization_generation) SELECT $1,id,$2,'premium-code-browser',now()+interval '1 hour',now()+interval '1 day',native_security_generation FROM accounts WHERE id=$3")
            .bind(Sha256::digest(bearer.as_bytes()).to_vec()).bind(&player_csrf).bind(player).execute(&pool).await?;
        sqlx::query(restriction).bind(player).execute(&pool).await?;
        let before:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM code_redemptions),(SELECT count(*) FROM premium_ledger),(SELECT count(*) FROM audit_log)").fetch_one(&pool).await?;
        let response=app.clone().oneshot(Request::post("/en/shop/codes/redeem")
            .header("host","admin.example.test").header("origin","https://admin.example.test")
            .header("cookie",format!("__Host-oteryn_session={bearer}")).header("x-csrf-token",player_csrf)
            .header("content-type","application/json").body(Body::from(serde_json::json!({"code":premium_code["codes"][0],"operation":Uuid::now_v7()}).to_string()))?).await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let after:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM code_redemptions),(SELECT count(*) FROM premium_ledger),(SELECT count(*) FROM audit_log)").fetch_one(&pool).await?;
        assert_eq!(
            before, after,
            "restricted browser acquisition must have no reward or audit effect"
        );
    }
    sqlx::query("DELETE FROM role_permissions WHERE permission='store.codes.generate'")
        .execute(&pool)
        .await?;
    assert_eq!(
        send("/en/admin/store/codes", batch)?.await?.status(),
        StatusCode::FORBIDDEN
    );
    for expected in [422, 422, 422, 429] {
        assert_eq!(
            send("/en/shop/codes/redeem", serde_json::json!({"unknown":true}))?
                .await?
                .status()
                .as_u16(),
            expected
        );
    }
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    eprintln!(
        "PG db_codes_production_dispatch_issues_redeems_and_refuses_unfulfillable_items duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

#[tokio::test]
async fn a3_l6_02_premium_code_route_refuses_restricted_acquisitions()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let admin = PgPool::connect(&url).await?;
    let schema = format!("code_dispatch_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let search = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(12)
        .after_connect(move |c, _| {
            let search = search.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(search)
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale) VALUES($1,'code-dispatch@example.test',now(),'test-credential-hash','active','en')").bind(actor).execute(&pool).await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).await?;
    // Persist a valid current MFA grant and browser session. Dispatch still runs
    // the real host, session, CSRF, enrollment, generation and permission checks.
    let generation = Uuid::now_v7();
    let grant = oteryn_identity::mfa::random_token()?;
    let cookie = oteryn_identity::mfa::random_token()?;
    let csrf = oteryn_identity::mfa::random_token()?;
    sqlx::query("INSERT INTO account_mfa_totp(account_id,secret,confirmed,generation) VALUES($1,$2,true,$3)").bind(actor).bind(Sha256::digest(grant.as_bytes()).to_vec()).bind(generation).execute(&pool).await?;
    sqlx::query("INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) VALUES($1,$2,$3,$4,'grant',now()+interval '10 minutes')").bind(oteryn_identity::mfa::digest(&grant)).bind(actor).bind(generation).bind(oteryn_identity::mfa::digest("test-credential-hash")).execute(&pool).await?;
    sqlx::query("INSERT INTO web_sessions(id_hash,csrf_token,account_id,data,user_agent_summary,idle_expires_at,absolute_expires_at,authorization_generation) VALUES($1,$2,$3,$4,'code-dispatch-test',now()+interval '10 minutes',now()+interval '1 hour',(SELECT native_security_generation FROM accounts WHERE id=$3))").bind(Sha256::digest(cookie.as_bytes()).to_vec()).bind(&csrf).bind(actor).bind(serde_json::json!({"mfa_grant":grant})).execute(&pool).await?;
    let config = |key: &str| match key {
        "OTERYN_STORE_CODES_ENABLED"
        | "OTERYN_ADMIN_PANEL"
        | "OTERYN_WALLET_ENABLED"
        | "OTERYN_PREMIUM_ENABLED" => Some("true".into()),
        "OTERYN_ADMIN_HOST" => Some("admin.example.test".into()),
        "OTERYN_MFA_KEY" => Some("19".repeat(32)),
        _ => None,
    };
    let app = configured_from(pool.clone(), &config)?;
    let send = |path: &str, body: serde_json::Value| {
        let req = Request::post(path)
            .header("host", "admin.example.test")
            .header("cookie", format!("__Host-oteryn_session={cookie}"))
            .header("x-csrf-token", &csrf)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))?;
        Ok::<_, Box<dyn std::error::Error>>(app.clone().oneshot(req))
    };

    let now = OffsetDateTime::now_utc().unix_timestamp();
    for restriction in ["restriction", "suspension", "termination", "cooldown"] {
        let batch = serde_json::json!({"request_id":Uuid::now_v7(),"kind":"premium_days","amount":7,"count":1,"max_per_code":1,"max_per_account":1,"valid_from":now-60,"valid_until":now+3600,"note":"Acquisition regression"});
        let response = send("/en/admin/store/codes", batch)?.await?;
        assert_eq!(response.status(), StatusCode::OK);
        let generated: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 8192).await?)?;
        match restriction {
            "restriction" | "suspension" => {
                sqlx::query("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES($1,$2,'active','test',now()-interval '1 hour',now()+interval '1 day',$1)").bind(actor).bind(restriction).execute(&pool).await?;
            }
            "termination" => {
                sqlx::query("INSERT INTO account_terminations(account_id,scheduled_for) VALUES($1,now()+interval '14 days')").bind(actor).execute(&pool).await?;
            }
            _ => {
                sqlx::query("UPDATE accounts SET email_change_blocked_until=now()+interval '1 day' WHERE id=$1").bind(actor).execute(&pool).await?;
            }
        }
        let response = send(
            "/en/shop/codes/redeem",
            serde_json::json!({"code":generated["codes"][0],"operation":Uuid::now_v7()}),
        )?
        .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{restriction}");
        let counts: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM premium_ledger),(SELECT count(*) FROM code_redemptions),(SELECT count(*) FROM premium_authority)").fetch_one(&pool).await?;
        assert_eq!(counts, (0, 0, 0), "{restriction}");
        sqlx::query("UPDATE sanctions SET status='revoked' WHERE account_id=$1")
            .bind(actor)
            .execute(&pool)
            .await?;
        sqlx::query("UPDATE account_terminations SET cancelled_at=now() WHERE account_id=$1")
            .bind(actor)
            .execute(&pool)
            .await?;
        sqlx::query("UPDATE accounts SET email_change_blocked_until='-infinity' WHERE id=$1")
            .bind(actor)
            .execute(&pool)
            .await?;
    }
    let batch = serde_json::json!({"request_id":Uuid::now_v7(),"kind":"premium_days","amount":7,"count":1,"max_per_code":1,"max_per_account":1,"valid_from":now-60,"valid_until":now+3600,"note":"Premium flag regression"});
    let response = send("/en/admin/store/codes", batch)?.await?;
    assert_eq!(response.status(), StatusCode::OK);
    let generated: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await?)?;
    let redemption = serde_json::json!({"code":generated["codes"][0],"operation":Uuid::now_v7()});
    let disabled = configured_from(pool.clone(), &|key| {
        if key == "OTERYN_PREMIUM_ENABLED" {
            Some("false".into())
        } else {
            config(key)
        }
    })?;
    let request = Request::post("/en/shop/codes/redeem")
        .header("host", "admin.example.test")
        .header("cookie", format!("__Host-oteryn_session={cookie}"))
        .header("x-csrf-token", &csrf)
        .header("content-type", "application/json")
        .body(Body::from(redemption.to_string()))?;
    assert_eq!(
        disabled.oneshot(request).await?.status(),
        StatusCode::FORBIDDEN
    );
    let effects: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM code_redemptions), (SELECT count(*) FROM premium_ledger)",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(effects, (0, 0));
    sqlx::query("DELETE FROM web_rate_limits WHERE limiter_name='store-code-redeem'")
        .execute(&pool)
        .await?;

    assert_eq!(
        send("/en/shop/codes/redeem", redemption)?.await?.status(),
        StatusCode::OK
    );
    let effects: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM code_redemptions), (SELECT count(*) FROM premium_ledger)",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(effects, (1, 1));
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    eprintln!(
        "PG a3_l6_02_premium_code_route_refuses_restricted_acquisitions {:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
