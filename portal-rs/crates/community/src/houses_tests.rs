#![allow(clippy::unwrap_used)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use oteryn_internal_api::{
    canonical,
    guard::{Binding, Identity, VerifiedClient},
};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;
const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
// SOURCE: G-4 proposal house record, synthetic fixture adapted to task house-key rule.
fn snapshot(epoch: u64, revision: u64, house: &str) -> String {
    format!(
        r#"{{"contract_version":2,"operation":"PublishHouseV2","source_authority":"oteryn:game:primary","world_id":"{WORLD}","house_key":"oteryn:content.house.example_shop","projection_epoch":"{epoch}","projection_revision":"{revision}","source_observed_at":"{}","house":{house}}}"#,
        now().saturating_sub(1)
    )
}
const VACANT: &str = r#"[{"name":"Example Shop","kind":"shop","town":"example","size_sqm":"24","beds":"0","rent_gold":"5000","status":"VACANT"}]"#;
fn watermark(epoch: u64, complete: u64) -> String {
    format!(
        r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"oteryn:game:primary","family":"house","projection_epoch":"{epoch}","complete_through":"{complete}","observed_at":"{}"}}"#,
        now()
    )
}
fn config(enabled: bool) -> Config {
    Config {
        ingest: crate::ingest::Config {
            enabled,
            source_authority: "oteryn:game:primary".into(),
        },
        serving_enabled: enabled,
    }
}
fn app(db: PgPool, enabled: bool) -> Router {
    router(
        db,
        Arc::new(
            IdentityMap::new(vec![Binding {
                identity: Identity::san("house-test").unwrap(),
                group: "game-public".into(),
            }])
            .unwrap(),
        ),
        Arc::new(DenyOwners),
        config(enabled),
    )
}
async fn send(app: Router, path: &str, body: String, peer: bool) -> Response {
    let mut request = Request::post(format!("/internal/v2/game-public/{path}"))
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    if peer {
        request
            .extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(
                Identity::san("house-test").unwrap(),
            ));
    }
    app.oneshot(request).await.unwrap()
}
async fn get(app: Router, path: &str) -> Response {
    app.oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}
#[test]
fn schema_success_and_bad_input() {
    let raw = snapshot(1, 1, VACANT);
    let decode = |s: &str| canonical::decode(s.as_bytes(), 4096, 5).unwrap();
    assert!(houses_schema::validate(&decode(&raw), false, now()).is_ok());
    for bad in [
        raw.replace("example_shop", "example-shop"),
        raw.replace("\"beds\":\"0\"", "\"beds\":0"),
        raw.replace(
            "\"rent_gold\":\"5000\"",
            "\"rent_gold\":\"9223372036854775808\"",
        ),
        raw.replace("\"status\":\"VACANT\"", "\"status\":\"UNKNOWN\""),
        raw.replace("\"contract_version\":2", "\"contract_version\":1"),
        raw.replace("\"beds\":\"0\"", "\"beds\":\"00\""),
        raw.replace(
            "\"name\":\"Example Shop\"",
            "\"name\":\"Example Shop\",\"private\":true",
        ),
    ] {
        assert!(
            houses_schema::validate(&decode(&bad), false, now()).is_err(),
            "{bad}"
        );
    }
    assert!(houses_schema::validate(&decode(&watermark(1, now() + 2)), true, now()).is_err());
    assert!(
        houses_schema::validate(
            &decode(&watermark(1, now()).replace("\"house\"", "\"character_activity\"")),
            true,
            now()
        )
        .is_err()
    );
    for (en, title) in [(true, "Houses"), (false, "Domy")] {
        assert_eq!(label(en, "title"), title);
        assert!(!label(en, "VACANT").is_empty());
    }
}
#[tokio::test]
async fn routes_refuse_disabled_wrong_peer_and_bad_input() -> TestResult {
    let db = PgPool::connect_lazy("postgres://localhost/unused")?;
    assert_eq!(
        get(app(db.clone(), false), "/en/houses").await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(
            app(db.clone(), false),
            "house",
            snapshot(1, 1, VACANT),
            true
        )
        .await
        .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        send(
            app(db.clone(), true),
            "house",
            snapshot(1, 1, VACANT),
            false
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(app(db.clone(), true), "house", watermark(1, now()), true)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            app(db.clone(), true),
            "watermark",
            snapshot(1, 1, VACANT),
            true
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(app(db.clone(), true), "house", " ".repeat(2049), true)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    for path in [
        "/en/houses?world=bad",
        "/en/houses?unknown=yes",
        "/en/houses/invalid",
        "/en/houses/oteryn:content.house.example_shop",
    ] {
        assert_eq!(
            get(app(db.clone(), true), path).await.status(),
            StatusCode::BAD_REQUEST
        );
    }
    Ok(())
}
#[tokio::test]
async fn postgres_ingest_presentation_privacy_freshness_and_fences() -> TestResult {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping houses PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let root = PgPool::connect(&url).await?;
    let schema = "houses_t24f_test";
    sqlx::query("DROP SCHEMA IF EXISTS houses_t24f_test CASCADE")
        .execute(&root)
        .await?;
    sqlx::query("CREATE SCHEMA houses_t24f_test")
        .execute(&root)
        .await?;
    let options = root
        .connect_options()
        .as_ref()
        .clone()
        .options([("search_path", schema)]);
    let db = PgPool::connect_with(options).await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105180000_t24_public_snapshots.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105231103_t24f_houses.sql"
    ))
    .execute(&db)
    .await?;
    assert_eq!(
        get(app(db.clone(), true), "/en/houses").await.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let raw = snapshot(1, 1, VACANT);
    for _ in 0..2 {
        let response = send(app(db.clone(), true), "house", raw.clone(), true).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(response.into_body(), 4096).await?.as_ref(),
            b"{\"contract_version\":2,\"result\":\"accepted\"}"
        );
    }
    assert_eq!(
        send(
            app(db.clone(), true),
            "house",
            raw.replace("Example Shop", "Other Shop"),
            true
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        get(app(db.clone(), true), "/en/houses").await.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        send(
            app(db.clone(), true),
            "watermark",
            watermark(1, now()),
            true
        )
        .await
        .status(),
        StatusCode::OK
    );
    let response = get(app(db.clone(), true), "/en/houses").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("Example Shop"));
    assert!(html.contains("Vacant"));
    assert!(html.contains("viewport"));
    let detail = format!("/en/houses/oteryn:content.house.example_shop?world={WORLD}");
    assert_eq!(
        get(app(db.clone(), true), &detail).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        get(
            app(db.clone(), true),
            &detail.replace("example_shop", "missing")
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    let owned=VACANT.replace("\"status\":\"VACANT\"","\"status\":\"OWNED\",\"owner_kind\":\"CHARACTER\",\"owner_name\":\"Private Owner\",\"paid_until\":\"1799999999\"");
    assert_eq!(
        send(app(db.clone(), true), "house", snapshot(1, 2, &owned), true)
            .await
            .status(),
        StatusCode::OK
    );
    let html = String::from_utf8(
        to_bytes(get(app(db.clone(), true), &detail).await.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(!html.contains("Private Owner"));
    assert!(!html.contains("1799999999"));
    assert!(html.contains("Occupancy unavailable"));
    struct PublicOwner;
    impl OwnerVisibility for PublicOwner {
        fn visible<'a>(
            &'a self,
            _: &'a str,
            _: &'a str,
            _: &'a str,
        ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
            Box::pin(async { true })
        }
    }
    let public_app = router(
        db.clone(),
        Arc::new(IdentityMap::default()),
        Arc::new(PublicOwner),
        config(true),
    );
    let public_html = String::from_utf8(
        to_bytes(get(public_app, &detail).await.into_body(), 65536)
            .await?
            .to_vec(),
    )?;
    assert!(public_html.contains("Private Owner"));
    assert!(public_html.contains("Paid until (UTC)"));

    assert_eq!(
        send(app(db.clone(), true), "house", snapshot(1, 3, "[]"), true)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        get(app(db.clone(), true), &detail).await.status(),
        StatusCode::NOT_FOUND
    );
    let old = send(app(db.clone(), true), "house", raw, true).await;
    assert!(
        String::from_utf8(to_bytes(old.into_body(), 4096).await?.to_vec())?.contains("superseded")
    );
    sqlx::query("UPDATE public_houses_v2_watermark SET complete_through=$1")
        .bind(now() as i64 - 31)
        .execute(&db)
        .await?;
    assert_eq!(
        get(app(db.clone(), true), &detail).await.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        send(app(db.clone(), true), "house", snapshot(2, 1, VACANT), true)
            .await
            .status(),
        StatusCode::OK
    );
    assert!(
        crate::ingest::latest(&db, "house", "anything")
            .await?
            .is_none()
    );
    assert_eq!(
        get(app(db.clone(), true), &detail).await.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    send(
        app(db.clone(), true),
        "watermark",
        watermark(2, now()),
        true,
    )
    .await;
    assert_eq!(
        get(app(db.clone(), true), &detail).await.status(),
        StatusCode::OK
    );
    // A v1 epoch raise fences v2 too; no old-generation fallback.
    sqlx::query("UPDATE public_projection_state SET highest_epoch=3 WHERE id=1")
        .execute(&db)
        .await?;
    assert_eq!(
        get(app(db.clone(), true), &detail).await.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    db.close().await;
    sqlx::query("DROP SCHEMA houses_t24f_test CASCADE")
        .execute(&root)
        .await?;
    eprintln!("houses: 1 PostgreSQL integration test ran");
    Ok(())
}
