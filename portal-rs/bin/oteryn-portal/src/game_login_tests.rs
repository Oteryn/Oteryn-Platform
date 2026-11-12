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
        self.admission_with(admission::Issuance::default())
    }
    fn admission_with(
        &self,
        issuance: admission::Issuance,
    ) -> Result<Router, Box<dyn std::error::Error>> {
        let signer = issuance.signer.clone();
        self.admission_composed(issuance, signer)
    }
    /// The listener composition with the attempt-store signer given separately.
    fn admission_composed(
        &self,
        issuance: admission::Issuance,
        signer: Option<Arc<dyn admission::GrantSigner>>,
    ) -> Result<Router, Box<dyn std::error::Error>> {
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
                signer,
            },
            Arc::new(admission_accounts()),
            Arc::new(admission_authority(&Self::projection_settings(), issuance)),
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

// T19c: FND-04 grant issuance with ephemeral custody, the public trust
// registry, a mode 34a route record and a fresh ownership-bound runtime report.
const CHANNEL: &str = "01934f10-7c03-7001-805b-3b1122334401";
const ROUTE_HOST: &str = "game-eu1.example.test";
const KID: &str = "fresh-t19c-1";
const PURPOSE: &str = "fresh_entry";

struct Grants {
    _dir: tempfile::TempDir,
    env: Vec<(&'static str, String)>,
    issuance: admission::Issuance,
    revision: String,
}
impl Grants {
    /// The custody variables with `overrides`; an empty override value unsets.
    fn lookup(&self, overrides: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let mut env: Vec<(String, String)> = self
            .env
            .iter()
            .filter(|(k, _)| !overrides.iter().any(|(o, _)| o == k))
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect();
        env.extend(
            overrides
                .iter()
                .filter(|(_, v)| !v.is_empty())
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned())),
        );
        move |key| env.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }
}
fn runtime_settings() -> Result<oteryn_gameauth::runtime::Settings, Box<dyn std::error::Error>> {
    Ok(oteryn_gameauth::runtime::Settings {
        enabled: true,
        peers: vec![oteryn_gameauth::runtime::Peer {
            name: "node".into(),
            identity: Identity::san("node")?,
            scopes: vec![(WORLD.parse()?, CHANNEL.parse()?)],
        }],
        ..Default::default()
    })
}
impl Database {
    async fn now(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&self.pool)
            .await
    }
    async fn native_ticket(&self, account: Uuid) -> Result<String, Box<dyn std::error::Error>> {
        let token = self.access_token(account).await?;
        let (status, body) = ticket(self.player(), &token).await?;
        assert_eq!(status, StatusCode::OK);
        issued(&body)
    }
    /// Publishes a runtime report for the route scope through T19b ingestion.
    async fn report(&self, revision: &str, source: u64) -> TestResult {
        self.report_with(revision, source, &[]).await
    }
    /// `report` with some members replaced.
    async fn report_with(
        &self,
        revision: &str,
        source: u64,
        overrides: &[(&str, oteryn_internal_api::canonical::Value)],
    ) -> TestResult {
        use oteryn_internal_api::canonical::Value;
        let text = |s: &str| Value::String(s.to_owned());
        let mut members = vec![
            ("contract_version".to_owned(), Value::Unsigned(1)),
            ("operation".into(), text("ReportRuntimeStatusV1")),
            ("source_authority".into(), text("game")),
            ("world_id".into(), text(WORLD)),
            ("channel_id".into(), text(CHANNEL)),
            (
                "node_id".into(),
                text("019c0000-0000-4000-8000-000000000004"),
            ),
            ("assignment_epoch".into(), text("1")),
            ("scope_ownership_generation".into(), text("1")),
            ("source_revision".into(), text(&source.to_string())),
            ("decision_identity".into(), text("publication")),
            ("ready".into(), Value::Bool(true)),
            ("published_at".into(), text("10")),
            ("protocol_major".into(), Value::Unsigned(1)),
            ("transport_profile".into(), Value::Unsigned(1)),
            ("route_revision".into(), text(revision)),
        ];
        for (key, value) in [
            ("runtime_observation_revision", "obs.1"),
            ("ruleset_revision", "rules.1"),
            ("content_revision", "content.1"),
            ("map_revision", "map.1"),
            ("world_policy_revision", "policy.1"),
            ("offer_revision", "offer.1"),
        ] {
            members.push((key.into(), text(value)));
        }
        members.push(("observed_at".into(), text(&self.now().await?.to_string())));
        for (key, value) in overrides {
            if let Some(member) = members.iter_mut().find(|(k, _)| k == key) {
                member.1 = value.clone();
            }
        }
        let report = oteryn_gameauth::runtime::Report::decode(&Value::Object(members))
            .map_err(|status| format!("runtime report rejected: {status}"))?;
        oteryn_gameauth::runtime::ingest(&self.pool, "node", &report)
            .await
            .map_err(|status| format!("runtime report not ingested: {status}"))?;
        Ok(())
    }
    /// Ephemeral custody with one trusted Active key, composed through the
    /// binary's `issuance` exactly as from `PORTAL_*` variables.
    async fn grants(&self) -> Result<Grants, Box<dyn std::error::Error>> {
        use oteryn_gameauth::keys;
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir()?;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))?;
        let wrapping = dir.path().join("wrapping");
        std::fs::write(&wrapping, Sha256::digest(Uuid::now_v7().as_bytes()))?;
        std::fs::set_permissions(&wrapping, std::fs::Permissions::from_mode(0o600))?;
        let custody = dir.path().join("custody");
        let witness = dir.path().join("witness");
        std::fs::create_dir(&witness)?;
        std::fs::set_permissions(&witness, std::fs::Permissions::from_mode(0o700))?;
        let settings = keys::Settings {
            enabled: true,
            key_purpose: PURPOSE.into(),
            clock_uncertainty_seconds: 1,
        };
        let registry =
            keys::PublicTrustRegistry::new(self.pool.clone(), settings.clone(), witness.clone())?;
        {
            let mut ring = keys::Keyring::create(
                settings,
                keys::EncryptedFileStorage::from_key_file(&custody, &wrapping)?,
            )?;
            let public = ring.generate_next(KID)?;
            registry.publish(&public).await?;
            sqlx::query("UPDATE native_game_signing_trust_key_versions SET trusted_at=clock_timestamp()-interval '10 seconds'")
                .execute(&self.pool).await?;
            let snapshot = registry.snapshot().await?;
            ring.activate(KID, u64::try_from(self.now().await?)?, &snapshot)?;
        }
        let path = |p: &std::path::Path| p.to_string_lossy().into_owned();
        let env = vec![
            (admission::GRANTS_ENABLED, "true".to_owned()),
            ("OTERYN_ENV", "testing".to_owned()),
            (admission::KEY_CUSTODY_FILE, path(&custody)),
            (admission::KEY_WRAPPING_FILE, path(&wrapping)),
            (admission::KEY_PURPOSE, PURPOSE.to_owned()),
            (admission::WITNESS_DIRECTORY, path(&witness)),
        ];
        let mut grants = Grants {
            _dir: dir,
            env,
            issuance: admission::Issuance::default(),
            revision: admission::route_revision(
                1,
                WORLD.parse()?,
                CHANNEL.parse()?,
                ROUTE_HOST,
                7172,
                ROUTE_HOST,
            )?,
        };
        let mut issuance = issuance(self.pool.clone(), grants.lookup(&[]));
        assert!(issuance.permitted());
        assert!(issuance.signer.is_some(), "custody must open");
        issuance.runtime = runtime_settings()?;
        grants.issuance = issuance;
        sqlx::query("INSERT INTO game_channels (world_id,channel_id,native_route_host,native_route_port,native_route_tls_server_name,native_route_version,native_route_revision,native_login_enabled) VALUES ($1,$2,$3,7172,$3,1,$4,true)")
            .bind(WORLD.parse::<Uuid>()?).bind(CHANNEL.parse::<Uuid>()?).bind(ROUTE_HOST).bind(&grants.revision)
            .execute(&self.pool).await?;
        sqlx::query("INSERT INTO native_scope_assignments (world_id,channel_id,assignment_epoch,ownership_generation,node_identity,assigned_at) VALUES ($1,$2,1,1,'node',0)")
            .bind(WORLD.parse::<Uuid>()?).bind(CHANNEL.parse::<Uuid>()?)
            .execute(&self.pool).await?;
        self.report(&grants.revision, 1).await?;
        Ok(grants)
    }
    /// The public key the trust registry currently publishes for `kid`.
    async fn registry_key(&self, kid: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
        let encoded: String = sqlx::query_scalar("SELECT public_key FROM native_game_signing_trust_key_versions WHERE key_id=$1 ORDER BY key_revision DESC LIMIT 1")
            .bind(kid).fetch_one(&self.pool).await?;
        Ok(URL_SAFE_NO_PAD
            .decode(encoded)?
            .try_into()
            .map_err(|_| "public key length")?)
    }
    /// A refused issuance left the ticket unused and stored no attempt.
    async fn nothing_committed(&self, ticket: &str) -> TestResult {
        assert!(!self.ticket_used(ticket).await?);
        let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM native_admissions")
            .fetch_one(&self.pool)
            .await?;
        assert_eq!(rows, 0);
        Ok(())
    }
    /// Waits until the database clock has passed `second`.
    async fn after(&self, second: i64) -> TestResult {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(40);
        while self.now().await? <= second {
            assert!(
                tokio::time::Instant::now() < deadline,
                "clock did not advance"
            );
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        Ok(())
    }
    async fn execute(&self, sql: &'static str) -> TestResult {
        sqlx::query(sql).execute(&self.pool).await?;
        Ok(())
    }
}
async fn refused(
    app: Router,
    body: impl Fn(Uuid) -> String,
    status: StatusCode,
    code: &str,
    class: &str,
) -> TestResult {
    let attempt = Uuid::now_v7();
    let (actual, raw) = admit(app, SERVICE, body(attempt)).await?;
    assert_eq!(
        (actual, String::from_utf8(raw)?),
        (status, String::from_utf8(refusal(code, class, attempt))?),
        "{code}"
    );
    Ok(())
}
async fn unavailable(app: Router, body: impl Fn(Uuid) -> String) -> TestResult {
    refused(
        app,
        body,
        StatusCode::SERVICE_UNAVAILABLE,
        "NATIVE_LOGIN_UNAVAILABLE",
        "TEMPORARILY_UNAVAILABLE",
    )
    .await
}
async fn no_route(app: Router, body: impl Fn(Uuid) -> String) -> TestResult {
    refused(
        app,
        body,
        StatusCode::SERVICE_UNAVAILABLE,
        "NATIVE_LOGIN_ROUTE_UNAVAILABLE",
        "TEMPORARILY_UNAVAILABLE",
    )
    .await
}

#[tokio::test]
async fn pg_testing_issuance_signs_a_grant_the_trust_registry_verifies() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let grants = db.grants().await?;
    let account = db.account().await?;
    db.publish(account).await?;
    let native = db.native_ticket(account).await?;
    let attempt = Uuid::now_v7();
    let raw = login_body(&native, attempt, CHARACTER);
    let app = db.admission_with(grants.issuance.clone())?;
    let before = u64::try_from(db.now().await?)?;
    let (status, body) = admit(app.clone(), SERVICE, raw.clone()).await?;
    let after = u64::try_from(db.now().await?)?;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let request: oteryn_gateway::wire::Request = serde_json::from_str(&raw)?;
    let success =
        oteryn_gateway::wire::parse_success(&body, &request).expect("contract success shape");
    assert_eq!(
        (success.world_id.as_str(), success.channel_id.as_str()),
        (WORLD, CHANNEL)
    );
    assert_eq!(
        (
            success.endpoint.host.as_str(),
            success.endpoint.port,
            success.endpoint.tls_server_name.as_str()
        ),
        (ROUTE_HOST, 7172, ROUTE_HOST)
    );
    assert!((18..=20).contains(&success.grant.valid_for_seconds));

    let token = success.grant.token.clone();
    let (input, signature) = token.rsplit_once('.').expect("three parts");
    let (header, payload) = input.split_once('.').expect("three parts");
    assert_eq!(
        URL_SAFE_NO_PAD.decode(header)?,
        br#"{"alg":"Ed25519","kid":"fresh-t19c-1","typ":"oteryn-admission+jwt"}"#
    );
    let payload = String::from_utf8(URL_SAFE_NO_PAD.decode(payload)?)?;
    let claims: serde_json::Value = serde_json::from_str(&payload)?;
    let iat = claims["iat"].as_u64().expect("iat");
    assert!(
        (before..=after).contains(&iat),
        "{before} <= {iat} <= {after}"
    );
    let jti = claims["jti"].as_str().expect("jti");
    assert_eq!(jti.len(), 43);
    let generation: String =
        sqlx::query_scalar("SELECT native_security_generation::text FROM accounts WHERE id=$1")
            .bind(account)
            .fetch_one(&db.pool)
            .await?;
    let revision = &grants.revision;
    assert_eq!(
        payload,
        format!(
            r#"{{"iss":"urn:oteryn:platform:game-admission","aud":"urn:oteryn:game:admission","iat":{iat},"nbf":{iat},"exp":{},"jti":"{jti}","profile":"oteryn-pre-admission-v1","purpose":"fresh_entry","attempt_ref":"{attempt}","account_id":"{account}","character_id":"{CHARACTER}","world_id":"{WORLD}","channel_id":"{CHANNEL}","account_security_generation":"{generation}","route_revision":"{revision}","runtime_observation_revision":"obs.1","scope_ownership_generation":"1","protocol_major":1,"transport_profile":1,"ruleset_revision":"rules.1","content_revision":"content.1","map_revision":"map.1","world_policy_revision":"policy.1","offer_revision":"offer.1"}}"#,
            iat + 20
        )
    );
    // Pure Ed25519 over the exact signing input with the key the public trust
    // registry publishes for the header's kid.
    let public_key = db.registry_key(KID).await?;
    let signature_bytes = URL_SAFE_NO_PAD.decode(signature)?;
    oteryn_gameauth::keys::verify(public_key, input.as_bytes(), &signature_bytes)?;
    let forged = format!("{input}A");
    assert!(
        oteryn_gameauth::keys::verify(public_key, forged.as_bytes(), &signature_bytes).is_err()
    );
    assert!(db.ticket_used(&native).await?);

    // §6.1: the committed attempt holds the signing input, never the token.
    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT response_body FROM native_admissions WHERE attempt_ref=$1")
            .bind(attempt)
            .fetch_one(&db.pool)
            .await?;
    let stored = String::from_utf8(stored)?;
    assert!(stored.contains(&format!(r#""token":"{input}""#)));
    assert!(!stored.contains(signature));

    // §6.2: the same attempt replays the same grant, re-signed from the input.
    let (status, again) = admit(app.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    let replay = oteryn_gateway::wire::parse_success(&again, &request).expect("replay");
    assert_eq!(replay.grant.token, token);
    // A different request under the same attempt_ref is a conflict.
    let (status, body) = admit(
        app.clone(),
        SERVICE,
        login_body(&native, attempt, &Uuid::now_v7().to_string()),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        body,
        refusal("NATIVE_LOGIN_ATTEMPT_CONFLICT", "RETRY_LOGIN", attempt)
    );
    // The consumed ticket under a new attempt_ref is refused.
    let (status, _) = admit(
        app.clone(),
        SERVICE,
        login_body(&native, Uuid::now_v7(), CHARACTER),
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // Replay never re-signs once issuance is off or the bound route changed;
    // a stored kid that cannot be re-signed needs reconciliation (§6.2 step 5).
    let (status, body) = admit(db.admission()?, SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body,
        refusal(
            "ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED",
            "TEMPORARILY_UNAVAILABLE",
            attempt
        )
    );
    db.execute("UPDATE game_channels SET native_login_enabled=false")
        .await?;
    let (status, body) = admit(app, SERVICE, raw).await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body,
        refusal(
            "NATIVE_LOGIN_ROUTE_UNAVAILABLE",
            "TEMPORARILY_UNAVAILABLE",
            attempt
        )
    );
    db.close().await
}

#[tokio::test]
async fn pg_issuance_refusals_fail_closed_without_consuming_the_ticket() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let grants = db.grants().await?;
    let account = db.account().await?;
    db.publish(account).await?;
    let native = db.native_ticket(account).await?;
    let app = db.admission_with(grants.issuance.clone())?;
    let login = |attempt| login_body(&native, attempt, CHARACTER);

    // Flag off, not testing/preproduction or a bad TTL: the key is never loaded.
    for overrides in [
        &[(admission::GRANTS_ENABLED, "")][..],
        &[(admission::GRANTS_ENABLED, "false")],
        &[("OTERYN_ENV", "production")],
        &[("OTERYN_ENV", "")],
        &[(admission::GRANT_TTL_SECONDS, "31")],
    ] {
        let mut issuance = issuance(db.pool.clone(), grants.lookup(overrides));
        assert!(issuance.signer.is_none(), "{overrides:?}");
        issuance.runtime = runtime_settings()?;
        unavailable(db.admission_with(issuance)?, login).await?;
    }
    // A loaded signer still refuses once the policy says production or off.
    let production = admission::Issuance {
        environment: "production".into(),
        ..grants.issuance.clone()
    };
    unavailable(db.admission_with(production)?, login).await?;
    let disabled = admission::Issuance {
        enabled: false,
        ..grants.issuance.clone()
    };
    unavailable(db.admission_with(disabled)?, login).await?;
    // Custody that does not open leaves issuance unavailable.
    let missing = issuance(
        db.pool.clone(),
        grants.lookup(&[(admission::KEY_CUSTODY_FILE, "/nonexistent/custody")]),
    );
    assert!(missing.signer.is_none());

    // §7.4 / mode 34a: no enabled, intact route record means no route.
    db.execute("UPDATE game_channels SET native_login_enabled=false")
        .await?;
    no_route(app.clone(), login).await?;
    db.execute("UPDATE game_channels SET native_login_enabled=true")
        .await?;
    db.execute("UPDATE game_channels SET native_route_port=7173")
        .await?;
    no_route(app.clone(), login).await?;
    db.execute("UPDATE game_channels SET native_route_port=7172")
        .await?;
    // A requested channel must be an issuable candidate.
    no_route(app.clone(), |attempt| {
        login(attempt).replace(
            r#""channel_id":null"#,
            &format!(r#""channel_id":"{}""#, Uuid::now_v7()),
        )
    })
    .await?;
    // Runtime evidence: stale, another route revision, lost ownership, off.
    db.execute("UPDATE native_runtime_status_reports SET observed_at=observed_at-60")
        .await?;
    no_route(app.clone(), login).await?;
    db.report("rt.1.00000000000000000000000000000000", 2)
        .await?;
    no_route(app.clone(), login).await?;
    db.report(&grants.revision, 3).await?;
    db.execute("UPDATE native_scope_assignments SET ownership_generation=2")
        .await?;
    no_route(app.clone(), login).await?;
    db.execute("UPDATE native_scope_assignments SET ownership_generation=1")
        .await?;
    let blind = admission::Issuance {
        runtime: oteryn_gameauth::runtime::Settings::default(),
        ..grants.issuance.clone()
    };
    no_route(db.admission_with(blind)?, login).await?;

    // §5.4: the character must be AVAILABLE in a live projection.
    db.execute("UPDATE account_characters SET availability='UNAVAILABLE'")
        .await?;
    refused(
        app.clone(),
        login,
        StatusCode::CONFLICT,
        "NATIVE_LOGIN_CHARACTER_CONFLICT",
        "SESSION_UNAVAILABLE",
    )
    .await?;
    db.execute("UPDATE account_characters SET availability='AVAILABLE'")
        .await?;
    db.execute("UPDATE account_character_feed SET complete_through=complete_through-120, observed_at=observed_at-120")
        .await?;
    no_route(app.clone(), login).await?;
    db.execute("UPDATE account_character_feed SET complete_through=complete_through+120, observed_at=observed_at+120")
        .await?;

    // Bad input never reaches issuance.
    let (status, _) = admit(app.clone(), SERVICE, "{".into()).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = admit(
        app.clone(),
        SERVICE,
        login(Uuid::now_v7()).replace(r#""protocol_version":2"#, r#""protocol_version":3"#),
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = admit(app.clone(), SERVICE, login(Uuid::new_v4())).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Nothing above consumed the ticket or stored an attempt; it still issues.
    assert!(!db.ticket_used(&native).await?);
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM native_admissions")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(rows, 0);
    let (status, body) = admit(app.clone(), SERVICE, login(Uuid::now_v7())).await?;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

    // The account security generation moved after the ticket was minted.
    let second = db.native_ticket(account).await?;
    db.execute("UPDATE accounts SET native_security_generation=native_security_generation+1")
        .await?;
    refused(
        app,
        |attempt| login_body(&second, attempt, CHARACTER),
        StatusCode::UNAUTHORIZED,
        "NATIVE_LOGIN_AUTHENTICATION_REQUIRED",
        "AUTHENTICATION_REQUIRED",
    )
    .await?;
    db.close().await
}

/// Token, decoded claims and `valid_for_seconds` of a success body.
fn granted(
    raw: &str,
    body: &[u8],
) -> Result<(String, serde_json::Value, i64), Box<dyn std::error::Error>> {
    let request: oteryn_gateway::wire::Request = serde_json::from_str(raw)?;
    let success = oteryn_gateway::wire::parse_success(body, &request).expect("success shape");
    let payload = success.grant.token.split('.').nth(1).expect("payload");
    let claims = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload)?)?;
    Ok((
        success.grant.token.clone(),
        claims,
        success.grant.valid_for_seconds,
    ))
}

#[tokio::test]
async fn pg_signer_self_check_failures_refuse_issuance_without_consuming_the_ticket() -> TestResult
{
    use oteryn_gameauth::keys;
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let mut grants = db.grants().await?;
    // Custody is exclusive to one open signer; each case below opens its own.
    grants.issuance.signer = None;
    let account = db.account().await?;
    db.publish(account).await?;
    let native = db.native_ticket(account).await?;
    let login = |attempt| login_body(&native, attempt, CHARACTER);
    // Each case composes a new signer, so no earlier self-check is cached.
    let fresh =
        |overrides: &[(&str, &str)]| -> Result<admission::Issuance, Box<dyn std::error::Error>> {
            let mut issuance = issuance(db.pool.clone(), grants.lookup(overrides));
            issuance.runtime = runtime_settings()?;
            Ok(issuance)
        };
    let refuses = || async {
        let issuance = fresh(&[])?;
        assert!(issuance.signer.is_some(), "custody must open");
        unavailable(db.admission_with(issuance)?, login).await?;
        db.nothing_committed(&native).await
    };

    // Registry public key differs from the custody key.
    let original: String = sqlx::query_scalar(
        "SELECT public_key FROM native_game_signing_trust_key_versions WHERE key_id=$1",
    )
    .bind(KID)
    .fetch_one(&db.pool)
    .await?;
    sqlx::query("UPDATE native_game_signing_trust_key_versions SET public_key=$1")
        .bind(URL_SAFE_NO_PAD.encode([7_u8; 32]))
        .execute(&db.pool)
        .await?;
    refuses().await?;
    sqlx::query("UPDATE native_game_signing_trust_key_versions SET public_key=$1")
        .bind(&original)
        .execute(&db.pool)
        .await?;
    // The active kid is not trusted: unknown to the registry, or not yet trusted.
    db.execute("UPDATE native_game_signing_trust_key_versions SET key_id='fresh-t19c-other'")
        .await?;
    refuses().await?;
    db.execute("UPDATE native_game_signing_trust_key_versions SET key_id='fresh-t19c-1'")
        .await?;
    db.execute("UPDATE native_game_signing_trust_key_versions SET trusted_at=clock_timestamp()+interval '1 hour'")
        .await?;
    refuses().await?;
    db.execute("UPDATE native_game_signing_trust_key_versions SET trusted_at=clock_timestamp()-interval '10 seconds'")
        .await?;
    // The trust snapshot is unavailable: the registry and its witness disagree.
    db.execute("UPDATE native_game_signing_trust_profiles SET issuer_revision=issuer_revision+1")
        .await?;
    refuses().await?;
    db.execute("UPDATE native_game_signing_trust_profiles SET issuer_revision=issuer_revision-1")
        .await?;
    // Another key purpose: custody or registry scope does not match.
    let other = fresh(&[(admission::KEY_PURPOSE, "fresh_other")])?;
    unavailable(db.admission_with(other)?, login).await?;
    db.nothing_committed(&native).await?;

    // The restored registry trusts the key again; a snapshot older than the
    // §9.2 bound (5 s) never authorizes a signature.
    let signer = fresh(&[])?.signer.expect("custody opens");
    signer.prepare().await?;
    let now = u64::try_from(db.now().await?)?;
    signer.sign(b"{}", now + 20, now)?;
    for later in [now + 5, now + 60] {
        assert!(matches!(
            signer.sign(b"{}", later + 20, later),
            Err(admission::Error::Unavailable)
        ));
    }
    drop(signer);

    // A revoked kid refuses issuance.
    let witness = grants.lookup(&[])(admission::WITNESS_DIRECTORY).expect("witness");
    let registry = keys::PublicTrustRegistry::new(
        db.pool.clone(),
        keys::Settings {
            enabled: true,
            key_purpose: PURPOSE.into(),
            clock_uncertainty_seconds: 1,
        },
        witness.into(),
    )?;
    registry.revoke(KID).await?;
    refuses().await?;

    // configured_signer: no key is loaded unless issuance is permitted, and a
    // custody failure names no path or secret.
    for policy in [
        admission::Issuance::default(),
        admission::Issuance {
            environment: "production".into(),
            ..grants.issuance.clone()
        },
    ] {
        assert!(matches!(
            admission::configured_signer(db.pool.clone(), &policy, grants.lookup(&[])),
            Ok(None)
        ));
    }
    let dir = tempfile::tempdir()?;
    let short = dir.path().join("short-wrapping-key");
    std::fs::write(&short, b"0123")?;
    let wrong = dir.path().join("wrong-wrapping-key");
    std::fs::write(&wrong, Sha256::digest(b"another wrapping key"))?;
    {
        use std::os::unix::fs::PermissionsExt;
        for file in [&short, &wrong] {
            std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600))?;
        }
    }
    let short = short.to_string_lossy().into_owned();
    let wrong = wrong.to_string_lossy().into_owned();
    let custody = grants.lookup(&[])(admission::KEY_CUSTODY_FILE).expect("custody");
    for overrides in [
        vec![(admission::KEY_CUSTODY_FILE, "/nonexistent/custody-secret")],
        vec![(admission::KEY_WRAPPING_FILE, "/nonexistent/wrapping-secret")],
        vec![(admission::KEY_WRAPPING_FILE, short.as_str())],
        vec![(admission::KEY_WRAPPING_FILE, wrong.as_str())],
        vec![(admission::KEY_CUSTODY_FILE, "")],
        vec![(admission::WITNESS_DIRECTORY, "")],
        vec![(admission::CLOCK_UNCERTAINTY_SECONDS, "6")],
    ] {
        let error = admission::configured_signer(
            db.pool.clone(),
            &grants.issuance,
            grants.lookup(&overrides),
        )
        .expect_err("custody must not open");
        let text = format!("{error} {error:?}");
        for (_, value) in overrides.iter().filter(|(_, v)| !v.is_empty()) {
            assert!(!text.contains(value), "{text}");
        }
        assert!(
            !text.contains(&custody) && !text.contains("secret"),
            "{text}"
        );
    }
    db.nothing_committed(&native).await?;
    db.close().await
}

#[tokio::test]
async fn pg_grant_ttl_bounds_hold_end_to_end() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let mut grants = db.grants().await?;
    // Custody is exclusive to one open signer; each TTL below opens its own.
    grants.issuance.signer = None;
    let account = db.account().await?;
    db.publish(account).await?;
    for ttl in [21_i64, 25, 30] {
        let text = ttl.to_string();
        let mut issuance = issuance(
            db.pool.clone(),
            grants.lookup(&[(admission::GRANT_TTL_SECONDS, &text)]),
        );
        assert!(issuance.permitted() && issuance.signer.is_some(), "{ttl}");
        issuance.runtime = runtime_settings()?;
        let native = db.native_ticket(account).await?;
        let raw = login_body(&native, Uuid::now_v7(), CHARACTER);
        let (status, body) = admit(db.admission_with(issuance)?, SERVICE, raw.clone()).await?;
        assert_eq!(
            status,
            StatusCode::OK,
            "{ttl}: {}",
            String::from_utf8_lossy(&body)
        );
        let (_, claims, valid_for) = granted(&raw, &body)?;
        let (iat, exp) = (
            claims["iat"].as_i64().expect("iat"),
            claims["exp"].as_i64().expect("exp"),
        );
        assert_eq!(exp - iat, ttl);
        assert!((ttl - 2..=ttl).contains(&valid_for), "{ttl}: {valid_for}");
    }
    // Out of range: issuance is disabled and the key is never loaded.
    let native = db.native_ticket(account).await?;
    for ttl in ["4", "31"] {
        let mut issuance = issuance(
            db.pool.clone(),
            grants.lookup(&[(admission::GRANT_TTL_SECONDS, ttl)]),
        );
        assert!(!issuance.permitted() && issuance.signer.is_none(), "{ttl}");
        issuance.runtime = runtime_settings()?;
        unavailable(db.admission_with(issuance)?, |attempt| {
            login_body(&native, attempt, CHARACTER)
        })
        .await?;
    }
    assert!(!db.ticket_used(&native).await?);
    db.close().await
}

#[tokio::test]
async fn pg_replay_is_byte_identical_until_expiry_and_refuses_a_changed_route() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let grants = db.grants().await?;
    let account = db.account().await?;
    db.publish(account).await?;
    let app = db.admission_with(grants.issuance.clone())?;
    let native = db.native_ticket(account).await?;
    let attempt = Uuid::now_v7();
    let raw = login_body(&native, attempt, CHARACTER);
    let (status, first) = admit(app.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&first)
    );
    let (token, claims, valid_for) = granted(&raw, &first)?;
    // A later retry returns the same token with less time left.
    db.after(claims["iat"].as_i64().expect("iat") + 1).await?;
    let (status, again) = admit(app.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    let (replayed, _, remaining) = granted(&raw, &again)?;
    assert_eq!(replayed.as_bytes(), token.as_bytes());
    assert!(remaining < valid_for, "{remaining} < {valid_for}");

    // The bound route record changed between issue and replay.
    let route = |version: i64, host: &str| -> Result<String, Box<dyn std::error::Error>> {
        Ok(admission::route_revision(
            u64::try_from(version)?,
            WORLD.parse()?,
            CHANNEL.parse()?,
            host,
            7172,
            host,
        )?)
    };
    let route_unavailable = refusal(
        "NATIVE_LOGIN_ROUTE_UNAVAILABLE",
        "TEMPORARILY_UNAVAILABLE",
        attempt,
    );
    for (version, host) in [(2, ROUTE_HOST), (2, "game-eu2.example.test")] {
        sqlx::query("UPDATE game_channels SET native_route_version=$1, native_route_host=$2, native_route_tls_server_name=$2, native_route_revision=$3")
            .bind(version).bind(host).bind(route(version, host)?).execute(&db.pool).await?;
        let (status, body) = admit(app.clone(), SERVICE, raw.clone()).await?;
        assert_eq!(
            (status, &body),
            (StatusCode::SERVICE_UNAVAILABLE, &route_unavailable)
        );
    }
    sqlx::query("UPDATE game_channels SET native_route_version=1, native_route_host=$1, native_route_tls_server_name=$1, native_route_revision=$2")
        .bind(ROUTE_HOST).bind(&grants.revision).execute(&db.pool).await?;
    let (status, body) = admit(app.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(granted(&raw, &body)?.0, token);

    // After expiry the same attempt is refused as expired.
    let short = admission::Issuance {
        ttl_seconds: 5,
        ..grants.issuance.clone()
    };
    let app = db.admission_with(short)?;
    let native = db.native_ticket(account).await?;
    let attempt = Uuid::now_v7();
    let raw = login_body(&native, attempt, CHARACTER);
    let (status, body) = admit(app.clone(), SERVICE, raw.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let (_, claims, _) = granted(&raw, &body)?;
    db.after(claims["exp"].as_i64().expect("exp") - 1).await?;
    let (status, body) = admit(app, SERVICE, raw).await?;
    assert_eq!(
        (status, body),
        (
            StatusCode::CONFLICT,
            refusal("NATIVE_LOGIN_GRANT_EXPIRED", "RETRY_LOGIN", attempt)
        )
    );
    db.close().await
}

#[tokio::test]
async fn pg_runtime_report_not_ready_or_off_profile_refuses_issuance() -> TestResult {
    use oteryn_internal_api::canonical::Value;
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let grants = db.grants().await?;
    let account = db.account().await?;
    db.publish(account).await?;
    let native = db.native_ticket(account).await?;
    let app = db.admission_with(grants.issuance.clone())?;
    let login = |attempt| login_body(&native, attempt, CHARACTER);
    db.report_with(&grants.revision, 2, &[("ready", Value::Bool(false))])
        .await?;
    no_route(app.clone(), login).await?;
    db.report(&grants.revision, 3).await?;
    // Ingestion refuses another protocol or transport; a stored report that
    // carries one never routes.
    for (from, to) in [
        (r#""protocol_major":1"#, r#""protocol_major":2"#),
        (r#""transport_profile":1"#, r#""transport_profile":2"#),
    ] {
        sqlx::query(
            "UPDATE native_runtime_status_reports SET content_json=replace(content_json,$1,$2)",
        )
        .bind(from)
        .bind(to)
        .execute(&db.pool)
        .await?;
        unavailable(app.clone(), login).await?;
        sqlx::query(
            "UPDATE native_runtime_status_reports SET content_json=replace(content_json,$2,$1)",
        )
        .bind(from)
        .bind(to)
        .execute(&db.pool)
        .await?;
    }
    db.nothing_committed(&native).await?;
    let (status, body) = admit(app, SERVICE, login(Uuid::now_v7())).await?;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    db.close().await
}

#[tokio::test]
async fn pg_route_selection_filters_the_channel_first_and_locks_only_its_record() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let grants = db.grants().await?;
    let account = db.account().await?;
    db.publish(account).await?;
    let app = db.admission_with(grants.issuance.clone())?;
    // 64 enabled records that sort before the bound channel fill the bound.
    db.execute("INSERT INTO game_channels (world_id,channel_id,native_route_host,native_route_port,native_route_tls_server_name,native_route_version,native_route_revision,native_login_enabled) SELECT '01934f10-7c02-7001-805b-3b1122334401', ('01934f10-7c03-7001-805b-'||lpad(to_hex(g),12,'0'))::uuid, 'other.example.test', 7172, 'other.example.test', 1, 'rt.1.00000000000000000000000000000000', true FROM generate_series(1,64) g")
        .await?;
    // Another scope's record is held by a concurrent writer.
    let mut writer = db.pool.begin().await?;
    sqlx::query("SELECT channel_id FROM game_channels WHERE channel_id='01934f10-7c03-7001-805b-000000000001' FOR UPDATE")
        .execute(&mut *writer)
        .await?;
    let native = db.native_ticket(account).await?;
    let attempt = Uuid::now_v7();
    let raw = login_body(&native, attempt, CHARACTER).replace(
        r#""channel_id":null"#,
        &format!(r#""channel_id":"{CHANNEL}""#),
    );
    // A selection that locks every record blocks on the writer: fail, not hang.
    let (status, body) = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        admit(app.clone(), SERVICE, raw.clone()),
    )
    .await
    .map_err(|_| "route selection blocked on another scope's record")??;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(granted(&raw, &body)?.1["channel_id"], CHANNEL);
    writer.rollback().await?;
    db.close().await
}

#[tokio::test]
async fn pg_grant_is_never_stored_without_the_composed_replay_signer() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let grants = db.grants().await?;
    let account = db.account().await?;
    db.publish(account).await?;
    let native = db.native_ticket(account).await?;
    // The authority signs, but the attempt store has no signer to re-sign with.
    let app = db.admission_composed(grants.issuance.clone(), None)?;
    unavailable(app, |attempt| login_body(&native, attempt, CHARACTER)).await?;
    db.nothing_committed(&native).await?;
    db.close().await
}
