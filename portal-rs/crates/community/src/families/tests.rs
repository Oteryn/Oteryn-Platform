#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
    routing::get,
};
use oteryn_internal_api::guard::{Binding, Identity, VerifiedClient};
use oteryn_web::session::{self, BrowserSession, PrivilegeChange, SessionLayer};
use tower::ServiceExt;
use uuid::Uuid;

// Frozen golden payloads of the accepted contract, read byte for byte.
macro_rules! fixture {
    ($name:literal) => {
        include_bytes!(concat!(
            "../../../../../docs/contracts/fixtures/public-game-families-v2/",
            $name,
            ".json"
        ))
        .as_slice()
    };
}
const SKILLS: &[u8] = fixture!("valid-character-skills");
const SUPPRESSION: &[u8] = fixture!("valid-character-skills-suppression");
const ACHIEVEMENTS: &[u8] = fixture!("valid-character-achievements");
const EQUIPMENT: &[u8] = fixture!("valid-character-equipment");
const OUTFIT: &[u8] = fixture!("valid-character-outfit");
const WATERMARK: &[u8] = fixture!("valid-character-skills-watermark");
const BAD_SKILLS: &[u8] = fixture!("invalid-character-skills-zero-epoch-wrong-type-null-body");
const BAD_ACCOUNT: &[u8] = fixture!("invalid-character-achievements-account-member");
const BAD_WATERMARK: &[u8] = fixture!("invalid-character-skills-watermark-complete-after-observed");
const CONFLICT: &[u8] = fixture!("conflict-character-equipment-revision-9-changed-content");

const AUTHORITY: &str = "oteryn:game:primary";
const CHARACTER: &str = "01934f10-7c04-7001-805b-3b1122334401";
const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
const PEER: &str = "families-test";
const ACCEPTED: &str = r#"{"contract_version":2,"result":"accepted"}"#;

fn config(enabled: bool) -> IngestConfig {
    IngestConfig {
        skills: enabled,
        achievements: enabled,
        equipment: enabled,
        outfit: enabled,
        source_authority: AUTHORITY.into(),
        requests_per_minute: 6000,
    }
}
fn ingest(db: PgPool, enabled: bool) -> Router {
    ingest_router(
        db,
        Arc::new(
            IdentityMap::new(vec![Binding {
                identity: Identity::san(PEER).unwrap(),
                group: "game-public".into(),
            }])
            .unwrap(),
        ),
        config(enabled),
    )
}
async fn post(app: &Router, path: &str, body: Vec<u8>, peer: bool) -> (StatusCode, String) {
    let length = body.len();
    let mut request = Request::post(format!("/internal/v2/game-public/{path}"))
        .header("content-type", "application/json")
        .header("content-length", length)
        .body(Body::from(body))
        .unwrap();
    if peer {
        request
            .extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(
                Identity::san(PEER).unwrap(),
            ));
    }
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}
fn watermark(family: Family, complete: u64) -> Vec<u8> {
    format!(
        r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"{AUTHORITY}","family":"{}","projection_epoch":"1","complete_through":"{complete}","observed_at":"{complete}"}}"#,
        family.name()
    )
    .into_bytes()
}
/// Generated, not a golden payload: the contract has no oversized example.
fn achievements(revision: u64, earned: usize, key_len: usize) -> Vec<u8> {
    let rows: Vec<String> = (0..earned)
        .map(|i| {
            let suffix = format!("{i:06}");
            let key = format!("{}{suffix}", "a".repeat(key_len.saturating_sub(6)));
            format!(r#"{{"key":"oteryn:achievement/{key}","earned_at":"1789990000"}}"#)
        })
        .collect();
    format!(
        r#"{{"contract_version":2,"operation":"PublishCharacterAchievementsV2","source_authority":"{AUTHORITY}","character_id":"{CHARACTER}","projection_epoch":"1","projection_revision":"{revision}","source_observed_at":"1790000000","achievements":[{{"world_id":"{WORLD}","character_revision":"19","catalogue_revision":"fixture-v1","earned":[{}]}}]}}"#,
        rows.join(",")
    )
    .into_bytes()
}

struct Db {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Db {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED G-4 families PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = oteryn_db::lazy_pool(&url).unwrap();
        let schema = format!("g4_families_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |conn, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
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
        Some(Self {
            pool,
            admin,
            schema,
        })
    }
    async fn exec(&self, sql: &str, binds: &[&str]) {
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()));
        for b in binds {
            q = q.bind(*b);
        }
        q.execute(&self.pool).await.unwrap();
    }
    async fn stored(&self, family: Family) -> Option<(String, serde_json::Value)> {
        sqlx::query_as("SELECT revision::text,body FROM public_character_families_v2 WHERE family=$1 AND character_id=$2::uuid")
            .bind(family.name())
            .bind(CHARACTER)
            .fetch_optional(&self.pool)
            .await
            .unwrap()
    }
    async fn through(&self, family: Family) -> Option<i64> {
        sqlx::query_scalar(
            "SELECT complete_through FROM public_character_families_v2_watermarks WHERE family=$1",
        )
        .bind(family.name())
        .fetch_optional(&self.pool)
        .await
        .unwrap()
    }
    async fn account(&self, id: &str) {
        // Match a live LCFA publication; snapshots alone cannot prove ownership.
        self.exec("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='test',complete_through=floor(extract(epoch FROM statement_timestamp()))::bigint,observed_at=floor(extract(epoch FROM statement_timestamp()))::bigint WHERE id=1", &[]).await;
        self.exec(
            "INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1::uuid,$1||'@example.test','x','active','en')",
            &[id],
        )
        .await;
        self.exec(
            "INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1::uuid,'test','1','1',0,decode(repeat('00',32),'hex'))",
            &[id],
        )
        .await;
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await
        .unwrap();
    }
}

#[test]
fn golden_payloads_validate_and_invalid_ones_do_not() {
    let decode = |raw: &[u8], max: usize| canonical::decode(raw, max, DEPTH).unwrap();
    let now = 1_790_000_010;
    for (family, raw) in [
        (Family::Skills, SKILLS),
        (Family::Skills, SUPPRESSION),
        (Family::Achievements, ACHIEVEMENTS),
        (Family::Equipment, EQUIPMENT),
        (Family::Equipment, CONFLICT),
        (Family::Outfit, OUTFIT),
    ] {
        let value = decode(raw, family.max_bytes());
        let snapshot: wire::Snapshot = super::decode(&value, raw, family.max_bytes()).unwrap();
        assert!(snapshot.validate(family, now).is_ok(), "{family:?}");
        // A fixture is valid only on its own route.
        let other = Family::ALL.into_iter().find(|f| *f != family).unwrap();
        assert!(snapshot.validate(other, now).is_err());
    }
    let mark: wire::Watermark = super::decode(&decode(WATERMARK, 512), WATERMARK, 512).unwrap();
    assert_eq!(mark.validate(now), Ok(Family::Skills));
    let bad: wire::Watermark =
        super::decode(&decode(BAD_WATERMARK, 512), BAD_WATERMARK, 512).unwrap();
    assert!(bad.validate(now).is_err());
    assert!(
        super::decode::<wire::Snapshot>(&decode(BAD_ACCOUNT, 262144), BAD_ACCOUNT, 262144).is_err()
    );
    assert!(super::decode::<wire::Snapshot>(&decode(BAD_SKILLS, 4096), BAD_SKILLS, 4096).is_err());
    // Source observations more than 1 s into the future are refused.
    let value = decode(SKILLS, 4096);
    let snapshot: wire::Snapshot = super::decode(&value, SKILLS, 4096).unwrap();
    assert!(snapshot.validate(Family::Skills, 1_789_999_998).is_err());
    // Bad input inside otherwise exact bodies.
    for (family, raw) in [
        (
            Family::Skills,
            String::from_utf8_lossy(SKILLS).replace("\"fist_level\":10", "\"fist_level\":9"),
        ),
        (
            Family::Skills,
            String::from_utf8_lossy(SKILLS).replace("\"magic_level\":12", "\"magic_level\":1001"),
        ),
        (
            Family::Equipment,
            String::from_utf8_lossy(EQUIPMENT).replace("\"Head\"", "\"Hat\""),
        ),
        (
            Family::Outfit,
            String::from_utf8_lossy(OUTFIT).replace("[1,2,3,4]", "[1,2,3]"),
        ),
        (
            Family::Outfit,
            String::from_utf8_lossy(OUTFIT).replace("\"addons\":0", "\"addons\":256"),
        ),
        (
            Family::Achievements,
            String::from_utf8_lossy(ACHIEVEMENTS).replace("fixture_earned", "Fixture"),
        ),
    ] {
        let raw = raw.into_bytes();
        let Ok(value) = canonical::decode(&raw, family.max_bytes(), DEPTH) else {
            continue;
        };
        let parsed = super::decode::<wire::Snapshot>(&value, &raw, family.max_bytes());
        assert!(
            parsed.is_err() || parsed.unwrap().validate(family, now).is_err(),
            "{}",
            String::from_utf8_lossy(&raw)
        );
    }
}

#[tokio::test]
async fn ingest_refuses_when_disabled_or_without_the_peer() {
    let db = PgPool::connect_lazy("postgres://localhost/unused").unwrap();
    for family in Family::ALL {
        let (status, body) = post(
            &ingest(db.clone(), false),
            family.path(),
            SKILLS.to_vec(),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(body.is_empty());
        let (status, _) = post(
            &ingest(db.clone(), true),
            family.path(),
            SKILLS.to_vec(),
            false,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let (status, _) = post(
        &ingest(db.clone(), false),
        "watermark",
        WATERMARK.to_vec(),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn postgres_golden_payloads_through_the_ingest_routes() {
    let Some(db) = Db::open().await else {
        return;
    };
    let app = ingest(db.pool.clone(), true);
    // Invalid first: nothing may be stored.
    assert_eq!(
        post(&app, "character-skills", BAD_SKILLS.to_vec(), true).await,
        (StatusCode::BAD_REQUEST, String::new())
    );
    assert_eq!(
        post(&app, "character-achievements", BAD_ACCOUNT.to_vec(), true)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert!(db.stored(Family::Skills).await.is_none());
    assert!(db.stored(Family::Achievements).await.is_none());
    // A golden payload is accepted only on its own route.
    assert_eq!(
        post(&app, "character-outfit", SKILLS.to_vec(), true)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    for (path, raw) in [
        ("character-skills", SKILLS),
        ("character-achievements", ACHIEVEMENTS),
        ("character-equipment", EQUIPMENT),
        ("character-outfit", OUTFIT),
        ("watermark", WATERMARK),
    ] {
        assert_eq!(
            post(&app, path, raw.to_vec(), true).await,
            (StatusCode::OK, ACCEPTED.to_owned()),
            "{path}"
        );
        // An equal retry is accepted again without a change.
        assert_eq!(
            post(&app, path, raw.to_vec(), true).await.0,
            StatusCode::OK,
            "{path}"
        );
    }
    let (revision, body) = db.stored(Family::Skills).await.unwrap();
    assert_eq!(revision, "7");
    assert_eq!(body[0]["level"], "412");
    assert_eq!(db.through(Family::Skills).await, Some(1_790_000_000));
    // Completeness later than observation: 400, previous watermark kept.
    assert_eq!(
        post(&app, "watermark", BAD_WATERMARK.to_vec(), true)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(db.through(Family::Skills).await, Some(1_790_000_000));
    // Equal key with changed content: 409, first record preserved.
    assert_eq!(
        post(&app, "character-equipment", CONFLICT.to_vec(), true).await,
        (StatusCode::CONFLICT, String::new())
    );
    let (revision, body) = db.stored(Family::Equipment).await.unwrap();
    assert_eq!(revision, "9");
    assert_eq!(body[0]["slots"][0]["slot"], "Head");
    // Suppression replaces the earlier skills snapshot; the older one is superseded.
    assert_eq!(
        post(&app, "character-skills", SUPPRESSION.to_vec(), true)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        db.stored(Family::Skills).await.unwrap(),
        ("8".to_owned(), serde_json::json!([]))
    );
    assert_eq!(
        post(&app, "character-skills", SKILLS.to_vec(), true).await,
        (
            StatusCode::OK,
            r#"{"contract_version":2,"result":"superseded"}"#.to_owned()
        )
    );
    assert_eq!(db.stored(Family::Skills).await.unwrap().0, "8");
    db.close().await;
}

#[tokio::test]
async fn postgres_oversized_achievements_are_refused_whole() {
    let Some(db) = Db::open().await else {
        return;
    };
    let app = ingest(db.pool.clone(), true);
    assert_eq!(
        post(&app, "character-achievements", ACHIEVEMENTS.to_vec(), true)
            .await
            .0,
        StatusCode::OK
    );
    let before = db.stored(Family::Achievements).await.unwrap();
    // 1024 earned facts is the bound; the body still fits the route.
    let at_bound = achievements(9, 1024, 20);
    assert!(at_bound.len() <= Family::Achievements.max_bytes());
    // One over the earned bound, inside the byte bound: refused, not truncated.
    let over_count = achievements(10, 1025, 20);
    assert!(over_count.len() <= Family::Achievements.max_bytes());
    assert_eq!(
        post(&app, "character-achievements", over_count, true).await,
        (StatusCode::BAD_REQUEST, String::new())
    );
    assert_eq!(db.stored(Family::Achievements).await.unwrap(), before);
    // Over the route byte bound: refused before any storage.
    let over_bytes = achievements(11, 1600, 141);
    assert!(over_bytes.len() > Family::Achievements.max_bytes());
    assert_eq!(
        post(&app, "character-achievements", over_bytes, true).await,
        (StatusCode::BAD_REQUEST, String::new())
    );
    assert_eq!(db.stored(Family::Achievements).await.unwrap(), before);
    assert_eq!(
        post(&app, "character-achievements", at_bound, true).await.0,
        StatusCode::OK
    );
    let (revision, body) = db.stored(Family::Achievements).await.unwrap();
    assert_eq!(revision, "9");
    assert_eq!(body[0]["earned"].as_array().unwrap().len(), 1024);
    db.close().await;
}

/// Public skills privacy decision for tests: `Some(allow)` or unavailable.
struct Fixed(Option<bool>);
impl SkillsPrivacy for Fixed {
    fn allows_skills<'a>(&'a self, _: &'a str, _: &'a str) -> PrivacyFuture<'a> {
        let decision = self.0;
        Box::pin(async move { decision.ok_or(Unavailable) })
    }
}
fn serving(enabled: bool) -> ServingConfig {
    ServingConfig {
        skills: enabled,
        achievements: enabled,
        equipment: enabled,
        outfit: enabled,
        source_authority: AUTHORITY.into(),
    }
}
fn site(db: &PgPool, privacy: Option<bool>, enabled: bool) -> Router {
    let layer = SessionLayer::new(db.clone());
    router(
        db.clone(),
        Arc::new(Fixed(privacy)),
        serving(enabled),
        layer.clone(),
    )
    .merge(
        Router::new()
            .route(
                "/test-login/{id}",
                get(
                    |axum::extract::Path(id): axum::extract::Path<Uuid>,
                     Extension(session): Extension<BrowserSession>| async move {
                        session::rotate(&session, PrivilegeChange::Login(id))
                            .await
                            .unwrap();
                        StatusCode::NO_CONTENT
                    },
                ),
            )
            .route_layer(axum::middleware::from_fn_with_state(
                layer,
                session::middleware,
            )),
    )
}
async fn get_page(app: &Router, path: &str, cookie: Option<&str>) -> (StatusCode, String) {
    let mut request = Request::get(path);
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    if status == StatusCode::OK {
        assert_eq!(response.headers()["cache-control"], "private, no-store");
    }
    let body = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}
async fn login(app: &Router, account: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::get(format!("/test-login/{account}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

#[tokio::test]
async fn postgres_family_owner_fixture_has_live_lcfa_evidence() {
    let Some(db) = Db::open().await else {
        return;
    };
    let account = Uuid::now_v7();
    db.account(&account.to_string()).await;
    let mut tx = db.pool.begin().await.unwrap();
    assert_eq!(
        oteryn_playerdata::authority::account_epoch(&mut tx, account)
            .await
            .unwrap(),
        1
    );
    tx.rollback().await.unwrap();
    db.close().await;
}

#[tokio::test]
async fn postgres_owner_only_families_serve_only_the_current_owner() {
    let Some(db) = Db::open().await else {
        return;
    };
    let owner = Uuid::now_v7().to_string();
    let other = Uuid::now_v7().to_string();
    let staff = Uuid::now_v7().to_string();
    for account in [&owner, &other, &staff] {
        db.account(account).await;
    }
    // Staff holds real admin permissions, none of which is an owner grant.
    db.exec("INSERT INTO roles(key) VALUES('g4_staff')", &[])
        .await;
    db.exec("INSERT INTO role_permissions(role_id,permission) SELECT id,p FROM roles, unnest(ARRAY['admin.access','support.tickets.manage','support.reports.manage']) p WHERE key='g4_staff'", &[]).await;
    db.exec("INSERT INTO account_roles(account_id,role_id) SELECT $1::uuid,id FROM roles WHERE key='g4_staff'", &[&staff]).await;
    db.exec("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1::uuid,$2::uuid,$3::uuid,'Aldric','AVAILABLE')", &[&owner, CHARACTER, WORLD]).await;
    // Character Authority ownership cut at character revision 19.
    db.exec("INSERT INTO pd_ownership_baselines(character_id,account_id,world_id,character_revision,from_projection_revision,to_projection_revision,projection_epoch,recovery_generation) VALUES($2::uuid,$1::uuid,$3::uuid,19,1,1,1,1)", &[&owner, CHARACTER, WORLD]).await;

    let app = ingest(db.pool.clone(), true);
    let now = now();
    for (family, raw) in [
        (Family::Achievements, ACHIEVEMENTS),
        (Family::Equipment, EQUIPMENT),
        (Family::Outfit, OUTFIT),
    ] {
        assert_eq!(
            post(&app, family.path(), raw.to_vec(), true).await.0,
            StatusCode::OK
        );
        assert_eq!(
            post(&app, "watermark", watermark(family, now), true)
                .await
                .0,
            StatusCode::OK
        );
    }
    let site = site(&db.pool, Some(true), true);
    let owner_cookie = login(&site, &owner).await;
    // LCFA freshness is independent of the public-family watermark.
    db.exec(
        "UPDATE account_character_feed SET watermark_epoch=NULL WHERE id=1",
        &[],
    )
    .await;
    assert_eq!(
        get_page(
            &site,
            "/en/account/characters/Aldric/achievements",
            Some(&owner_cookie)
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    db.exec(
        "UPDATE account_character_feed SET watermark_epoch='1' WHERE id=1",
        &[],
    )
    .await;

    let other_cookie = login(&site, &other).await;
    let staff_cookie = login(&site, &staff).await;
    for (segment, expected) in [
        ("achievements", "oteryn:achievement/fixture_earned"),
        ("equipment", "oteryn:item.fixture-helmet"),
        ("outfit", "outfit.citizen"),
    ] {
        let path = format!("/en/account/characters/Aldric/{segment}");
        assert_eq!(
            get_page(&site, &path, None).await.0,
            StatusCode::FORBIDDEN,
            "{segment}"
        );
        let (status, body) = get_page(&site, &path, Some(&owner_cookie)).await;
        assert_eq!(status, StatusCode::OK, "{segment}");
        assert!(body.contains(expected), "{body}");
        assert!(body.contains("width=device-width"));
        for cookie in [&other_cookie, &staff_cookie] {
            let (status, body) = get_page(&site, &path, Some(cookie)).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{segment}");
            assert!(!body.contains(expected));
        }
        // There is no public view of an owner-only family.
        assert_eq!(
            get_page(&site, &format!("/en/characters/Aldric/{segment}"), None)
                .await
                .0,
            StatusCode::NOT_FOUND
        );
        // Serving flag off: not mounted for anyone.
        let off = self::site(&db.pool, Some(true), false);
        assert_eq!(
            get_page(&off, &path, Some(&owner_cookie)).await.0,
            StatusCode::NOT_FOUND
        );
    }
    // A snapshot below the ownership-transition floor is denied even to the owner.
    db.exec(
        "UPDATE pd_ownership_baselines SET character_revision=20",
        &[],
    )
    .await;
    assert_eq!(
        get_page(
            &site,
            "/en/account/characters/Aldric/achievements",
            Some(&owner_cookie)
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    // Authority names another owner while the account list still lists this one: fail closed.
    db.exec(
        "UPDATE pd_ownership_baselines SET character_revision=19, account_id=$1::uuid",
        &[&other],
    )
    .await;
    let path = "/en/account/characters/Aldric/equipment";
    let (status, body) = get_page(&site, path, Some(&owner_cookie)).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(!body.contains("oteryn:item.fixture-helmet"));
    // After the transfer is projected the former owner is refused and the new owner served.
    db.exec(
        "UPDATE account_characters SET account_id=$1::uuid",
        &[&other],
    )
    .await;
    assert_eq!(
        get_page(&site, path, Some(&owner_cookie)).await.0,
        StatusCode::NOT_FOUND
    );
    let (status, body) = get_page(&site, path, Some(&other_cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("oteryn:item.fixture-helmet"));
    db.close().await;
}

#[tokio::test]
async fn postgres_public_skills_follow_profile_privacy_and_freshness() {
    let Some(db) = Db::open().await else {
        return;
    };
    let app = ingest(db.pool.clone(), true);
    let now = now();
    assert_eq!(
        post(&app, "character-skills", SKILLS.to_vec(), true)
            .await
            .0,
        StatusCode::OK
    );
    let site = site(&db.pool, Some(true), true);
    // No skills watermark yet: unavailable, never empty.
    assert_eq!(
        get_page(&site, "/en/characters/Aldric/skills", None)
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        post(&app, "watermark", watermark(Family::Skills, now), true)
            .await
            .0,
        StatusCode::OK
    );
    // No eligible current profile of that name: not found.
    assert_eq!(
        get_page(&site, "/en/characters/Aldric/skills", None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let profile = format!(
        r#"{{"character_id":"{CHARACTER}","profile":[{{"world_id":"{WORLD}","name":"Aldric"}}]}}"#
    );
    db.exec("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('character_profile',$1,false,1,1,0,$2::jsonb)", &[CHARACTER, &profile]).await;
    db.exec("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('character_profile','',true,1,1,$1::bigint,'{}'::jsonb)", &[&now.to_string()]).await;
    let (status, body) = get_page(&site, "/en/characters/Aldric/skills", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("<dd>412</dd>") && body.contains("<dd>90</dd>"),
        "{body}"
    );
    assert!(!body.contains(CHARACTER) && !body.contains(WORLD));
    assert_eq!(
        get_page(&site, "/pl/characters/Aldric/skills", None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        get_page(&site, "/en/characters/Nobody/skills", None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get_page(&site, "/en/characters/..%2F/skills", None).await.0,
        StatusCode::BAD_REQUEST
    );
    // Privacy denies or is unavailable.
    assert_eq!(
        get_page(
            &self::site(&db.pool, Some(false), true),
            "/en/characters/Aldric/skills",
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get_page(
            &self::site(&db.pool, None, true),
            "/en/characters/Aldric/skills",
            None
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        get_page(
            &self::site(&db.pool, Some(true), false),
            "/en/characters/Aldric/skills",
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    // Suppression hides the earlier snapshot at once.
    assert_eq!(
        post(&app, "character-skills", SUPPRESSION.to_vec(), true)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        get_page(&site, "/en/characters/Aldric/skills", None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    // A stale watermark makes the family unavailable.
    db.exec(
        "UPDATE public_character_families_v2_watermarks SET complete_through=complete_through-31",
        &[],
    )
    .await;
    assert_eq!(
        get_page(&site, "/en/characters/Aldric/skills", None)
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    db.close().await;
}

#[derive(Debug)]
struct PausedPrivacy {
    db: PgPool,
    entered: Arc<tokio::sync::Barrier>,
    release: Arc<tokio::sync::Barrier>,
}
impl SkillsPrivacy for PausedPrivacy {
    fn allows_skills<'a>(&'a self, _: &'a str, _: &'a str) -> PrivacyFuture<'a> {
        Box::pin(async move {
            self.entered.wait().await;
            self.release.wait().await;
            sqlx::query("SELECT 1")
                .execute(&self.db)
                .await
                .map_err(|_| Unavailable)?;
            Ok(true)
        })
    }
}
async fn seed_public_skills(db: &Db) {
    let app = ingest(db.pool.clone(), true);
    assert_eq!(
        post(&app, "character-skills", SKILLS.to_vec(), true)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        post(&app, "watermark", watermark(Family::Skills, now()), true)
            .await
            .0,
        StatusCode::OK
    );
    let profile = format!(
        r#"{{"character_id":"{CHARACTER}","profile":[{{"world_id":"{WORLD}","name":"Aldric"}}]}}"#
    );
    db.exec("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('character_profile',$1,false,1,1,0,$2::jsonb)", &[CHARACTER, &profile]).await;
    db.exec("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('character_profile','',true,1,1,$1::bigint,'{}'::jsonb)", &[&now().to_string()]).await;
}
#[tokio::test]
async fn audit_w2_c01_skills_two_connections_complete_nested_privacy() {
    let started = std::time::Instant::now();
    let db = Db::open().await.expect("PostgreSQL required");
    seed_public_skills(&db).await;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect_with(
            db.pool
                .connect_options()
                .as_ref()
                .clone()
                .options([("search_path", db.schema.as_str())]),
        )
        .await
        .unwrap();
    let entered = Arc::new(tokio::sync::Barrier::new(3));
    let release = Arc::new(tokio::sync::Barrier::new(3));
    let app = router(
        pool.clone(),
        Arc::new(PausedPrivacy {
            db: pool.clone(),
            entered: entered.clone(),
            release: release.clone(),
        }),
        serving(true),
        SessionLayer::new(pool.clone()),
    );
    let one = tokio::spawn({
        let app = app.clone();
        async move { get_page(&app, "/en/characters/Aldric/skills", None).await.0 }
    });
    let two =
        tokio::spawn(async move { get_page(&app, "/en/characters/Aldric/skills", None).await.0 });
    tokio::time::timeout(std::time::Duration::from_secs(2), entered.wait())
        .await
        .unwrap();
    release.wait().await;
    let responses = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        (one.await.unwrap(), two.await.unwrap())
    })
    .await
    .unwrap();
    assert_eq!(responses, (StatusCode::OK, StatusCode::OK));
    pool.close().await;
    db.close().await;
    eprintln!(
        "audit_w2_c01_skills_two_connections_complete_nested_privacy {:?}",
        started.elapsed()
    );
}
#[tokio::test]
async fn audit_w2_c02_skills_suppression_during_privacy_withholds_body() {
    let started = std::time::Instant::now();
    let db = Db::open().await.expect("PostgreSQL required");
    seed_public_skills(&db).await;
    let entered = Arc::new(tokio::sync::Barrier::new(2));
    let release = Arc::new(tokio::sync::Barrier::new(2));
    let app = router(
        db.pool.clone(),
        Arc::new(PausedPrivacy {
            db: db.pool.clone(),
            entered: entered.clone(),
            release: release.clone(),
        }),
        serving(true),
        SessionLayer::new(db.pool.clone()),
    );
    let read =
        tokio::spawn(async move { get_page(&app, "/en/characters/Aldric/skills", None).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), entered.wait())
        .await
        .unwrap();
    assert_eq!(
        post(
            &ingest(db.pool.clone(), true),
            "character-skills",
            SUPPRESSION.to_vec(),
            true
        )
        .await
        .0,
        StatusCode::OK
    );
    release.wait().await;
    let (status, body) = read.await.unwrap();
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(!body.contains("<dd>412</dd>"));
    db.close().await;
    eprintln!(
        "audit_w2_c02_skills_suppression_during_privacy_withholds_body {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn audit_w2_c02_owner_body_wait_rechecks_epoch_for_every_family() {
    let started = std::time::Instant::now();
    let db = Db::open().await.expect("PostgreSQL required");
    let owner = Uuid::now_v7().to_string();
    db.account(&owner).await;
    db.exec("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1::uuid,$2::uuid,$3::uuid,'Aldric','AVAILABLE')", &[&owner,CHARACTER,WORLD]).await;
    db.exec("INSERT INTO pd_ownership_baselines(character_id,account_id,world_id,character_revision,from_projection_revision,to_projection_revision,projection_epoch,recovery_generation) VALUES($2::uuid,$1::uuid,$3::uuid,19,1,1,1,1)", &[&owner,CHARACTER,WORLD]).await;
    let site = site(&db.pool, Some(true), true);
    let cookie = login(&site, &owner).await;
    for (family, raw) in [
        (Family::Achievements, ACHIEVEMENTS),
        (Family::Equipment, EQUIPMENT),
        (Family::Outfit, OUTFIT),
    ] {
        db.exec("UPDATE public_projection_state SET highest_epoch=1", &[])
            .await;
        let app = ingest(db.pool.clone(), true);
        assert_eq!(
            post(&app, family.path(), raw.to_vec(), true).await.0,
            StatusCode::OK
        );
        assert_eq!(
            post(&app, "watermark", watermark(family, now()), true)
                .await
                .0,
            StatusCode::OK
        );
        let mut lock = db.pool.begin().await.unwrap();
        sqlx::query("LOCK TABLE public_character_families_v2 IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *lock)
            .await
            .unwrap();
        let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *lock)
            .await
            .unwrap();
        let app = site.clone();
        let cookie = cookie.clone();
        let read = tokio::spawn(async move {
            get_page(
                &app,
                &format!(
                    "/en/account/characters/Aldric/{}",
                    family.path().trim_start_matches("character-")
                ),
                Some(&cookie),
            )
            .await
            .0
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            let blocked:bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)) AND wait_event_type='Lock' AND query LIKE 'SELECT body,%')").bind(blocker_pid).fetch_one(&db.admin).await.unwrap();
            if blocked {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "owner body did not block"
            );
            tokio::task::yield_now().await;
        }
        db.exec("UPDATE public_projection_state SET highest_epoch=2", &[])
            .await;
        lock.commit().await.unwrap();
        assert_eq!(read.await.unwrap(), StatusCode::SERVICE_UNAVAILABLE);
    }
    db.close().await;
    eprintln!(
        "audit_w2_c02_owner_body_wait_rechecks_epoch_for_every_family {:?}",
        started.elapsed()
    );
}

struct SuppressDuringPrivacy(Router);
impl SkillsPrivacy for SuppressDuringPrivacy {
    fn allows_skills<'a>(&'a self, _: &'a str, _: &'a str) -> PrivacyFuture<'a> {
        Box::pin(async move {
            assert_eq!(
                post(&self.0, "character-skills", SUPPRESSION.to_vec(), true)
                    .await
                    .0,
                StatusCode::OK
            );
            Ok(true)
        })
    }
}
#[tokio::test]
async fn postgres_skills_suppression_during_privacy_denies_old_snapshot() {
    let started = std::time::Instant::now();
    let db = Db::open().await.expect("real PostgreSQL is required");
    let app = ingest(db.pool.clone(), true);
    let now = super::now();
    assert_eq!(
        post(&app, "character-skills", SKILLS.to_vec(), true)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        post(&app, "watermark", watermark(Family::Skills, now), true)
            .await
            .0,
        StatusCode::OK
    );
    let profile = format!(
        r#"{{"character_id":"{CHARACTER}","profile":[{{"world_id":"{WORLD}","name":"Aldric"}}]}}"#
    );
    db.exec("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('character_profile',$1,false,1,1,0,$2::jsonb)", &[CHARACTER, &profile]).await;
    db.exec("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('character_profile','',true,1,1,$1::bigint,'{}'::jsonb)", &[&now.to_string()]).await;
    let site = router(
        db.pool.clone(),
        Arc::new(SuppressDuringPrivacy(app)),
        serving(true),
        SessionLayer::new(db.pool.clone()),
    );
    let (status, body) = get_page(&site, "/en/characters/Aldric/skills", None).await;
    assert!(
        status == StatusCode::NOT_FOUND || status == StatusCode::SERVICE_UNAVAILABLE,
        "{status}: {body}"
    );
    assert!(!body.contains("<dd>412</dd>"));
    db.close().await;
    eprintln!(
        "PG postgres_skills_suppression_during_privacy_denies_old_snapshot {:.3}s",
        started.elapsed().as_secs_f64()
    );
}

#[tokio::test]
async fn postgres_equipment_conflict_quarantines_owner_and_stalls_completeness() {
    let started = std::time::Instant::now();
    let db = Db::open().await.expect("real PostgreSQL is required");
    let owner = Uuid::now_v7().to_string();
    db.account(&owner).await;
    db.exec("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1::uuid,$2::uuid,$3::uuid,'Aldric','AVAILABLE')", &[&owner, CHARACTER, WORLD]).await;
    db.exec("INSERT INTO pd_ownership_baselines(character_id,account_id,world_id,character_revision,from_projection_revision,to_projection_revision,projection_epoch,recovery_generation) VALUES($2::uuid,$1::uuid,$3::uuid,19,1,1,1,1)", &[&owner, CHARACTER, WORLD]).await;
    let app = ingest(db.pool.clone(), true);
    let now = now();
    assert_eq!(
        post(&app, "character-equipment", EQUIPMENT.to_vec(), true)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        post(
            &app,
            "watermark",
            watermark(Family::Equipment, now - 1),
            true
        )
        .await
        .0,
        StatusCode::OK
    );
    let site = site(&db.pool, Some(true), true);
    let cookie = login(&site, &owner).await;
    assert_eq!(
        get_page(
            &site,
            "/en/account/characters/Aldric/equipment",
            Some(&cookie)
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        post(&app, "character-equipment", CONFLICT.to_vec(), true)
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        post(&app, "watermark", watermark(Family::Equipment, now), true)
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(db.through(Family::Equipment).await, Some((now - 1) as i64));
    assert_eq!(
        get_page(
            &site,
            "/en/account/characters/Aldric/equipment",
            Some(&cookie)
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        post(&app, "watermark", watermark(Family::Skills, now), true)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(db.stored(Family::Equipment).await.unwrap().0, "9");
    db.close().await;
    eprintln!(
        "PG postgres_equipment_conflict_quarantines_owner_and_stalls_completeness {:.3}s",
        started.elapsed().as_secs_f64()
    );
}

#[tokio::test]
async fn w3_c06_013_final_family_wait_rechecks_wall_time() {
    let started = std::time::Instant::now();
    let db = Db::open().await.expect("PostgreSQL required");
    seed_public_skills(&db).await;
    let entered = Arc::new(tokio::sync::Barrier::new(2));
    let release = Arc::new(tokio::sync::Barrier::new(2));
    let app = router(
        db.pool.clone(),
        Arc::new(PausedPrivacy {
            db: db.pool.clone(),
            entered: entered.clone(),
            release: release.clone(),
        }),
        serving(true),
        SessionLayer::new(db.pool.clone()),
    );
    let pending =
        tokio::spawn(async move { get_page(&app, "/en/characters/Aldric/skills", None).await });
    entered.wait().await;
    let complete = now() - 29;
    sqlx::query("UPDATE public_character_families_v2_watermarks SET complete_through=$1 WHERE family='character_skills'").bind(i64::try_from(complete).unwrap()).execute(&db.pool).await.unwrap();
    let mut blocker = db.pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    sqlx::query("LOCK TABLE public_character_families_v2_watermarks IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    release.wait().await;
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(pid)
            .fetch_one(&db.admin)
            .await
            .unwrap();
            if blocked {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    while now() < complete + 30 {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    blocker.commit().await.unwrap();
    let (status, body) = pending.await.unwrap();
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(!body.contains("<dd>412</dd>"));
    db.close().await;
    eprintln!(
        "w3_c06_013_final_family_wait_rechecks_wall_time {:?}",
        started.elapsed()
    );
}
