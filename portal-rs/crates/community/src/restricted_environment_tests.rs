use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use oteryn_internal_api::guard::{Identity, VerifiedClient};
use tower::ServiceExt;
#[tokio::test]
async fn canonical_production_vetoes_all_projection_constructors()
-> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("L9_PROJECTIONS_CHILD").is_err() {
        for canonical in ["production", "testing", "preprod"] {
            let output = std::process::Command::new(std::env::current_exe()?)
                .args(["--exact", "restricted_environment_tests::canonical_production_vetoes_all_projection_constructors", "--nocapture"])
                .env("L9_PROJECTIONS_CHILD", "1").env("OTERYN_ENV", canonical).env("APP_ENV", "testing")
                .env("PORTAL_PUBLIC_PROJECTIONS_ENABLED", "true")
                .env("PORTAL_PUBLIC_PROJECTIONS_SAN", "projection.oteryn.internal")
                .env("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY", "oteryn:game:primary")
                .env("PORTAL_DEATHS_V2_INGEST_ENABLED", "true")
                .env("PORTAL_GUILDS_INGEST_ENABLED", "true")
                .env("PORTAL_HOUSES_INGEST_ENABLED", "true")
                .env("PORTAL_G4_SKILLS_INGEST_ENABLED", "true")
                .env("PORTAL_WORLD_DAILY_INGEST_ENABLED", "true")
                .output()?;
            assert!(
                output.status.success(),
                "{canonical}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        return Ok(());
    }
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://projection@127.0.0.1:1/unreachable")?;
    let family = crate::families::ingest_router(
        pool.clone(),
        std::sync::Arc::new(oteryn_internal_api::guard::IdentityMap::new(vec![
            oteryn_internal_api::guard::Binding {
                identity: Identity::san("projection.oteryn.internal")?,
                group: "game-public".into(),
            },
        ])?),
        crate::families::ingest_config(|key| std::env::var(key).ok()),
    );
    let cases = [
        (crate::ingest::configured_router(pool.clone()), "watermark"),
        (
            crate::deaths::configured_router(pool.clone()),
            "character-deaths",
        ),
        (crate::guilds::configured_router(pool.clone()), "guild"),
        (crate::houses::configured_router(pool), "house"),
        (family, "character-skills"),
    ];
    let production = std::env::var("OTERYN_ENV")?.as_str() == "production";
    for (app, path) in cases {
        let version = if path == "watermark" { "v1" } else { "v2" };
        let req = Request::post(format!("/internal/{version}/game-public/{path}"))
            .header("content-type", "application/json")
            .extension(VerifiedClient::from_verified_tls13_http11(Identity::san(
                "projection.oteryn.internal",
            )?))
            .body(Body::from("not json"))?;
        let response = app.oneshot(req).await?;
        assert_eq!(
            response.status(),
            if production {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::BAD_REQUEST
            },
            "{path}"
        );
    }
    let today = crate::today::Config::configured();
    assert_eq!(today.ingest_enabled, !production);
    Ok(())
}
