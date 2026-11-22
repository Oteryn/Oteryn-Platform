use super::{Entry, Error, player_entries, submit_player};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State, rejection::FormRejection},
    http::{StatusCode, header},
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::{
    Language,
    ratelimit::{ClientSource, Keys, NamedPolicy, RateLimiter},
    session::{BrowserSession, SessionLayer},
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

pub fn router(pool: PgPool, origin: &str) -> Router {
    let mut session = SessionLayer::new(pool.clone());
    session.csrf.trusted_origins = vec![origin.to_owned()];
    Router::new()
        .route("/{lang}/support/appeals", get(index))
        .route("/{lang}/support/appeals/{id}", get(detail).post(submit))
        .route_layer(middleware::from_fn_with_state(
            session,
            oteryn_web::session::middleware,
        ))
        .layer(middleware::map_response(
            |mut response: Response| async move {
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("no-store"),
                );
                response
            },
        ))
        .with_state(pool)
}

#[derive(Template)]
#[template(path = "appeals/page.html")]
struct Page<'a> {
    lang: &'a str,
    rows: Vec<Entry>,
    detail: bool,
    csrf: String,
    next: Option<u32>,
}

impl Page<'_> {
    fn label(&self, key: &str) -> &'static str {
        let (pl, en) = match key {
            "none" => ("Brak odwołania", "No appeal"),
            "requested" => ("Oczekuje na rozpatrzenie", "Awaiting review"),
            "reviewing" => ("W trakcie rozpatrywania", "Under review"),
            "accepted" => ("Uwzględnione", "Overturned"),
            "rejected" => ("Odrzucone", "Upheld"),
            "empty" => (
                "Brak sankcji do wyświetlenia.",
                "There are no sanctions to display.",
            ),
            "message" => ("Uzasadnienie odwołania", "Reason for appeal"),
            "outcome" => ("Uzasadnienie decyzji", "Decision reason"),
            "submit" => ("Złóż odwołanie", "Submit appeal"),
            "next" => ("Następna strona", "Next page"),
            _ => ("Odwołania od sankcji", "Sanction appeals"),
        };
        if self.lang == "pl" { pl } else { en }
    }

    fn respond(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    }
}

async fn actor(pool: &PgPool, session: &BrowserSession, lang: &str) -> Result<Uuid, StatusCode> {
    if !matches!(lang, "en" | "pl") {
        return Err(StatusCode::NOT_FOUND);
    }
    let id = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    let account: Option<String> = sqlx::query_scalar("SELECT status FROM accounts WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if !matches!(account.as_deref(), Some("active" | "suspended")) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(id)
}

fn status(error: Error) -> StatusCode {
    StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::SERVICE_UNAVAILABLE)
}

#[derive(Deserialize)]
struct Paging {
    #[serde(default)]
    page: u32,
}

async fn index(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    Query(paging): Query<Paging>,
) -> Result<Response, StatusCode> {
    let actor = actor(&pool, &session, &lang).await?;
    let mut rows = player_entries(&pool, actor, None, i64::from(paging.page) * 20)
        .await
        .map_err(status)?;
    let next = if rows.len() > 20 {
        rows.truncate(20);
        paging.page.checked_add(1)
    } else {
        None
    };
    Ok(Page {
        lang: &lang,
        rows,
        detail: false,
        csrf: String::new(),
        next,
    }
    .respond())
}

async fn detail(
    State(pool): State<PgPool>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    let actor = actor(&pool, &session, &lang).await?;
    let rows = player_entries(&pool, actor, Some(id), 0)
        .await
        .map_err(status)?;
    let row = rows.first().ok_or(StatusCode::NOT_FOUND)?;
    let csrf = if row.can_submit {
        session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    } else {
        String::new()
    };
    Ok(Page {
        lang: &lang,
        rows,
        detail: true,
        csrf,
        next: None,
    }
    .respond())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    version: i64,
    message: String,
    #[serde(rename = "_csrf")]
    _csrf: String,
}

async fn submit(
    State(pool): State<PgPool>,
    Path((lang, id)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    peer: Option<ClientSource>,
    form: Result<Form<Input>, FormRejection>,
) -> Result<Redirect, Response> {
    let language = match lang.as_str() {
        "pl" => Language::Pl,
        "en" => Language::En,
        _ => return Err(StatusCode::NOT_FOUND.into_response()),
    };
    let path = format!("/{lang}/support/appeals/{id}");
    let error = |status| {
        let chrome = oteryn_ui::Registry::default().chrome(
            language.code(),
            &path,
            oteryn_ui::Account::Anonymous,
        );
        oteryn_ui::error(&chrome, status)
    };
    let Form(input) = form.map_err(|rejection| error(rejection.status()))?;
    let actor = actor(&pool, &session, &lang).await.map_err(error)?;
    let session_hash = session
        .identity_hash()
        .await
        .ok_or_else(|| error(StatusCode::FORBIDDEN))?;
    let peer = peer.ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE))?;
    let mut keys = Keys::source(peer.0);
    keys.account = Some(actor.to_string());
    RateLimiter::new(pool.clone())
        .check(NamedPolicy::SupportEnforcementAppeal, &[keys])
        .await
        .map_err(|denial| denial.response(language, false))?;
    submit_player(
        &pool,
        actor,
        &session_hash,
        id,
        input.version,
        &input.message,
    )
    .await
    .map_err(|failure| error(status(failure)))?;
    Ok(Redirect::to(&path))
}
