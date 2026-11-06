use super::*;
use ed25519_dalek::{Signer, SigningKey};
use prost::Message;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, postgres::PgPoolOptions};
type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
const NOW: u64 = 1790000200;
fn vectors() -> std::result::Result<Vec<serde_json::Value>, serde_json::Error> {
    let v: serde_json::Value = serde_json::from_slice(include_bytes!("fixtures/snapshots.json"))?;
    Ok(v["vectors"].as_array().cloned().unwrap_or_default())
}
fn wire(name: &str) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
    vectors()?
        .iter()
        .find(|v| v["name"] == name)
        .and_then(|v| v["wire"].as_str())
        .map(|s| s.as_bytes().to_vec())
        .ok_or_else(|| "fixture missing".into())
}
fn flags() -> Flags {
    Flags {
        enabled: true,
        ingest: true,
        public: true,
        accepted_sources: true,
        accepted_season_rules: true,
        accepted_final_freeze: true,
        accepted_pki: true,
    }
}
#[test]
fn exact_pinned_files_and_wire_hashes() -> TestResult {
    for (bytes, expected) in [
        (
            include_bytes!("fixtures/snapshots.json").as_slice(),
            "e3a695d4ff9ca1e73bdd0f5a82184e098adb3b40970babe388bb2580c3586e30",
        ),
        (
            include_bytes!("fixtures/plat3_authority.proto").as_slice(),
            "d8a306474f5c01275e5d87c4fd056d8f6ccad2ec577e7640c6fd1c66b72dd642",
        ),
        (
            include_bytes!("fixtures/GAME_EVENT_FOUNDATION_REGISTRY.json").as_slice(),
            "a0ea5a1494b8ff464c15683d788e4e2360eb674dc0a6362ed17854465d0ddfb6",
        ),
        (
            include_bytes!("fixtures/CROSS_REPOSITORY_CONTRACT_LOCK.json").as_slice(),
            "fb9ed2f7e1f56d53336dc26409e37b94b1a5f27163544bcdab030ba1a90ff2ea",
        ),
    ] {
        assert_eq!(schema::hash(bytes), expected);
    }
    for v in vectors()? {
        let raw = v["wire"].as_str().ok_or("wire missing")?.as_bytes();
        assert_eq!(schema::hash(raw), v["sha256"]);
        let s = schema::Snapshot::decode(raw, v["name"] == "tied_guilds_frozen", NOW)?;
        if let Some(h) = v["record_sha256"].as_str() {
            assert_eq!(schema::hash(&s.record()?), h);
        }
    }
    Ok(())
}
#[test]
fn ed25519_over_exact_fixture_bytes() -> TestResult {
    let key = SigningKey::from_bytes(&[42; 32]);
    for v in vectors()? {
        let raw = v["wire"].as_str().ok_or("wire missing")?.as_bytes();
        let sig = key.sign(raw).to_bytes();
        verify_ed25519(key.verifying_key().as_bytes(), raw, &sig)?;
        let mut changed = raw.to_vec();
        changed.push(b'\n');
        assert!(verify_ed25519(key.verifying_key().as_bytes(), &changed, &sig).is_err());
        assert!(
            verify_ed25519(
                SigningKey::from_bytes(&[43; 32]).verifying_key().as_bytes(),
                raw,
                &sig
            )
            .is_err()
        );
        assert!(verify_ed25519(key.verifying_key().as_bytes(), raw, &[]).is_err());
    }
    Ok(())
}
#[test]
fn strict_grammar_and_missing_facts() -> TestResult {
    let raw = wire("settlement")?;
    let s = String::from_utf8(raw.clone())?;
    for bad in [s.replace("\"contract_version\":1","\"contract_version\":1,\"contract_version\":1"),s.replace("\"RAID\"","null"),s.replace("\"RAID\"","\"UNKNOWN\""),s.replace("\"evidence_sha256\":[\"1a21174f6bb3c2c0c7111bc7babae10659b8c9b1dc56f251cb0e0f45c46b6219\"]","\"evidence_sha256\":[]"),s.replace("\"3\"","\"03\""),format!("{s}\n"),s.replace("\"RAID\"","\"R\\u0041ID\"")]{assert!(schema::Snapshot::decode(bad.as_bytes(),false,NOW).is_err());}
    assert!(schema::Snapshot::decode(&raw, false, NOW - 1000).is_err());
    assert!(schema::number("18446744073709551616", true).is_err());
    assert!(schema::id("0192a000-0000-4000-8000-000000000012").is_err());
    Ok(())
}
#[test]
fn correction_chain_and_freeze() -> TestResult {
    let a = schema::Snapshot::decode(&wire("settlement")?, false, NOW)?;
    let b = schema::Snapshot::decode(&wire("correction")?, false, NOW)?;
    let c = schema::Snapshot::decode(&wire("final_freeze")?, false, NOW)?;
    store::transition(&a, &b)?;
    store::transition(&b, &c)?;
    assert!(store::transition(&b, &a).is_err());
    let mut bad = b.clone();
    if let schema::Snapshot::Event(s) = &mut bad {
        s.event[0].lock_at = "1790000080".into();
    }
    assert!(store::transition(&a, &bad).is_err());
    let mut second = b.clone();
    if let schema::Snapshot::Event(s) = &mut second {
        s.event[0].correction_of = vec![schema::hash(&b.record()?)];
        s.event[0].evidence_sha256 = vec!["ab".repeat(32)];
    }
    store::transition(&b, &second)?;
    assert!(store::transition(&a, &second).is_err());
    Ok(())
}
#[test]
fn standings_order_bounds_and_checked_sums() -> TestResult {
    let raw = String::from_utf8(wire("tied_guilds_frozen")?)?;
    for bad in [
        raw.replace("\"rank\":\"2\"", "\"rank\":\"1\""),
        raw.replace("\"total_score\":\"10\"", "\"total_score\":\"11\""),
        raw.replace("\"score\":\"10\"", "\"score\":\"101\""),
        raw.replace(
            "\"source_complete_through\":\"1790000100\"",
            "\"source_complete_through\":\"1790000099\"",
        ),
    ] {
        assert!(schema::Snapshot::decode(bad.as_bytes(), true, NOW).is_err());
    }
    Ok(())
}
#[test]
fn defaults_and_clock_watermark_fail_closed() {
    assert!(!Flags::default().ready());
    for complete in [0, NOW - 30, NOW + 2] {
        assert!(
            !Fence {
                epoch: 3,
                recovery_generation: 1,
                complete_through: complete
            }
            .valid(NOW)
        );
    }
    assert!(
        Fence {
            epoch: 3,
            recovery_generation: 1,
            complete_through: NOW - 29
        }
        .valid(NOW)
    );
    assert!(
        !Fence {
            epoch: 3,
            recovery_generation: 0,
            complete_through: NOW
        }
        .valid(NOW)
    );
}
#[derive(Debug)]
struct Trusted {
    generation: u64,
    complete: u64,
}
impl Authority for Trusted {
    async fn fence(&self, _: &mut Transaction<'_, Postgres>, _: u64) -> Result<Fence> {
        Ok(Fence {
            epoch: 3,
            recovery_generation: self.generation,
            complete_through: self.complete,
        })
    }
    async fn verify(
        &self,
        _: &mut Transaction<'_, Postgres>,
        d: &Delivery<'_>,
        _: &schema::Snapshot,
    ) -> Result<()> {
        verify_ed25519(
            SigningKey::from_bytes(&[42; 32]).verifying_key().as_bytes(),
            d.wire,
            d.signature,
        )
    }
}
fn payload(raw: &[u8], season: bool) -> Result<Vec<u8>> {
    let s = schema::Snapshot::decode(raw, season, NOW)?;
    let record = s.record()?;
    if season {
        Ok(GuildSeasonStandingsDecidedV1 {
            record_sha256: Sha256::digest(&record).to_vec(),
            recovery_generation: 1,
            season_id: schema::id(s.subject())?.as_bytes().to_vec(),
        }
        .encode_to_vec())
    } else {
        let schema::Snapshot::Event(e) = s else {
            return Err(Error::Invalid);
        };
        let hex = e.event[0]
            .evidence_sha256
            .first()
            .ok_or(Error::Unavailable)?;
        let evidence = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| Error::Invalid))
            .collect::<Result<Vec<_>>>()?;
        Ok(WorldEventOutcomeDecidedV1 {
            record: record.clone(),
            record_sha256: Sha256::digest(record).to_vec(),
            source_evidence_sha256: evidence,
            recovery_generation: 1,
            subject_event_id: schema::id(&e.event_id)?.as_bytes().to_vec(),
        }
        .encode_to_vec())
    }
}
fn delivery<'a>(
    raw: &'a [u8],
    payload: &'a [u8],
    sig: &'a [u8],
    id: Uuid,
    season: bool,
) -> Delivery<'a> {
    Delivery {
        envelope_id: id,
        transaction_id: id,
        ordinal: 1,
        count: 1,
        event_type: if season { 11 } else { 10 },
        occurred_at_unix_ms: (NOW * 1000) as i64,
        payload,
        wire: raw,
        signature: sig,
        recovery_generation: 1,
        finalized_at: Some(NOW),
    }
}
struct Db {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Db {
    async fn new() -> std::result::Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("league DB tests skipped: PORTAL_TEST_DATABASE_URL unset");
            return Ok(None);
        };
        let admin = PgPool::connect(&url).await?;
        let schema = format!("league_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let search = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(4)
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
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
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
#[tokio::test]
async fn database_replay_correction_freeze_and_retention() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let league = League::new(
        db.pool.clone(),
        flags(),
        Trusted {
            generation: 1,
            complete: NOW,
        },
    );
    for name in [
        "settlement",
        "correction",
        "final_freeze",
        "tied_guilds_frozen",
    ] {
        let season = name == "tied_guilds_frozen";
        let raw = wire(name)?;
        let p = payload(&raw, season)?;
        let sig = SigningKey::from_bytes(&[42; 32]).sign(&raw).to_bytes();
        let id = Uuid::now_v7();
        let (a, b) = tokio::join!(
            league.ingest(delivery(&raw, &p, &sig, id, season), NOW),
            league.ingest(delivery(&raw, &p, &sig, id, season), NOW)
        );
        a?;
        b?;
    }
    assert_eq!(league.read(NOW).await?.len(), 2);
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let wall = u64::try_from(time::OffsetDateTime::now_utc().unix_timestamp())?;
    let app = web::router(League::new(
        db.pool.clone(),
        flags(),
        Trusted {
            generation: 1,
            complete: wall,
        },
    ));
    for lang in ["pl", "en"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/{lang}/league"))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
        assert!(html.contains("Aldric Guild"));
        assert!(html.contains("Second Guild"));
        assert!(html.contains("timed-out"));
        assert!(!html.contains("<form"));
        if std::env::var("OTERYN_LEAGUE_RENDER_EVIDENCE").as_deref() == Ok("true") {
            std::fs::write(format!("/tmp/plat3-t94-{lang}.html"), html)?;
        }
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM league_audit")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 4);
    // Restart reconstructs exclusively from durable accepted evidence.
    let restarted = League::new(
        db.pool.clone(),
        flags(),
        Trusted {
            generation: 1,
            complete: NOW,
        },
    );
    assert_eq!(restarted.read(NOW).await?.len(), 2);
    let restored = League::new(
        db.pool.clone(),
        flags(),
        Trusted {
            generation: 2,
            complete: NOW,
        },
    );
    assert!(restored.read(NOW).await.is_err());
    let stale = League::new(
        db.pool.clone(),
        flags(),
        Trusted {
            generation: 1,
            complete: NOW - 30,
        },
    );
    assert!(stale.read(NOW).await.is_err());
    let raw = String::from_utf8(wire("final_freeze")?)?
        .replace(
            "\"projection_revision\":\"4\"",
            "\"projection_revision\":\"5\"",
        )
        .replace(
            "\"timed-out\"],\"decided_at\"",
            "\"completed\"],\"decided_at\"",
        )
        .into_bytes();
    let p = payload(&raw, false)?;
    let sig = SigningKey::from_bytes(&[42; 32]).sign(&raw).to_bytes();
    assert!(matches!(
        league
            .ingest(delivery(&raw, &p, &sig, Uuid::now_v7(), false), NOW)
            .await,
        Err(Error::Conflict)
    ));
    assert!(league.read(NOW).await.is_err());
    league.expire(NOW + 90 * 86400).await?;
    let audit: i64 = sqlx::query_scalar("SELECT count(*) FROM league_audit")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(audit, 0);
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM league_receipts")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(receipts, 4);
    db.close().await
}
#[tokio::test]
async fn unavailable_defaults_and_http() -> TestResult {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let pool = PgPoolOptions::new().connect_lazy("postgres://localhost/unused")?;
    let disabled = web::router(League::new(pool.clone(), Flags::default(), Unavailable));
    assert_eq!(
        disabled
            .oneshot(Request::builder().uri("/en/league").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    let app = web::router(League::new(
        pool,
        Flags {
            enabled: true,
            public: true,
            ..Flags::default()
        },
        Unavailable,
    ));
    for lang in ["pl", "en"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/{lang}/league"))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let body = to_bytes(response.into_body(), 100000).await?;
        assert!(String::from_utf8(body.to_vec())?.contains(&format!("lang=\"{lang}\"")));
    }
    Ok(())
}
#[tokio::test]
async fn database_bad_evidence_rolls_back_and_pair_conflict_quarantines() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let league = League::new(
        db.pool.clone(),
        flags(),
        Trusted {
            generation: 1,
            complete: NOW,
        },
    );
    let raw = wire("settlement")?;
    let p = payload(&raw, false)?;
    let sig = SigningKey::from_bytes(&[42; 32]).sign(&raw).to_bytes();
    let id = Uuid::now_v7();
    assert!(
        league
            .ingest(delivery(&raw, &p, &[0; 64], id, false), NOW)
            .await
            .is_err()
    );
    let mut bad = delivery(&raw, &p, &sig, id, false);
    bad.recovery_generation = 2;
    assert!(league.ingest(bad, NOW).await.is_err());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM league_receipts")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    league
        .ingest(delivery(&raw, &p, &sig, id, false), NOW)
        .await?;
    let altered = String::from_utf8(raw)?
        .replace("\"outcome\":[\"completed\"]", "\"outcome\":[\"timed-out\"]")
        .into_bytes();
    let p = payload(&altered, false)?;
    let sig = SigningKey::from_bytes(&[42; 32]).sign(&altered).to_bytes();
    assert!(matches!(
        league
            .ingest(delivery(&altered, &p, &sig, Uuid::now_v7(), false), NOW)
            .await,
        Err(Error::Conflict)
    ));
    assert!(league.read(NOW).await.is_err());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM league_audit")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    db.close().await
}
