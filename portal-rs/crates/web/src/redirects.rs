//! Fixed legacy migration routes; unknown paths remain 404.
use axum::{
    Router,
    extract::OriginalUri,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
/// Old public GET routes from routes/web.php, plus migration aliases from plan §9.
/// Authenticated account/admin routes are deliberately excluded.
pub const MAPPINGS: &[(&str, &str)] = &[
    ("/support", "/support"),
    ("/support/report-a-bug", "/support/report-a-bug"),
    ("/legal/terms", "/legal/terms"),
    ("/legal/privacy", "/legal/privacy"),
    ("/legal/cookies", "/legal/cookies"),
    ("/today", "/today"),
    ("/server-information", "/server-information"),
    ("/rules", "/rules"),
    ("/guilds", "/guilds"),
    ("/deaths", "/deaths"),
    ("/events", "/events"),
    ("/events/{slug}", "/events/{slug}"),
    ("/download", "/download"),
    ("/download/{slug}", "/download/{slug}"),
    ("/wiki", "/wiki"),
    ("/wiki/search", "/wiki/search"),
    ("/wiki/items", "/wiki/items"),
    ("/wiki/creatures", "/wiki/creatures"),
    ("/register", "/register"),
    ("/login", "/login"),
    ("/mfa/challenge", "/mfa/challenge"),
    ("/forgot-password", "/forgot-password"),
    ("/reset-password/{token}", "/forgot-password"),
    ("/recovery-key", "/recovery-key"),
    ("/email-change/confirm/{token}", "/account/security"),
    ("/email-change/recover/{token}", "/account/security"),
    ("/news", "/news"),
    ("/news/{slug}", "/news/{slug}"),
    ("/pages/{slug}", "/pages/{slug}"),
    ("/highscores", "/rankings/level"),
    ("/characters", "/characters"),
    ("/characters/{name}", "/characters/{name}"),
    ("/guilds/{name}", "/guilds/{name}"),
    ("/online", "/online"),
    ("/servers", "/worlds"),
    ("/getting-started", "/start"),
    ("/wiki/{slug}", "/wiki/articles/{slug}"),
    ("/wiki/items/{slug}", "/wiki/items/{slug}"),
    ("/wiki/creatures/{slug}", "/wiki/creatures/{slug}"),
];
async fn redirect(OriginalUri(uri): OriginalUri) -> Response {
    for (old, new) in MAPPINGS {
        let mut target = new.to_string();
        let matched = if let Some((prefix, parameter)) = old.split_once('{') {
            let value = uri.path().strip_prefix(prefix).unwrap_or_default();
            if value.is_empty()
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
            {
                false
            } else {
                target = target.replace(&format!("{{{parameter}"), value);
                true
            }
        } else {
            uri.path() == *old
        };
        if matched {
            let mut location = format!("/pl{target}");
            // Preserve only non-sensitive pagination/search parameters, never redirect controls.
            if let Some(query) = uri.query() {
                let query: Vec<_> = query
                    .split('&')
                    .filter(|pair| {
                        pair.split_once('=').is_some_and(|(key, _)| {
                            matches!(key, "page" | "q" | "world" | "vocation")
                        })
                    })
                    .collect();
                if !query.is_empty() {
                    location.push('?');
                    location.push_str(&query.join("&"));
                }
            }
            return (
                StatusCode::MOVED_PERMANENTLY,
                [(header::LOCATION, location)],
            )
                .into_response();
        }
    }
    StatusCode::NOT_FOUND.into_response()
}
pub fn router() -> Router {
    MAPPINGS.iter().fold(Router::new(), |router, (old, _)| {
        router.route(old, get(redirect))
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    #[tokio::test]
    async fn all_mappings_and_refusals() -> Result<(), Box<dyn std::error::Error>> {
        for (old, new) in MAPPINGS {
            let old = old
                .replace("{slug}", "sample")
                .replace("{name}", "sample")
                .replace("{token}", "sample");
            let new = new.replace("{slug}", "sample").replace("{name}", "sample");
            let response = router()
                .oneshot(Request::get(old).body(Body::empty())?)
                .await?;
            assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
            assert_eq!(response.headers()[header::LOCATION], format!("/pl{new}"));
        }
        for uri in ["/unknown", "/news/%2f%2fevil.example"] {
            assert_eq!(
                router()
                    .oneshot(Request::get(uri).body(Body::empty())?)
                    .await?
                    .status(),
                StatusCode::NOT_FOUND
            );
        }
        let response = router()
            .oneshot(
                Request::get("/highscores?target=https://evil.example&page=2")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(
            response.headers()[header::LOCATION],
            "/pl/rankings/level?page=2"
        );
        Ok(())
    }
}
