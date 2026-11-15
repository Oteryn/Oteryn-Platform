//! Single `/internal/v2/game-public/watermark` endpoint shared by the V2 public families.
//!
//! The V2 watermark wire shape carries its `family`, so Game posts every family's watermark to
//! one path. Each family module keeps its own guarded watermark route (mTLS identity, body and
//! JSON limits, flags); this dispatcher only selects which module router receives the unchanged
//! request. Unknown or missing families go to the deaths router, whose guard and closed family
//! check deny them, so selection never grants access by itself.
use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use std::sync::Arc;
use tower::ServiceExt;

pub const PATH: &str = "/internal/v2/game-public/watermark";
/// Larger than every family's guard bound; the family guard enforces its own exact limit.
const BUFFER_LIMIT: usize = 4096;

/// Which ingest routes a family module mounts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
    /// Payload routes and the watermark route (stand-alone module routers and tests).
    All,
    /// Everything except the shared watermark route.
    Payload,
    /// Only the shared watermark route, for [`router`].
    Watermark,
}
impl Part {
    pub fn includes(self, path: &str) -> bool {
        match self {
            Part::All => true,
            Part::Payload => path != "watermark",
            Part::Watermark => path == "watermark",
        }
    }
}

#[derive(Clone)]
struct Targets {
    deaths: Router,
    guilds: Router,
    houses: Router,
    today: Router,
}

/// Mount once; each argument is a module router built with [`Part::Watermark`].
pub fn router(deaths: Router, guilds: Router, houses: Router, today: Router) -> Router {
    Router::new()
        .route(PATH, post(dispatch))
        .with_state(Arc::new(Targets {
            deaths,
            guilds,
            houses,
            today,
        }))
}

/// Production wiring from the same environment as each module's `configured_router`.
pub fn configured_router(db: sqlx::PgPool) -> Router {
    router(
        crate::deaths::configured_watermark_router(db.clone()),
        crate::guilds::configured_watermark_router(db.clone()),
        crate::houses::configured_watermark_router(db.clone()),
        crate::today::configured_watermark_router(db),
    )
}

#[derive(serde::Deserialize)]
struct Peek {
    family: Option<String>,
}

/// Reuse the exact family guard without polling a request body or invoking a handler.
pub fn probe_request(request: &Request) -> Request {
    let mut probe = Request::new(Body::empty());
    *probe.method_mut() = request.method().clone();
    *probe.uri_mut() = request.uri().clone();
    *probe.headers_mut() = request.headers().clone();
    *probe.extensions_mut() = request.extensions().clone();
    probe
        .extensions_mut()
        .insert(oteryn_internal_api::guard::PeerAdmissionProbe);
    probe
}
pub async fn probe(target: &Router, request: Request) -> Response {
    match target.clone().oneshot(request).await {
        Ok(r) => r,
        Err(never) => match never {},
    }
}
/// Admit the shared transport before reading the only family discriminator.
/// A selected family's stricter guard still runs on the unchanged real request.
pub async fn admission(targets: &[Router], request: Request) -> Option<Response> {
    let mut refusal = None;
    for target in targets {
        let result = probe(target, probe_request(&request)).await;
        if result.status() == StatusCode::NO_CONTENT {
            return None;
        }
        if refusal
            .as_ref()
            .is_none_or(|r: &Response| r.status() != StatusCode::UNAUTHORIZED)
        {
            refusal = Some(result);
        }
    }
    refusal
}
pub fn dispatch_failure(status: StatusCode) -> Response {
    let mut r = status.into_response();
    r.headers_mut().insert(
        "cache-control",
        axum::http::HeaderValue::from_static("no-store, private"),
    );
    r.headers_mut()
        .insert("pragma", axum::http::HeaderValue::from_static("no-cache"));
    r
}
async fn dispatch(State(targets): State<Arc<Targets>>, request: Request) -> Response {
    if let Some(response) = admission(
        &[
            targets.deaths.clone(),
            targets.guilds.clone(),
            targets.houses.clone(),
            targets.today.clone(),
        ],
        probe_request(&request),
    )
    .await
    {
        return response;
    }
    let (parts, body) = request.into_parts();
    let Ok(bytes): Result<Bytes, _> = axum::body::to_bytes(body, BUFFER_LIMIT).await else {
        return dispatch_failure(StatusCode::BAD_REQUEST);
    };
    let family = serde_json::from_slice::<Peek>(&bytes)
        .ok()
        .and_then(|p| p.family);
    let target = match family.as_deref() {
        Some("guild") => targets.guilds.clone(),
        Some("house") => targets.houses.clone(),
        Some("world_daily") => targets.today.clone(),
        _ => targets.deaths.clone(),
    };
    let request = Request::from_parts(parts, Body::from(bytes));
    match target.oneshot(request).await {
        Ok(response) => response,
        Err(never) => match never {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::post;

    fn tagged(tag: &'static str) -> Router {
        Router::new().route(
            PATH,
            post(move |request: Request| async move {
                if request
                    .extensions()
                    .get::<oteryn_internal_api::guard::PeerAdmissionProbe>()
                    .is_some()
                {
                    return StatusCode::NO_CONTENT.into_response();
                }
                tag.into_response()
            }),
        )
    }

    type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

    async fn send(app: &Router, body: &str) -> TestResult<(StatusCode, String)> {
        let response = app
            .clone()
            .oneshot(
                Request::post(PATH)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_owned()))?,
            )
            .await?;
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1024).await?;
        Ok((status, String::from_utf8(bytes.to_vec())?))
    }

    #[tokio::test]
    async fn routes_by_family_and_defaults_to_closed_deaths_check() -> TestResult<()> {
        let app = router(
            tagged("deaths"),
            tagged("guilds"),
            tagged("houses"),
            tagged("today"),
        );
        assert_eq!(send(&app, r#"{"family":"guild"}"#).await?.1, "guilds");
        assert_eq!(send(&app, r#"{"family":"house"}"#).await?.1, "houses");
        assert_eq!(send(&app, r#"{"family":"world_daily"}"#).await?.1, "today");
        assert_eq!(
            send(&app, r#"{"family":"character_deaths"}"#).await?.1,
            "deaths"
        );
        assert_eq!(
            send(&app, r#"{"family":"character_skills"}"#).await?.1,
            "deaths"
        );
        assert_eq!(send(&app, "not json").await?.1, "deaths");
        assert_eq!(
            send(&app, &"x".repeat(BUFFER_LIMIT + 1)).await?.0,
            StatusCode::BAD_REQUEST
        );
        Ok(())
    }

    #[test]
    fn parts_split_the_watermark_route() {
        assert!(Part::All.includes("watermark") && Part::All.includes("guild"));
        assert!(!Part::Payload.includes("watermark") && Part::Payload.includes("house"));
        assert!(Part::Watermark.includes("watermark") && !Part::Watermark.includes("guild"));
    }
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    use oteryn_internal_api::guard::{
        self, Binding, Identity, IdentityMap, JsonRule, RoutePolicy, VerifiedClient,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    fn guarded(enabled: bool) -> Result<Router, Box<dyn std::error::Error>> {
        let app = Router::new().route(PATH, post(|| async { StatusCode::OK }));
        let identities = Arc::new(IdentityMap::new(vec![Binding {
            identity: Identity::san("watermark-test")?,
            group: "game-public".into(),
        }])?);
        Ok(guard::protect(
            app,
            identities,
            RoutePolicy {
                enabled,
                group: "game-public".into(),
                max_body_bytes: 512,
                max_json_depth: 6,
                json: JsonRule::Object,
            },
        )
        .layer(axum::middleware::from_fn(
            |request: Request, next: axum::middleware::Next| async move {
                let mut r = next.run(request).await;
                r.headers_mut().insert(
                    "cache-control",
                    axum::http::HeaderValue::from_static("no-store, private"),
                );
                r
            },
        )))
    }
    #[tokio::test]
    async fn disabled_and_wrong_peer_refuse_before_body_poll()
    -> Result<(), Box<dyn std::error::Error>> {
        for (enabled, peer, status) in [
            (false, "watermark-test", StatusCode::SERVICE_UNAVAILABLE),
            (true, "wrong-purpose", StatusCode::UNAUTHORIZED),
        ] {
            let target = guarded(enabled)?;
            let app = router(target.clone(), target.clone(), target.clone(), target);
            for oversized in [false, true] {
                let polled = Arc::new(AtomicUsize::new(0));
                let counter = polled.clone();
                let body = if oversized {
                    Body::from(vec![b'x'; BUFFER_LIMIT + 1])
                } else {
                    Body::from_stream(futures_util::stream::poll_fn(move |_| {
                        counter.fetch_add(1, Ordering::SeqCst);
                        std::task::Poll::Ready(Some(Err::<Bytes, _>(std::io::Error::other(
                            "body polled",
                        ))))
                    }))
                };
                let request = Request::post(PATH)
                    .extension(VerifiedClient::from_verified_tls13_http11(Identity::san(
                        peer,
                    )?))
                    .body(body)?;
                let r = app.clone().oneshot(request).await?;
                assert_eq!(r.status(), status);
                assert_eq!(polled.load(Ordering::SeqCst), 0);
                assert!(r.headers()["cache-control"].to_str()?.contains("no-store"));
            }
        }
        Ok(())
    }
}
