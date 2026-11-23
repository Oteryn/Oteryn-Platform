#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_db::outbox::{self, Uuid};
use oteryn_gameauth::oauth;
use oteryn_player_api::account_feed::*;
use std::sync::Arc;
use tower::ServiceExt;
#[derive(Debug)]
struct Tokens(Uuid);
impl BearerAuthority for Tokens {
    fn owner<'a>(&'a self, token: &'a str) -> ReadFuture<'a, Option<Uuid>> {
        Box::pin(async move { Ok((token == "test-access").then_some(self.0)) })
    }
}
#[derive(Debug)]
struct Reads;
impl PlayerReads for Reads {
    fn summary(&self, owner: Uuid) -> ReadFuture<'_, Summary> {
        Box::pin(async move {
            Ok(Summary {
                account_id: owner,
                locale: "en".into(),
                coins: None,
            })
        })
    }
    fn feed(&self, _: Uuid, _: i64, _: bool) -> ReadFuture<'_, Vec<FeedItem>> {
        Box::pin(async { Ok(vec![]) })
    }
}
fn app() -> axum::Router {
    router(Api {
        enabled: true,
        authority: Arc::new(Tokens(Uuid::now_v7())),
        reads: Arc::new(Reads),
    })
}
async fn response(app: axum::Router, path: &str, token: &str) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .uri(path)
            .header("Authorization", token)
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn success_refusal_and_bad_input() {
    for path in ["/api/v1/account/summary", "/api/v1/feed"] {
        let result = response(app(), path, "Bearer test-access").await;
        assert_eq!(result.status(), StatusCode::OK);
        assert_eq!(result.headers()["cache-control"], "no-store, private");
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(result.into_body(), 16384).await.unwrap()).unwrap();
        assert!(!body.to_string().contains("email"));
        assert_eq!(
            response(app(), path, "Bearer wrong").await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    for path in [
        "/api/v1/feed?limit=0",
        "/api/v1/feed?limit=101",
        "/api/v1/feed?limit=wat",
        "/api/v1/feed?limit=",
        "/api/v1/feed?limit=1&limit=2",
        "/api/v1/feed?account_id=other",
        "/api/v1/feed?unread_only=wat",
        "/api/v1/feed?unread_only=false&unread_only=true",
        "/api/v1/account/summary?account_id=other",
    ] {
        assert_eq!(
            response(app(), path, "Bearer test-access").await.status(),
            StatusCode::BAD_REQUEST
        );
    }
    for token in ["Basic test-access", "Bearer ", "Bearer test-access extra"] {
        assert_eq!(
            response(app(), "/api/v1/feed", token).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let request = Request::builder()
        .uri("/api/v1/feed")
        .header("Authorization", "Bearer test-access")
        .header("Authorization", "Bearer test-access")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app().oneshot(request).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
}
async fn grant(
    pool: &outbox::PgPool,
    config: &oauth::Config,
    owner: Uuid,
    scope: &str,
) -> oauth::Tokens {
    // RFC 7636 Appendix B PKCE parameters.
    let request = oauth::Authorization {
        response_type: "code".into(),
        client_id: "account-read-client".into(),
        redirect_uri: "http://127.0.0.1:12345/callback".into(),
        scope: scope.into(),
        state: "account-read-test-state".into(),
        code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".into(),
        code_challenge_method: "S256".into(),
    };
    let code = oauth::authorize(pool, config, owner, &request)
        .await
        .unwrap();
    oauth::exchange(
        pool,
        config,
        &oauth::TokenRequest {
            grant_type: "authorization_code".into(),
            client_id: request.client_id,
            redirect_uri: request.redirect_uri,
            code,
            code_verifier: "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn postgres_private_reads() {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return;
    };
    let admin = outbox::PgPoolOptions::new().connect(&url).await.unwrap();
    let schema = format!("t36b_{}", Uuid::now_v7().simple());
    outbox::query(outbox::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await
        .unwrap();
    let path = schema.clone();
    let pool = outbox::PgPoolOptions::new()
        .after_connect(move |conn, _| {
            let path = path.clone();
            Box::pin(async move {
                outbox::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let owner = Uuid::now_v7();
    let other = Uuid::now_v7();
    for (account, email) in [
        (owner, "owner@example.invalid"),
        (other, "other@example.invalid"),
    ] {
        outbox::query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale) VALUES($1,$2,'unused','active',now(),'en')")
            .bind(account).bind(email).execute(&pool).await.unwrap();
    }
    let id = Uuid::now_v7();
    for (account, notification) in [(owner, id), (other, Uuid::now_v7())] {
        outbox::query("INSERT INTO notifications(id,account_id,category,subject,body) VALUES($1,$2,'security','Subject','Body')").bind(notification).bind(account).execute(&pool).await.unwrap();
    }
    let reads = PrivateReads {
        pool: pool.clone(),
        money_enabled: false,
        wallet: None,
    };
    let config = oauth::Config {
        enabled: true,
        clients: vec![oauth::Client {
            client_id: "account-read-client".into(),
            redirect_uris: vec!["http://127.0.0.1:12345/callback".into()],
        }],
    };
    let app = configured_router(pool.clone(), config.clone());
    let tokens = grant(&pool, &config, owner, oauth::ACCOUNT_READ_SCOPE).await;
    assert_eq!(
        oauth::verify_access(&pool, &config, &tokens.access_token).await,
        Err(oauth::Error::InvalidGrant)
    );
    assert_eq!(
        oauth::verify_account_services(&pool, &config, &tokens.access_token).await,
        Err(oauth::Error::InvalidGrant)
    );
    let bearer = format!("Bearer {}", tokens.access_token);
    let summary = response(app.clone(), "/api/v1/account/summary", &bearer).await;
    assert_eq!(summary.status(), StatusCode::OK);
    let summary: serde_json::Value =
        serde_json::from_slice(&to_bytes(summary.into_body(), 16384).await.unwrap()).unwrap();
    assert_eq!(
        summary,
        serde_json::json!({"account_id": owner, "locale": "en", "coins": null})
    );
    for scope in [oauth::TICKET_SCOPE, oauth::ACCOUNT_SERVICES_SCOPE] {
        let other_scope = grant(&pool, &config, owner, scope).await;
        let wrong_scope = format!("Bearer {}", other_scope.access_token);
        assert_eq!(
            response(app.clone(), "/api/v1/feed", &wrong_scope)
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let refresh = format!("Bearer {}", tokens.refresh_token);
    assert_eq!(
        response(app.clone(), "/api/v1/feed", &refresh)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let wrong_client = configured_router(
        pool.clone(),
        oauth::Config {
            enabled: true,
            clients: vec![oauth::Client {
                client_id: "second-client".into(),
                redirect_uris: vec!["http://127.0.0.1:12345/callback".into()],
            }],
        },
    );
    assert_eq!(
        response(wrong_client, "/api/v1/feed", &bearer)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let off = configured_router(pool.clone(), oauth::Config::default());
    assert_eq!(
        response(off, "/api/v1/feed", &bearer).await.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let result = response(app.clone(), "/api/v1/feed", &bearer).await;
    assert_eq!(result.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(result.into_body(), 16384).await.unwrap()).unwrap();
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["items"][0]["id"], id.to_string());
    assert!(reads.summary(owner).await.unwrap().coins.is_none());
    #[derive(Debug)]
    struct Coins;
    impl CoinBalance for Coins {
        fn available(&self, _: Uuid) -> ReadFuture<'_, i64> {
            Box::pin(async { Ok(42) })
        }
    }
    let mut money = PrivateReads {
        pool: pool.clone(),
        money_enabled: true,
        wallet: None,
    };
    assert_eq!(
        money.summary(owner).await.unwrap_err(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    money.wallet = Some(Arc::new(Coins));
    assert_eq!(money.summary(owner).await.unwrap().coins, Some(42));

    for path in [
        "/api/v1/feed?limit=-1".into(),
        format!("/api/v1/feed?account_id={other}"),
        format!("/api/v1/account/summary?account_id={other}"),
    ] {
        assert_eq!(
            response(app.clone(), &path, &bearer).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    let filtered = response(
        app.clone(),
        "/api/v1/feed?limit=1&unread_only=true",
        &bearer,
    )
    .await;
    assert_eq!(filtered.status(), StatusCode::OK);
    let filtered: serde_json::Value =
        serde_json::from_slice(&to_bytes(filtered.into_body(), 16384).await.unwrap()).unwrap();
    assert_eq!(filtered["items"][0]["id"], id.to_string());

    let mut tx = pool.begin().await.unwrap();
    oauth::revoke_account(&mut tx, owner).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        response(app.clone(), "/api/v1/feed", &bearer)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let renewed = grant(&pool, &config, owner, oauth::ACCOUNT_READ_SCOPE).await;
    let bearer = format!("Bearer {}", renewed.access_token);
    outbox::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    for path in ["/api/v1/feed", "/api/v1/account/summary"] {
        assert_eq!(
            response(app.clone(), path, &bearer).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        reads.summary(owner).await.unwrap_err(),
        StatusCode::FORBIDDEN
    );
    pool.close().await;
    outbox::query(outbox::AssertSqlSafe(format!(
        "DROP SCHEMA {schema} CASCADE"
    )))
    .execute(&admin)
    .await
    .unwrap();
    admin.close().await;
}
