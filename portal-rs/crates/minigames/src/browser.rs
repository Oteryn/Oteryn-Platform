//! Bounds apply before account/domain/adapter locks on every browser transaction.
use sqlx::{PgPool, Postgres, Transaction};
pub(crate) async fn begin(pool: &PgPool) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
    let mut tx = tokio::time::timeout(std::time::Duration::from_millis(250), pool.begin())
        .await
        .map_err(|_| sqlx::Error::PoolTimedOut)??;
    sqlx::raw_sql("SET LOCAL lock_timeout = '500ms'; SET LOCAL statement_timeout = '2s'")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}
pub(crate) async fn deadline(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let purchase = crate::pass::web::purchase_language(&req);
    match tokio::time::timeout(std::time::Duration::from_secs(3), next.run(req)).await {
        Ok(response) => response,
        Err(_) => match purchase {
            Some(lang) => crate::pass::web::uncertain_purchase(
                lang,
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
            ),
            None => (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                [("cache-control", "no-store")],
            )
                .into_response(),
        },
    }
}

pub(crate) async fn operation_deadline(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut segments = req.uri().path().split('/').skip(1);
    let bounded = segments.next().is_some_and(|lang| !lang.is_empty())
        && matches!(
            segments.next(),
            Some("puzzles" | "collection" | "collection-pvp" | "pass")
        );
    if bounded {
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
                |req: axum::extract::Request, next: axum::middleware::Next| async move {
                    // A stalled shared session/presentation dependency, outside the
                    // module router's own deadline, must still be cancelled.
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    next.run(req).await
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
