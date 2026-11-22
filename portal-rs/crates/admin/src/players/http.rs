//! SSR routes; always wrapped in the existing staff host/session/CSRF/MFA guard.
use super::*;
use crate::web::{self, PanelConfig, PortalIdentity, StaffSession};
use crate::{rbac::MutationAuthorization, staff_actions};
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
    configured_router_from(pool, &|key| std::env::var(key).ok())
}

/// [`configured_router`] reading its configuration through `lookup`.
pub(crate) fn configured_router_from(
    pool: PgPool,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Router, Box<dyn std::error::Error>> {
    let flags = Flags {
        players: lookup("OTERYN_ADMIN_PLAYERS").is_some_and(|v| v == "true"),
        characters: lookup("OTERYN_ADMIN_PLAYER_CHARACTERS").is_some_and(|v| v == "true"),
        coin_holds: oteryn_payments::coin::CoinSettings::from_lookup(lookup).enabled,
    };
    if !flags.players {
        return Ok(Router::new());
    }
    let characters_enabled = flags.characters;
    let mut players = Players::new(
        pool.clone(),
        flags,
        PortalIdentity::configured_from(pool.clone(), lookup),
        IdentityAccounts {
            accounts: oteryn_identity::accounts::Accounts { pool: pool.clone() },
            projection: Some(super::projection::configured(
                pool.clone(),
                characters_enabled,
                lookup,
            )),
        },
    );
    players.edits = edits::Settings::configured(pool.clone(), lookup);
    let routes = routes(players);
    // Every handler checks its own permission; the guard admits any staff member
    // so a kick-only moderator reaches the kick view.
    Ok(
        web::configured_guard_from(routes, pool, Permission::Access, lookup)?
            .layer(Extension(oteryn_web::csrf::LocalizedRefusal)),
    )
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
    web::guarded(routes, pool, panel, identity, staff, Permission::Access)
        .layer(Extension(oteryn_web::csrf::LocalizedRefusal))
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
            Self::Conflict => StatusCode::CONFLICT,
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
    hold: Option<Uuid>,
    ticket: Option<Uuid>,
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
    kick: String,
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
    issued_evidence: String,
    coin_hold_label: String,
    release_coin_hold: String,
    coin_debt_label: String,
    /// `Some` when the viewer may release coin purchase holds.
    coin_hold: Option<CoinHold>,
    /// `Some(id)` when the viewer may only kick that account.
    kick_target: Option<String>,
    status_edit: bool,
    email_edit: bool,
    flag_edit: bool,
    needs_review: bool,
    account_status: String,
    email_label: String,
    request_email: String,
    email_help: String,
    pending_email: bool,
    email_pending_label: String,
    review_flag: String,
    save_flag: String,
    save_status: String,
    active: String,
    suspended: String,
}
struct CoinHold {
    id: String,
    order: String,
    status: String,
    /// Coins still owed on held reversals; credits pay it first.
    debt: i64,
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
            kick: text(lang, "kick"),
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
            issued_evidence: String::new(),
            coin_hold_label: text(lang, "coin-hold"),
            release_coin_hold: text(lang, "coin-hold-release"),
            coin_debt_label: text(lang, "coin-debt"),
            coin_hold: None,
            kick_target: None,
            status_edit: false,
            email_edit: false,
            flag_edit: false,
            needs_review: false,
            account_status: String::new(),
            email_label: text(lang, "new-email"),
            request_email: text(lang, "request-email"),
            email_help: text(lang, "email-help"),
            pending_email: false,
            email_pending_label: text(lang, "email-pending"),
            review_flag: text(lang, "review-flag"),
            save_flag: text(lang, "save-flag"),
            save_status: text(lang, "save-status"),
            active: text(lang, "status-active"),
            suspended: text(lang, "status-suspended"),
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
    coin_hold: Option<String>,
    evidence: Option<Uuid>,
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
    let mut account = match s.account(context, id).await {
        Ok(account) => account,
        Err(Error::Authorization(crate::rbac::Error::Denied)) => {
            s.kick_view(context, id).await?;
            page.kick_target = Some(id.to_string());
            page.csrf = session.csrf_token().await?;
            return Ok(oteryn_web::render(&page));
        }
        Err(e) => return Err(e),
    };
    page.account_status = account.status.clone();
    page.flag_edit = can_edit(&s, context, edits::Field::Flag).await?;
    if page.flag_edit {
        page.needs_review = edits::needs_review(&mut *s.pool.acquire().await?, id).await?;
    }
    page.status_edit = s.edits.status
        && account.email_verified
        && matches!(account.status.as_str(), "active" | "suspended")
        && can_edit(&s, context, edits::Field::Status).await?;
    page.email_edit = s.edits.email_origin.is_some()
        && account.email_verified
        && account.status == "active"
        && can_edit(&s, context, edits::Field::Email).await?;
    if page.status_edit || page.email_edit {
        let protected = id == context.account
            || edits::target_state(&mut *s.pool.acquire().await?, id)
                .await?
                .is_none_or(|(_, _, protected)| protected);
        page.status_edit &= !protected;
        page.email_edit &= !protected;
    }
    if page.email_edit {
        page.pending_email = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM account_email_changes WHERE account_id=$1
             AND initiated_by_staff AND applied_at IS NULL AND cancelled_at IS NULL
             AND reverted_at IS NULL AND expires_at>clock_timestamp())",
        )
        .bind(id)
        .fetch_one(&s.pool)
        .await?;
    }
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
        Err(Error::Unavailable | Error::NotFound | Error::Denied) => {
            page.characters_unavailable = true;
        }
        Err(e) => return Err(e),
    }
    match s.coin_hold(context, id).await {
        Ok(hold) => {
            let status = match query.coin_hold.as_deref() {
                Some("released") => text(page.lang, "coin-hold-released"),
                Some("absent") => text(page.lang, "coin-hold-absent"),
                _ if hold.is_none() => text(page.lang, "coin-hold-none"),
                _ => String::new(),
            };
            let debt = s.coin_debt(context, id).await?;
            page.coin_hold = Some(CoinHold {
                id: hold
                    .as_ref()
                    .map(|h| h.hold_id.to_string())
                    .unwrap_or_default(),
                order: hold.map(|h| h.order_id.to_string()).unwrap_or_default(),
                status,
                debt,
            });
        }
        Err(Error::NotFound | Error::Authorization(crate::rbac::Error::Denied)) => (),
        Err(e) => return Err(e),
    }
    page.csrf = session.csrf_token().await?;
    page.issued_evidence = query.evidence.map(|v| v.to_string()).unwrap_or_default();
    if let Some(request) = query.request {
        page.result = request.to_string();
    }
    Ok(oteryn_web::render(&page))
}
async fn can_edit<A: AccountStateReader, I: Accounts>(
    players: &Players<A, I>,
    actor: CheckContext,
    field: edits::Field,
) -> Result<bool, Error> {
    match players.check(actor, field.permission()).await {
        Ok(()) => Ok(true),
        Err(Error::Denied | Error::Authorization(crate::rbac::Error::Denied)) => Ok(false),
        Err(error) => Err(error),
    }
}

async fn change<A: AccountStateReader, I: Accounts>(
    State(s): State<Arc<Players<A, I>>>,
    Extension(context): Extension<CheckContext>,
    Extension(authority): Extension<MutationAuthorization>,
    csrf: Option<Extension<oteryn_web::csrf::Verified>>,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Form(form): Form<Action>,
) -> Result<Response, Error> {
    let mut location = format!("/admin/players/{id}");
    if let Some(field) = edits::Field::parse(&form.action) {
        let view_authority = authority.clone();
        let result = staff_actions::run(
            &s.pool,
            staff_actions::Guard {
                actor: context,
                authority: &authority,
                csrf: csrf.map(|Extension(proof)| proof),
            },
            staff_actions::Action {
                permission: field.permission(),
                action: field.action(),
                reason: &form.reason,
            },
            || {
                if id.is_nil() {
                    return Err(staff_actions::Error::Invalid);
                }
                Ok((vec![id], field.validate(&form.body, &s.edits)?))
            },
            move |db, edit| {
                Box::pin(async move {
                    use crate::rbac::MutationCheck;
                    view_authority
                        .check(db, context, Permission::ManageSupportTickets)
                        .await?;
                    edit.apply(db, context.account, id).await
                })
            },
        )
        .await;
        let response = match result {
            Ok(()) => Redirect::to(&location).into_response(),
            Err(error) => {
                let fallback = oteryn_ui::Registry::default().chrome(
                    language(&headers),
                    &location,
                    oteryn_ui::Account::Anonymous,
                );
                error.response(chrome.as_ref().map_or(&fallback, |Extension(value)| value))
            }
        };
        return Ok(response);
    }
    match form.action.as_str() {
        "logout" => s.logout_all(context, id, &form.reason).await?,
        "kick" => {
            s.kick(context, id, &form.reason).await?;
        }
        "release-coin-hold" => {
            let released = s
                .release_coin_hold(context, id, form.hold.ok_or(Error::BadInput)?, &form.reason)
                .await?;
            location.push_str(if released.is_some() {
                "?coin_hold=released"
            } else {
                "?coin_hold=absent"
            });
        }
        "verify-identity" => {
            let evidence = s
                .attest_reset_evidence_for(context, form.ticket.ok_or(Error::BadInput)?, id)
                .await?;
            location.push_str(&format!("?evidence={evidence}"));
        }
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
