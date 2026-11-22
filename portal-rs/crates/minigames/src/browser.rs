//! Bounds apply before account/domain/adapter locks on every browser transaction.
use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sqlx::{PgPool, Postgres, Transaction};
use tokio::{sync::Semaphore, time::Instant};

// Reserve capacity for each landing page and leave half the portal pool
// available to other modules. Match League's own concurrency limit.
pub(crate) const LEAGUE_READ_SLOTS: usize = 4;

pub(crate) struct LandingSlots {
    collection: Arc<Semaphore>,
    puzzles: Arc<Semaphore>,
    league: Arc<Semaphore>,
}

impl LandingSlots {
    pub(crate) fn new() -> Self {
        Self {
            collection: Arc::new(Semaphore::new(2)),
            puzzles: Arc::new(Semaphore::new(2)),
            league: Arc::new(Semaphore::new(LEAGUE_READ_SLOTS)),
        }
    }

    fn for_module(&self, module: Option<&str>) -> Option<&Arc<Semaphore>> {
        match module {
            Some("collection") => Some(&self.collection),
            Some("puzzles") => Some(&self.puzzles),
            Some("league") => Some(&self.league),
            _ => None,
        }
    }
}

const OPERATION_TIMEOUT: Duration = Duration::from_secs(3);

pub(crate) fn landing_deadline() -> Instant {
    Instant::now() + OPERATION_TIMEOUT
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        [("cache-control", "no-store")],
    )
        .into_response()
}

pub(crate) async fn begin(pool: &PgPool) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
    let mut tx = tokio::time::timeout(std::time::Duration::from_millis(250), pool.begin())
        .await
        .map_err(|_| sqlx::Error::PoolTimedOut)??;
    sqlx::raw_sql("SET LOCAL lock_timeout = '500ms'; SET LOCAL statement_timeout = '2s'")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}
pub(crate) async fn deadline(req: Request, next: Next) -> Response {
    // The shared landing layer owns the deadline and waits for SQL cleanup.
    if req
        .extensions()
        .get::<oteryn_web::BoundedBrowserRead>()
        .is_some()
    {
        return next.run(req).await;
    }
    tokio::time::timeout(OPERATION_TIMEOUT, next.run(req))
        .await
        .unwrap_or_else(|_| unavailable())
}

pub(crate) async fn operation_deadline(
    State(read_slots): State<Arc<LandingSlots>>,
    mut req: Request,
    next: Next,
) -> Response {
    let mut segments = req.uri().path().split('/').skip(1);
    let localized = segments.next().is_some_and(|lang| !lang.is_empty());
    let module = segments.next();
    if localized
        && matches!(*req.method(), Method::GET | Method::HEAD)
        && segments.next().is_none()
        && let Some(slots) = read_slots.for_module(module)
    {
        let deadline = landing_deadline();
        let Ok(Ok(slot)) = tokio::time::timeout_at(deadline, slots.clone().acquire_owned()).await
        else {
            return unavailable();
        };
        req.extensions_mut()
            .insert(oteryn_web::BoundedBrowserRead(deadline));
        // The task owns admission until its bounded SQL has returned or closed
        // its connection, even if the request times out or the client leaves.
        let work = tokio::spawn(async move {
            let _slot = slot;
            if oteryn_db::check_deadline(deadline).is_err() {
                return unavailable();
            }
            next.run(req).await
        });
        return match tokio::time::timeout_at(deadline, work).await {
            Ok(Ok(response)) => response,
            _ => unavailable(),
        };
    }
    if localized
        && matches!(
            module,
            Some("puzzles" | "collection" | "collection-pvp" | "pass")
        )
    {
        deadline(req, next).await
    } else {
        next.run(req).await
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use tower::ServiceExt;
    #[tokio::test]
    async fn outer_deadline_includes_shared_browser_middleware()
    -> Result<(), Box<dyn std::error::Error>> {
        let app = Router::new()
            .route("/{lang}/puzzles", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(
                |_: axum::extract::Request, _: axum::middleware::Next| async {
                    // A stalled shared session/presentation dependency, outside the
                    // module router's own deadline, must still return a refusal.
                    std::future::pending::<axum::response::Response>().await
                },
            ));
        let app = crate::with_browser_deadlines(app);
        let started = std::time::Instant::now();
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(4),
            app.oneshot(Request::get("/invalid/puzzles").body(Body::empty())?),
        )
        .await??;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert!(started.elapsed() < std::time::Duration::from_secs(4));
        Ok(())
    }
}
