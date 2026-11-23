//! Browser reads and account preferences for settled Platform sales.
mod store;

use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::Error;
use store::{Account, Change, Filters, MAX_PAGE, PAGE_SIZE, Price, Sale};

pub fn router(pool: PgPool, origin: &str) -> Router {
    let mut sessions = SessionLayer::new(pool.clone());
    sessions.csrf.trusted_origins = vec![origin.to_owned()];
    Router::new()
        .route("/{lang}/bazaar/history", get(market))
        .route("/{lang}/account/bazaar/history", get(own).post(mutate))
        .layer(axum::middleware::from_fn_with_state(
            sessions,
            oteryn_web::session::middleware,
        ))
        .with_state(pool)
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
struct Search {
    #[serde(flatten)]
    filters: Filters,
    sale: Option<Uuid>,
    left: Option<Uuid>,
    right: Option<Uuid>,
    page: u16,
}

impl Search {
    fn selection(&self) -> Result<Option<Vec<Uuid>>, Error> {
        let selection = match (self.sale, self.left, self.right) {
            (None, None, None) => return Ok(None),
            (Some(id), None, None) if !id.is_nil() => vec![id],
            (None, Some(a), Some(b)) if a != b && !a.is_nil() && !b.is_nil() => vec![a, b],
            _ => return Err(Error::InvalidInput),
        };
        if self.page != 0 {
            return Err(Error::InvalidInput);
        }
        Ok(Some(selection))
    }

    fn page_url(&self, page: u16) -> String {
        let query = serde_urlencoded::to_string(&self.filters).unwrap_or_default();
        format!("?{query}&page={page}")
    }
}

#[derive(Template)]
#[template(path = "bazaar/history.html", config = "src/characters/askama.toml")]
struct History<'a> {
    lang: &'a str,
    own: bool,
    csrf: String,
    search: Search,
    sales: Vec<Sale>,
    prices: Vec<Price>,
    account: Account,
    message: &'static str,
    previous: Option<String>,
    next: Option<String>,
}

impl<'a> History<'a> {
    fn new(lang: &'a str, own: bool) -> Self {
        Self {
            lang,
            own,
            csrf: String::new(),
            search: Search::default(),
            sales: vec![],
            prices: vec![],
            account: Account::default(),
            message: "",
            previous: None,
            next: None,
        }
    }

    fn text(&self, key: &str) -> String {
        text(self.lang, key)
    }

    fn path(&self) -> String {
        format!(
            "/{}/{}bazaar/history",
            self.lang,
            if self.own { "account/" } else { "" }
        )
    }

    fn watched(&self, sale: &Uuid) -> bool {
        self.account.watched.iter().any(|row| &row.id == sale)
    }

    fn comparable(&self) -> bool {
        self.search.left.is_some()
            && self.sales.len() == 2
            && self.sales[0].currency == self.sales[1].currency
    }

    fn difference(&self) -> i64 {
        // Amounts are strictly positive, so the difference fits in a signed integer.
        self.sales
            .first()
            .zip(self.sales.get(1))
            .map_or(0, |(a, b)| a.amount.abs_diff(b.amount) as i64)
    }

    fn render_response(&self, status: StatusCode) -> Response {
        match self.render() {
            Ok(body) => (
                status,
                [(header::CACHE_CONTROL, "private, no-store")],
                Html(body),
            )
                .into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub(crate) fn text(lang: &str, key: &str) -> String {
    let locale = if lang == "pl" {
        unic_langid::langid!("pl")
    } else {
        unic_langid::langid!("en")
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
    let messages = if lang == "pl" {
        include_str!("pl.ftl")
    } else {
        include_str!("en.ftl")
    };
    let Ok(resource) = fluent_bundle::FluentResource::try_new(messages.to_owned()) else {
        return String::new();
    };
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    bundle
        .get_message(key)
        .and_then(|message| message.value())
        .map(|value| bundle.format_pattern(value, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}

fn valid_lang(lang: &str) -> bool {
    matches!(lang, "pl" | "en")
}

fn failure(lang: &str, own: bool, error: Error) -> Response {
    let mut page = History::new(lang, own);
    let (status, message) = match error {
        Error::InvalidInput => (StatusCode::UNPROCESSABLE_ENTITY, "invalid"),
        Error::NotFound => (StatusCode::NOT_FOUND, "missing"),
        Error::Denied => (StatusCode::FORBIDDEN, "denied"),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
    };
    page.message = message;
    page.render_response(status)
}

async fn market(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    query: Result<Query<Search>, QueryRejection>,
) -> Response {
    show(pool, &lang, session, query, false).await
}

async fn own(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    query: Result<Query<Search>, QueryRejection>,
) -> Response {
    show(pool, &lang, session, query, true).await
}

async fn show(
    pool: PgPool,
    lang: &str,
    session: BrowserSession,
    query: Result<Query<Search>, QueryRejection>,
    own: bool,
) -> Response {
    if !valid_lang(lang) {
        return failure("en", own, Error::NotFound);
    }
    let actor = session.account_id().await;
    if own && actor.is_none() {
        return Redirect::to(&format!("/{lang}/login")).into_response();
    }
    let Ok(Query(search)) = query else {
        return failure(lang, own, Error::InvalidInput);
    };
    let mut page = History::new(lang, own);
    match load(&pool, &mut page, search, actor, &session).await {
        Ok(()) => page.render_response(StatusCode::OK),
        Err(error) => failure(lang, own, error),
    }
}

async fn load(
    pool: &PgPool,
    page: &mut History<'_>,
    search: Search,
    actor: Option<Uuid>,
    session: &BrowserSession,
) -> Result<(), Error> {
    let ids = search.selection()?;
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let viewer = match actor {
        Some(actor) => match store::authorize(&mut tx, actor, false).await {
            Ok(()) => Some(actor),
            Err(Error::Denied) if !page.own => None,
            Err(error) => return Err(error),
        },
        None => None,
    };
    if let Some(viewer) = viewer {
        if page.own {
            page.account = store::account(&mut tx, viewer).await?;
        } else {
            page.account.watched = store::watched(&mut tx, viewer).await?;
        }
        page.csrf = session.csrf_token().await?;
    }
    let mut sales = store::sales(
        &mut tx,
        viewer.filter(|_| page.own),
        &search.filters,
        ids.as_deref(),
        search.page,
    )
    .await?;
    if let Some(ids) = ids {
        if sales.len() != ids.len() {
            return Err(Error::NotFound);
        }
        sales.sort_by_key(|sale| sale.id != ids[0]);
    } else {
        if sales.len() > PAGE_SIZE && search.page < MAX_PAGE {
            page.next = Some(search.page_url(search.page + 1));
        }
        sales.truncate(PAGE_SIZE);
        if search.page > 0 {
            page.previous = Some(search.page_url(search.page - 1));
        }
    }
    if !page.own {
        page.prices = store::prices(&mut tx).await?;
    }
    tx.commit().await?;
    page.sales = sales;
    page.search = search;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Action {
    Save,
    Delete,
    EnableAlerts,
    DisableAlerts,
    Watch,
    Unwatch,
    Dismiss,
    Share,
    Unshare,
}

#[derive(Debug, Deserialize)]
struct Fields {
    action: Action,
    #[serde(rename = "_csrf")]
    _csrf: Option<String>,
    #[serde(default)]
    name: String,
    sale: Option<Uuid>,
    #[serde(flatten)]
    filters: Filters,
}

impl Fields {
    fn change(self) -> Result<Change, Error> {
        let sale = || {
            self.sale
                .filter(|id| !id.is_nil())
                .ok_or(Error::InvalidInput)
        };
        Ok(match self.action {
            Action::Save => Change::Save {
                name: self.name,
                filters: self.filters,
            },
            Action::Delete => Change::Delete { name: self.name },
            Action::EnableAlerts => Change::EnableAlerts,
            Action::DisableAlerts => Change::DisableAlerts,
            Action::Watch => Change::Watch { sale: sale()? },
            Action::Unwatch => Change::Unwatch { sale: sale()? },
            Action::Dismiss => Change::Dismiss { sale: sale()? },
            Action::Share => Change::Share { sale: sale()? },
            Action::Unshare => Change::Unshare { sale: sale()? },
        })
    }
}

async fn mutate(
    State(pool): State<PgPool>,
    Path(lang): Path<String>,
    Extension(session): Extension<BrowserSession>,
    Form(fields): Form<Fields>,
) -> Response {
    if !valid_lang(&lang) {
        return failure("en", true, Error::NotFound);
    }
    let Some(actor) = session.account_id().await else {
        return failure(&lang, true, Error::Denied);
    };
    let destination = match &fields.action {
        Action::Share | Action::Unshare => fields.sale.map(|id| format!("?sale={id}")),
        _ => None,
    };
    let change = match fields.change() {
        Ok(change) => change,
        Err(error) => return failure(&lang, true, error),
    };
    match store::change(&pool, actor, change).await {
        Ok(()) => Redirect::to(&format!(
            "/{lang}/account/bazaar/history{}",
            destination.as_deref().unwrap_or_default()
        ))
        .into_response(),
        Err(error) => failure(&lang, true, error),
    }
}

#[cfg(test)]
mod tests;
