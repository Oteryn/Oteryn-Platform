use super::*;
use oteryn_internal_api::{canonical, guard::Identity};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
type TestResult = crate::runtime::tests::TestResult;
// SOURCE: Oteryn/Oteryn-Game@857f15f376907c592bb28d854fccff18a15cdf87
// docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md §§5,16.1.
const ASSIGN: &str = r#"{"contract_version":1,"operation":"ReportScopeAssignmentV1","assignment_epoch":"1","world_id":"01934f10-7c02-7001-805b-3b1122334401","channel_id":"01934f10-7c03-7001-805b-3b1122334401","ownership_generation":"3","node_identity":"<certificate subject of the assigned node host's runtime-status identity>","assigned_at":"1789999990"}"#;
const REVOKE: &str = r#"{"contract_version":1,"operation":"ReportScopeRevocationV1","assignment_epoch":"1","world_id":"01934f10-7c02-7001-805b-3b1122334401","channel_id":"01934f10-7c03-7001-805b-3b1122334401","ownership_generation":"4","revoked_at":"1790000100"}"#;
fn publication(revoke: bool) -> Result<Publication, StatusCode> {
    Publication::decode(
        &canonical::decode(if revoke { REVOKE } else { ASSIGN }.as_bytes(), 2048, 2)
            .map_err(|_| bad())?,
        revoke,
    )
}
fn settings(p: &Publication) -> Result<Settings, guard::ConfigError> {
    Ok(Settings {
        enabled: true,
        authorities: vec![Peer {
            name: "authority".into(),
            identity: Identity::san("authority")?,
            scopes: vec![(p.world, p.channel)],
        }],
        nodes: vec![Peer {
            name: publication(false)
                .map_err(|_| guard::ConfigError)?
                .node
                .ok_or(guard::ConfigError)?,
            identity: Identity::san("node")?,
            scopes: vec![(p.world, p.channel)],
        }],
        ..Settings::default()
    })
}
#[test]
fn golden_and_bad_input() -> TestResult {
    assert_eq!(publication(false)?.generation, 3);
    assert_eq!(publication(true)?.generation, 4);
    for (from, to) in [
        ("\"1\"", "\"01\""),
        ("\"3\"", "0"),
        ("1789999990", "9223372036854775808"),
        ("ReportScopeAssignmentV1", "unknown"),
    ] {
        let raw = ASSIGN.replacen(from, to, 1);
        assert!(Publication::decode(&canonical::decode(raw.as_bytes(), 2048, 2)?, false).is_err());
    }
    assert!(Publication::decode(&canonical::decode(REVOKE.as_bytes(), 2048, 2)?, false).is_err());
    assert!(
        canonical::decode(
            ASSIGN
                .replacen("{", "{\"contract_version\":1,", 1)
                .as_bytes(),
            2048,
            2
        )
        .is_err()
    );
    Ok(())
}
#[tokio::test]
async fn default_off_and_wrong_purpose() -> TestResult {
    let pool = PgPoolOptions::new().connect_lazy("postgres://localhost/unreachable")?;
    let app: Router = configured_routes(pool.clone(), |_| None);
    let req = || {
        axum::http::Request::post("/internal/v1/game-auth/native-scope-assignments")
            .header(header::CONTENT_TYPE, "application/json")
            .body(axum::body::Body::from(ASSIGN))
    };
    assert_eq!(
        app.oneshot(req()?).await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let p = publication(false)?;
    let identities = Arc::new(IdentityMap::new(vec![
        Binding {
            identity: Identity::san("authority")?,
            group: "native-scope-assignments".into(),
        },
        Binding {
            identity: Identity::san("node")?,
            group: "native-runtime-status".into(),
        },
    ])?);
    let app: Router = router(pool, settings(&p)?, identities, Arc::new(NoAudit));
    let mut request = req()?;
    request
        .extensions_mut()
        .insert(VerifiedClient::from_verified_tls13_http11(Identity::san(
            "node",
        )?));
    let response = app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(
        axum::body::to_bytes(response.into_body(), 1024)
            .await?
            .is_empty()
    );
    Ok(())
}
#[tokio::test]
async fn db_assignment_revocation_ordering_and_admission_denial() -> TestResult {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await?;
    let schema = format!("t20_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = PgPoolOptions::new()
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
        .await?;
    oteryn_db::migrate(&pool).await?;
    let p = publication(false)?;
    let settings = settings(&p)?;
    let identities = Arc::new(IdentityMap::new(vec![Binding {
        identity: Identity::san("authority")?,
        group: "native-scope-assignments".into(),
    }])?);
    let scope_app: Router = router(
        pool.clone(),
        settings.clone(),
        identities,
        Arc::new(NoAudit),
    );
    let request = crate::runtime::tests::request(
        "/internal/v1/game-auth/native-scope-assignments",
        Some("authority"),
        ASSIGN.as_bytes().to_vec(),
    )?;
    let response = scope_app.oneshot(request).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        axum::body::to_bytes(response.into_body(), 256)
            .await?
            .as_ref(),
        br#"{"contract_version":1,"result":"accepted"}"#
    );

    assert_eq!(
        publish(&pool, &settings, "authority", &p, &NoAudit).await?,
        "accepted"
    );
    assert_eq!(
        publish(&pool, &settings, "authority", &p, &NoAudit).await?,
        "accepted"
    );
    let mut tx = pool.begin().await?;
    assert!(
        allows(
            &mut tx,
            p.world,
            p.channel,
            p.node.as_deref().ok_or("missing node")?,
            1,
            3
        )
        .await?
    );
    tx.commit().await?;
    let mut conflict = p.clone();
    conflict.decided_at += 1;
    assert_eq!(
        publish(&pool, &settings, "authority", &conflict, &NoAudit).await,
        Err(StatusCode::CONFLICT)
    );
    conflict.channel = Uuid::now_v7();
    assert_eq!(
        publish(&pool, &settings, "authority", &conflict, &NoAudit).await,
        Err(StatusCode::UNAUTHORIZED)
    );
    // Exercise the runtime evidence path consumed by admission selection.
    let at = runtime::now()?;
    let mut report = crate::runtime::tests::value("1", at)?;
    if let Value::Object(fields) = &mut report {
        for (key, value) in fields {
            match key.as_str() {
                "world_id" => *value = string(p.world),
                "channel_id" => *value = string(p.channel),
                "scope_ownership_generation" => *value = string(p.generation),
                _ => {}
            }
        }
    }
    let runtime_settings = runtime::Settings {
        enabled: true,
        peers: settings.nodes.clone(),
        ..runtime::Settings::default()
    };
    let identities = Arc::new(IdentityMap::new(vec![Binding {
        identity: Identity::san("node")?,
        group: "native-runtime-status".into(),
    }])?);
    let app: Router = runtime::router(pool.clone(), runtime_settings.clone(), identities);
    let request = crate::runtime::tests::request(
        "/internal/v1/game-auth/native-runtime-status",
        Some("node"),
        canonical::encode(&report)?,
    )?;
    assert_eq!(app.clone().oneshot(request).await?.status(), StatusCode::OK);
    assert_eq!(
        runtime::read(&pool, &runtime_settings, p.world, p.channel, at)
            .await?
            .state,
        runtime::EvidenceState::Fresh
    );
    let r = publication(true)?;
    assert_eq!(
        publish(&pool, &settings, "authority", &r, &NoAudit).await?,
        "accepted"
    );
    assert_eq!(
        publish(&pool, &settings, "authority", &r, &NoAudit).await?,
        "accepted"
    );
    assert_eq!(
        publish(&pool, &settings, "authority", &p, &NoAudit).await?,
        "superseded"
    );
    let mut tx = pool.begin().await?;
    assert!(
        !allows(
            &mut tx,
            p.world,
            p.channel,
            p.node.as_deref().ok_or("missing node")?,
            1,
            3
        )
        .await?
    );
    tx.commit().await?;
    assert_eq!(
        runtime::read(&pool, &runtime_settings, p.world, p.channel, at)
            .await?
            .state,
        runtime::EvidenceState::Invalid
    );
    let request = crate::runtime::tests::request(
        "/internal/v1/game-auth/native-runtime-status",
        Some("node"),
        canonical::encode(&report)?,
    )?;
    // The runtime reader currently treats a null revoked holder as unavailable.
    assert_eq!(
        app.oneshot(request).await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let mut unknown_node = p.clone();
    unknown_node.node = Some("unknown".into());
    assert_eq!(
        publish(&pool, &settings, "authority", &unknown_node, &NoAudit).await,
        Err(StatusCode::CONFLICT)
    );
    let mut changed_revocation = r.clone();
    changed_revocation.decided_at += 1;
    assert_eq!(
        publish(&pool, &settings, "authority", &changed_revocation, &NoAudit).await,
        Err(StatusCode::CONFLICT)
    );
    let mut same = p.clone();
    same.generation = r.generation;
    assert_eq!(
        publish(&pool, &settings, "authority", &same, &NoAudit).await,
        Err(StatusCode::CONFLICT)
    );
    same.epoch = 2;
    assert_eq!(
        publish(&pool, &settings, "authority", &same, &RefuseAudit).await,
        Err(StatusCode::SERVICE_UNAVAILABLE)
    );
    assert_eq!(
        publish(&pool, &settings, "authority", &r, &NoAudit).await?,
        "accepted"
    );
    assert_eq!(
        publish(&pool, &settings, "authority", &same, &NoAudit).await?,
        "accepted"
    );
    assert_eq!(
        publish(&pool, &settings, "authority", &r, &NoAudit).await?,
        "superseded"
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    Ok(())
}

#[derive(Debug)]
struct RefuseAudit;
impl AuditHook for RefuseAudit {
    fn changed<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: &'a str,
        _: &'a Publication,
    ) -> Pin<Box<dyn Future<Output = Result<(), StatusCode>> + Send + 'a>> {
        Box::pin(async { Err(StatusCode::SERVICE_UNAVAILABLE) })
    }
}
#[test]
fn configuration_refuses_ambiguous_identity_and_bad_scope() -> TestResult {
    let p = publication(false)?;
    let scopes = format!(r#"["{}/{}"]"#, p.world, p.channel);
    let authority = format!(r#"{{"authority":{scopes}}}"#);
    let node = format!(r#"{{"node":{scopes}}}"#);
    let lookup = |key: &str| match key {
        "GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED" => Some("true".into()),
        "GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES" => Some(authority.clone()),
        "GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES" => Some(node.clone()),
        _ => None,
    };
    assert!(configuration(lookup).is_some());
    assert!(
        configuration(
            |key| if key == "GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY" {
                Some("authority".into())
            } else {
                lookup(key)
            }
        )
        .is_none()
    );
    assert!(
        configuration(
            |key| if key == "GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES" {
                Some(authority.clone())
            } else {
                lookup(key)
            }
        )
        .is_none()
    );
    assert!(
        configuration(
            |key| if key == "GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES" {
                Some(r#"{"authority":["unknown"]}"#.into())
            } else {
                lookup(key)
            }
        )
        .is_none()
    );
    Ok(())
}
