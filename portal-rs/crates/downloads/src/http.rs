use std::collections::BTreeMap;

use askama::Template;
use axum::{
    Extension, Router,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use oteryn_ui::{Account, Chrome, Registry};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{Channel, Downloads, Error, FeatureFlags, browser, storage::Release};

/// Static-like public GET surface. Staff uploads use a separate guarded router.
/// Trust roots are unnecessary here: only previously verified database publications can be read.
pub fn router(pool: PgPool, flags: FeatureFlags) -> Router {
    let mut router = Router::new()
        .route("/{lang}/download", get(browser::download))
        .route("/{lang}/start", get(browser::start))
        .route(
            "/{lang}/download/{channel}/{build}/{platform}/{architecture}",
            get(browser::package),
        )
        .route("/updates/{channel}/metadata/{filename}", get(metadata))
        .route("/updates/{channel}/policy-v1.json", get(policy))
        .route("/updates/{channel}/targets/{*target}", get(target));
    if flags.downloads {
        // Static paths take precedence over the site's localized channel routes.
        for lang in ["pl", "en"] {
            for platform in ["windows", "macos", "linux"] {
                router = router.route(
                    &format!("/{lang}/download/{platform}"),
                    get(move |downloads, chrome, selection| {
                        platform_download(downloads, chrome, selection, lang, platform)
                    }),
                );
            }
        }
    }
    router.with_state(Downloads::new(pool, flags, BTreeMap::new()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlatformSelection {
    channel: Option<Channel>,
}

#[derive(Template)]
#[template(path = "downloads/index.html")]
struct PlatformPage {
    chrome: Chrome,
    channel: Channel,
    release: Option<Release>,
}

async fn platform_download(
    State(downloads): State<Downloads>,
    chrome: Option<Extension<Chrome>>,
    selection: Result<Query<PlatformSelection>, QueryRejection>,
    lang: &'static str,
    platform: &'static str,
) -> Response {
    let mut chrome = chrome.map_or_else(
        || {
            Registry::default().chrome(
                lang,
                &format!("/{lang}/download/{platform}"),
                Account::Anonymous,
            )
        },
        |Extension(chrome)| chrome,
    );
    let reply = |response: Response| {
        (
            [
                (header::CACHE_CONTROL, "no-store"),
                (header::CONTENT_LANGUAGE, lang),
            ],
            response,
        )
            .into_response()
    };
    let Ok(Query(selection)) = selection else {
        return reply(oteryn_ui::error(&chrome, StatusCode::BAD_REQUEST));
    };
    let channel = selection.channel.unwrap_or(Channel::Stable);
    if channel == Channel::Beta {
        chrome.alternate_url.push_str("?channel=beta");
    }
    let mut release = match downloads.active_release(channel).await {
        Ok(release) => release,
        Err(Error::NotFound | Error::Disabled) => {
            return reply(oteryn_ui::error(&chrome, StatusCode::NOT_FOUND));
        }
        Err(_) => return reply(oteryn_ui::error(&chrome, StatusCode::SERVICE_UNAVAILABLE)),
    };
    release
        .packages
        .retain(|package| package.platform == platform);
    if release.packages.is_empty() {
        return reply(oteryn_ui::error(&chrome, StatusCode::NOT_FOUND));
    }
    let page = PlatformPage {
        chrome,
        channel,
        release: Some(release),
    };
    reply(match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => oteryn_ui::error(&page.chrome, StatusCode::INTERNAL_SERVER_ERROR),
    })
}

async fn metadata(
    State(downloads): State<Downloads>,
    Path((channel, filename)): Path<(String, String)>,
) -> Response {
    let result = match Channel::parse(&channel) {
        Ok(channel) => downloads.metadata(channel, &filename).await,
        Err(error) => Err(error),
    };
    document(
        result,
        if filename
            .split('.')
            .next()
            .is_some_and(|v| v.parse::<u64>().is_ok())
        {
            IMMUTABLE
        } else {
            MUTABLE
        },
    )
}

async fn policy(State(downloads): State<Downloads>, Path(channel): Path<String>) -> Response {
    let result = match Channel::parse(&channel) {
        Ok(channel) => downloads.policy(channel, None).await,
        Err(error) => Err(error),
    };
    document(result, MUTABLE)
}

async fn target(
    State(downloads): State<Downloads>,
    Path((channel, target)): Path<(String, String)>,
) -> Response {
    let result = match Channel::parse(&channel) {
        Ok(channel) => downloads.policy(channel, Some(&target)).await,
        Err(error) => Err(error),
    };
    document(result, IMMUTABLE)
}

const IMMUTABLE: &str = "public, max-age=31536000, immutable, no-transform";
const MUTABLE: &str = "no-store, no-transform";

fn document(result: Result<Vec<u8>, Error>, cache: &'static str) -> Response {
    match result {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, cache),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            bytes,
        )
            .into_response(),
        Err(error) => {
            let status = match error {
                Error::Disabled | Error::NotFound => StatusCode::NOT_FOUND,
                Error::Invalid(_) => StatusCode::BAD_REQUEST,
                Error::Denied => StatusCode::FORBIDDEN,
                _ => StatusCode::SERVICE_UNAVAILABLE,
            };
            (status, [(header::CACHE_CONTROL, "no-store")]).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_are_unavailable_or_refused_and_never_cached() {
        for (error, status) in [
            (Error::Disabled, StatusCode::NOT_FOUND),
            (Error::NotFound, StatusCode::NOT_FOUND),
            (Error::Invalid("channel"), StatusCode::BAD_REQUEST),
            (Error::Verification, StatusCode::SERVICE_UNAVAILABLE),
            (
                Error::Storage(sqlx::Error::PoolClosed),
                StatusCode::SERVICE_UNAVAILABLE,
            ),
        ] {
            let response = document(Err(error), IMMUTABLE);
            assert_eq!(response.status(), status);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        }
    }
}
