//! SSR wiki routes and trusted staff-session integration.
use crate::{
    domain::{self, Action, Error, Translation},
    repository::{Guide, PublicRepository, Repository, Revision},
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, OriginalUri, Path, Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_admin::{
    rbac::{AccountStateReader, CheckContext, Permission},
    web::{PanelConfig, PortalIdentity, StaffSession},
};
use oteryn_web::ratelimit::{RateLimit, RateLimiter, WikiSearch};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
struct Staff<A> {
    repository: Repository<A>,
}
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let routes = staff_routes(pool.clone(), PortalIdentity::configured(pool.clone()));
    let staff = oteryn_admin::web::configured_guard(routes, pool.clone(), Permission::WikiAccess)?;
    Ok(public_router(pool).merge(staff))
}
pub fn public_router(pool: PgPool) -> Router {
    Router::new()
        .route("/wiki", get(index))
        .route("/wiki/search", get(index))
        .route("/wiki/articles/{slug}", get(article))
        .route("/{locale}/wiki", get(index))
        .route("/{locale}/wiki/search", get(index))
        .route("/{locale}/wiki/articles/{slug}", get(article))
        .layer(Extension(RateLimiter::new(pool.clone())))
        .layer(middleware::from_fn(response_policy))
        .with_state(PublicRepository::new(pool))
}
/// Uses the shared staff guard with trusted identity adapters in integration tests.
pub fn staff_router<I: StaffSession, A: AccountStateReader + Clone + 'static>(
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
) -> Router {
    let routes = staff_routes(pool.clone(), accounts.clone());
    oteryn_admin::web::guarded(
        routes,
        pool,
        config,
        identity,
        accounts,
        Permission::WikiAccess,
    )
}
fn staff_routes<A: AccountStateReader + 'static>(pool: PgPool, accounts: A) -> Router {
    let state = Arc::new(Staff {
        repository: Repository::new(pool, accounts),
    });
    Router::new()
        .route("/admin/wiki", get(list::<A>))
        .route("/admin/wiki/articles/create", get(create_form::<A>))
        .route("/admin/wiki/articles", get(list::<A>).post(create::<A>))
        .route("/admin/wiki/articles/{id}/edit", get(edit_form::<A>))
        .route("/admin/wiki/articles/{id}", axum::routing::post(edit::<A>))
        .route("/admin/wiki/articles/{id}/revisions", get(history::<A>))
        .route(
            "/admin/wiki/articles/{id}/preview/{locale}",
            get(preview::<A>),
        )
        .route(
            "/admin/wiki/articles/{id}/revisions/{source}/restore",
            axum::routing::post(restore::<A>),
        )
        .route(
            "/admin/wiki/articles/{id}/{action}",
            axum::routing::post(transition::<A>),
        )
        .layer(DefaultBodyLimit::max(900_000))
        .layer(middleware::from_fn(response_policy))
        .with_state(state)
}
fn language(headers: &HeaderMap) -> &'static str {
    if headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("pl"))
    {
        "pl"
    } else {
        "en"
    }
}
fn locale(uri: &axum::http::Uri, headers: &HeaderMap) -> Result<&'static str, Error> {
    match uri.path().split('/').nth(1) {
        Some("wiki") => Ok(language(headers)),
        Some("pl") => Ok("pl"),
        Some("en") => Ok("en"),
        _ => Err(Error::NotFound),
    }
}
fn text(lang: &str, key: &str) -> String {
    let Ok(id) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![id]);
    let source = if lang == "pl" {
        include_str!("i18n/pl.ftl")
    } else {
        include_str!("i18n/en.ftl")
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
fn page<T: Template>(template: T) -> Response {
    match template.render() {
        Ok(body) => Html(body).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
fn failure(error: Error, lang: &str) -> Response {
    let (status, key) = match error {
        Error::Invalid => (StatusCode::UNPROCESSABLE_ENTITY, "invalid"),
        Error::Conflict => (StatusCode::CONFLICT, "conflict"),
        Error::NotFound => (StatusCode::NOT_FOUND, "missing"),
        Error::Denied | Error::Authorization(oteryn_admin::rbac::Error::Denied) => {
            (StatusCode::FORBIDDEN, "denied")
        }
        _ => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
    };
    notice(status, lang, text(lang, key))
}
/// Reuse the localized document for domain refusals without changing their status.
pub(crate) fn notice(status: StatusCode, lang: &str, message: String) -> Response {
    (status, page(Message { lang, message })).into_response()
}
#[derive(Template)]
#[template(path = "wiki/message.html", config = "wiki_askama.toml")]
struct Message<'a> {
    lang: &'a str,
    message: String,
}
#[derive(Template)]
#[template(path = "wiki/index.html", config = "wiki_askama.toml")]
struct Index<'a> {
    lang: &'a str,
    query: String,
    rows: Vec<Translation>,
    next: i64,
}
impl Index<'_> {
    fn t(&self, key: &str) -> String {
        text(self.lang, key)
    }
}
#[derive(Debug, Deserialize, Default)]
struct Search {
    #[serde(default)]
    q: String,
    page: Option<i64>,
}
async fn index(
    _: RateLimit<WikiSearch>,
    State(repo): State<PublicRepository>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    query: Result<Query<Search>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let lang = match locale(&uri, &headers) {
        Ok(v) => v,
        Err(e) => return failure(e, "en"),
    };
    let Query(query) = match query {
        Ok(value) => value,
        Err(_) => return failure(Error::Invalid, lang),
    };
    let number = query.page.unwrap_or(1);
    match repo
        .search(lang, &query.q, number, !uri.path().ends_with("/search"))
        .await
    {
        Ok(rows) => page(Index {
            lang,
            query: query.q,
            rows,
            next: number + 1,
        }),
        Err(e) => failure(e, lang),
    }
}
#[derive(Template)]
#[template(path = "wiki/article.html", config = "wiki_askama.toml")]
struct Article<'a> {
    lang: &'a str,
    value: Translation,
    rendered: String,
}
async fn article(
    State(repo): State<PublicRepository>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Response {
    let lang = match locale(&uri, &headers) {
        Ok(v) => v,
        Err(e) => return failure(e, "en"),
    };
    let slug = uri.path().rsplit('/').next().unwrap_or_default();
    match repo.article(lang, slug).await {
        Ok(value) => {
            let rendered = domain::render(&value.markdown);
            page(Article {
                lang,
                value,
                rendered,
            })
        }
        Err(e) => failure(e, lang),
    }
}
#[derive(Template)]
#[template(path = "admin/wiki/list.html", config = "wiki_askama.toml")]
struct List<'a> {
    lang: &'a str,
    rows: Vec<Guide>,
}
impl List<'_> {
    fn t(&self, key: &str) -> String {
        text(self.lang, key)
    }
}
async fn list<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(context): Extension<CheckContext>,
    headers: HeaderMap,
) -> Response {
    let lang = language(&headers);
    match s.repository.list(context).await {
        Ok(rows) => page(List { lang, rows }),
        Err(e) => failure(e, lang),
    }
}
#[derive(Template)]
#[template(path = "admin/wiki/edit.html", config = "wiki_askama.toml")]
struct Edit<'a> {
    lang: &'a str,
    id: String,
    version: i64,
    status: String,
    values: Vec<Translation>,
    csrf: String,
    existing: bool,
}
impl Edit<'_> {
    fn t(&self, key: &str) -> String {
        text(self.lang, key)
    }
}
fn blank(locale: &str) -> Translation {
    Translation {
        locale: locale.into(),
        title: String::new(),
        slug: String::new(),
        summary: String::new(),
        markdown: String::new(),
    }
}
async fn create_form<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
) -> Response {
    let lang = language(&headers);
    if let Err(e) = s
        .repository
        .authorize(context, Permission::ManageWikiArticles)
        .await
    {
        return failure(e, lang);
    }
    match session.csrf_token().await {
        Ok(csrf) => page(Edit {
            lang,
            id: String::new(),
            version: 0,
            status: "draft".into(),
            values: vec![blank("pl"), blank("en")],
            csrf,
            existing: false,
        }),
        Err(e) => failure(e.into(), lang),
    }
}
async fn edit_form<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    let lang = language(&headers);
    match s.repository.get(context, id).await {
        Ok((guide, mut values)) => {
            for locale in ["pl", "en"] {
                if !values.iter().any(|v| v.locale == locale) {
                    values.push(blank(locale));
                }
            }
            match session.csrf_token().await {
                Ok(csrf) => page(Edit {
                    lang,
                    id: id.to_string(),
                    version: guide.version,
                    status: guide.status,
                    values,
                    csrf,
                    existing: true,
                }),
                Err(e) => failure(e.into(), lang),
            }
        }
        Err(e) => failure(e, lang),
    }
}
#[derive(Debug, Deserialize)]
struct Write {
    pl_title: String,
    pl_slug: String,
    pl_summary: String,
    pl_markdown: String,
    en_title: String,
    en_slug: String,
    en_summary: String,
    en_markdown: String,
    #[serde(default)]
    note: String,
    #[serde(default)]
    lock_version: i64,
}
impl Write {
    fn values(&self) -> Vec<Translation> {
        vec![
            Translation {
                locale: "pl".into(),
                title: self.pl_title.clone(),
                slug: self.pl_slug.clone(),
                summary: self.pl_summary.clone(),
                markdown: self.pl_markdown.clone(),
            },
            Translation {
                locale: "en".into(),
                title: self.en_title.clone(),
                slug: self.en_slug.clone(),
                summary: self.en_summary.clone(),
                markdown: self.en_markdown.clone(),
            },
        ]
    }
}
async fn create<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(context): Extension<CheckContext>,
    headers: HeaderMap,
    input: Result<Form<Write>, axum::extract::rejection::FormRejection>,
) -> Response {
    let Form(input) = match input {
        Ok(value) => value,
        Err(_) => return failure(Error::Invalid, language(&headers)),
    };
    match s
        .repository
        .create(context, &input.values(), None, &input.note)
        .await
    {
        Ok(id) => Redirect::to(&format!("/admin/wiki/articles/{id}/edit")).into_response(),
        Err(e) => failure(e, language(&headers)),
    }
}
async fn edit<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(context): Extension<CheckContext>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    input: Result<Form<Write>, axum::extract::rejection::FormRejection>,
) -> Response {
    let Form(input) = match input {
        Ok(value) => value,
        Err(_) => return failure(Error::Invalid, language(&headers)),
    };
    changed(
        s.repository
            .edit(
                context,
                id,
                input.lock_version,
                &input.values(),
                &input.note,
            )
            .await,
        id,
        language(&headers),
    )
}
fn changed(result: Result<(), Error>, id: Uuid, lang: &str) -> Response {
    match result {
        Ok(()) => Redirect::to(&format!("/admin/wiki/articles/{id}/edit")).into_response(),
        Err(e) => failure(e, lang),
    }
}
#[derive(Debug, Deserialize)]
struct Version {
    lock_version: i64,
}
async fn transition<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(context): Extension<CheckContext>,
    Path((id, action)): Path<(Uuid, String)>,
    headers: HeaderMap,
    input: Result<Form<Version>, axum::extract::rejection::FormRejection>,
) -> Response {
    let Form(input) = match input {
        Ok(value) => value,
        Err(_) => return failure(Error::Invalid, language(&headers)),
    };
    let lang = language(&headers);
    let action = match action.as_str() {
        "submit-review" => Action::Submit,
        "return-draft" => Action::ReturnDraft,
        "publish" => Action::Publish,
        "unpublish" => Action::Unpublish,
        "archive" => Action::Archive,
        _ => return failure(Error::NotFound, lang),
    };
    changed(
        s.repository
            .transition(context, id, input.lock_version, action)
            .await,
        id,
        lang,
    )
}
#[derive(Template)]
#[template(path = "admin/wiki/history.html", config = "wiki_askama.toml")]
struct History<'a> {
    lang: &'a str,
    id: Uuid,
    version: i64,
    rows: Vec<Revision>,
    csrf: String,
}
impl History<'_> {
    fn t(&self, key: &str) -> String {
        text(self.lang, key)
    }
}
async fn history<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    let lang = language(&headers);
    let result = async {
        let (guide, _) = s.repository.get(context, id).await?;
        let rows = s.repository.history(context, id).await?;
        let csrf = session.csrf_token().await?;
        Ok::<_, Error>((guide, rows, csrf))
    }
    .await;
    match result {
        Ok((guide, rows, csrf)) => page(History {
            lang,
            id,
            version: guide.version,
            rows,
            csrf,
        }),
        Err(e) => failure(e, lang),
    }
}
async fn restore<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(context): Extension<CheckContext>,
    Path((id, source)): Path<(Uuid, i64)>,
    headers: HeaderMap,
    input: Result<Form<Version>, axum::extract::rejection::FormRejection>,
) -> Response {
    let Form(input) = match input {
        Ok(value) => value,
        Err(_) => return failure(Error::Invalid, language(&headers)),
    };
    changed(
        s.repository
            .restore(context, id, input.lock_version, source)
            .await,
        id,
        language(&headers),
    )
}
async fn preview<A: AccountStateReader>(
    State(s): State<Arc<Staff<A>>>,
    Extension(context): Extension<CheckContext>,
    Path((id, locale)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> Response {
    let lang = language(&headers);
    if let Err(e) = s
        .repository
        .authorize(context, Permission::ManageWikiArticles)
        .await
    {
        return failure(e, lang);
    }
    match s.repository.get(context, id).await {
        Ok((_, values)) => match values.into_iter().find(|v| v.locale == locale) {
            Some(value) => {
                let rendered = domain::render(&value.markdown);
                page(Article {
                    lang: &locale,
                    value,
                    rendered,
                })
            }
            None => failure(Error::NotFound, lang),
        },
        Err(e) => failure(e, lang),
    }
}

async fn response_policy(mut request: Request, next: Next) -> Response {
    let lang = locale(request.uri(), request.headers()).unwrap_or(language(request.headers()));
    request.extensions_mut().insert(if lang == "pl" {
        oteryn_web::Language::Pl
    } else {
        oteryn_web::Language::En
    });
    let mut response = next.run(request).await;
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
        (
            "strict-transport-security",
            "max-age=63072000; includeSubDomains",
        ),
        ("vary", "Accept-Language"),
    ] {
        response
            .headers_mut()
            .insert(name, axum::http::HeaderValue::from_static(value));
    }
    response
}
#[cfg(test)]
mod tests;
