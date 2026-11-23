//! Localized entry pages for the existing setup guide and signed client releases.
use askama::Template;
use axum::{
    Router,
    extract::{Path, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use oteryn_ui::{Chrome, Group, Registry};
use serde_json::Value;
use sqlx::PgPool;

use crate::{
    AppState, render,
    surface::{Language, request_chrome},
};

const CHANNELS: [&str; 2] = ["stable", "beta"];

pub(super) fn routes() -> Router<AppState> {
    let mut router = Router::new();
    for lang in [Language::Pl, Language::En] {
        let code = lang.code();
        router = router
            .route(
                &format!("/{code}/start"),
                get(move |request: Request| async move {
                    render(&Start {
                        chrome: request_chrome(&request, lang),
                    })
                }),
            )
            .route(
                &format!("/{code}/download"),
                get(
                    move |State(state): State<AppState>, request: Request| async move {
                        let chrome = request_chrome(&request, lang);
                        downloads(&state.db, chrome, None, downloads_enabled()).await
                    },
                ),
            )
            .route(
                &format!("/{code}/download/{{channel}}"),
                get(
                    move |State(state): State<AppState>,
                          Path(channel): Path<String>,
                          request: Request| async move {
                        let chrome = request_chrome(&request, lang);
                        downloads(&state.db, chrome, Some(&channel), downloads_enabled()).await
                    },
                ),
            );
    }
    router
}

pub fn register(registry: &mut Registry) {
    registry.section(
        true,
        "start",
        "thais-nav-start",
        "start",
        Group::Play,
        false,
    );
    registry.section(
        downloads_enabled(),
        "downloads",
        "shell-downloads",
        "download",
        Group::Play,
        true,
    );
}

fn downloads_enabled() -> bool {
    std::env::var("OTERYN_DOWNLOADS_ENABLED").as_deref() == Ok("true")
}

#[derive(Template)]
#[template(path = "web/start.html")]
struct Start {
    chrome: Chrome,
}

#[derive(Template)]
#[template(path = "web/download.html")]
struct Downloads {
    chrome: Chrome,
    detail: bool,
    releases: Vec<Release>,
}

struct Release {
    channel: String,
    label: &'static str,
    version: String,
    packages: Vec<Package>,
}

struct Package {
    url: String,
    platform: &'static str,
    architecture: String,
}

async fn downloads(
    pool: &PgPool,
    chrome: Chrome,
    channel: Option<&str>,
    enabled: bool,
) -> Response {
    if !enabled || channel.is_some_and(|channel| !CHANNELS.contains(&channel)) {
        return StatusCode::NOT_FOUND.into_response();
    }
    // The downloads publisher advances this pointer only after verifying the
    // signed policy and matching it to the signed package descriptors.
    let rows = sqlx::query_as::<_, (String, String, sqlx::types::Json<Value>)>(
        "SELECT c.name, r.version_display, r.artifacts
         FROM client_channels c JOIN client_releases r ON r.id = c.active_publication_id
         WHERE c.name IN ('stable', 'beta') AND ($1::text IS NULL OR c.name = $1)
           AND r.channel_id = c.id AND r.expires_at > now()
         ORDER BY CASE c.name WHEN 'stable' THEN 0 ELSE 1 END",
    )
    .bind(channel)
    .fetch_all(pool)
    .await;
    let Ok(rows) = rows else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let releases = rows
        .into_iter()
        .filter_map(|(channel, version, artifacts)| release(channel, version, &artifacts.0))
        .collect::<Vec<_>>();
    let detail = channel.is_some();
    if detail && releases.is_empty() {
        return StatusCode::NOT_FOUND.into_response();
    }
    render(&Downloads {
        chrome,
        detail,
        releases,
    })
}

fn release(channel: String, version: String, artifacts: &Value) -> Option<Release> {
    let label = match channel.as_str() {
        "stable" => "download-stable",
        "beta" => "download-beta",
        _ => return None,
    };
    let packages = artifacts
        .as_object()?
        .iter()
        .map(|(path, descriptor)| package(&channel, path, descriptor))
        .collect::<Option<Vec<_>>>()?;
    (!packages.is_empty()).then_some(Release {
        channel,
        label,
        version,
        packages,
    })
}

fn package(channel: &str, path: &str, descriptor: &Value) -> Option<Package> {
    let [
        scope,
        target_channel,
        kind,
        version,
        system,
        architecture,
        filename,
    ]: [&str; 7] = path.split('/').collect::<Vec<_>>().try_into().ok()?;
    if scope != "channels"
        || target_channel != channel
        || kind != "releases"
        || !matches!(architecture, "x86_64" | "arm64" | "x86")
        || ![version, filename].into_iter().all(safe_segment)
    {
        return None;
    }
    let platform = match system {
        "windows" => "Windows",
        "linux" => "Linux",
        "macos" => "macOS",
        _ => return None,
    };
    let digest = descriptor.get("hashes")?.get("sha256")?.as_str()?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(Package {
        url: format!(
            "https://downloads.oteryn.com/updates/{channel}/targets/channels/{channel}/releases/{version}/{system}/{architecture}/{digest}.{filename}"
        ),
        platform,
        architecture: architecture.to_owned(),
    })
}

fn safe_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.len() <= 96
        && !matches!(segment, "." | "..")
        && segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_packages_use_the_signed_target_name() -> Result<(), Box<dyn std::error::Error>> {
        let descriptor = serde_json::json!({"hashes": {"sha256": "ab".repeat(32)}});
        let target = "channels/stable/releases/release-a/linux/arm64/client.tar.gz";
        let package = package("stable", target, &descriptor).ok_or("expected the package link")?;
        assert_eq!(
            package.url,
            format!(
                "https://downloads.oteryn.com/updates/stable/targets/channels/stable/releases/release-a/linux/arm64/{}.client.tar.gz",
                "ab".repeat(32)
            )
        );
        assert_eq!(package.platform, "Linux");
        Ok(())
    }

    #[test]
    fn package_descriptors_require_safe_segments_and_matching_channels() {
        let digest = serde_json::json!({"hashes": {"sha256": "ab".repeat(32)}});
        for path in [
            "channels/beta/releases/release-a/linux/arm64/client.tar.gz",
            "channels/stable/releases/../linux/arm64/client.tar.gz",
            "channels/stable/releases/release-a/linux/arm64/%2f",
            "channels/stable/releases/release-a/linux/arm64/..",
            "channels/stable/releases/release-a/linux/other/client.tar.gz",
        ] {
            assert!(package("stable", path, &digest).is_none(), "{path}");
        }
        assert!(
            package(
                "stable",
                "channels/stable/releases/release-a/linux/arm64/client.tar.gz",
                &serde_json::json!({"hashes": {"sha256": "short"}}),
            )
            .is_none()
        );
    }

    #[tokio::test]
    async fn disabled_downloads_and_unknown_channels_are_refused_before_storage()
    -> Result<(), sqlx::Error> {
        let pool = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
        for (channel, enabled) in [
            (None, false),
            (Some("stable"), false),
            (Some("unknown"), true),
        ] {
            let response = downloads(&pool, Chrome::default(), channel, enabled).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
        Ok(())
    }
}
