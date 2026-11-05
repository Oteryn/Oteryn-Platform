//! SSR routes; always wrapped in the existing staff host/session/CSRF/MFA guard.
use super::*;
use crate::web::{self, PanelConfig, PendingAuthentication, StaffSession};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use std::sync::Arc;

pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let flags = Flags {
        players: std::env::var("OTERYN_ADMIN_PLAYERS").is_ok_and(|v| v == "true"),
        characters: std::env::var("OTERYN_ADMIN_PLAYER_CHARACTERS").is_ok_and(|v| v == "true"),
    };
    let characters_enabled = flags.characters;
    let routes = routes(Players::new(
        pool.clone(),
        flags,
        PendingAuthentication,
        IdentityAccounts {
            accounts: oteryn_identity::accounts::Accounts { pool: pool.clone() },
            projection: Some(super::projection::configured(
                pool.clone(),
                characters_enabled,
            )),
        },
    ));
    web::configured_guard(routes, pool, Permission::ManageSupportTickets)
}
pub fn router<A, I, S>(
    pool: PgPool,
    flags: Flags,
    panel: PanelConfig,
    identity: S,
    staff: A,
    accounts: I,
) -> Router
where
    A: AccountStateReader + Clone + 'static,
    I: Accounts,
    S: StaffSession,
{
    let routes = routes(Players::new(pool.clone(), flags, staff.clone(), accounts));
    web::guarded(
        routes,
        pool,
        panel,
        identity,
        staff,
        Permission::ManageSupportTickets,
    )
}
fn routes<A: AccountStateReader + 'static, I: Accounts>(players: Players<A, I>) -> Router {
    Router::new()
        .route("/admin/players", get(search::<A, I>))
        .route(
            "/admin/players/{id}",
            get(detail::<A, I>).post(change::<A, I>),
        )
        .with_state(Arc::new(players))
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::BadInput => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Denied | Self::Authorization(crate::rbac::Error::Denied) => StatusCode::FORBIDDEN,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        }
        .into_response()
    }
}
#[derive(Deserialize)]
struct Search {
    #[serde(default)]
    q: String,
}
#[derive(Deserialize)]
struct Action {
    action: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    reason: String,
    evidence: Option<Uuid>,
    request: Option<Uuid>,
}
#[derive(Template)]
#[template(path = "page.html", config = "src/players/askama.toml")]
struct Page {
    lang: &'static str,
    title: String,
    search: String,
    query_label: String,
    query: String,
    accounts: Vec<Account>,
    account: Option<Account>,
    sessions: Vec<Session>,
    characters: Vec<Character>,
    notes: Vec<String>,
    sessions_label: String,
    characters_label: String,
    notes_label: String,
    unavailable: String,
    sessions_unavailable: bool,
    characters_unavailable: bool,
    csrf: String,
    add_note: String,
    logout: String,
    reason: String,
    reset: String,
    approve: String,
    evidence: String,
    request: String,
    status: String,
    locale: String,
    verified: String,
    yes: String,
    no: String,
    result: String,
}
fn language(headers: &HeaderMap) -> &'static str {
    if headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("en"))
    {
        "en"
    } else {
        "pl"
    }
}
fn text(lang: &str, key: &str) -> String {
    let Ok(locale) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
    let source = if lang == "en" {
        include_str!("en.ftl")
    } else {
        include_str!("pl.ftl")
    };
    let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) else {
        return String::new();
    };
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
impl Page {
    fn new(lang: &'static str) -> Self {
        Self {
            lang,
            title: text(lang, "title"),
            search: text(lang, "search"),
            query_label: text(lang, "query"),
            query: String::new(),
            accounts: vec![],
            account: None,
            sessions: vec![],
            characters: vec![],
            notes: vec![],
            sessions_label: text(lang, "sessions"),
            characters_label: text(lang, "characters"),
            notes_label: text(lang, "notes"),
            unavailable: text(lang, "unavailable"),
            sessions_unavailable: false,
            characters_unavailable: false,
            csrf: String::new(),
            add_note: text(lang, "add-note"),
            logout: text(lang, "logout"),
            reason: text(lang, "reason"),
            reset: text(lang, "reset"),
            approve: text(lang, "approve"),
            evidence: text(lang, "evidence"),
            request: text(lang, "request"),
            status: text(lang, "status"),
            locale: text(lang, "locale"),
            verified: text(lang, "verified"),
            yes: text(lang, "yes"),
            no: text(lang, "no"),
            result: String::new(),
        }
    }
}
async fn search<A: AccountStateReader, I: Accounts>(
    State(s): State<Arc<Players<A, I>>>,
    Extension(context): Extension<CheckContext>,
    headers: HeaderMap,
    Query(query): Query<Search>,
) -> Result<Response, Error> {
    s.check(context, Permission::ManageSupportTickets).await?;
    let mut page = Page::new(language(&headers));
    if !query.q.is_empty() {
        page.accounts = s.search(context, &query.q).await?;
    }
    page.query = query.q;
    Ok(oteryn_web::render(&page))
}
#[derive(Deserialize)]
struct ResultQuery {
    request: Option<Uuid>,
}
async fn detail<A: AccountStateReader, I: Accounts>(
    State(s): State<Arc<Players<A, I>>>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Query(query): Query<ResultQuery>,
) -> Result<Response, Error> {
    let mut page = Page::new(language(&headers));
    let mut account = s.account(context, id).await?;
    account.status = text(page.lang, &format!("status-{}", account.status));
    page.account = Some(account);
    page.notes = s.notes(context, id).await?;
    match s.sessions(context, id).await {
        Ok(rows) => page.sessions = rows,
        Err(Error::Unavailable) => page.sessions_unavailable = true,
        Err(e) => return Err(e),
    }
    match s.characters(context, id).await {
        Ok(rows) => page.characters = rows,
        Err(Error::Unavailable | Error::NotFound) => page.characters_unavailable = true,
        Err(e) => return Err(e),
    }
    page.csrf = session.csrf_token().await?;
    if let Some(request) = query.request {
        page.result = request.to_string();
    }
    Ok(oteryn_web::render(&page))
}
async fn change<A: AccountStateReader, I: Accounts>(
    State(s): State<Arc<Players<A, I>>>,
    Extension(context): Extension<CheckContext>,
    Path(id): Path<Uuid>,
    Form(form): Form<Action>,
) -> Result<Response, Error> {
    let mut location = format!("/admin/players/{id}");
    match form.action.as_str() {
        "note" => s.note(context, id, &form.body).await?,
        "logout" => s.logout_all(context, id, &form.reason).await?,
        "request-reset" => {
            let request = s
                .request_reset(
                    context,
                    id,
                    form.evidence.ok_or(Error::BadInput)?,
                    &form.reason,
                )
                .await?;
            location.push_str(&format!("?request={request}"));
        }
        "approve-reset" => {
            s.approve_reset(context, id, form.request.ok_or(Error::BadInput)?)
                .await?
        }
        _ => return Err(Error::BadInput),
    }
    Ok(Redirect::to(&location).into_response())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn localized_escaped_responsive_page() -> Result<(), Box<dyn std::error::Error>> {
        for lang in ["pl", "en"] {
            let mut page = Page::new(lang);
            page.account = Some(Account {
                id: Uuid::now_v7(),
                status: "active".into(),
                locale: lang.into(),
                email_verified: true,
            });
            page.notes = vec!["<script>alert(1)</script>".into()];
            let html = page.render()?;
            if let Ok(directory) = std::env::var("PORTAL_TEST_RENDER_DIR") {
                std::fs::write(
                    std::path::Path::new(&directory).join(format!("players-{lang}.html")),
                    &html,
                )?;
            }
            assert!(html.contains("width=device-width"));
            assert!(html.contains("&#60;script&#62;") || html.contains("&lt;script&gt;"));
            assert!(!html.contains("<script>"));
            for key in [
                "title",
                "query",
                "sessions",
                "characters",
                "notes",
                "reset",
                "approve",
                "unavailable",
            ] {
                assert!(!text(lang, key).is_empty());
            }
            assert!(html.contains("name=\"_csrf\""));
            assert!(html.contains("for=\"note\""));
        }
        Ok(())
    }
}
