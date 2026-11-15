use super::coin_listener_apps;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use oteryn_internal_api::guard::{Binding, Identity, IdentityMap};
use oteryn_wallet::{
    Ledger, WalletFlags,
    wire::{self, DatabasePolicy, Flags},
};
use tower::ServiceExt;

#[tokio::test]
async fn sw3_05_coin_routes_are_internal_only() -> Result<(), Box<dyn std::error::Error>> {
    for enabled in [false, true] {
        let pool =
            sqlx::postgres::PgPoolOptions::new().connect_lazy("postgres://localhost/unused")?;
        let coins = wire::router(
            Ledger::new(
                pool,
                WalletFlags { enabled },
                DatabasePolicy,
                DatabasePolicy,
                DatabasePolicy,
            ),
            Flags {
                transfers: enabled,
                market: enabled,
            },
            IdentityMap::new(vec![Binding {
                identity: Identity::san("coins.test.invalid")?,
                group: "game-coins".into(),
            }])?,
        );
        let (public, internal) = coin_listener_apps(Router::new(), coins);
        let native = public.clone();
        for (path, get) in [
            ("transfers", false),
            ("market/offers", false),
            ("market/settlements", false),
            ("market/cancellations", false),
            ("operations/01900000-0000-7000-8000-000000000001", true),
            (
                "accounts/01900000-0000-7000-8000-000000000001/balance",
                true,
            ),
            (
                "accounts/01900000-0000-7000-8000-000000000001/history",
                true,
            ),
        ] {
            for (app, expected) in [
                (&public, StatusCode::NOT_FOUND),
                (&native, StatusCode::NOT_FOUND),
                (
                    &internal,
                    if enabled {
                        StatusCode::UNAUTHORIZED
                    } else {
                        StatusCode::SERVICE_UNAVAILABLE
                    },
                ),
            ] {
                let request = Request::builder()
                    .method(if get { "GET" } else { "POST" })
                    .uri(format!("/internal/v1/coins/{path}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))?;
                assert_eq!(
                    app.clone().oneshot(request).await?.status(),
                    expected,
                    "enabled={enabled} path={path}"
                );
            }
        }
    }
    Ok(())
}
