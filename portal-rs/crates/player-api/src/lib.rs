//! Private native bootstrap routes for the game client.
use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use oteryn_characters::lcfa::{AccountId, AccountStateLookup, Availability, Projection, ReadState};
use oteryn_gameauth::tickets::{Error, OAuthAuthority, Settings, Tickets};
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone)]
pub struct Api {
    pub tickets: Tickets,
    pub projection: Option<Projection>,
}
/// Production composition. `PORTAL_GAME_TICKETS_ENABLED=true` with `OTERYN_ENV`
/// `testing` or `preprod` enables both routes (contract §17); otherwise they
/// deny. The caller supplies the OAuth authority (suspension-guarded in the
/// binary) and the LCFA account lookup; the projection keeps its own flags.
pub fn configured_router(
    pool: sqlx::PgPool,
    oauth: Arc<dyn OAuthAuthority>,
    accounts: Arc<dyn AccountStateLookup>,
) -> Router {
    router(Api {
        tickets: Tickets::new(pool.clone(), settings(|key| std::env::var(key).ok()), oauth),
        projection: Some(Projection::new(
            pool,
            oteryn_characters::overview::configured_projection(),
            accounts,
        )),
    })
}
fn settings(lookup: impl Fn(&str) -> Option<String>) -> Settings {
    Settings {
        enabled: lookup("PORTAL_GAME_TICKETS_ENABLED").as_deref() == Some("true"),
        nonproduction: matches!(lookup("OTERYN_ENV").as_deref(), Some("testing" | "preprod")),
    }
}
pub fn router(api: Api) -> Router {
    Router::new()
        .route("/api/v1/game-auth/tickets", post(issue))
        .route("/api/v1/game-auth/native-characters", get(characters))
        .layer(DefaultBodyLimit::max(2048))
        .layer(axum::middleware::from_fn(no_store))
        .with_state(api)
}
async fn no_store(request: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store, private"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
    response
}
fn bearer(headers: &HeaderMap) -> Result<&str, Error> {
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let value = values
        .next()
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::Unauthorized)?;
    if values.next().is_some() {
        return Err(Error::Unauthorized);
    }
    let token = value.strip_prefix("Bearer ").ok_or(Error::Unauthorized)?;
    if token.is_empty() || token.len() > 8192 || !token.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(Error::Unauthorized);
    }
    Ok(token)
}
fn failure(error: Error, issuance: bool) -> Response {
    match error {
        Error::Unauthorized => (StatusCode::UNAUTHORIZED, axum::Json(serde_json::json!({"error": if issuance { "game_login_unavailable" } else { "unauthenticated" }}))).into_response(),
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        Error::RateLimited => (StatusCode::TOO_MANY_REQUESTS, [(header::RETRY_AFTER, "30")]).into_response(),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TicketRequest {
    protocol_version: u8,
}
async fn issue(State(api): State<Api>, headers: HeaderMap, body: Bytes) -> Response {
    let token = match bearer(&headers) {
        Ok(token) => token,
        Err(error) => return failure(error, false),
    };
    if !api.tickets.available() {
        return failure(Error::Unauthorized, true);
    }
    if let Err(error) = api.tickets.account(token).await {
        return failure(error, false);
    }
    let request = serde_json::from_slice::<TicketRequest>(&body);
    if !matches!(
        request,
        Ok(TicketRequest {
            protocol_version: 1
        })
    ) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            axum::Json(serde_json::json!({"error":"invalid_request"})),
        )
            .into_response();
    }
    if let Err(error) = api.tickets.budget(token, true).await {
        return failure(error, true);
    }
    match api.tickets.issue(token).await {
        Ok(issued) => axum::Json(issued).into_response(),
        Err(error) => failure(error, true),
    }
}
#[derive(Serialize)]
struct Character {
    character_id: String,
    world_id: String,
    name: String,
    availability: &'static str,
}
#[derive(Serialize)]
struct CharacterList {
    protocol_version: u8,
    characters: Vec<Character>,
}
async fn characters(State(api): State<Api>, headers: HeaderMap) -> Response {
    let token = match bearer(&headers) {
        Ok(token) => token,
        Err(error) => return failure(error, false),
    };
    if let Err(error) = api.tickets.budget(token, false).await {
        return failure(error, false);
    }
    let principal = match api.tickets.account(token).await {
        Ok(principal) => principal,
        Err(error) => return failure(error, false),
    };
    let Some(projection) = api.projection else {
        return failure(Error::Unavailable, false);
    };
    let Ok(account) = AccountId::parse(&principal.account_id.to_string()) else {
        return failure(Error::Unauthorized, false);
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|time| i64::try_from(time.as_secs()).ok());
    let Some(now) = now else {
        return failure(Error::Unavailable, false);
    };
    let view = match projection.list_characters(account, now).await {
        Ok(view) => view,
        Err(_) => return failure(Error::Unavailable, false),
    };
    if view.characters.len() > 64 || !matches!(view.state, ReadState::Ready | ReadState::Missing) {
        return failure(Error::Unavailable, false);
    }
    // Recheck authorization after waiting on projection locks; reads never consume access.
    match api.tickets.account(token).await {
        Ok(current) if current == principal => {}
        Ok(_) => return failure(Error::Unauthorized, false),
        Err(error) => return failure(error, false),
    }
    let payload = CharacterList {
        protocol_version: 2,
        characters: view
            .characters
            .into_iter()
            .map(|item| Character {
                character_id: item.character_id.to_string(),
                world_id: item.world_id.to_string(),
                name: item.name,
                availability: match item.availability {
                    Availability::Available => "AVAILABLE",
                    Availability::Unavailable => "UNAVAILABLE",
                },
            })
            .collect(),
    };
    match serde_json::to_vec(&payload) {
        Ok(body) if body.len() <= 16384 => {
            ([(header::CONTENT_TYPE, "application/json")], body).into_response()
        }
        _ => failure(Error::Unavailable, false),
    }
}

pub mod account_feed;
#[cfg(test)]
mod client_fixtures_tests;
#[cfg(test)]
mod tests;
