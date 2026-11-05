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

async fn dispatch(State(targets): State<Arc<Targets>>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let Ok(bytes): Result<Bytes, _> = axum::body::to_bytes(body, BUFFER_LIMIT).await else {
        return StatusCode::BAD_REQUEST.into_response();
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
        Router::new().route(PATH, post(move || async move { tag }))
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
