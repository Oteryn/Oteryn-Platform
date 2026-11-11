use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use oteryn_gameauth::tickets::{AccountStatus, OAuthAuthority, Principal, ResultFuture};
use sqlx::{PgConnection, PgPool, Row, postgres::PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;
// SOURCE: Oteryn/Oteryn-Game@857f15f376907c592bb28d854fccff18a15cdf87,
// crates/platform-client/src/native_login/tests.rs, owner character response shape.
const GOLDEN_EMPTY: &[u8] = br#"{"protocol_version":2,"characters":[]}"#;
const ACCOUNT: &str = "0190f2a1-3b4c-7d5e-8f60-718293a4b5c6";

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("PostgreSQL test skipped: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let schema = format!("t17_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |connection, _| {
                let sql = format!("SET search_path TO {path}");
                Box::pin(async move {
                    sqlx::query(sqlx::AssertSqlSafe(sql))
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        sqlx::query("CREATE TABLE test_oauth (token text PRIMARY KEY, active boolean NOT NULL DEFAULT true, revoked boolean NOT NULL DEFAULT false, refresh_revoked boolean NOT NULL DEFAULT false)").execute(&pool).await?;
        sqlx::query("INSERT INTO test_oauth (token) VALUES ('access-1')")
            .execute(&pool)
            .await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    fn api(&self) -> Api {
        Api {
            tickets: Tickets::new(
                self.pool.clone(),
                Settings {
                    enabled: true,
                    nonproduction: true,
                },
                Arc::new(TestOAuth),
            ),
            projection: None,
        }
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
#[derive(Debug)]
struct TestOAuth;
impl OAuthAuthority for TestOAuth {
    fn verify<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        bearer: &'a str,
        consume: bool,
    ) -> ResultFuture<'a, Principal> {
        Box::pin(async move {
            let row =
                sqlx::query("SELECT active, revoked FROM test_oauth WHERE token = $1 FOR UPDATE")
                    .bind(bearer)
                    .fetch_optional(&mut *connection)
                    .await
                    .map_err(|_| Error::Unavailable)?
                    .ok_or(Error::Unauthorized)?;
            if row
                .try_get::<bool, _>("revoked")
                .map_err(|_| Error::Unavailable)?
            {
                return Err(Error::Unauthorized);
            }
            let active = row
                .try_get::<bool, _>("active")
                .map_err(|_| Error::Unavailable)?;
            if consume {
                sqlx::query(
                    "UPDATE test_oauth SET revoked = true, refresh_revoked = true WHERE token = $1",
                )
                .bind(bearer)
                .execute(connection)
                .await
                .map_err(|_| Error::Unavailable)?;
            }
            Ok(Principal {
                account_id: Uuid::parse_str(ACCOUNT).map_err(|_| Error::Unavailable)?,
                status: if active {
                    AccountStatus::Active
                } else {
                    AccountStatus::Suspended
                },
                game_generation: 2,
                native_generation: 3,
            })
        })
    }
}
async fn call(app: Router, method: &str, path: &str, body: &str, auth: bool) -> Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if auth {
        request = request.header(header::AUTHORIZATION, "Bearer access-1");
    }
    let request = request
        .body(Body::from(body.to_owned()))
        .map_err(|error| error.to_string());
    match request {
        Ok(request) => app
            .oneshot(request)
            .await
            .unwrap_or_else(|never| match never {}),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error).into_response(),
    }
}
const TICKETS: &str = "/api/v1/game-auth/tickets";
const CHARACTERS: &str = "/api/v1/game-auth/native-characters";

#[tokio::test]
async fn postgres_ticket_consumes_once_and_stores_only_digest() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let app = router(db.api());
    for body in [
        r#"{"protocol_version":2}"#,
        r#"{"protocol_version":1,"account_id":"x"}"#,
        r#"{"protocol_version":1,"protocol_version":1}"#,
        "null",
        "{}",
    ] {
        assert_eq!(
            call(app.clone(), "POST", TICKETS, body, true)
                .await
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let response = call(
        app.clone(),
        "POST",
        TICKETS,
        r#"{"protocol_version":1}"#,
        true,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "no-store, private"
    );
    let payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 2048).await?)?;
    assert_eq!(payload["protocol_version"], 1);
    assert_eq!(payload["expires_in"], 60);
    let ticket = payload["ticket"].as_str().ok_or("ticket missing")?;
    assert_eq!(ticket.len(), 43);
    let row = sqlx::query("SELECT ticket_hash, audience, native_generation, expires_at > clock_timestamp() AS live FROM game_login_tickets").fetch_one(&db.pool).await?;
    assert_ne!(row.try_get::<Vec<u8>, _>("ticket_hash")?, ticket.as_bytes());
    assert_eq!(
        row.try_get::<String, _>("audience")?,
        "oteryn-native-game-gateway"
    );
    assert_eq!(row.try_get::<String, _>("native_generation")?, "3");
    assert!(row.try_get::<bool, _>("live")?);
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT revoked AND refresh_revoked FROM test_oauth")
            .fetch_one(&db.pool)
            .await?
    );
    assert_eq!(
        call(app, "POST", TICKETS, r#"{"protocol_version":1}"#, true)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    db.close().await
}
#[tokio::test]
async fn postgres_suspended_account_and_failed_insert_roll_back_consumption() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    sqlx::query("UPDATE test_oauth SET active = false")
        .execute(&db.pool)
        .await?;
    let app = router(db.api());
    assert_eq!(
        call(
            app.clone(),
            "POST",
            TICKETS,
            r#"{"protocol_version":1}"#,
            true
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT revoked FROM test_oauth")
            .fetch_one(&db.pool)
            .await?
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_login_tickets")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    sqlx::query("UPDATE test_oauth SET active = true")
        .execute(&db.pool)
        .await?;
    sqlx::query("ALTER TABLE game_login_tickets ADD CONSTRAINT refuse_insert CHECK (false)")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        call(app, "POST", TICKETS, r#"{"protocol_version":1}"#, true)
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT revoked OR refresh_revoked FROM test_oauth")
            .fetch_one(&db.pool)
            .await?
    );
    db.close().await
}
#[tokio::test]
async fn bad_input_authentication_and_default_flag() -> TestResult {
    let pool = PgPoolOptions::new().connect_lazy("postgresql://localhost/unused")?;
    let app = configured_router(
        pool,
        Arc::new(oteryn_gameauth::tickets::DenyOAuth),
        Arc::new(ActiveAccounts),
    );
    for body in [
        r#"{"protocol_version":2}"#,
        r#"{"protocol_version":1,"account_id":"x"}"#,
        r#"{"protocol_version":1,"protocol_version":1}"#,
        "null",
        "{}",
        r#"{"protocol_version":"1"}"#,
    ] {
        assert!(!matches!(
            serde_json::from_str::<TicketRequest>(body),
            Ok(TicketRequest {
                protocol_version: 1
            })
        ));
    }
    assert_eq!(
        call(
            app.clone(),
            "POST",
            TICKETS,
            r#"{"protocol_version":1}"#,
            false
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            app.clone(),
            "POST",
            TICKETS,
            r#"{"protocol_version":1}"#,
            true
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(app.clone(), "GET", CHARACTERS, "", true)
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        call(app, "POST", TICKETS, &"x".repeat(2049), true)
            .await
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    Ok(())
}
#[tokio::test]
async fn postgres_budget_shared_and_separate_by_purpose() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let service = db.api().tickets;
    for _ in 0..5 {
        assert!(service.budget("access-1", true).await.is_ok());
    }
    assert_eq!(
        db.api().tickets.budget("access-1", true).await,
        Err(Error::RateLimited)
    );
    assert!(service.budget("access-1", false).await.is_ok());
    db.close().await
}
#[derive(Debug)]
struct ActiveAccounts;
impl oteryn_characters::lcfa::AccountStateLookup for ActiveAccounts {
    fn state(
        &self,
        _: AccountId,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = oteryn_characters::lcfa::AccountState> + Send + '_>,
    > {
        Box::pin(async { oteryn_characters::lcfa::AccountState::Active })
    }
}
#[tokio::test]
async fn postgres_character_golden_owner_only_and_stale_refusal() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let mut api = db.api();
    api.projection = Some(Projection::new(
        db.pool.clone(),
        oteryn_characters::lcfa::Settings {
            enabled: true,
            environment: oteryn_core::Environment::Testing,
            source_authority: "oteryn:character-authority:primary".into(),
            ..Default::default()
        },
        Arc::new(ActiveAccounts),
    ));
    let app = router(api);
    assert_eq!(
        call(app.clone(), "GET", CHARACTERS, "", true)
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    sqlx::query("UPDATE account_character_feed SET highest_epoch = '1', watermark_epoch = '1', source_authority = 'oteryn:character-authority:primary', complete_through = extract(epoch FROM clock_timestamp())::bigint, observed_at = extract(epoch FROM clock_timestamp())::bigint").execute(&db.pool).await?;
    let response = call(app.clone(), "GET", CHARACTERS, "", true).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 16384).await?.as_ref(),
        GOLDEN_EMPTY
    );
    sqlx::query("INSERT INTO account_character_snapshots (account_id, source_authority, projection_epoch, projection_revision, source_observed_at, content_digest) VALUES ($1,'oteryn:character-authority:primary','1','1',0,decode(repeat('00',32),'hex'))").bind(Uuid::parse_str(ACCOUNT)?).execute(&db.pool).await?;
    sqlx::query("INSERT INTO account_characters (account_id, character_id, world_id, name, availability) VALUES ($1,'01890a5d-ac96-774b-bcce-b302099a8058','01890a5d-ac96-774b-bcce-b302099a8059','Ari','AVAILABLE'), ('0190f2a1-3b4c-7d5e-8f60-718293a4b5c7','01890a5d-ac96-774b-bcce-b302099a805a','01890a5d-ac96-774b-bcce-b302099a8059','Other','UNAVAILABLE')").bind(Uuid::parse_str(ACCOUNT)?).execute(&db.pool).await?;
    let response = call(app.clone(), "GET", CHARACTERS, "", true).await;
    assert_eq!(response.status(), StatusCode::OK);
    // SOURCE: same pinned Game response vector as GOLDEN_EMPTY above.
    let golden = br#"{"protocol_version":2,"characters":[{"character_id":"01890a5d-ac96-774b-bcce-b302099a8058","world_id":"01890a5d-ac96-774b-bcce-b302099a8059","name":"Ari","availability":"AVAILABLE"}]}"#;
    assert_eq!(
        to_bytes(response.into_body(), 16384).await?.as_ref(),
        golden
    );
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT revoked OR refresh_revoked FROM test_oauth")
            .fetch_one(&db.pool)
            .await?
    );
    sqlx::query("UPDATE account_character_snapshots SET invalid = true")
        .execute(&db.pool)
        .await?;
    let response = call(app.clone(), "GET", CHARACTERS, "", true).await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(to_bytes(response.into_body(), 100).await?.is_empty());
    sqlx::raw_sql("UPDATE account_character_snapshots SET invalid = false; UPDATE account_character_feed SET complete_through = 0").execute(&db.pool).await?;
    assert_eq!(
        call(app.clone(), "GET", CHARACTERS, "", true)
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    sqlx::query("UPDATE test_oauth SET active = false")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        call(app, "GET", CHARACTERS, "", true).await.status(),
        StatusCode::UNAUTHORIZED
    );
    db.close().await
}
#[test]
fn ticket_flag_is_off_by_default_and_refuses_production() {
    let lookup = |pairs: &'static [(&'static str, &'static str)]| {
        move |key: &str| {
            pairs
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| (*value).to_owned())
        }
    };
    let off = settings(lookup(&[]));
    assert!(!off.enabled && !off.nonproduction);
    for (pairs, available) in [
        (
            &[
                ("PORTAL_GAME_TICKETS_ENABLED", "true"),
                ("OTERYN_ENV", "testing"),
            ][..],
            true,
        ),
        (
            &[
                ("PORTAL_GAME_TICKETS_ENABLED", "true"),
                ("OTERYN_ENV", "preprod"),
            ][..],
            true,
        ),
        (
            &[
                ("PORTAL_GAME_TICKETS_ENABLED", "true"),
                ("OTERYN_ENV", "production"),
            ][..],
            false,
        ),
        (
            &[
                ("PORTAL_GAME_TICKETS_ENABLED", "TRUE"),
                ("OTERYN_ENV", "testing"),
            ][..],
            false,
        ),
        (&[("OTERYN_ENV", "testing")][..], false),
    ] {
        let settings = settings(lookup(pairs));
        assert_eq!(settings.enabled && settings.nonproduction, available);
    }
}
