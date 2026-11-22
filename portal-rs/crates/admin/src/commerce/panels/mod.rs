pub(super) mod catalog;
pub(super) mod payments;

use crate::{rbac::Permission, web};
use axum::{Router, response::Response};
use oteryn_ui::Chrome;
use sqlx::PgPool;

pub fn configured(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let mut routes = Router::new();
    for (enabled, permission, router) in [
        (
            enabled("OTERYN_PANEL_STORE_CATALOG_ENABLED"),
            Permission::ImportStoreCatalog,
            catalog::staging(pool.clone()),
        ),
        (
            enabled("OTERYN_PANEL_STORE_CATALOG_ENABLED"),
            Permission::ActivateStoreCatalog,
            catalog::activation(pool.clone()),
        ),
        (
            enabled("OTERYN_PANEL_PAYMENTS_ENABLED"),
            Permission::ReconcilePayments,
            payments::routes(pool.clone()),
        ),
    ] {
        if enabled {
            routes = routes.merge(web::configured_guard(router, pool.clone(), permission)?);
        }
    }
    Ok(routes)
}

fn payment_actions_enabled() -> bool {
    enabled("OTERYN_ADMIN_COMMERCE")
        && enabled("OTERYN_PAYMENTS_ENABLED")
        && std::env::var("OTERYN_ENV").as_deref() != Ok("production")
        && std::env::var_os("OTERYN_COMMERCE_TEST_PROVIDER_KEY_FILE").is_some()
}

fn enabled(flag: &str) -> bool {
    std::env::var(flag).as_deref() == Ok("true")
}

fn render(template: &impl askama::Template) -> Response {
    use axum::http::{HeaderValue, header};
    let mut response = oteryn_web::render(template);
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Language {
    lang: Option<String>,
}

fn localized(mut chrome: Chrome, lang: Option<&str>) -> Chrome {
    if let Some(lang @ ("pl" | "en")) = lang {
        chrome.lang = lang.to_owned();
    }
    chrome
}
