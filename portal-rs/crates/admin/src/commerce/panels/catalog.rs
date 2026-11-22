use super::{Language, localized, render};
use crate::{
    rbac::{CheckContext, MutationAuthorization, Permission},
    staff_actions::{self, Action, AuditChange, AuditValue, Error, Guard, Snapshot},
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Query, State, rejection::FormRejection},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_store::catalog::{self, Catalog, Offer};
use oteryn_ui::Chrome;
use oteryn_web::{csrf::Verified, session::BrowserSession};
use serde::Deserialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub fn staging(pool: PgPool) -> Router {
    Router::new()
        .route("/admin/store-catalog", get(offers).post(stage))
        .with_state(pool)
}

pub fn activation(pool: PgPool) -> Router {
    Router::new()
        .route("/admin/store-activation", get(revisions).post(activate))
        .with_state(pool)
}

#[derive(Template)]
#[template(
    path = "commerce/panels/catalog.html",
    config = "src/commerce/askama.toml"
)]
struct Page {
    chrome: Chrome,
    title: &'static str,
    csrf: String,
    current: String,
    offers: Vec<Offer>,
    revisions: Vec<Comparison>,
    activation: bool,
}

struct Comparison {
    revision: String,
    changes: Vec<PriceChange>,
}

struct PriceChange {
    offer: String,
    before: String,
    after: String,
}

const CURRENT: &str = "SELECT catalog_revision FROM store_active_catalog WHERE id=1";
const DOCUMENT: &str =
    "SELECT canonical_bytes FROM store_price_list_imports WHERE catalog_revision=$1";

fn parse(bytes: &[u8]) -> Result<Catalog, Error> {
    Catalog::parse(bytes).map_err(|_| Error::Unavailable)
}

async fn offers(
    State(pool): State<PgPool>,
    Extension(chrome): Extension<Chrome>,
    Extension(session): Extension<BrowserSession>,
    Query(query): Query<Language>,
) -> Response {
    let chrome = localized(chrome, query.lang.as_deref());
    match offer_page(&pool, &session, chrome.clone()).await {
        Ok(page) => render(&page),
        Err(error) => error.response(&chrome),
    }
}

async fn offer_page(
    pool: &PgPool,
    session: &BrowserSession,
    chrome: Chrome,
) -> Result<Page, Error> {
    let csrf = session.csrf_token().await.map_err(|_| Error::Unavailable)?;
    let current: Option<String> = sqlx::query_scalar(CURRENT).fetch_one(pool).await?;
    let offers = if let Some(revision) = &current {
        let bytes: Vec<u8> = sqlx::query_scalar(DOCUMENT)
            .bind(revision)
            .fetch_one(pool)
            .await?;
        parse(&bytes)?.offers().to_vec()
    } else {
        vec![]
    };
    Ok(Page {
        chrome,
        title: "commerce-catalog",
        csrf,
        current: current.unwrap_or_default(),
        offers,
        revisions: vec![],
        activation: false,
    })
}

async fn revisions(
    State(pool): State<PgPool>,
    Extension(chrome): Extension<Chrome>,
    Extension(session): Extension<BrowserSession>,
    Query(query): Query<Language>,
) -> Response {
    let chrome = localized(chrome, query.lang.as_deref());
    match revision_page(&pool, &session, chrome.clone()).await {
        Ok(page) => render(&page),
        Err(error) => error.response(&chrome),
    }
}

async fn revision_page(
    pool: &PgPool,
    session: &BrowserSession,
    chrome: Chrome,
) -> Result<Page, Error> {
    let csrf = session.csrf_token().await.map_err(|_| Error::Unavailable)?;
    let current: Option<String> = sqlx::query_scalar(CURRENT).fetch_one(pool).await?;
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT catalog_revision FROM store_price_list_imports
         WHERE catalog_revision IS DISTINCT FROM $1 ORDER BY created_at DESC LIMIT 10",
    )
    .bind(&current)
    .fetch_all(pool)
    .await?;
    let before = if let Some(revision) = &current {
        let bytes: Vec<u8> = sqlx::query_scalar(DOCUMENT)
            .bind(revision)
            .fetch_one(pool)
            .await?;
        parse(&bytes)?
            .offers()
            .iter()
            .map(|offer| (offer.offer_key.clone(), offer.unit_price.clone()))
            .collect()
    } else {
        std::collections::BTreeMap::new()
    };
    let mut revisions = Vec::new();
    for (revision,) in rows {
        let prices: Vec<(String, String)> = sqlx::query_as(
            "SELECT offer_key,unit_price::text FROM store_offers WHERE catalog_revision=$1 ORDER BY offer_key",
        ).bind(&revision).fetch_all(pool).await?;
        let mut remaining = before.clone();
        let mut changes = Vec::new();
        for (offer, after) in prices {
            let previous = remaining.remove(&offer).unwrap_or_default();
            if previous != after {
                changes.push(PriceChange {
                    offer,
                    before: previous,
                    after,
                });
            }
        }
        changes.extend(remaining.into_iter().map(|(offer, before)| PriceChange {
            offer,
            before,
            after: String::new(),
        }));
        revisions.push(Comparison { revision, changes });
    }
    Ok(Page {
        chrome,
        title: "commerce-activation",
        csrf,
        current: current.unwrap_or_default(),
        offers: vec![],
        revisions,
        activation: true,
    })
}

#[derive(Deserialize)]
struct Stage {
    current: String,
    offer: String,
    price: i64,
    reason: String,
}

#[derive(Deserialize)]
struct Activate {
    current: String,
    revision: String,
    reason: String,
}

fn revision_valid(revision: &str) -> bool {
    revision.len() == 64
        && revision
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}

async fn stage(
    State(pool): State<PgPool>,
    Extension(chrome): Extension<Chrome>,
    Extension(actor): Extension<CheckContext>,
    Extension(authority): Extension<MutationAuthorization>,
    csrf: Option<Extension<Verified>>,
    Query(query): Query<Language>,
    form: Result<Form<Stage>, FormRejection>,
) -> Response {
    let chrome = localized(chrome, query.lang.as_deref());
    let Ok(Form(Stage {
        current,
        offer,
        price,
        reason,
    })) = form
    else {
        return Error::Invalid.response(&chrome);
    };
    let result = staff_actions::run(
        &pool,
        Guard {
            actor,
            authority: &authority,
            csrf: csrf.map(|v| v.0),
        },
        Action {
            permission: Permission::ImportStoreCatalog,
            action: "store.catalog.price.staged",
            reason: &reason,
        },
        || {
            if !revision_valid(&current) || !(1..=128).contains(&offer.len()) || price <= 0 {
                return Err(Error::Invalid);
            }
            Ok((vec![], (current, offer, price)))
        },
        |conn, (current, offer, price)| {
            Box::pin(async move {
                lock_current(conn, &current).await?;
                let bytes: Vec<u8> = sqlx::query_scalar(DOCUMENT)
                    .bind(&current)
                    .fetch_one(&mut *conn)
                    .await?;
                let base = parse(&bytes)?;
                let old_price: i64 = base
                    .offers()
                    .iter()
                    .find(|item| item.offer_key == offer)
                    .and_then(|item| item.unit_price.parse().ok())
                    .ok_or(Error::Invalid)?;
                if old_price == price {
                    return Ok(None);
                }
                let proposed = base.with_price(&offer, price).map_err(|_| Error::Invalid)?;
                if !catalog::insert_revision(conn, &proposed, actor.account)
                    .await
                    .map_err(catalog_error)?
                {
                    return Ok(None);
                }
                Ok(Some(AuditChange {
                    target: revision_id(proposed.revision())?,
                    before: price_snapshot(&current, old_price)?,
                    after: price_snapshot(proposed.revision(), price)?,
                }))
            })
        },
    )
    .await;
    finish(result, &chrome, "/admin/store-catalog")
}

async fn activate(
    State(pool): State<PgPool>,
    Extension(chrome): Extension<Chrome>,
    Extension(actor): Extension<CheckContext>,
    Extension(authority): Extension<MutationAuthorization>,
    csrf: Option<Extension<Verified>>,
    Query(query): Query<Language>,
    form: Result<Form<Activate>, FormRejection>,
) -> Response {
    let chrome = localized(chrome, query.lang.as_deref());
    let Ok(Form(Activate {
        current,
        revision,
        reason,
    })) = form
    else {
        return Error::Invalid.response(&chrome);
    };
    let result = staff_actions::run(
        &pool, Guard { actor, authority: &authority, csrf: csrf.map(|v| v.0) },
        Action { permission: Permission::ActivateStoreCatalog, action: "store.catalog.activated", reason: &reason },
        || {
            if !revision_valid(&revision) || (!current.is_empty() && !revision_valid(&current)) {
                return Err(Error::Invalid);
            }
            Ok((vec![], (current, revision)))
        },
        |conn, (current, revision)| Box::pin(async move {
            let prior = lock_current(conn, &current).await?;
            if prior.as_deref() == Some(&revision) {
                return Ok(None);
            }
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM store_price_list_imports WHERE catalog_revision=$1)",
            ).bind(&revision).fetch_one(&mut *conn).await?;
            if !exists {
                return Err(Error::Invalid);
            }
            let activation = Uuid::now_v7();
            sqlx::query("INSERT INTO store_catalog_activations (previous_revision,catalog_revision,actor_id,activation_id) VALUES ($1,$2,$3,$4)")
                .bind(&prior).bind(&revision).bind(actor.account).bind(activation).execute(&mut *conn).await?;
            sqlx::query("UPDATE store_active_catalog SET catalog_revision=$1,activated_by=$2,updated_at=now() WHERE id=1")
                .bind(&revision).bind(actor.account).execute(&mut *conn).await?;
            let before = prior.as_deref().map(revision_snapshot).transpose()?.unwrap_or_default();
            Ok(Some(AuditChange { target: activation, before, after: revision_snapshot(&revision)? }))
        }),
    ).await;
    finish(result, &chrome, "/admin/store-activation")
}

async fn lock_current(conn: &mut PgConnection, expected: &str) -> Result<Option<String>, Error> {
    let current: Option<String> = sqlx::query_scalar(
        "SELECT catalog_revision FROM store_active_catalog WHERE id=1 FOR UPDATE",
    )
    .fetch_one(conn)
    .await?;
    if current.as_deref().unwrap_or_default() != expected {
        return Err(Error::Conflict);
    }
    Ok(current)
}

fn catalog_error(error: catalog::Error) -> Error {
    match error {
        catalog::Error::Conflict => Error::Conflict,
        catalog::Error::Database(error) => error.into(),
        _ => Error::Unavailable,
    }
}

fn revision_id(revision: &str) -> Result<Uuid, Error> {
    Uuid::parse_str(revision.get(..32).ok_or(Error::Unavailable)?).map_err(|_| Error::Unavailable)
}

fn revision_snapshot(revision: &str) -> Result<Snapshot, Error> {
    Ok(Snapshot::from([
        (
            "revision_first_128_bits",
            AuditValue::Id(revision_id(revision)?),
        ),
        (
            "revision_last_128_bits",
            AuditValue::Id(
                Uuid::parse_str(revision.get(32..).ok_or(Error::Unavailable)?)
                    .map_err(|_| Error::Unavailable)?,
            ),
        ),
    ]))
}

fn price_snapshot(revision: &str, price: i64) -> Result<Snapshot, Error> {
    let mut snapshot = revision_snapshot(revision)?;
    snapshot.insert("unit_price", AuditValue::Count(price));
    Ok(snapshot)
}

fn finish(result: Result<(), Error>, chrome: &Chrome, route: &str) -> Response {
    match result {
        Ok(()) => Redirect::to(&format!("{route}?lang={}", chrome.lang)).into_response(),
        Err(error) => error.response(chrome),
    }
}
