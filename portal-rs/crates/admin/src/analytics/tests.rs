#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;
const NOW: u64 = 1794000000;
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/admin_analytics"
);
fn page() -> wire::Page {
    serde_json::from_slice(
        &std::fs::read(format!("{FIXTURE}/valid/page-creature_kills_daily.json")).unwrap(),
    )
    .unwrap()
}
#[test]
fn byte_exact_consumer_goldens() {
    for (directory, valid) in [("valid", true), ("invalid", false)] {
        for path in std::fs::read_dir(format!("{FIXTURE}/{directory}")).unwrap() {
            let path = path.unwrap().path();
            let name = path.file_name().unwrap().to_str().unwrap();
            let op = if name.starts_with("page") {
                "page"
            } else if name.starts_with("commit") {
                "commit"
            } else {
                "watermark"
            };
            let result = wire::parse(op, &std::fs::read(&path).unwrap(), NOW);
            assert_eq!(result.is_ok(), valid, "{name}: {result:?}");
        }
    }
}
#[test]
fn uint64_and_activity_arithmetic_boundaries() {
    assert_eq!(wire::uint("18446744073709551615").unwrap(), u64::MAX);
    let mut p = page();
    p.family = wire::Family::WorldActivityHourly;
    p.bucket_end = (1793664000u64 + 3600).to_string();
    p.rows[0].key = "world".into();
    for values in [["1", "3600", "1", "1000"], ["0", "0", "0", "0"]] {
        p.rows[0].values = values.map(str::to_owned).to_vec();
        assert!(wire::parse("page", &serde_json::to_vec(&p).unwrap(), NOW).is_ok());
    }
    for values in [
        ["1", "3600", "0", "1000"],
        ["1", "3600", "1", "999"],
        ["1", "18446744073709551615", "1", "0"],
    ] {
        p.rows[0].values = values.map(str::to_owned).to_vec();
        assert!(wire::parse("page", &serde_json::to_vec(&p).unwrap(), NOW).is_err());
    }
}
#[tokio::test]
async fn wrong_purpose_transport_and_bounds() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused@127.0.0.1:1/unused")
        .unwrap();
    let identities = Arc::new(
        guard::IdentityMap::new(vec![
            guard::Binding {
                identity: guard::Identity::san("analytics-test").unwrap(),
                group: "game-admin-analytics".into(),
            },
            guard::Binding {
                identity: guard::Identity::san("public-test").unwrap(),
                group: "public-projection".into(),
            },
            guard::Binding {
                identity: guard::Identity::san("control-test").unwrap(),
                group: "admin-control".into(),
            },
        ])
        .unwrap(),
    );
    let app = router(
        pool,
        Config {
            ingest: true,
            requests_per_minute: 1200,
            ..Config::default()
        },
        identities,
    );
    for peer in [None, Some("public-test"), Some("control-test")] {
        let mut request = Request::post("/internal/v1/admin-analytics/page")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        if let Some(peer) = peer {
            request
                .extensions_mut()
                .insert(guard::VerifiedClient::from_verified_tls13_http11(
                    guard::Identity::san(peer).unwrap(),
                ));
        }
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(
            to_bytes(response.into_body(), 256)
                .await
                .unwrap()
                .is_empty()
        );
    }
    for (body, length) in [
        ("{}".to_owned(), "01"),
        ("x".repeat(131073), "131073"),
        ("{}".into(), "3"),
    ] {
        let mut request = Request::post("/internal/v1/admin-analytics/page")
            .header("content-type", "application/json")
            .header("content-length", length)
            .body(Body::from(body))
            .unwrap();
        request
            .extensions_mut()
            .insert(guard::VerifiedClient::from_verified_tls13_http11(
                guard::Identity::san("analytics-test").unwrap(),
            ));
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
    }
}
async fn database() -> Option<(PgPool, PgPool, String)> {
    let url = std::env::var("PORTAL_TEST_DATABASE_URL").ok()?;
    let root = PgPool::connect(&url).await.unwrap();
    let name = format!("analytics_test_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&root)
        .await
        .unwrap();
    let options: sqlx::postgres::PgConnectOptions = url.parse().unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect_with(options.database(&name))
        .await
        .unwrap();
    for migration in oteryn_db::MIGRATOR.iter() {
        if migration.migration_type.is_down_migration() {
            continue;
        }
        sqlx::raw_sql(AssertSqlSafe(migration.sql.as_ref()))
            .execute(&pool)
            .await
            .unwrap();
    }
    Some((pool, root, name))
}
async fn clean(pool: PgPool, root: PgPool, name: String) {
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP DATABASE {name} WITH (FORCE)")))
        .execute(&root)
        .await
        .unwrap();
    root.close().await;
}
async fn store(pool: PgPool) -> store::Store {
    sqlx::query("INSERT INTO admin_analytics.fence(authority,epoch,privacy_floor) VALUES('test-game',1,'policy-1')").execute(&pool).await.unwrap();
    store::Store {
        pool,
        authority: "test-game".into(),
        privacy: "policy-1".into(),
        rate: 1200,
        max_staging_bytes: 268435456,
    }
}
async fn publish(store: &store::Store, p: &wire::Page) -> Result<&'static str, store::Error> {
    let raw = serde_json::to_vec(p).unwrap();
    store
        .ingest(wire::parse("page", &raw, NOW).unwrap(), &raw, NOW)
        .await
}
fn commit(p: &wire::Page) -> wire::Commit {
    wire::Commit {
        contract_version: 1,
        operation: "CommitAdminAnalyticsSnapshotV1".into(),
        source_authority: p.source_authority.clone(),
        family: p.family,
        world_id: p.world_id.clone(),
        bucket_start: p.bucket_start.clone(),
        bucket_end: p.bucket_end.clone(),
        analytics_epoch: p.analytics_epoch.clone(),
        snapshot_revision: p.snapshot_revision.clone(),
        source_observed_at: p.source_observed_at.clone(),
        privacy_policy_revision: p.privacy_policy_revision.clone(),
        state: "replace".into(),
        row_count: p.rows.len().to_string(),
        page_digests: vec![store::digest(&serde_json::to_vec(p).unwrap())],
    }
}
async fn apply(store: &store::Store, c: &wire::Commit) -> Result<&'static str, store::Error> {
    let raw = serde_json::to_vec(c).unwrap();
    store
        .ingest(wire::parse("commit", &raw, NOW).unwrap(), &raw, NOW)
        .await
}
#[tokio::test]
async fn postgres_retries_conflicts_revisions_pages_and_fence() {
    let Some((pool, root, name)) = database().await else {
        eprintln!("PORTAL_TEST_DATABASE_URL unset");
        return;
    };
    let s = store(pool.clone()).await;
    let mut p = page();
    assert_eq!(publish(&s, &p).await.unwrap(), "accepted");
    assert_eq!(publish(&s, &p).await.unwrap(), "accepted");
    let c = commit(&p);
    assert_eq!(apply(&s, &c).await.unwrap(), "accepted");
    // Equal retry models a lost acknowledgement after the durable commit.
    assert_eq!(apply(&s, &c).await.unwrap(), "accepted");
    p.snapshot_revision = "3".into();
    p.rows[0].state = "incomplete".into();
    p.rows[0].values.clear();
    publish(&s, &p).await.unwrap();
    apply(&s, &commit(&p)).await.unwrap();
    let state: String = sqlx::query_scalar("SELECT state FROM admin_analytics.rows")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "incomplete");
    p.snapshot_revision = "2".into();
    assert_eq!(publish(&s, &p).await.unwrap(), "superseded");
    assert_eq!(apply(&s, &commit(&p)).await.unwrap(), "superseded");
    p.snapshot_revision = "4".into();
    p.page_count = "2".into();
    publish(&s, &p).await.unwrap();
    let mut c = commit(&p);
    c.page_digests.push("a".repeat(64));
    c.row_count = "2".into();
    assert!(matches!(
        apply(&s, &c).await,
        Err(store::Error::Unavailable)
    ));
    let mut second = p.clone();
    second.page_index = "1".into();
    second.source_observed_at = "1793836801".into();
    second.rows[0].key = "z".into();
    publish(&s, &second).await.unwrap();
    c.page_digests[1] = store::digest(&serde_json::to_vec(&second).unwrap());
    assert!(matches!(apply(&s, &c).await, Err(store::Error::Invalid)));
    second.source_observed_at = p.source_observed_at.clone();
    assert!(matches!(
        publish(&s, &second).await,
        Err(store::Error::Conflict)
    ));
    let mut conflict = commit(&page());
    conflict.state = "erase".into();
    conflict.row_count = "0".into();
    conflict.page_digests.clear();
    assert!(matches!(
        apply(&s, &conflict).await,
        Err(store::Error::Conflict)
    ));
    p.analytics_epoch = "2".into();
    p.snapshot_revision = "1".into();
    p.page_count = "1".into();
    publish(&s, &p).await.unwrap();
    assert_eq!(publish(&s, &page()).await.unwrap(), "superseded");
    let mut altered = page();
    altered.rows[0].values[0] = "1".into();
    assert!(matches!(
        publish(&s, &altered).await,
        Err(store::Error::Conflict)
    ));
    assert!(matches!(
        apply(&s, &commit(&altered)).await,
        Err(store::Error::Conflict)
    ));
    let mut unknown_policy = p.clone();
    unknown_policy.analytics_epoch = "3".into();
    unknown_policy.privacy_policy_revision = "unknown-policy".into();
    assert!(matches!(
        publish(&s, &unknown_policy).await,
        Err(store::Error::Unavailable)
    ));

    sqlx::raw_sql("DELETE FROM admin_analytics.rows; DELETE FROM admin_analytics.subjects; DELETE FROM admin_analytics.generations").execute(&pool).await.unwrap();
    let epoch: String = sqlx::query_scalar("SELECT epoch::text FROM admin_analytics.fence")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(epoch, "2");
    clean(pool, root, name).await;
}

#[derive(Clone)]
struct Identity(Option<crate::web::StaffIdentity>);
impl crate::web::StaffSession for Identity {
    async fn resolve(
        &self,
        _: &oteryn_web::session::BrowserSession,
    ) -> Option<crate::web::StaffIdentity> {
        self.0
    }
}
impl crate::rbac::AccountStateReader for Identity {
    async fn state(
        &self,
        _: Uuid,
    ) -> Result<crate::rbac::AccountState, crate::rbac::AccountStateUnavailable> {
        Ok(crate::rbac::AccountState::Active)
    }
}
fn panel(pool: PgPool, actor: Option<crate::web::StaffIdentity>, present: bool) -> Router {
    web::router(
        pool,
        Config {
            present,
            authority: "test-game".into(),
            privacy: "policy-1".into(),
            ..Config::default()
        },
        crate::web::PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Identity(actor),
        Identity(actor),
    )
}
async fn get(app: Router, path: &str) -> axum::response::Response {
    app.oneshot(
        Request::get(path)
            .header("host", "staff.test")
            .header("accept-language", "en")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}
async fn staff(pool: &PgPool, role: &str) -> crate::web::StaffIdentity {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2",
    )
    .bind(id)
    .bind(role)
    .execute(pool)
    .await
    .unwrap();
    crate::web::StaffIdentity {
        account: id,
        mfa_satisfied: true,
    }
}
#[tokio::test]
async fn postgres_audit_before_read_outage_and_rbac_denials() {
    let Some((pool, root, name)) = database().await else {
        eprintln!("PORTAL_TEST_DATABASE_URL unset");
        return;
    };
    let admin = staff(&pool, "platform_admin").await;
    let support = staff(&pool, "support").await;
    let mut unconfirmed = admin;
    unconfirmed.mfa_satisfied = false;
    sqlx::query("INSERT INTO roles(key) VALUES('custom_analytics')")
        .execute(&pool)
        .await
        .unwrap();
    let custom = staff(&pool, "custom_analytics").await;
    for actor in [
        None,
        Some(support),
        Some(custom),
        Some(unconfirmed),
        Some(crate::web::StaffIdentity {
            account: Uuid::now_v7(),
            mfa_satisfied: true,
        }),
    ] {
        let before: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM audit_log WHERE action='admin.analytics.denied'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        for path in [
            "/admin/analytics",
            "/admin/analytics/economy",
            "/admin/analytics/economy/data",
        ] {
            assert_eq!(
                get(panel(pool.clone(), actor, true), path).await.status(),
                StatusCode::FORBIDDEN
            );
        }
        let after: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM audit_log WHERE action='admin.analytics.denied'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(after, before + 3 * i64::from(actor.is_some()));
    }
    for path in [
        "/admin/analytics",
        "/admin/analytics/creatures",
        "/admin/analytics/activity",
        "/admin/analytics/levels",
        "/admin/analytics/health",
        "/admin/analytics/economy",
        "/admin/analytics/economy/data",
        "/admin/analytics/data",
    ] {
        let response = get(panel(pool.clone(), Some(admin), true), path).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "no-store, private"
        );
        assert_eq!(
            response.headers()[header::STRICT_TRANSPORT_SECURITY],
            "max-age=63072000; includeSubDomains"
        );
        assert_eq!(
            response.headers()[header::X_CONTENT_TYPE_OPTIONS],
            "nosniff"
        );
        assert_eq!(
            response.headers()[header::REFERRER_POLICY],
            "strict-origin-when-cross-origin"
        );
        assert!(
            response.headers()[header::CONTENT_SECURITY_POLICY]
                .to_str()
                .unwrap()
                .contains("frame-ancestors 'none'")
        );
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM audit_log WHERE action='admin.analytics.read' AND target=$1",
        )
        .bind(path)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1);
        let body = to_bytes(response.into_body(), 100000).await.unwrap();
        assert!(
            String::from_utf8_lossy(&body)
                .to_lowercase()
                .contains("unavailable")
        );
    }
    let authorizer = crate::rbac::Authorizer::new(pool.clone(), Identity(Some(admin)));
    let context = crate::rbac::CheckContext {
        account: admin.account,
        require_staff_mfa: true,
        mfa_verified: true,
    };
    let role: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE key='custom_analytics'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(matches!(
        authorizer
            .change(
                context,
                Uuid::now_v7(),
                crate::rbac::Change::SetPermissions {
                    role,
                    permissions: &[crate::rbac::Permission::AdminAnalyticsView]
                }
            )
            .await,
        Err(crate::rbac::Error::Denied)
    ));
    assert!(matches!(
        authorizer
            .change(
                context,
                Uuid::now_v7(),
                crate::rbac::Change::CreateRole {
                    key: "analyst_new",
                    permissions: &[crate::rbac::Permission::AdminAnalyticsExport]
                }
            )
            .await,
        Err(crate::rbac::Error::Denied)
    ));
    assert!(
        sqlx::query(
            "INSERT INTO role_permissions(role_id,permission) VALUES($1,'admin.analytics.view')"
        )
        .bind(role)
        .execute(&pool)
        .await
        .is_err()
    );
    sqlx::query("DELETE FROM role_permissions WHERE permission='admin.analytics.view'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        get(
            panel(pool.clone(), Some(admin), true),
            "/admin/analytics/data"
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("INSERT INTO role_permissions(role_id,permission) SELECT id,'admin.analytics.view' FROM roles WHERE key='platform_admin'").execute(&pool).await.unwrap();
    sqlx::raw_sql("CREATE FUNCTION reject_test_access_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test audit outage'; END $$; CREATE TRIGGER test_access_audit_outage BEFORE INSERT ON audit_log FOR EACH STATEMENT EXECUTE FUNCTION reject_test_access_audit()")
        .execute(&pool).await.unwrap();
    for actor in [Some(admin), None] {
        let response = get(panel(pool.clone(), actor, true), "/admin/analytics/data").await;
        assert_eq!(
            response.status(),
            if actor.is_some() {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::FORBIDDEN
            }
        );
        assert!(
            to_bytes(response.into_body(), 100000)
                .await
                .unwrap()
                .is_empty()
        );
    }
    clean(pool, root, name).await;
}

#[tokio::test]
async fn postgres_retention_privacy_watermark_and_baseline_gates() {
    let Some((pool, root, name)) = database().await else {
        eprintln!("PORTAL_TEST_DATABASE_URL unset");
        return;
    };
    let s = store(pool.clone()).await;
    let admin = staff(&pool, "platform_admin").await;
    let mut p = page();
    publish(&s, &p).await.unwrap();
    apply(&s, &commit(&p)).await.unwrap();
    let path = format!(
        "/admin/analytics/data?world={}&family=creature_kills_daily&start={}&end={}",
        p.world_id, p.bucket_start, p.bucket_end
    );
    let response = get(panel(pool.clone(), Some(admin), true), &path).await;
    let body = to_bytes(response.into_body(), 100000).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("unavailable"));
    // TEST ONLY: represents an already-accepted baseline for read-boundary tests.
    // No production API can create this evidence before §10.6 is accepted.
    sqlx::query("UPDATE admin_analytics.generations SET baseline_digest=$1,baseline_cut=1,accepted_at=now()").bind("a".repeat(64)).execute(&pool).await.unwrap();
    sqlx::query("UPDATE admin_analytics.fence SET active_generation=(SELECT id FROM admin_analytics.generations LIMIT 1)").execute(&pool).await.unwrap();
    let current = now();
    let mut w: wire::Watermark =
        serde_json::from_slice(&std::fs::read(format!("{FIXTURE}/valid/watermark.json")).unwrap())
            .unwrap();
    w.complete_through = current.to_string();
    w.observed_at = current.to_string();
    let raw = serde_json::to_vec(&w).unwrap();
    s.ingest(
        wire::parse("watermark", &raw, current).unwrap(),
        &raw,
        current,
    )
    .await
    .unwrap();
    let response = get(panel(pool.clone(), Some(admin), true), &path).await;
    let body = to_bytes(response.into_body(), 100000).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("\"visible\""));
    sqlx::query(
        "UPDATE admin_analytics.watermarks SET observed=$1::text::numeric,cut=$1::text::numeric",
    )
    .bind((current - 121).to_string())
    .execute(&pool)
    .await
    .unwrap();
    let response = get(panel(pool.clone(), Some(admin), true), &path).await;
    let body = to_bytes(response.into_body(), 100000).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("unavailable"));
    sqlx::query(
        "UPDATE admin_analytics.watermarks SET observed=$1::text::numeric,cut=$1::text::numeric",
    )
    .bind(current.to_string())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE admin_analytics.fence SET privacy_floor='policy-2'")
        .execute(&pool)
        .await
        .unwrap();
    let response = get(panel(pool.clone(), Some(admin), true), &path).await;
    let body = to_bytes(response.into_body(), 100000).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("unavailable"));
    sqlx::query("UPDATE admin_analytics.fence SET privacy_floor='policy-1'")
        .execute(&pool)
        .await
        .unwrap();
    for family in [
        wire::Family::WorldActivityHourly,
        wire::Family::CharactersCreatedDaily,
        wire::Family::EconomyTradesDaily,
    ] {
        p.family = family;
        p.rows[0].key = "world".into();
        p.rows[0].values = if family == wire::Family::WorldActivityHourly {
            vec!["0".into(); 4]
        } else if family == wire::Family::EconomyTradesDaily {
            vec!["0".into(); 2]
        } else {
            vec!["0".into()]
        };
        let end = (current - family.retention() - 86400) / family.interval() * family.interval();
        p.bucket_start = (end - family.interval()).to_string();
        p.bucket_end = end.to_string();
        p.source_observed_at = (end + 86400).to_string();
        let raw = serde_json::to_vec(&p).unwrap();
        let at = end + 86400;
        s.ingest(wire::parse("page", &raw, at).unwrap(), &raw, at)
            .await
            .unwrap();
        let c = commit(&p);
        let raw = serde_json::to_vec(&c).unwrap();
        s.ingest(wire::parse("commit", &raw, at).unwrap(), &raw, at)
            .await
            .unwrap();
        w.family = family;
        let raw = serde_json::to_vec(&w).unwrap();
        s.ingest(
            wire::parse("watermark", &raw, current).unwrap(),
            &raw,
            current,
        )
        .await
        .unwrap();
        let prefix = if family.economy() {
            "/admin/analytics/economy/data"
        } else {
            "/admin/analytics/data"
        };
        let family = serde_json::to_value(family).unwrap();
        let path = format!(
            "{prefix}?world={}&family={}&start={}&end={}",
            p.world_id,
            family.as_str().unwrap(),
            p.bucket_start,
            p.bucket_end
        );
        let response = get(panel(pool.clone(), Some(admin), true), &path).await;
        let body = to_bytes(response.into_body(), 100000).await.unwrap();
        let body = String::from_utf8_lossy(&body);
        assert!(body.contains("expired"), "{body}");
        assert!(!body.contains("\"values\":[\"0\"]"));
    }
    store::purge(&pool, current).await.unwrap();
    let retained:i64=sqlx::query_scalar("SELECT count(*) FROM admin_analytics.rows WHERE family IN ('world_activity_hourly','characters_created_daily')").fetch_one(&pool).await.unwrap();
    assert_eq!(retained, 0);
    sqlx::query("UPDATE admin_analytics.pages SET staged_at=now()-interval '25 hours'")
        .execute(&pool)
        .await
        .unwrap();
    store::purge(&pool, current).await.unwrap();
    let staged: i64 = sqlx::query_scalar("SELECT count(*) FROM admin_analytics.pages")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(staged, 0);
    clean(pool, root, name).await;
}

#[tokio::test]
async fn postgres_host_gate_refuses_before_session_and_audit() {
    let Some((pool, root, name)) = database().await else {
        eprintln!("PORTAL_TEST_DATABASE_URL unset");
        return;
    };
    for (host, duplicate, enabled, present) in [
        ("public.test", false, true, true),
        ("staff.test", true, true, true),
        ("staff.test", false, false, true),
        ("staff.test", false, true, false),
        ("public.test", false, false, false),
    ] {
        let app = web::router(
            pool.clone(),
            Config {
                present,
                ..Config::default()
            },
            crate::web::PanelConfig {
                admin_panel: enabled,
                host: "staff.test".into(),
            },
            Identity(None),
            Identity(None),
        );
        // An invalid cookie would be rejected by session middleware if the outer
        // host guard did not return first. Repeated probes must write no audit.
        for _ in 0..3 {
            let mut request = Request::get("/admin/analytics/economy")
                .header("host", host)
                .header("cookie", "__Host-oteryn_session=invalid")
                .body(Body::empty())
                .unwrap();
            if duplicate {
                request.headers_mut().append(
                    header::HOST,
                    axum::http::HeaderValue::from_static("staff.test"),
                );
            }
            assert_eq!(
                app.clone().oneshot(request).await.unwrap().status(),
                StatusCode::NOT_FOUND
            );
        }
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_log")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let support = staff(&pool, "support").await;
    assert_eq!(
        get(panel(pool.clone(), Some(support), true), "/admin/analytics")
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let denied: (Option<Uuid>, String) = sqlx::query_as("SELECT actor,action FROM audit_log")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        denied,
        (Some(support.account), "admin.analytics.denied".into())
    );
    clean(pool, root, name).await;
}

#[tokio::test]
async fn postgres_refused_messages_preserve_epoch_and_active_generation() {
    let Some((pool, root, name)) = database().await else {
        eprintln!("PORTAL_TEST_DATABASE_URL unset");
        return;
    };
    let s = store(pool.clone()).await;
    let p = page();
    publish(&s, &p).await.unwrap();
    apply(&s, &commit(&p)).await.unwrap();
    let generation: Uuid = sqlx::query_scalar("SELECT id FROM admin_analytics.generations")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE admin_analytics.fence SET active_generation=$1,rate_window=$2,rate_count=0",
    )
    .bind(generation)
    .bind((NOW / 60) as i64)
    .execute(&pool)
    .await
    .unwrap();
    let mut higher = p.clone();
    higher.analytics_epoch = "2".into();
    let w: wire::Watermark =
        serde_json::from_slice(&std::fs::read(format!("{FIXTURE}/valid/watermark.json")).unwrap())
            .unwrap();
    for wrong_policy in [true, false] {
        let limit = if wrong_policy { 0 } else { 1200 };
        sqlx::query("UPDATE admin_analytics.fence SET rate_count=$1")
            .bind(limit)
            .execute(&pool)
            .await
            .unwrap();
        let before:(String,Option<Uuid>,i32,i64)=sqlx::query_as("SELECT epoch::text,active_generation,rate_count,rate_window FROM admin_analytics.fence").fetch_one(&pool).await.unwrap();
        let mut page = higher.clone();
        if wrong_policy {
            page.privacy_policy_revision = "unaccepted-policy".into();
        }
        let mut watermark = w.clone();
        watermark.analytics_epoch = "2".into();
        watermark.privacy_policy_revision = page.privacy_policy_revision.clone();
        for (operation, raw) in [
            ("page", serde_json::to_vec(&page).unwrap()),
            ("commit", serde_json::to_vec(&commit(&page)).unwrap()),
            ("watermark", serde_json::to_vec(&watermark).unwrap()),
        ] {
            let result = s
                .ingest(wire::parse(operation, &raw, NOW).unwrap(), &raw, NOW)
                .await;
            assert!(if wrong_policy {
                matches!(result, Err(store::Error::Unavailable))
            } else {
                matches!(result, Err(store::Error::Rate))
            });
            let after:(String,Option<Uuid>,i32,i64)=sqlx::query_as("SELECT epoch::text,active_generation,rate_count,rate_window FROM admin_analytics.fence").fetch_one(&pool).await.unwrap();
            assert_eq!(after, before, "{operation}, wrong policy={wrong_policy}");
        }
    }
    sqlx::query("UPDATE admin_analytics.fence SET rate_count=0")
        .execute(&pool)
        .await
        .unwrap();
    // A dependency/capacity refusal must roll back the tentative fence too.
    assert!(matches!(
        apply(&s, &commit(&higher)).await,
        Err(store::Error::Unavailable)
    ));
    let mut capped = s.clone();
    capped.max_staging_bytes = 0;
    assert!(matches!(
        publish(&capped, &higher).await,
        Err(store::Error::Unavailable)
    ));
    let after: (String, Option<Uuid>) =
        sqlx::query_as("SELECT epoch::text,active_generation FROM admin_analytics.fence")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(after, ("1".into(), Some(generation)));
    assert_eq!(publish(&s, &higher).await.unwrap(), "accepted");
    let after: (String, Option<Uuid>) =
        sqlx::query_as("SELECT epoch::text,active_generation FROM admin_analytics.fence")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(after, ("2".into(), None));
    clean(pool, root, name).await;
}

#[tokio::test]
async fn postgres_reversible_analytics_migration_preserves_role_catalog() {
    let Some((pool, root, name)) = database().await else {
        eprintln!("PORTAL_TEST_DATABASE_URL unset");
        return;
    };
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261107140001_admin_analytics.down.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname='admin_analytics')"
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261107140001_admin_analytics.up.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let grants:Vec<String>=sqlx::query_scalar("SELECT r.key FROM roles r JOIN role_permissions p ON p.role_id=r.id WHERE p.permission IN ('admin.analytics.view','admin.analytics.export') ORDER BY p.permission").fetch_all(&pool).await.unwrap();
    assert_eq!(grants, vec!["platform_admin", "platform_admin"]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM admin_analytics.fence")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    clean(pool, root, name).await;
}

#[test]
fn economy_contract_byte_exact_vectors() {
    for (name, op, hash) in [
        (
            "circulation",
            "page",
            "d17393e355da190bb5a9e04658519835fa4b55fcffb29f35625fc3d458794b0b",
        ),
        (
            "market",
            "page",
            "d6ee5157fef3e99e8da53a6bf8207d41fc3e07e46e3db42d9dbd1f26f8b017ac",
        ),
        (
            "commit-market",
            "commit",
            "d53044c346577e0cffe14962110b831b52054d2fda35c0e73ec5a6e8573e70db",
        ),
        (
            "watermark-market",
            "watermark",
            "49f9807db6bfa7102a73a738eebb235c94493018ed09c72fb25bf47b361d6e46",
        ),
    ] {
        let raw = std::fs::read(format!("{FIXTURE}/economy/{name}.json")).unwrap();
        assert_eq!(store::digest(&raw), hash);
        assert!(wire::parse(op, &raw, 1793664600).is_ok(), "{name}");
    }
}

fn economy_page(name: &str) -> wire::Page {
    serde_json::from_slice(&std::fs::read(format!("{FIXTURE}/economy/{name}.json")).unwrap())
        .unwrap()
}
fn economy_members() -> Vec<wire::Page> {
    let circulation = economy_page("circulation");
    let mut result = vec![circulation.clone(), economy_page("market")];
    for (family, keys, values) in [
        (
            "economy_flows_daily",
            vec![
                "creature_loot",
                "house_rent",
                "market_fee",
                "npc_service",
                "npc_trade",
                "refund",
            ],
            vec!["0", "0"],
        ),
        ("economy_trades_daily", vec!["world"], vec!["1", "5"]),
        (
            "economy_store_daily",
            vec!["refund/coin", "spent_in_game/coin"],
            vec!["1", "5"],
        ),
    ] {
        let mut p = circulation.clone();
        p.family = serde_json::from_value(serde_json::json!(family)).unwrap();
        p.rows = keys
            .into_iter()
            .map(|key| wire::Row {
                key: key.into(),
                state: "visible".into(),
                values: values.iter().map(|v| (*v).into()).collect(),
            })
            .collect();
        result.push(p);
    }
    result
}
#[test]
fn economy_low_n_depends_on_contributors_not_values() {
    // §11.5: an all-zero row with one to four contributors is a valid `low_n` row.
    for p in economy_members() {
        let mut rows = p.rows.clone();
        for row in rows.iter_mut().filter(|row| !row.values.is_empty()) {
            row.state = "low_n".into();
            row.values.iter_mut().for_each(|v| *v = "0".into());
        }
        assert!(wire::rows(p.family, &rows, true).is_ok(), "{:?}", p.family);
    }
}
#[test]
fn economy_member_validation_and_family_states() {
    for p in economy_members() {
        assert!(wire::rows(p.family, &p.rows, true).is_ok());
        assert!(wire::parse("page", &serde_json::to_vec(&p).unwrap(), NOW).is_ok());
        for state in ["visible", "low_n", "incomplete", "suppressed"] {
            let mut rows = p.rows.clone();
            rows[0].state = state.into();
            if matches!(state, "incomplete" | "suppressed") {
                rows[0].values.clear();
            }
            assert_eq!(
                wire::rows(p.family, &rows, true).is_ok(),
                state != "suppressed",
                "{:?} {state}",
                p.family
            );
            if state == "low_n" {
                rows[0].values.clear();
                assert!(wire::rows(p.family, &rows, true).is_err());
            }
            if state == "incomplete" {
                rows[0].values.push("1".into());
                assert!(wire::rows(p.family, &rows, true).is_err());
            }
        }
        let mut bad = p.clone();
        bad.rows[0].values = vec!["18446744073709551616".into()];
        assert!(wire::parse("page", &serde_json::to_vec(&bad).unwrap(), NOW).is_err());
        if p.rows.len() > 1 {
            let mut rows = p.rows.clone();
            rows.swap(0, 1);
            assert!(wire::rows(p.family, &rows, true).is_err());
            rows = p.rows.clone();
            rows[1].key = rows[0].key.clone();
            assert!(wire::rows(p.family, &rows, true).is_err());
        }
        if !matches!(
            p.family,
            wire::Family::EconomyMarketDaily | wire::Family::EconomyStoreDaily
        ) {
            let mut rows = p.rows.clone();
            rows.pop();
            assert!(wire::rows(p.family, &rows, true).is_err());
            let mut missing = p.clone();
            missing.rows = rows;
            assert!(wire::parse("page", &serde_json::to_vec(&missing).unwrap(), NOW).is_err());
            rows = p.rows.clone();
            rows[0].key = "extra".into();
            rows.sort_by(|a, b| a.key.cmp(&b.key));
            assert!(wire::rows(p.family, &rows, true).is_err());
        }
        let mut bad = p.clone();
        bad.source_observed_at = (wire::uint(&p.bucket_end).unwrap() + 86399).to_string();
        assert!(wire::parse("page", &serde_json::to_vec(&bad).unwrap(), NOW).is_err());
        bad = p.clone();
        bad.bucket_start = (wire::uint(&p.bucket_start).unwrap() + 1).to_string();
        assert!(wire::parse("page", &serde_json::to_vec(&bad).unwrap(), NOW).is_err());
    }
    for family in [
        wire::Family::CreatureKillsDaily,
        wire::Family::BossKillsDaily,
        wire::Family::PlayerDeathsDaily,
        wire::Family::WorldActivityHourly,
        wire::Family::CharactersCreatedDaily,
        wire::Family::LevelDistributionDaily,
    ] {
        let key = if family == wire::Family::LevelDistributionDaily {
            "unknown"
        } else {
            "world"
        };
        let mut row = wire::Row {
            key: key.into(),
            state: "low_n".into(),
            values: vec!["1".into()],
        };
        assert!(wire::rows(family, &[row.clone()], false).is_err());
        row.state = "suppressed".into();
        row.values.clear();
        assert!(wire::rows(family, &[row], false).is_ok());
    }
}
#[test]
fn economy_inclusive_bounds_fixed_zero_and_store_coverage() {
    for mut p in economy_members() {
        for row in &mut p.rows {
            row.state = "visible".into();
            let bounds: Vec<u64> = match p.family {
                wire::Family::EconomyCirculationDaily => vec![1_000_000_000_000_000_000],
                wire::Family::EconomyFlowsDaily => vec![
                    if matches!(
                        row.key.as_str(),
                        "house_rent" | "market_fee" | "npc_service"
                    ) {
                        0
                    } else {
                        1_000_000_000_000_000_000
                    },
                    if row.key == "creature_loot" {
                        0
                    } else {
                        1_000_000_000_000_000_000
                    },
                ],
                wire::Family::EconomyMarketDaily => vec![
                    1_000_000_000_000_000,
                    1_000_000_000_000_000_000,
                    1_000_000_000_000,
                    999_999_999_999,
                ],
                _ => vec![1_000_000_000_000, 1_000_000_000_000_000_000],
            };
            row.values = bounds.iter().map(u64::to_string).collect();
            assert!(wire::rows(p.family, std::slice::from_ref(row), false).is_ok());
            for (i, bound) in bounds.iter().enumerate() {
                let mut bad = row.clone();
                bad.values[i] = (bound + 1).to_string();
                assert!(
                    wire::rows(p.family, &[bad], false).is_err(),
                    "{:?} {} {i}",
                    p.family,
                    row.key
                );
            }
            for value in ["-1", "01", "1.0", "", "18446744073709551616"] {
                let mut bad = row.clone();
                bad.values[0] = value.into();
                assert!(wire::rows(p.family, &[bad], false).is_err());
            }
        }
    }
    let mut p = economy_page("market");
    for values in [["1", "0", "0", "0"], ["0", "0", "0", "1"]] {
        p.rows[0].values = values.map(str::to_owned).to_vec();
        assert!(wire::rows(p.family, &p.rows, true).is_err());
    }
    for key in ["/item", "bad key", "é", "", &"a".repeat(513)] {
        p.rows[0].key = key.into();
        assert!(wire::rows(p.family, &p.rows, false).is_err());
    }
    let mut p = economy_members().pop().unwrap();
    for key in [
        "purchased/coin",
        "coin",
        "spent_in_game/",
        "refund/a/b",
        "refund/é",
        "refund/a b",
        &format!("refund/{}", "a".repeat(129)),
    ] {
        let mut row = p.rows[0].clone();
        row.key = key.into();
        assert!(wire::rows(p.family, &[row], false).is_err());
    }
    assert!(wire::rows(p.family, &p.rows[..1], true).is_err());
    p.rows.clear();
    for i in 0..16 {
        for prefix in ["refund", "spent_in_game"] {
            p.rows.push(wire::Row {
                key: format!("{prefix}/c{i:02}.A_:-"),
                state: "visible".into(),
                values: vec!["0".into(); 2],
            });
        }
    }
    p.rows.sort_by(|a, b| a.key.cmp(&b.key));
    assert!(wire::rows(p.family, &p.rows, true).is_ok());
    p.rows.push(wire::Row {
        key: "spent_in_game/extra".into(),
        state: "visible".into(),
        values: vec!["0".into(); 2],
    });
    assert!(wire::rows(p.family, &p.rows, true).is_err());
}

async fn accept_test_baseline(pool: &PgPool) {
    // TEST ONLY; no production resync operation is introduced.
    sqlx::query("UPDATE admin_analytics.generations SET baseline_digest=$1,baseline_cut=1,accepted_at=now()")
        .bind("a".repeat(64)).execute(pool).await.unwrap();
    sqlx::query("UPDATE admin_analytics.fence SET active_generation=(SELECT id FROM admin_analytics.generations LIMIT 1)")
        .execute(pool).await.unwrap();
}
async fn economy_watermark(s: &store::Store, p: &wire::Page) {
    let current = now();
    let w = wire::Watermark {
        contract_version: 1,
        operation: "PublishAdminAnalyticsWatermarkV1".into(),
        source_authority: p.source_authority.clone(),
        family: p.family,
        world_id: p.world_id.clone(),
        analytics_epoch: p.analytics_epoch.clone(),
        privacy_policy_revision: p.privacy_policy_revision.clone(),
        complete_through: current.to_string(),
        observed_at: current.to_string(),
    };
    let raw = serde_json::to_vec(&w).unwrap();
    s.ingest(
        wire::parse("watermark", &raw, current).unwrap(),
        &raw,
        current,
    )
    .await
    .unwrap();
}
#[tokio::test]
async fn postgres_economy_members_panel_and_refusal_fence() {
    let Some((pool, root, name)) = database().await else {
        return;
    };
    let s = store(pool.clone()).await;
    let admin = staff(&pool, "platform_admin").await;
    let mut members = economy_members();
    for p in &mut members {
        p.source_authority = "test-game".into();
        p.privacy_policy_revision = "policy-1".into();
        if p.rows[0].state == "visible" {
            p.rows[0].state = "low_n".into();
            if p.rows[0].values.iter().all(|v| v == "0") {
                p.rows[0].values[0] = "1".into();
            }
        }
        publish(&s, p).await.unwrap();
        apply(&s, &commit(p)).await.unwrap();
        economy_watermark(&s, p).await;
    }
    accept_test_baseline(&pool).await;
    let p = &members[0];
    let path = format!(
        "/admin/analytics/economy?world={}&start={}&end={}",
        p.world_id, p.bucket_start, p.bucket_end
    );
    let response = get(panel(pool.clone(), Some(admin), true), &path).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "no-store, private"
    );
    let body = String::from_utf8(
        to_bytes(response.into_body(), 200000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    for text in [
        "48210377",
        "low-n (below five)",
        "depot source coverage not yet delivered by Game",
        "gold_in / gold_out",
        "median_unit_price",
        "purchased",
        "Platform wallet ledger",
        "spent_in_game/coin",
        "refund/coin",
        "Data cut",
        "freshness",
    ] {
        assert!(body.contains(text), "{text}");
    }
    assert!(!body.contains("/characters/"));
    assert!(!body.contains("/journal"));
    let audit:i64=sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='admin.analytics.read' AND target='/admin/analytics/economy'").fetch_one(&pool).await.unwrap();
    assert_eq!(audit, 1);
    for query in [
        "family=characters_created_daily",
        "account=0192f4a0-7c00-7000-8000-000000000001",
        "start=1793491200&end=1793664000",
        "start=1793491201&end=1793577601",
    ] {
        assert_eq!(
            get(
                panel(pool.clone(), Some(admin), true),
                &format!("/admin/analytics/economy?{query}")
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        get(
            panel(pool.clone(), Some(admin), true),
            "/admin/analytics/data?family=economy_market_daily"
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    // Serve-time release delay is independent of the ingestion check.
    let recent_end = now() / 86400 * 86400;
    sqlx::query("UPDATE admin_analytics.subjects SET bucket_start=$1::text::numeric,bucket_end=$2::text::numeric WHERE family='economy_circulation_daily'")
        .bind((recent_end-86400).to_string()).bind(recent_end.to_string()).execute(&pool).await.unwrap();
    let recent = format!(
        "/admin/analytics/economy/data?world={}&start={}&end={recent_end}",
        p.world_id,
        recent_end - 86400
    );
    let response = get(panel(pool.clone(), Some(admin), true), &recent).await;
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 200000).await.unwrap()).unwrap();
    assert_eq!(body["groups"][0]["state"], "unavailable");
    assert_eq!(body["groups"][0]["cells"], serde_json::json!([]));
    sqlx::query("UPDATE admin_analytics.subjects SET bucket_start=$1::text::numeric,bucket_end=$2::text::numeric WHERE family='economy_circulation_daily'")
        .bind(&p.bucket_start).bind(&p.bucket_end).execute(&pool).await.unwrap();
    // Missing fixed key is detected at commit, rolling back a higher tentative epoch.
    let mut incomplete = p.clone();
    incomplete.analytics_epoch = "2".into();
    incomplete.rows.pop();
    incomplete.page_count = "2".into();
    let mut second = incomplete.clone();
    second.page_index = "1".into();
    second.rows = incomplete.rows.split_off(2);
    publish(&s, &incomplete).await.unwrap();
    publish(&s, &second).await.unwrap();
    let mut missing_commit = commit(&incomplete);
    missing_commit.row_count = "3".into();
    missing_commit
        .page_digests
        .push(store::digest(&serde_json::to_vec(&second).unwrap()));
    // Re-establish an old active generation to prove the refusal leaves it unchanged.
    sqlx::query("UPDATE admin_analytics.fence SET epoch=1,active_generation=(SELECT id FROM admin_analytics.generations LIMIT 1)").execute(&pool).await.unwrap();
    let before: (String, Option<Uuid>, i32, i64) = sqlx::query_as(
        "SELECT epoch::text,active_generation,rate_count,rate_window FROM admin_analytics.fence",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(matches!(
        apply(&s, &missing_commit).await,
        Err(store::Error::Invalid)
    ));
    let after: (String, Option<Uuid>, i32, i64) = sqlx::query_as(
        "SELECT epoch::text,active_generation,rate_count,rate_window FROM admin_analytics.fence",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(before, after);
    // Every malformed page must fail HTTP before it can affect the fence.
    let identities = Arc::new(
        guard::IdentityMap::new(vec![guard::Binding {
            identity: guard::Identity::san("economy-test").unwrap(),
            group: "game-admin-analytics".into(),
        }])
        .unwrap(),
    );
    let app = router(
        pool.clone(),
        Config {
            ingest: true,
            authority: "test-game".into(),
            privacy: "policy-1".into(),
            requests_per_minute: 1200,
            max_staging_bytes: 268435456,
            ..Config::default()
        },
        identities,
    );
    let mut invalids = Vec::new();
    for p in &members {
        let mut bad = p.clone();
        bad.analytics_epoch = "99".into();
        bad.rows[0].state = "suppressed".into();
        bad.rows[0].values.clear();
        invalids.push(bad);
        let mut bad = p.clone();
        bad.analytics_epoch = "99".into();
        bad.rows[0].values[0] = "1000000000000000001".into();
        invalids.push(bad);
        let mut bad = p.clone();
        bad.analytics_epoch = "99".into();
        bad.rows[0].state = "low_n".into();
        bad.rows[0].values.clear();
        invalids.push(bad);
    }
    for bad in invalids {
        let raw = serde_json::to_vec(&bad).unwrap();
        let mut request = Request::post("/internal/v1/admin-analytics/page")
            .header("content-type", "application/json")
            .body(Body::from(raw))
            .unwrap();
        request
            .extensions_mut()
            .insert(guard::VerifiedClient::from_verified_tls13_http11(
                guard::Identity::san("economy-test").unwrap(),
            ));
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
        let after:(String,Option<Uuid>,i32,i64)=sqlx::query_as("SELECT epoch::text,active_generation,rate_count,rate_window FROM admin_analytics.fence").fetch_one(&pool).await.unwrap();
        assert_eq!(before, after);
    }
    // Audit failure prevents values from leaving either economy response.
    sqlx::raw_sql("CREATE FUNCTION reject_economy_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test outage'; END $$; CREATE TRIGGER economy_audit_outage BEFORE INSERT ON audit_log FOR EACH STATEMENT EXECUTE FUNCTION reject_economy_audit()") .execute(&pool).await.unwrap();
    for path in [&path, &path.replacen("economy?", "economy/data?", 1)] {
        let response = get(panel(pool.clone(), Some(admin), true), path).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(
            to_bytes(response.into_body(), 200000)
                .await
                .unwrap()
                .is_empty()
        );
    }
    clean(pool, root, name).await;
}
#[tokio::test]
async fn postgres_economy_migration_rollback_reapply() {
    let Some((pool, root, name)) = database().await else {
        return;
    };
    let s = store(pool.clone()).await;
    let p = page();
    publish(&s, &p).await.unwrap();
    apply(&s, &commit(&p)).await.unwrap();
    let before: (String, i64) = sqlx::query_as(
        "SELECT epoch::text,(SELECT count(*) FROM admin_analytics.rows) FROM admin_analytics.fence",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    for _ in 0..2 {
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261107170001_admin_analytics_economy.down.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261107170001_admin_analytics_economy.up.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
    }
    let after: (String, i64) = sqlx::query_as(
        "SELECT epoch::text,(SELECT count(*) FROM admin_analytics.rows) FROM admin_analytics.fence",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(before, after);
    let mut p = economy_page("market");
    p.source_authority = "test-game".into();
    p.privacy_policy_revision = "policy-1".into();
    publish(&s, &p).await.unwrap();
    apply(&s, &commit(&p)).await.unwrap();
    // Rollback with low_n data fails transactionally without losing fences/data.
    let mut tx = pool.begin().await.unwrap();
    assert!(
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261107170001_admin_analytics_economy.down.sql"
        ))
        .execute(&mut *tx)
        .await
        .is_err()
    );
    tx.rollback().await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM admin_analytics.rows WHERE state='low_n'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    clean(pool, root, name).await;
}

#[test]
fn economy_wire_refuses_non_strings_unknown_members_and_purchased() {
    let p = economy_page("market");
    let raw = String::from_utf8(serde_json::to_vec(&p).unwrap()).unwrap();
    for value in ["-1", "1.5", "null"] {
        let bad = raw.replacen("\"412\"", value, 1);
        assert!(wire::parse("page", bad.as_bytes(), NOW).is_err());
    }
    for bad in [
        raw.replace("economy_market_daily", "economy_purchased_daily"),
        raw.replace("\"rows\":[", "\"account_id\":\"hidden\",\"rows\":["),
    ] {
        assert!(wire::parse("page", bad.as_bytes(), NOW).is_err());
    }
}
