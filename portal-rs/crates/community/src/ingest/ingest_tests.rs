#![allow(clippy::unwrap_used)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use oteryn_internal_api::guard::{Binding, Identity, VerifiedClient};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn app(db: PgPool, enabled: bool) -> Router {
    router(
        db,
        Arc::new(
            IdentityMap::new(vec![Binding {
                identity: Identity::san("projection-test").unwrap(),
                group: "game-public".into(),
            }])
            .unwrap(),
        ),
        Config {
            enabled,
            source_authority: "oteryn:game:primary".into(),
        },
    )
}
async fn send(app: Router, path: &str, body: &str, identity: &str) -> Response {
    app.oneshot(
        Request::post(format!("/internal/v1/game-public/{path}"))
            .header("content-type", "application/json")
            .extension(VerifiedClient::from_verified_tls13_http11(
                Identity::san(identity).unwrap(),
            ))
            .body(Body::from(body.to_owned()))
            .unwrap(),
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn disabled_identity_limits_and_malformed() -> TestResult {
    let db = PgPool::connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let response = send(
        app(db.clone(), false),
        "world-online",
        "{}",
        "projection-test",
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(to_bytes(response.into_body(), 100).await?.is_empty());
    assert_eq!(
        send(app(db.clone(), true), "world-online", "{}", "other-test")
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    for f in FAMILIES {
        assert_eq!(
            send(
                app(db.clone(), true),
                f.path,
                &"x".repeat(f.max_bytes + 1),
                "projection-test"
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            send(app(db.clone(), true), f.path, "{}", "projection-test")
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    Ok(())
}
#[test]
fn every_family_schema_and_golden_harness() -> TestResult {
    let root = std::env::temp_dir().join(format!("projection-fixtures-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    for (name, raw) in FIXTURES {
        std::fs::write(root.join(format!("{name}.json")), raw)?;
        let value = canonical::decode(raw.as_bytes(), 524288, 6)?;
        let family = FAMILIES.iter().find(|f| f.name == *name).copied();
        assert!(publication(family, &value, 1790000100).is_ok(), "{name}");
    }
    assert_eq!(oteryn_internal_api::fixtures::load(&root)?.len(), 7);
    std::fs::remove_dir_all(root)?;
    Ok(())
}
#[test]
fn rejects_invalid_envelopes_and_family_content() -> TestResult {
    for (name, raw) in FIXTURES {
        let value = canonical::decode(raw.as_bytes(), 524288, 6)?;
        let family = FAMILIES.iter().find(|f| f.name == *name).copied();
        for (key, bad) in [
            ("contract_version", Value::Unsigned(2)),
            ("projection_epoch", Value::String("01".into())),
            ("source_authority", Value::String("bad authority".into())),
            ("operation", Value::String("unknown".into())),
        ] {
            let mut invalid = value.clone();
            if let Value::Object(ref mut fields) = invalid {
                fields.iter_mut().find(|(k, _)| k == key).unwrap().1 = bad;
            }
            assert!(
                publication(family, &invalid, 1790000100).is_err(),
                "{name}: {key}"
            );
        }
        assert!(publication(family, &value, 1).is_err(), "future evidence");
        if let Some(family) = family {
            let mut invalid = value.clone();
            if let Value::Object(ref mut fields) = invalid {
                fields.push(("account_id".into(), Value::String("private".into())));
            }
            assert!(publication(Some(family), &invalid, 1790000100).is_err());
        }
    }
    Ok(())
}

#[tokio::test]
async fn database_families_ordering_rollback_and_watermarks() -> TestResult {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping public projection DB test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let pool = PgPool::connect(&url).await?;
    let schema = "projection_ingest_test";
    sqlx::query("CREATE SCHEMA projection_ingest_test")
        .execute(&pool)
        .await?;
    let options = pool
        .connect_options()
        .as_ref()
        .clone()
        .options([("search_path", schema)]);
    let db = PgPool::connect_with(options).await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105180000_t24_public_snapshots.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261115120019_fix_a2-l8_projection_conflicts.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261115110016_fix_w2a_ingest_budget.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261117200006_r7_com_a.sql"
    ))
    .execute(&db)
    .await?;
    for f in FAMILIES {
        let raw = FIXTURES.iter().find(|(name, _)| *name == f.name).unwrap().1;
        let response = send(app(db.clone(), true), f.path, raw, "projection-test").await;
        assert_eq!(response.status(), StatusCode::OK, "{}", f.name);
        assert_eq!(
            to_bytes(response.into_body(), 1000).await?.as_ref(),
            b"{\"contract_version\":1,\"result\":\"accepted\"}"
        );
        let p = publication(
            Some(*f),
            &canonical::decode(raw.as_bytes(), 524288, 6)?,
            1790000100,
        )
        .unwrap();
        assert!(latest(&db, f.name, &p.subject).await?.is_some());
        assert_eq!(apply(&db, &p).await.unwrap(), "accepted");
        let mut newer = p;
        newer.revision = "99999".into();
        newer.payload["test"] = true.into();
        assert_eq!(apply(&db, &newer).await.unwrap(), "accepted");
        newer.revision = "1".into();
        assert_eq!(apply(&db, &newer).await.unwrap(), "superseded");
        newer.revision = "99999".into();
        newer.payload["test"] = false.into();
        assert!(matches!(
            apply(&db, &newer).await,
            Err(ApplyError::Conflict)
        ));
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public_snapshots WHERE previous_snapshot IS NOT NULL",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(count, 6);
    let raw = FIXTURES.last().unwrap().1;
    let p = publication(
        None,
        &canonical::decode(raw.as_bytes(), 512, 6)?,
        1790000100,
    )
    .unwrap();
    assert!(matches!(apply(&db, &p).await, Err(ApplyError::Conflict)));
    sqlx::query("DELETE FROM public_projection_conflicts")
        .execute(&db)
        .await?;
    assert_eq!(apply(&db, &p).await.unwrap(), "accepted");
    assert_eq!(apply(&db, &p).await.unwrap(), "superseded");
    let mut old = p;
    old.observed -= 1;
    assert_eq!(apply(&db, &old).await.unwrap(), "superseded");
    old.epoch = "2".into();
    assert_eq!(apply(&db, &old).await.unwrap(), "accepted");
    let world = "01934f10-7c02-7001-805b-3b1122334401";
    assert!(latest(&db, "world_online", world).await?.is_none());
    db.close().await;
    sqlx::query("DROP SCHEMA projection_ingest_test CASCADE")
        .execute(&pool)
        .await?;
    Ok(())
}

#[tokio::test]
async fn highscores_ingest_accepts_filtered_world_ranks_and_game_categories() -> TestResult {
    let fixture = crate::audit06_test_db::Database::new().await?;
    let router = app(fixture.db.clone(), true);
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let snapshot = |category: u8, filter: u8, ranks: &[u64]| {
        let rows = ranks
            .iter()
            .enumerate()
            .map(|(i, rank)| {
                serde_json::json!({
                    "rank": rank.to_string(),
                    "character_id": format!("01934f10-7c04-7001-805b-{i:012x}"),
                    "name": "Aldric",
                    "vocation": 2,
                    "level": "412",
                    "value": "1000",
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "contract_version": 1,
            "operation": "PublishWorldHighscoresV1",
            "source_authority": "oteryn:game:primary",
            "world_id": "01934f10-7c02-7001-805b-3b1122334401",
            "category": category,
            "filter": filter,
            "projection_epoch": "1",
            "projection_revision": "1",
            "source_observed_at": now.to_string(),
            "computed_at": now.to_string(),
            "rows": rows,
        })
        .to_string()
    };
    let publish = |body: String| {
        let router = router.clone();
        async move { send(router, "highscores", &body, "projection-test").await }
    };
    for (category, filter, ranks) in [
        (1, 2, &[3, 3, 5][..]),
        (12, 0, &[1, 1, 3]),
        (14, 0, &[1]),
        (16, 0, &[1]),
        (17, 0, &[1]),
    ] {
        let response = publish(snapshot(category, filter, ranks)).await;
        assert_eq!(response.status(), StatusCode::OK, "{category}:{filter}");
        assert_eq!(
            to_bytes(response.into_body(), 1000).await?.as_ref(),
            br#"{"contract_version":1,"result":"accepted"}"#
        );
    }
    for (category, filter, ranks) in [
        (1, 0, &[3, 3, 5][..]),
        (1, 2, &[3, 3, 4]),
        (1, 2, &[3, 2]),
        (1, 2, &[0]),
        (1, 2, &[u64::MAX - 1, u64::MAX - 1, u64::MAX]),
        (13, 0, &[1]),
        (15, 0, &[1]),
    ] {
        assert_eq!(
            publish(snapshot(category, filter, ranks)).await.status(),
            StatusCode::BAD_REQUEST,
            "{category}:{filter}, ranks {ranks:?}"
        );
    }
    fixture.close().await
}

const FIXTURES: &[(&str, &str)] = &[
    (
        "highscores",
        r#"{"contract_version":1,"operation":"PublishWorldHighscoresV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","category":1,"filter":0,"projection_epoch":"1","projection_revision":"17","source_observed_at":"1790000000","computed_at":"1790000000","rows":[{"rank":"1","character_id":"01934f10-7c04-7001-805b-3b1122334401","name":"Aldric","vocation":4,"level":"412","value":"11400000000"}]}"#,
    ),
    (
        "character_profile",
        r#"{"contract_version":1,"operation":"PublishCharacterProfileV1","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"9","source_observed_at":"1790000000","profile":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Aldric","vocation":4,"level":"412","guild":[{"guild_id":"01934f10-7c06-7001-805b-3b1122334401","rank_level":3}],"guild_visible":true}]}"#,
    ),
    (
        "character_deaths",
        r#"{"contract_version":1,"operation":"PublishCharacterDeathsV1","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"3","source_observed_at":"1790000000","deaths":[{"occurred_at":"1789990000","level":"413","pvp":true,"killers":["Beren","Mira"]}]}"#,
    ),
    (
        "guild",
        r#"{"contract_version":1,"operation":"PublishGuildV1","source_authority":"oteryn:game:primary","guild_id":"01934f10-7c06-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"5","source_observed_at":"1790000000","guild":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Red Rose","state":"ACTIVE","founded_at":"1789000000","ranks":[{"level":1,"name":"Leader"},{"level":2,"name":"Vice Leader"},{"level":3,"name":"Member"}],"members":[{"name":"Aldric","rank_level":1,"joined_at":"1789000000"}]}]}"#,
    ),
    (
        "house",
        r#"{"contract_version":1,"operation":"PublishHouseV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","house_key":"oteryn:content.house.example-street-1","projection_epoch":"1","projection_revision":"12","source_observed_at":"1790000000","house":[{"name":"Example Street 1","kind":"private_house","town":"oteryn:content.area.city.example","size_sqm":"24","beds":"2","rent_gold":"5000","status":"OWNED","owner_kind":"CHARACTER","owner_name":"Aldric","paid_until":"1792000000"}]}"#,
    ),
    (
        "world_online",
        r#"{"contract_version":1,"operation":"PublishWorldOnlineV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"8812","source_observed_at":"1790000000","characters":[{"name":"Aldric","vocation":4,"level":"412"}]}"#,
    ),
    (
        "watermark",
        r#"{"contract_version":1,"operation":"PublishPublicProjectionWatermarkV1","source_authority":"oteryn:game:primary","family":"world_online","projection_epoch":"1","complete_through":"1790000000","observed_at":"1790000003"}"#,
    ),
];

#[tokio::test]
async fn guild_versions_accept_same_revision() -> TestResult {
    let fixture = crate::audit06_test_db::Database::new().await?;
    let raw = FIXTURES
        .iter()
        .find(|(name, _)| *name == "guild")
        .unwrap()
        .1;
    let v1 = app(fixture.db.clone(), true);
    let ids = IdentityMap::new(vec![Binding {
        identity: Identity::san("projection-test")?,
        group: "game-public".into(),
    }])?;
    let v2 = crate::guilds::ingest_router(
        fixture.db.clone(),
        Arc::new(ids),
        crate::guilds::IngestConfig {
            enabled: true,
            source_authority: "oteryn:game:primary".into(),
        },
    );
    // Preserve the canonical wire field order of the signed-off V1 vector.
    let body = raw
        .replace("PublishGuildV1", "PublishGuildV2")
        .replace(r#""contract_version":1"#, r#""contract_version":2"#)
        .replace(
            r#""name":"Aldric""#,
            r#""character_id":"01934f10-7c04-7001-805b-3b1122334401","name":"Aldric""#,
        );
    let publication: serde_json::Value = serde_json::from_str(&body)?;
    for _ in 0..2 {
        assert_eq!(
            send(v1.clone(), "guild", raw, "projection-test")
                .await
                .status(),
            StatusCode::OK
        );
        let response = v2
            .clone()
            .oneshot(
                Request::post("/internal/v2/game-public/guild")
                    .header("content-type", "application/json")
                    .extension(VerifiedClient::from_verified_tls13_http11(Identity::san(
                        "projection-test",
                    )?))
                    .body(Body::from(body.clone()))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
    }
    let id = publication["guild_id"].as_str().unwrap();
    let legacy = latest(&fixture.db, "guild_v1", id).await?.unwrap();
    let current = latest(&fixture.db, "guild", id).await?.unwrap();
    assert_eq!(current["guild"], publication["guild"]);
    assert!(
        legacy["guild"][0]["members"][0]
            .get("character_id")
            .is_none()
    );
    fixture.close().await
}
