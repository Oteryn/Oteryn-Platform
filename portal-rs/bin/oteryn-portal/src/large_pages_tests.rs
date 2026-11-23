//! Real PostgreSQL fixtures through the same module routers and decorators as Serve.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    response::Response,
};
use ed25519_dalek::{Signer, SigningKey};
use oteryn_admin::{
    rbac::{AccountState, AccountStateReader, AccountStateUnavailable},
    web::{PanelConfig, StaffIdentity, StaffSession},
};
use oteryn_minigames::league::{self, Authority};
use prost::Message;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use std::time::Instant;
use tower::ServiceExt;
use uuid::Uuid;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Database {
    pool: PgPool,
    root: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Self> {
        let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
        let root = PgPool::connect(&url).await?;
        let schema = format!("l5a_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(4)
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
        oteryn_db::migrate(&pool).await?;
        Ok(Self { pool, root, schema })
    }
    async fn close(self) -> Result {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await?;
        self.root.close().await;
        Ok(())
    }
    async fn staff(&self) -> Result<Staff> {
        let staff = Staff(Uuid::now_v7());
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(staff.0).execute(&self.pool).await?;
        Ok(staff)
    }
}
#[derive(Clone)]
struct Staff(Uuid);
impl StaffSession for Staff {
    async fn resolve(&self, _: &oteryn_web::session::BrowserSession) -> Option<StaffIdentity> {
        Some(StaffIdentity {
            account: self.0,
            mfa_satisfied: true,
        })
    }
}
impl AccountStateReader for Staff {
    // Synthetic session authority for this fixture; production denies missing proofs.
    async fn authenticated_in(
        &self,
        connection: &mut sqlx::PgConnection,
        context: oteryn_admin::rbac::CheckContext,
    ) -> std::result::Result<bool, oteryn_admin::rbac::AccountStateUnavailable> {
        if let Some(proof) = context.session_proof {
            oteryn_admin::rbac::durable_session_in(connection, context.account, proof).await
        } else {
            Ok(!context.require_staff_mfa || context.mfa_verified)
        }
    }

    async fn state(&self, _: Uuid) -> std::result::Result<AccountState, AccountStateUnavailable> {
        Ok(AccountState::Active)
    }
}
fn panel() -> PanelConfig {
    PanelConfig {
        admin_panel: true,
        host: "staff.test".into(),
    }
}
fn compose(app: Router, pool: PgPool) -> Router {
    let app = app.layer(axum::middleware::from_fn_with_state(
        pool.clone(),
        oteryn_content::announcements::banner,
    ));
    let app = oteryn_community::today::mount(
        app,
        pool.clone(),
        oteryn_community::today::Config {
            public_enabled: true,
            source_authority: "oteryn:game:primary".into(),
            worlds: [Uuid::now_v7()].into(),
            ..Default::default()
        },
    );
    app.layer(axum::middleware::from_fn_with_state(
        oteryn_web::chrome::Shell {
            registry: oteryn_ui::Registry::default(),
            sessions: oteryn_web::session::SessionLayer::new(pool),
        },
        oteryn_web::chrome::middleware,
    ))
}
async fn fetch(app: Router, path: String) -> Result<(Response, Vec<u8>)> {
    let response = app
        .oneshot(
            Request::get(path)
                .header(header::HOST, "staff.test")
                .body(Body::empty())?,
        )
        .await?;
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, 20 * 1024 * 1024).await?.to_vec();
    Ok((Response::from_parts(parts, Body::empty()), bytes))
}

#[tokio::test]
async fn l5_wiki_five_bilingual_large_revisions_are_complete() -> Result {
    let started = Instant::now();
    let db = Database::new().await?;
    let staff = db.staff().await?;
    let guide = Uuid::now_v7();
    sqlx::query("INSERT INTO wiki_guides(id,version,revision_version) VALUES($1,5,5)")
        .bind(guide)
        .execute(&db.pool)
        .await?;
    for version in 1..=5i64 {
        for locale in ["pl", "en"] {
            let translation = oteryn_wiki::domain::Translation {
                locale: locale.into(),
                title: "Large guide".into(),
                slug: "large-guide".into(),
                summary: "Revision history".into(),
                markdown: "&".repeat(100_000),
            };
            sqlx::query("INSERT INTO wiki_revisions(id,guide_id,version,locale,title,slug,summary,markdown,editor,change_note) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,'large revision')")
            .bind(Uuid::now_v7()).bind(guide).bind(version).bind(locale).bind(translation.title).bind(translation.slug).bind(translation.summary).bind(translation.markdown).bind(staff.0).execute(&db.pool).await?;
        }
    }
    let routes = oteryn_web::router(
        oteryn_web::AppState {
            db: db.pool.clone(),
        },
        "static",
    )
    .merge(oteryn_wiki::web::staff_router(
        db.pool.clone(),
        panel(),
        staff.clone(),
        staff,
    ));
    let (response, bytes) = fetch(
        compose(routes, db.pool.clone()),
        format!("/admin/wiki/articles/{guide}/revisions"),
    )
    .await?;
    db.close().await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(bytes.len() > 4 * 1024 * 1024);
    let html = std::str::from_utf8(&bytes)?;
    assert_eq!(html.matches("&#38;").count(), 1_000_000);
    assert!(html.ends_with("</html>"));
    eprintln!(
        "l5_wiki_five_bilingual_large_revisions_are_complete: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn l5_world_large_comparison_is_complete() -> Result {
    let started = Instant::now();
    let db = Database::new().await?;
    let staff = db.staff().await?;
    let old = Uuid::now_v7();
    let new = Uuid::now_v7();
    for id in [old, new] {
        sqlx::query(
            "INSERT INTO world_versions(id,source_commit,status,document_count) VALUES($1,repeat('a',40),'ready',100)",
        )
        .bind(id)
        .execute(&db.pool)
        .await?;
    }
    for i in 0..100 {
        for (id, prefix) in [(old, "old"), (new, "new")] {
            let data = serde_json::json!({"description": format!("{prefix}{}", "&".repeat(5000))});
            sqlx::query("INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES($1,'monster',$2,$3,true)").bind(id).bind(format!("creature-{i:03}")).bind(data).execute(&db.pool).await?;
        }
    }
    let routes = oteryn_web::router(
        oteryn_web::AppState {
            db: db.pool.clone(),
        },
        "static",
    )
    .merge(oteryn_world::admin::router(
        db.pool.clone(),
        panel(),
        oteryn_world::WorldOptions { enabled: true },
        staff.clone(),
        staff,
    ));
    let (response, bytes) = fetch(
        compose(routes, db.pool.clone()),
        format!("/admin/world?version={new}&compare={old}"),
    )
    .await?;
    db.close().await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(bytes.len() > 4 * 1024 * 1024);
    let html = std::str::from_utf8(&bytes)?;
    assert_eq!(html.matches("&#38;").count(), 1_000_000);
    assert!(html.ends_with("</html>"));
    eprintln!(
        "l5_world_large_comparison_is_complete: {:?}",
        started.elapsed()
    );
    Ok(())
}

// This test adapter checks real Ed25519 signatures over the admitted snapshot.
struct SignedAuthority;
impl Authority for SignedAuthority {
    async fn fence(
        &self,
        _: &mut Transaction<'_, Postgres>,
        now: u64,
    ) -> league::Result<league::Fence> {
        Ok(league::Fence {
            epoch: 3,
            recovery_generation: 1,
            complete_through: now,
        })
    }
    async fn verify(
        &self,
        _: &mut Transaction<'_, Postgres>,
        d: &league::Delivery<'_>,
        _: &league::schema::Snapshot,
    ) -> league::Result<()> {
        league::verify_ed25519(
            SigningKey::from_bytes(&[42; 32]).verifying_key().as_bytes(),
            d.wire,
            d.signature,
        )
    }
    async fn verify_served(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        s: &league::schema::Snapshot,
        wire: &[u8],
    ) -> league::Result<()> {
        let signature: Vec<u8> = sqlx::query_scalar(
            "SELECT signature FROM league_audit WHERE subject_id=$1 AND revision::text=$2",
        )
        .bind(Uuid::parse_str(s.subject()).map_err(|_| league::Error::Invalid)?)
        .bind(s.revision())
        .fetch_one(&mut **tx)
        .await?;
        league::verify_ed25519(
            SigningKey::from_bytes(&[42; 32]).verifying_key().as_bytes(),
            wire,
            &signature,
        )
    }
}
#[tokio::test]
async fn l5_league_fourteen_full_seasons_are_complete() -> Result {
    let started = Instant::now();
    let db = Database::new().await?;
    let now = u64::try_from(time::OffsetDateTime::now_utc().unix_timestamp())?;
    let league = league::League::new(
        db.pool.clone(),
        league::Flags {
            enabled: true,
            ingest: true,
            public: true,
            accepted_sources: true,
            accepted_season_rules: true,
            accepted_final_freeze: true,
            accepted_pki: true,
            ..league::Flags::default()
        },
        SignedAuthority,
    );
    let vectors: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../crates/minigames/src/league/fixtures/snapshots.json"
    ))?;
    let vector = vectors["vectors"]
        .as_array()
        .ok_or("missing vectors")?
        .iter()
        .find(|v| v["name"] == "tied_guilds_frozen")
        .ok_or("missing season")?;
    let base: league::schema::SeasonSnapshot =
        serde_json::from_str(vector["wire"].as_str().ok_or("missing wire")?)?;
    for _ in 0..14 {
        let mut season = base.clone();
        let id = Uuid::now_v7();
        season.season_id = id.to_string();
        season.source_observed_at = now.to_string();
        let row = season.season.first_mut().ok_or("missing row")?;
        let guild = row.standings.first().ok_or("missing guild")?.clone();
        let base_id = Uuid::parse_str(&guild.guild_id)?.as_u128();
        row.standings = (0..1000)
            .map(|i| {
                let mut guild = guild.clone();
                guild.guild_id = Uuid::from_u128(base_id + i).to_string();
                guild.guild_name = "&".repeat(64);
                guild.rank = (i + 1).to_string();
                guild
            })
            .collect();
        let wire = serde_json::to_vec(&season)?;
        let snapshot = league::schema::Snapshot::decode(&wire, true, now)?;
        let payload = league::GuildSeasonStandingsDecidedV1 {
            record_sha256: Sha256::digest(snapshot.record()?).to_vec(),
            recovery_generation: 1,
            season_id: id.as_bytes().to_vec(),
        }
        .encode_to_vec();
        let signature = SigningKey::from_bytes(&[42; 32]).sign(&wire).to_bytes();
        league
            .ingest(
                league::Delivery {
                    envelope_id: Uuid::now_v7(),
                    transaction_id: Uuid::now_v7(),
                    ordinal: 1,
                    count: 1,
                    event_type: 11,
                    occurred_at_unix_ms: i64::try_from(now * 1000)?,
                    payload: &payload,
                    wire: &wire,
                    signature: &signature,
                    recovery_generation: 1,
                    finalized_at: Some(now),
                },
                now,
            )
            .await?;
    }
    assert_eq!(league.read(now).await?.len(), 14);
    let routes = oteryn_web::router(
        oteryn_web::AppState {
            db: db.pool.clone(),
        },
        "static",
    )
    .merge(league::web::router(league));
    let (response, bytes) = fetch(compose(routes, db.pool.clone()), "/en/league".into()).await?;
    db.close().await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(bytes.len() > 4 * 1024 * 1024);
    let html = std::str::from_utf8(&bytes)?;
    assert_eq!(html.matches("&#38;").count(), 14 * 1000 * 64);
    assert!(html.ends_with("</html>"));
    eprintln!(
        "l5_league_fourteen_full_seasons_are_complete: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn l5_all_decorators_preserve_large_status_headers_and_body() -> Result {
    let started = Instant::now();
    let db = Database::new().await?;
    let document = format!(
        "<html><body><main>{}</main></body></html>",
        "&amp;".repeat(1_000_000)
    );
    let expected = document.clone();
    let routes = Router::new().route(
        "/en/news/large",
        axum::routing::get(move || {
            let document = document.clone();
            async move {
                let length = document.len();
                let mut response = Response::new(Body::from(document));
                *response.status_mut() = StatusCode::ACCEPTED;
                response.headers_mut().insert(
                    header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_static("text/html"),
                );
                response
                    .headers_mut()
                    .insert(header::CONTENT_LENGTH, length.into());
                response.headers_mut().insert(
                    "x-original",
                    axum::http::HeaderValue::from_static("preserved"),
                );
                response
            }
        }),
    );
    let (response, bytes) =
        fetch(compose(routes, db.pool.clone()), "/en/news/large".into()).await?;
    db.close().await?;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(response.headers()["x-original"], "preserved");
    assert_eq!(
        response.headers()[header::CONTENT_LENGTH].to_str()?,
        expected.len().to_string()
    );
    assert_eq!(bytes, expected.as_bytes());
    eprintln!(
        "l5_all_decorators_preserve_large_status_headers_and_body: {:?}",
        started.elapsed()
    );
    Ok(())
}
