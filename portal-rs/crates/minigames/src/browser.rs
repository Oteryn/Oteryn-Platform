//! Bounds apply before account/domain/adapter locks on every browser transaction.
use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sqlx::{PgPool, Postgres, Transaction};
use tokio::sync::Semaphore;

// Leave half of the 16-connection portal pool available to other modules.
pub(crate) const CRAWL_READ_SLOTS: usize = 8;
const OPERATION_TIMEOUT: Duration = Duration::from_secs(3);

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
    tokio::time::timeout(OPERATION_TIMEOUT, next.run(req))
        .await
        .unwrap_or_else(|_| unavailable())
}

pub(crate) async fn operation_deadline(
    State(read_slots): State<Arc<Semaphore>>,
    req: Request,
    next: Next,
) -> Response {
    let mut segments = req.uri().path().split('/').skip(1);
    let localized = segments.next().is_some_and(|lang| !lang.is_empty());
    let module = segments.next();
    let read = matches!(*req.method(), Method::GET | Method::HEAD)
        && matches!(module, Some("puzzles" | "collection" | "league"))
        && segments.next().is_none();
    if localized && read {
        // Queue landing pages before session refresh and rate-limit writes can
        // occupy pooled connections. Charge the wait to the existing deadline.
        return tokio::time::timeout(OPERATION_TIMEOUT, async {
            let Ok(_slot) = read_slots.acquire().await else {
                return unavailable();
            };
            next.run(req).await
        })
        .await
        .unwrap_or_else(|_| unavailable());
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
                    // module router's own deadline, must still be cancelled.
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
