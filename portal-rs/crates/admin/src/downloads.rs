//! Staff presentation and request decoding for offline-signed releases.
use std::{collections::BTreeMap, future::Future};

use askama::Template;
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Request, State, rejection::JsonRejection},
    handler::Handler,
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::rbac::{CheckContext, MutationAuthorization, Permission};

const MAX_FILE: usize = 1024 * 1024;
const MAX_METADATA: usize = 8 * MAX_FILE;
const MAX_POLICY: usize = 256 * 1024;
const MAX_FILES: usize = 32;
const MAX_REQUEST: usize = 12 * MAX_FILE;
const PUBLICATIONS_PER_MINUTE: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationFailure {
    Invalid,
    Verification,
    Conflict,
    Denied,
    Unavailable,
}

impl PublicationFailure {
    fn response(self) -> Response {
        let (status, code) = match self {
            Self::Invalid => (StatusCode::UNPROCESSABLE_ENTITY, "invalid"),
            Self::Verification => (StatusCode::UNPROCESSABLE_ENTITY, "verification"),
            Self::Conflict => (StatusCode::CONFLICT, "conflict"),
            Self::Denied => (StatusCode::FORBIDDEN, "denied"),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
        };
        (status, Json(json!({ "error": code }))).into_response()
    }
}

#[derive(Debug)]
pub struct SignedBundle {
    pub channel: String,
    pub policy: Vec<u8>,
    pub metadata: BTreeMap<String, Vec<u8>>,
}

/// Implemented by the release owner so browser and operator publications use the same service.
pub trait ReleasePublisher: Clone + Send + Sync + 'static {
    fn ready(&self, channel: &str) -> bool;

    fn publish(
        &self,
        context: CheckContext,
        authority: MutationAuthorization,
        bundle: SignedBundle,
    ) -> impl Future<Output = Result<Uuid, PublicationFailure>> + Send;
}

pub fn configured_router(
    pool: PgPool,
    publisher: impl ReleasePublisher,
) -> Result<Router, Box<dyn std::error::Error>> {
    let routes = Router::new()
        .route(
            "/admin/downloads",
            get(page).post(publish.layer(middleware::from_fn_with_state(
                pool.clone(),
                publication_quota,
            ))),
        )
        .layer(DefaultBodyLimit::max(MAX_REQUEST))
        .with_state((pool.clone(), publisher));
    crate::web::configured_guard(routes, pool, Permission::ManageDownloads)
}

async fn publication_quota(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    request: Request,
    next: Next,
) -> Response {
    let result = sqlx::query_as::<_, (i64, i64)>(
        "INSERT INTO web_rate_limits(limiter_name,key_hash,window_start,count)
         VALUES('downloads-publish',sha256(uuid_send($1)),date_trunc('minute',statement_timestamp()),1)
         ON CONFLICT(limiter_name,key_hash,window_start) DO UPDATE
         SET count=least(web_rate_limits.count+1,$2+1)
         RETURNING count, greatest(1,ceil(extract(epoch FROM
           window_start + interval '1 minute' - statement_timestamp())))::bigint",
    )
    .bind(context.account)
    .bind(PUBLICATIONS_PER_MINUTE)
    .fetch_one(&pool)
    .await;
    match result {
        Ok((count, _)) if count <= PUBLICATIONS_PER_MINUTE => next.run(request).await,
        Ok((_, retry_after)) => (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, retry_after.to_string())],
            Json(json!({ "error": "limited" })),
        )
            .into_response(),
        Err(_) => PublicationFailure::Unavailable.response(),
    }
}

#[derive(sqlx::FromRow)]
struct Publication {
    id: Uuid,
    release_id: String,
    version_display: String,
    policy_revision: i64,
    actor: Uuid,
    previous: Option<Uuid>,
    published: String,
    expires: String,
    active: bool,
    expired: bool,
}

struct ChannelHistory {
    label: &'static str,
    ready: bool,
    publications: Vec<Publication>,
}

#[derive(Template)]
#[template(path = "admin/downloads.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    csrf: String,
    ready: Vec<(&'static str, &'static str)>,
    history: Vec<ChannelHistory>,
    labels: fluent_bundle::FluentBundle<fluent_bundle::FluentResource>,
}

impl Page {
    fn text(&self, key: &str) -> String {
        crate::web::message(&self.labels, key)
    }
}

async fn page<P: ReleasePublisher>(
    State((pool, publisher)): State<(PgPool, P)>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
) -> Response {
    let lang = crate::web::language(&headers);
    let Ok(csrf) = session.csrf_token().await else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let mut ready = Vec::new();
    let mut history = Vec::new();
    for (name, label) in [("stable", "downloads-stable"), ("beta", "downloads-beta")] {
        let channel_ready = publisher.ready(name);
        if channel_ready {
            ready.push((name, label));
        }
        let Ok(publications) = sqlx::query_as(
            "SELECT r.id, r.release_id, r.version_display, r.policy_revision,
                    r.published_by AS actor, r.previous_publication_id AS previous,
                    to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS published,
                    to_char(r.expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') AS expires,
                    COALESCE(r.id = c.active_publication_id, false) AS active,
                    r.expires_at <= now() AS expired
             FROM client_channels c JOIN client_releases r ON r.channel_id = c.id
             WHERE c.name = $1 ORDER BY r.policy_revision DESC LIMIT 20",
        )
        .bind(name)
        .fetch_all(&pool)
        .await
        else {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        };
        history.push(ChannelHistory {
            label,
            ready: channel_ready,
            publications,
        });
    }
    let Some(labels) = crate::web::messages(lang) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    oteryn_web::render(&Page {
        lang,
        csrf,
        ready,
        history,
        labels,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Submission {
    channel: String,
    policy: String,
    metadata: Vec<MetadataFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MetadataFile {
    name: String,
    content: String,
}

impl Submission {
    fn decode(self) -> Result<SignedBundle, PublicationFailure> {
        if !matches!(self.channel.as_str(), "stable" | "beta")
            || !(1..=MAX_FILES).contains(&self.metadata.len())
        {
            return Err(PublicationFailure::Invalid);
        }
        let mut metadata = BTreeMap::new();
        let mut total = 0;
        for file in self.metadata {
            if file.name.len() > 64
                || !file.name.ends_with(".json")
                || !file
                    .name
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'.')
            {
                return Err(PublicationFailure::Invalid);
            }
            let content = decode_file(&file.content, MAX_FILE)?;
            total += content.len();
            if total > MAX_METADATA || metadata.insert(file.name, content).is_some() {
                return Err(PublicationFailure::Invalid);
            }
        }
        Ok(SignedBundle {
            channel: self.channel,
            policy: decode_file(&self.policy, MAX_POLICY)?,
            metadata,
        })
    }
}

fn decode_file(encoded: &str, limit: usize) -> Result<Vec<u8>, PublicationFailure> {
    if encoded.is_empty() || encoded.len() > limit.div_ceil(3) * 4 {
        return Err(PublicationFailure::Invalid);
    }
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| PublicationFailure::Invalid)?;
    if bytes.len() > limit {
        return Err(PublicationFailure::Invalid);
    }
    Ok(bytes)
}

async fn publish<P: ReleasePublisher>(
    State((_, publisher)): State<(PgPool, P)>,
    Extension(context): Extension<CheckContext>,
    Extension(authority): Extension<MutationAuthorization>,
    result: Result<Json<Submission>, JsonRejection>,
) -> Response {
    let bundle = match result {
        Ok(Json(submission)) => submission.decode(),
        Err(error) => {
            return (error.status(), Json(json!({ "error": "invalid" }))).into_response();
        }
    };
    match bundle {
        Ok(bundle) => match publisher.publish(context, authority, bundle).await {
            Ok(id) => (StatusCode::CREATED, Json(json!({ "id": id }))).into_response(),
            Err(error) => error.response(),
        },
        Err(error) => error.response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(names: &[&str]) -> Submission {
        Submission {
            channel: "stable".into(),
            policy: STANDARD.encode(b"{\n}"),
            metadata: names
                .iter()
                .map(|name| MetadataFile {
                    name: (*name).into(),
                    content: STANDARD.encode(b"{}\r\n"),
                })
                .collect(),
        }
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn submission_preserves_exact_bytes_and_validates_filenames() {
        let bundle = payload(&["1.root.json", "timestamp.json"])
            .decode()
            .unwrap();
        assert_eq!(bundle.policy, b"{\n}");
        assert_eq!(bundle.metadata["timestamp.json"], b"{}\r\n");
        for names in [vec![], vec!["../timestamp.json"], vec!["timestamp.json"; 2]] {
            assert_eq!(
                payload(&names).decode().err(),
                Some(PublicationFailure::Invalid)
            );
        }
    }

    #[test]
    fn submission_enforces_count_size_and_encoding() {
        assert!(
            payload(&["timestamp.json"; MAX_FILES + 1])
                .decode()
                .is_err()
        );
        let mut submission = payload(&["timestamp.json"]);
        submission.policy = STANDARD.encode(vec![0; MAX_POLICY + 1]);
        assert!(submission.decode().is_err());
        assert_eq!(
            decode_file("not base64", MAX_FILE),
            Err(PublicationFailure::Invalid)
        );
        assert_eq!(
            PublicationFailure::Denied.response().status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            PublicationFailure::Conflict.response().status(),
            StatusCode::CONFLICT
        );
    }
}
