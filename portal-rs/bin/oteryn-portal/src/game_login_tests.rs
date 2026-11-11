#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! OAuth access token -> ticket -> native admission through the real routers,
//! with the LCFA projection fed through the internal ingestion router.
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use oteryn_internal_api::guard::VerifiedClient;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const AUTHORITY: &str = "oteryn:character-authority:primary";
const PRODUCER: &str = "lcfa.game.internal";
const SERVICE: &str = "synthetic-gateway-service-credential";
const REDIRECT: &str = "http://127.0.0.1:41000/callback";
const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
const CHARACTER: &str = "01934f10-7c04-7001-805b-3b1122334401";

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    client: Uuid,
    config: oauth::Config,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED game login PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        // Generated from a UUID only, never from input.
        let schema = format!("game_login_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(6)
            .after_connect(move |connection, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path', $1, false)")
                        .bind(path)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        let client = Uuid::now_v7();
        sqlx::query("INSERT INTO oauth_native_clients(client_id,enabled,first_party,public_client,native_ticket) VALUES($1,true,true,true,true)")
            .bind(client).execute(&pool).await?;
        let config = oauth::Config {
            enabled: true,
            clients: vec![oauth::Client {
                client_id: client.to_string(),
                redirect_uris: vec![REDIRECT.into()],
            }],
        };
        Ok(Some(Self {
            pool,
            admin,
            schema,
            client,
            config,
        }))
    }
    async fn account(&self) -> Result<Uuid, sqlx::Error> {
        sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,'test-hash','active','en',now()) RETURNING id")
            .bind(format!("{}@example.test", Uuid::now_v7())).fetch_one(&self.pool).await
    }
    /// Authorization code + PKCE exchange, as the native client performs it.
    async fn access_token(&self, account: Uuid) -> Result<String, Box<dyn std::error::Error>> {
        let verifier = "v".repeat(43);
        let request = oauth::Authorization {
            response_type: "code".into(),
            client_id: self.client.to_string(),
            redirect_uri: REDIRECT.into(),
            scope: "game:ticket".into(),
            state: "test-state-for-browser".into(),
            code_challenge: URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())),
            code_challenge_method: "S256".into(),
        };
        let code = oauth::authorize(&self.pool, &self.config, account, &request).await?;
        let tokens = oauth::exchange(
            &self.pool,
            &self.config,
            &oauth::TokenRequest {
                grant_type: "authorization_code".into(),
                client_id: self.client.to_string(),
                redirect_uri: REDIRECT.into(),
                code,
                code_verifier: verifier,
                ..Default::default()
            },
        )
        .await?;
        Ok(tokens.access_token)
    }
    fn projection_settings() -> lcfa::Settings {
        lcfa::Settings {
            enabled: true,
            environment: oteryn_core::Environment::Testing,
            source_authority: AUTHORITY.into(),
            ..Default::default()
        }
    }
    fn player(&self) -> Router {
        oteryn_player_api::router(oteryn_player_api::Api {
            tickets: tickets::Tickets::new(
                self.pool.clone(),
                tickets::Settings {
                    enabled: true,
                    nonproduction: true,
                },
                ticket_authority(self.config.clone()),
            ),
            projection: Some(lcfa::Projection::new(
                self.pool.clone(),
                Self::projection_settings(),
                Arc::new(overview::Accounts(self.pool.clone())),
            )),
        })
    }
    fn admission(&self) -> Result<Router, Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let current = dir.path().join("current");
        std::fs::write(
            &current,
            format!("{:x}\n", Sha256::digest(SERVICE.as_bytes())),
        )?;
        let tokens = admission::ServiceTokens::from_files(&current, None)?;
        Ok(admission::router(
            self.pool.clone(),
            admission::AdmissionConfig {
                enabled: true,
                tokens: Some(tokens),
                requests_per_minute: 120,
            },
            Arc::new(admission_accounts()),
            Arc::new(admission_authority(&Self::projection_settings())),
        ))
    }
    fn lcfa(&self) -> Result<Router, Box<dyn std::error::Error>> {
        Ok(lcfa_router(
            self.pool.clone(),
            Some(PRODUCER.into()),
            Self::projection_settings(),
        )?
        .expect("identity configured"))
    }
    /// Publishes one AVAILABLE character for `account` and a live watermark.
    async fn publish(&self, account: Uuid) -> TestResult {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let snapshot = format!(
            r#"{{"contract_version":1,"operation":"PublishAccountCharactersV1","source_authority":"{AUTHORITY}","account_id":"{account}","projection_epoch":"1","projection_revision":"1","source_observed_at":"{}","characters":[{{"character_id":"{CHARACTER}","world_id":"{WORLD}","name":"Aldric","availability":"AVAILABLE"}}]}}"#,
            now - 1
        );
        let watermark = format!(
            r#"{{"contract_version":1,"operation":"PublishProjectionWatermarkV1","source_authority":"{AUTHORITY}","projection_epoch":"1","complete_through":"{}","observed_at":"{now}"}}"#,
            now - 1
        );
        for (path, raw) in [
            ("/internal/v1/game-auth/native-account-characters", snapshot),
            (
                "/internal/v1/game-auth/native-account-characters/watermark",
                watermark,
            ),
        ] {
            let (status, body) = send(self.lcfa()?, internal(path, raw, PRODUCER)?).await?;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body, br#"{"contract_version":1,"result":"accepted"}"#);
        }
        Ok(())
    }
    async fn suspend(&self, account: Uuid) -> TestResult {
        let staff = self.account().await?;
        let approver = self.account().await?;
        sqlx::query("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,created_by,approved_by) VALUES($1,'suspension','active','test suspension',now()-interval '1 second',$2,$3)")
            .bind(account).bind(staff).bind(approver).execute(&self.pool).await?;
        Ok(())
    }
    async fn ticket_used(&self, ticket: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT used_at IS NOT NULL FROM game_login_tickets WHERE ticket_hash=$1",
        )
        .bind(&Sha256::digest(ticket.as_bytes())[..])
        .fetch_one(&self.pool)
        .await
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}

fn internal(
    path: &str,
    raw: String,
    peer: &str,
) -> Result<Request<Body>, Box<dyn std::error::Error>> {
    let mut request = Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(raw))?;
    request
        .extensions_mut()
        .insert(VerifiedClient::from_verified_tls13_http11(Identity::san(
            peer,
        )?));
    Ok(request)
}
fn bearer(method: &str, path: &str, token: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .expect("request")
}
async fn send(
    app: Router,
    request: Request<Body>,
) -> Result<(StatusCode, Vec<u8>), Box<dyn std::error::Error>> {
    let response = app.oneshot(request).await?;
    let status = response.status();
    Ok((
        status,
        to_bytes(response.into_body(), 65536).await?.to_vec(),
    ))
}
async fn ticket(
    app: Router,
    token: &str,
) -> Result<(StatusCode, Vec<u8>), Box<dyn std::error::Error>> {
    send(
        app,
        bearer(
            "POST",
            "/api/v1/game-auth/tickets",
            token,
            r#"{"protocol_version":1}"#,
        ),
    )
    .await
}
fn login_body(ticket: &str, attempt: Uuid, character: &str) -> String {
    format!(
        r#"{{"protocol_version":2,"game_login_ticket":"{ticket}","attempt_ref":"{attempt}","character_id":"{character}","channel_id":null,"offer":{{"client_build":"0.1.0+test","client_platform":"windows","transports":[{{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}}]}}}}"#
    )
}
async fn admit(
    app: Router,
    service: &str,
    body: String,
) -> Result<(StatusCode, Vec<u8>), Box<dyn std::error::Error>> {
    send(
        app,
        bearer(
            "POST",
            "/internal/v1/game-auth/native-admissions",
            service,
            &body,
        ),
    )
    .await
}
fn refusal(code: &str, class: &str, attempt: Uuid) -> Vec<u8> {
    format!(
        r#"{{"protocol_version":2,"error":{{"code":"{code}","public_class":"{class}"}},"attempt_ref":"{attempt}"}}"#
    )
    .into_bytes()
}
fn issued(body: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let value: serde_json::Value = serde_json::from_slice(body)?;
    assert_eq!(value["protocol_version"], 1);
    assert_eq!(value["expires_in"], 60);
    Ok(value["ticket"].as_str().expect("ticket").to_owned())
}

#[tokio::test]
async fn pg_oauth_token_ticket_and_admission_through_real_routers() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    db.publish(account).await?;
    let token = db.access_token(account).await?;

    let (status, body) = send(
        db.player(),
        bearer("GET", "/api/v1/game-auth/native-characters", &token, ""),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        format!(r#"{{"protocol_version":2,"characters":[{{"character_id":"{CHARACTER}","world_id":"{WORLD}","name":"Aldric","availability":"AVAILABLE"}}]}}"#).into_bytes()
    );
    let (status, body) = ticket(db.player(), &token).await?;
    assert_eq!(status, StatusCode::OK);
    let native = issued(&body)?;
    // Issuance consumed the access token.
    assert_eq!(
        ticket(db.player(), &token).await?.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, _) = ticket(db.player(), "not-a-valid-access-token").await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let attempt = Uuid::now_v7();
    let app = db.admission()?;
    let (status, body) = admit(
        app.clone(),
        "wrong-service-credential",
        login_body(&native, attempt, CHARACTER),
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, br#"{"error":"unauthorized_service"}"#);
    let (status, _) = admit(app.clone(), SERVICE, "{".into()).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, body) = admit(
        app.clone(),
        SERVICE,
        login_body(&native, attempt, &Uuid::now_v7().to_string()),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        body,
        refusal(
            "NATIVE_LOGIN_CHARACTER_CONFLICT",
            "SESSION_UNAVAILABLE",
            attempt
        )
    );
    assert!(!db.ticket_used(&native).await?);
    // Redemption and the character check pass; no grant signer is configured.
    let (status, body) = admit(
        app.clone(),
        SERVICE,
        login_body(&native, attempt, CHARACTER),
    )
    .await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body,
        refusal(
            "NATIVE_LOGIN_UNAVAILABLE",
            "TEMPORARILY_UNAVAILABLE",
            attempt
        )
    );
    assert!(!db.ticket_used(&native).await?);
    let (status, body) = admit(
        app.clone(),
        SERVICE,
        login_body("unknown-ticket", attempt, CHARACTER),
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        body,
        refusal(
            "NATIVE_LOGIN_AUTHENTICATION_REQUIRED",
            "AUTHENTICATION_REQUIRED",
            attempt
        )
    );
    sqlx::query("UPDATE game_login_tickets SET expires_at=clock_timestamp()-interval '1 second'")
        .execute(&db.pool)
        .await?;
    let (status, _) = admit(app, SERVICE, login_body(&native, attempt, CHARACTER)).await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM native_admissions")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(rows, 0);
    db.close().await
}

#[tokio::test]
async fn pg_suspended_account_gets_no_ticket_characters_or_admission() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let account = db.account().await?;
    db.publish(account).await?;
    let first = db.access_token(account).await?;
    let second = db.access_token(account).await?;
    let (status, body) = ticket(db.player(), &first).await?;
    assert_eq!(status, StatusCode::OK);
    let native = issued(&body)?;

    db.suspend(account).await?;
    let (status, _) = ticket(db.player(), &second).await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = send(
        db.player(),
        bearer("GET", "/api/v1/game-auth/native-characters", &second, ""),
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // The refused issuance did not consume the token.
    let unused: bool = sqlx::query_scalar("SELECT t.used_at IS NULL AND f.revoked_at IS NULL FROM oauth_tokens t JOIN oauth_token_families f ON f.id=t.family_id WHERE t.access_hash=$1")
        .bind(&Sha256::digest(second.as_bytes())[..]).fetch_one(&db.pool).await?;
    assert!(unused);

    let attempt = Uuid::now_v7();
    let (status, body) = admit(
        db.admission()?,
        SERVICE,
        login_body(&native, attempt, CHARACTER),
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        body,
        refusal(
            "NATIVE_LOGIN_AUTHENTICATION_REQUIRED",
            "AUTHENTICATION_REQUIRED",
            attempt
        )
    );
    assert!(!db.ticket_used(&native).await?);
    db.close().await
}

#[tokio::test]
async fn pg_lcfa_internal_identity_is_exclusive() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let (status, _) = send(
        db.lcfa()?,
        internal(
            "/internal/v1/game-auth/native-account-characters/watermark",
            "{}".into(),
            "runtime-status.game.internal",
        )?,
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = send(
        db.lcfa()?,
        internal(
            "/internal/v1/game-auth/native-account-characters/watermark",
            "{}".into(),
            PRODUCER,
        )?,
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    db.close().await
}

#[tokio::test]
async fn lcfa_router_needs_a_valid_identity() -> TestResult {
    let pool = PgPoolOptions::new().connect_lazy("postgresql://localhost/unused")?;
    let settings = Database::projection_settings;
    assert!(lcfa_router(pool.clone(), None, settings())?.is_none());
    assert!(lcfa_router(pool.clone(), Some("  ".into()), settings())?.is_none());
    assert!(lcfa_router(pool, Some("bad\nname".into()), settings()).is_err());
    Ok(())
}
