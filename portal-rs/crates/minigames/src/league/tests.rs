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
        sqlx::query("INSERT INTO world_status(id,name) VALUES($1,'Synthetic fixture world') ON CONFLICT DO NOTHING").bind(schema::id("0192a000-0000-7000-8000-000000000012")?).execute(&pool).await?;
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
    let app = web::router_with_clock(
        League::new(
            db.pool.clone(),
            flags(),
            Trusted {
                generation: 1,
                complete: NOW,
            },
        ),
        || NOW as i64,
    );
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

fn real_config(path: &std::path::Path) -> config::Config {
    config::Config {
        authority: "oteryn:game:primary".into(),
        audience: "https://platform.example.test".into(),
        bind: "127.0.0.1:0".into(),
        ca: path.join("unused-ca.der"),
        certificate: path.join("unused-cert.der"),
        private_key: path.join("unused-key.der"),
        peer_certificate_sha256: "ab".repeat(32),
        keys: std::collections::BTreeMap::from([(
            "synthetic".into(),
            SigningKey::from_bytes(&[42; 32]).verifying_key().to_bytes(),
        )]),
        recovery_file: path.join("recovery.json"),
        requests_per_minute: 120,
        max_envelope_bytes: 4096,
    }
}
fn write_recovery(c: &config::Config, generation: &str) -> TestResult {
    std::fs::write(
        &c.recovery_file,
        serde_json::to_vec(&config::Recovery {
            source_authority: c.authority.clone(),
            projection_epoch: "3".into(),
            recovery_generation: generation.into(),
        })?,
    )?;
    Ok(())
}
struct Publication {
    path: &'static str,
    body: Vec<u8>,
    meta: Vec<u8>,
    signature: Vec<u8>,
}
fn publication(
    c: &config::Config,
    path: &'static str,
    body: Vec<u8>,
    envelope: Option<admission::Envelope>,
    finalized: bool,
    clock: u64,
) -> std::result::Result<Publication, Box<dyn std::error::Error>> {
    use base64::Engine;
    let id = match &envelope {
        Some(e) => Uuid::from_slice(&e.event_id)?,
        None => Uuid::now_v7(),
    };
    let meta = serde_json::to_vec(&admission::Meta {
        contract_version: 1,
        source_authority: c.authority.clone(),
        projection_epoch: "3".into(),
        recovery_generation: "1".into(),
        key_id: "synthetic".into(),
        request_id: id.to_string(),
        envelope: envelope
            .map(|e| base64::engine::general_purpose::STANDARD.encode(e.encode_to_vec()))
            .unwrap_or_default(),
        finalized_at: if finalized {
            vec![clock.to_string()]
        } else {
            vec![]
        },
    })?;
    let signature = SigningKey::from_bytes(&[42; 32])
        .sign(&admission::signable(c, path, &meta, &body))
        .to_bytes()
        .to_vec();
    Ok(Publication {
        path,
        body,
        meta,
        signature,
    })
}
fn native_envelope(raw: &[u8], season: bool, clock: u64) -> Result<admission::Envelope> {
    let s = schema::Snapshot::decode(raw, season, clock)?;
    let payload = payload(raw, season)?;
    let id = Uuid::now_v7();
    Ok(admission::Envelope {
        envelope_revision: 1,
        event_id: id.as_bytes().to_vec(),
        event_type_id: if season { 11 } else { 10 },
        event_schema_revision: 1,
        durability_class: 2,
        privacy_class: 3,
        retention_profile_id: "PLAT3_AUTHORITY_AUDIT_RETENTION_V1".into(),
        occurred_at_unix_ms: (clock * 1000) as i64,
        world_id: Some(schema::id(s.world())?.as_bytes().to_vec()),
        transaction_event: Some(admission::TransactionRef {
            transaction_id: id.as_bytes().to_vec(),
            ordinal: 1,
            count: 1,
        }),
        server_build_id: "synthetic-only".into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
    })
}
async fn enqueue_publication(db: &Db, c: &config::Config, p: &Publication) -> Result<bool> {
    runtime::receive(&db.pool, c, p.path, &p.body, &p.meta, &p.signature).await
}
#[tokio::test]
async fn real_authority_worker_enumeration_replay_and_restore() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let dir = tempfile::tempdir()?;
    let c = real_config(dir.path());
    write_recovery(&c, "1")?;
    let clock = runtime::now()?;
    let mut subjects = vec![];
    let mut publications = vec![];
    for name in ["settlement", "tied_guilds_frozen"] {
        let raw = wire(name)?;
        let season = name == "tied_guilds_frozen";
        let s = schema::Snapshot::decode(&raw, season, clock)?;
        subjects.push(runtime::subject(&s, false)?);
        let envelope = native_envelope(&raw, season, clock)?;
        publications.push(publication(
            &c,
            if season {
                admission::SEASON
            } else {
                admission::EVENT
            },
            raw,
            Some(envelope),
            s.final_state(),
            clock,
        )?);
    }
    subjects.sort();
    let cut = admission::Cut {
        contract_version: 1,
        operation: "PublishLeagueCutV1".into(),
        source_authority: c.authority.clone(),
        projection_epoch: "3".into(),
        recovery_generation: "1".into(),
        subjects,
    };
    let cut = publication(
        &c,
        admission::CUT,
        serde_json::to_vec(&cut)?,
        None,
        false,
        clock,
    )?;
    assert!(!enqueue_publication(&db, &c, &cut).await?);
    for p in &publications {
        assert!(!enqueue_publication(&db, &c, p).await?);
    }
    let league = League::new(
        db.pool.clone(),
        config::owner_flags(true),
        runtime::GameAuthority::new(c.clone()),
    );
    let job = runtime::FeedJob {
        pool: db.pool.clone(),
    };
    job.run_config(c.clone()).await?;
    assert!(league.read(runtime::now()?).await.is_err());
    for family in ["world_event_outcome", "guild_season"] {
        let clock = runtime::now()?;
        let w = admission::Watermark {
            contract_version: 1,
            operation: "PublishLeagueWatermarkV1".into(),
            source_authority: c.authority.clone(),
            family: family.into(),
            projection_epoch: "3".into(),
            complete_through: clock.to_string(),
            observed_at: clock.to_string(),
        };
        let p = publication(
            &c,
            admission::WATERMARK,
            serde_json::to_vec(&w)?,
            None,
            false,
            clock,
        )?;
        enqueue_publication(&db, &c, &p).await?;
    }
    job.run_config(c.clone()).await?;
    assert_eq!(league.read(runtime::now()?).await?.len(), 2);
    for p in &publications {
        assert!(enqueue_publication(&db, &c, p).await?);
    }
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM league_receipts")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(rows, 2);
    let restarted = League::new(
        db.pool.clone(),
        config::owner_flags(true),
        runtime::GameAuthority::new(c.clone()),
    );
    assert_eq!(restarted.read(runtime::now()?).await?.len(), 2);
    let first: admission::Meta = admission::json(&publications[0].meta)?;
    let request_id = schema::id(&first.request_id)?;
    sqlx::query("UPDATE league_feed_inbox SET signature=$2 WHERE request_id=$1")
        .bind(request_id)
        .bind(vec![0u8; 64])
        .execute(&db.pool)
        .await?;
    assert!(restarted.read(runtime::now()?).await.is_err());
    sqlx::query("UPDATE league_feed_inbox SET signature=$2 WHERE request_id=$1")
        .bind(request_id)
        .bind(&publications[0].signature)
        .execute(&db.pool)
        .await?;
    let mut bad = publication(&c, admission::CUT, cut.body.clone(), None, false, clock)?;
    bad.signature[0] ^= 1;
    assert!(enqueue_publication(&db, &c, &bad).await.is_err());
    assert_eq!(restarted.read(runtime::now()?).await?.len(), 2);
    sqlx::query("UPDATE league_feed_state SET event_complete=$1")
        .bind(runtime::now()?.saturating_sub(30) as i64)
        .execute(&db.pool)
        .await?;
    assert!(league.read(runtime::now()?).await.is_err());
    write_recovery(&c, "2")?;
    assert!(restarted.read(runtime::now()?).await.is_err());
    sqlx::query("UPDATE league_feed_state SET conflicted=true,ready=false")
        .execute(&db.pool)
        .await?;
    sqlx::query("UPDATE league_feed_inbox SET expires_at=$1")
        .bind(runtime::now()? as i64 - 1)
        .execute(&db.pool)
        .await?;
    runtime::Retention {
        pool: db.pool.clone(),
    }
    .run()
    .await?;
    let raw: i64 = sqlx::query_scalar("SELECT count(*) FROM league_feed_inbox")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(raw, 0);
    let control: i64 = sqlx::query_scalar("SELECT count(*) FROM league_feed_requests")
        .fetch_one(&db.pool)
        .await?;
    assert!(control > 0);
    db.close().await
}
#[tokio::test]
async fn real_authority_refuses_signed_bad_native_hash_and_wrong_audience() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let dir = tempfile::tempdir()?;
    let c = real_config(dir.path());
    write_recovery(&c, "1")?;
    let clock = runtime::now()?;
    let raw = wire("settlement")?;
    let mut envelope = native_envelope(&raw, false, clock)?;
    envelope.payload_sha256[0] ^= 1;
    let bad = publication(&c, admission::EVENT, raw, Some(envelope), false, clock)?;
    let mut other = c.clone();
    other.audience = "https://other.example.test".into();
    assert!(
        admission::authenticate(&other, bad.path, &bad.meta, &bad.body, &bad.signature).is_err()
    );
    assert!(enqueue_publication(&db, &c, &bad).await.is_err());
    let poisoned: bool =
        sqlx::query_scalar("SELECT conflicted FROM league_feed_state WHERE authority=$1")
            .bind(&c.authority)
            .fetch_one(&db.pool)
            .await?;
    assert!(poisoned);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM league_receipts")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    db.close().await
}
#[test]
fn names_follow_nfc_wire_policy_instead_of_ascii_only() -> TestResult {
    let raw = String::from_utf8(wire("tied_guilds_frozen")?)?;
    let unicode = raw.replace("Aldric Guild", "Łowcy Żmij");
    schema::Snapshot::decode(unicode.as_bytes(), true, NOW)?;
    for name in ["A\u{200d}Guild", "A\nGuild", "Z\u{307}mije", "../guild"] {
        let bad = raw.replace("Aldric Guild", name);
        assert!(schema::Snapshot::decode(bad.as_bytes(), true, NOW).is_err());
    }
    Ok(())
}
#[tokio::test]
async fn real_worker_keeps_scheduled_locked_pending_and_refuses_reopening() -> TestResult {
    let Some(db) = Db::new().await? else {
        return Ok(());
    };
    let dir = tempfile::tempdir()?;
    let c = real_config(dir.path());
    write_recovery(&c, "1")?;
    let clock = runtime::now()?;
    let scheduled = String::from_utf8(wire("advance_schedule")?)?;
    let job = runtime::FeedJob {
        pool: db.pool.clone(),
    };
    for raw in [
        scheduled.clone(),
        scheduled
            .replace(
                "\"projection_revision\":\"1\"",
                "\"projection_revision\":\"2\"",
            )
            .replace("\"state\":\"SCHEDULED\"", "\"state\":\"LOCKED\""),
    ] {
        let p = publication(&c, admission::EVENT, raw.into_bytes(), None, false, clock)?;
        enqueue_publication(&db, &c, &p).await?;
        job.run_config(c.clone()).await?;
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM league_receipts")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    let reopened = scheduled.replace(
        "\"projection_revision\":\"1\"",
        "\"projection_revision\":\"3\"",
    );
    let p = publication(
        &c,
        admission::EVENT,
        reopened.into_bytes(),
        None,
        false,
        clock,
    )?;
    enqueue_publication(&db, &c, &p).await?;
    job.run_config(c.clone()).await?;
    let blocked: bool =
        sqlx::query_scalar("SELECT conflicted FROM league_feed_state WHERE authority=$1")
            .bind(&c.authority)
            .fetch_one(&db.pool)
            .await?;
    assert!(blocked);
    write_recovery(&c, "2")?;
    assert!(matches!(
        admission::authenticate(&c, p.path, &p.meta, &p.body, &p.signature),
        Err(Error::Superseded)
    ));
    let mut forged = p.signature.clone();
    forged[0] ^= 1;
    assert!(!matches!(
        admission::authenticate(&c, p.path, &p.meta, &p.body, &forged),
        Err(Error::Superseded)
    ));
    db.close().await
}
#[test]
fn duplicate_public_key_ids_are_not_an_owner_trust_decision() -> TestResult {
    assert!(config::public_keys(r#"{"same":"a","same":"b"}"#).is_err());
    assert!(config::public_keys(r#"{"same":"a"}{}"#).is_err());
    assert_eq!(
        config::public_keys(r#"{ "first": "a", "second": "b" }"#)?.len(),
        2
    );
    Ok(())
}
