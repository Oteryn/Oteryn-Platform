//! Staff Bazaar surface, guarded by the panel's host, session, CSRF and MFA policy.
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{AccountStateReader, Authorizer, CheckContext, Permission},
    web::{self, PanelConfig, StaffSession},
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_bazaar::moderation::{self, StaffCommand, web::Locale};
use serde::Deserialize;
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
#[cfg(test)]
#[path = "bazaar_tests.rs"]
mod tests;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bazaar moderation is disabled")]
    Disabled,
    #[error(transparent)]
    Authorization(#[from] crate::rbac::Error),
    #[error(transparent)]
    Domain(#[from] oteryn_bazaar::Error),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}
#[derive(Debug)]
pub struct StaffBazaar<A> {
    pool: PgPool,
    auth: Authorizer<A>,
    enabled: bool,
}
impl<A: AccountStateReader> StaffBazaar<A> {
    pub fn new(pool: PgPool, accounts: A, enabled: bool) -> Self {
        Self {
            auth: Authorizer::new(pool.clone(), accounts),
            pool,
            enabled,
        }
    }
    async fn check(&self, context: CheckContext) -> Result<(), Error> {
        if !context.mfa_verified || !context.require_staff_mfa {
            return Err(crate::rbac::Error::Denied.into());
        }
        if !self.enabled {
            return Err(Error::Disabled);
        }
        self.auth.check(context, Permission::Access).await?;
        self.auth
            .check(context, Permission::ManageMarketplace)
            .await?;
        Ok(())
    }
    pub async fn execute(&self, context: CheckContext, command: StaffCommand) -> Result<(), Error> {
        self.check(context).await?;
        let mut tx = self.pool.begin().await?;
        moderation::lock_staff_target(&mut tx, &command).await?;
        let before = snapshot(&mut tx, command.target()).await?;
        moderation::execute_staff(&mut tx, context.account, &command).await?;
        self.check(context).await?;
        let after = snapshot(&mut tx, command.target()).await?;
        audit::append(
            &mut tx,
            Entry {
                context: AuditContext {
                    actor: Some(context.account),
                    request_id: Uuid::now_v7(),
                },
                action: command.action(),
                target: &command.target().to_string(),
                before,
                after,
            },
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
async fn snapshot(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
) -> Result<Value, sqlx::Error> {
    sqlx::query_scalar("SELECT jsonb_build_object('hold',(SELECT to_jsonb(h) FROM bazaar_moderation_holds h WHERE listing_id=$1),'invalidation',(SELECT to_jsonb(r) FROM bazaar_invalidation_requests r WHERE id=$1 OR listing_id=$1),'report',(SELECT jsonb_build_object('state',state,'reviewed_by',reviewed_by) FROM bazaar_reports WHERE id=$1),'fees',(SELECT to_jsonb(f) FROM bazaar_fee_revisions f WHERE id=$1),'restriction',(SELECT to_jsonb(p) FROM bazaar_item_restrictions p WHERE id=$1))")
        .bind(id).fetch_one(&mut **tx).await
}
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let enabled = std::env::var("OTERYN_BAZAAR_MODERATION").as_deref() == Ok("true");
    if !enabled {
        return Ok(Router::new());
    }
    web::configured_guard(
        routes(pool.clone(), web::PendingAuthentication, enabled),
        pool,
        Permission::ManageMarketplace,
    )
}
pub fn router<I: StaffSession, A: AccountStateReader + Clone + 'static>(
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
    enabled: bool,
) -> Router {
    if !enabled {
        return Router::new();
    }
    web::guarded(
        routes(pool.clone(), accounts.clone(), enabled),
        pool,
        config,
        identity,
        accounts,
        Permission::ManageMarketplace,
    )
}
fn routes<A: AccountStateReader + 'static>(pool: PgPool, accounts: A, enabled: bool) -> Router {
    Router::new()
        .route("/admin/{lang}/bazaar", get(index::<A>).post(store::<A>))
        .layer(axum::extract::DefaultBodyLimit::max(8192))
        .layer(axum::middleware::from_fn(no_store))
        .with_state(Arc::new(StaffBazaar::new(pool, accounts, enabled)))
}
async fn no_store(req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut r = next.run(req).await;
    r.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("private, no-store"),
    );
    r
}
fn status(error: Error) -> StatusCode {
    match error {
        Error::Disabled => StatusCode::NOT_FOUND,
        Error::Domain(e) => moderation::web::status(e),
        Error::Authorization(crate::rbac::Error::Storage(_)) | Error::Database(_) => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        Error::Authorization(_) => StatusCode::FORBIDDEN,
    }
}
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Input {
    action: String,
    id: String,
    listing: String,
    reason: String,
    coin_bps: String,
    gold_bps: String,
    listing_fee: String,
    promotion_price: String,
    world: String,
    family: String,
    production_key: String,
    revision_ref: String,
    minimum_value: String,
    accepted: String,
    #[serde(rename = "_csrf")]
    _csrf: String,
}
impl Input {
    fn command(self) -> Result<StaffCommand, StatusCode> {
        let uuid = |s: &str| {
            s.parse::<Uuid>()
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
        };
        let number = |s: &str| {
            s.parse::<i64>()
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
        };
        Ok(match self.action.as_str() {
            "suspend" => StaffCommand::Suspend {
                listing: uuid(&self.listing)?,
                reason: self.reason,
            },
            "request" => StaffCommand::RequestInvalidation {
                id: uuid(&self.id)?,
                listing: uuid(&self.listing)?,
                reason: self.reason,
            },
            "approve" => StaffCommand::ApproveInvalidation {
                id: uuid(&self.id)?,
            },
            "review" => StaffCommand::ReviewReport {
                id: uuid(&self.id)?,
                accepted: match self.accepted.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
                },
            },
            "fees" => StaffCommand::Fees {
                id: uuid(&self.id)?,
                coin_bps: self
                    .coin_bps
                    .parse()
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
                gold_bps: self
                    .gold_bps
                    .parse()
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
                listing_fee: number(&self.listing_fee)?,
                promotion_price: number(&self.promotion_price)?,
                reason: self.reason,
            },
            "restrict" => StaffCommand::RestrictItem {
                id: uuid(&self.id)?,
                world: uuid(&self.world)?,
                family: self.family,
                production_key: self.production_key,
                revision_ref: self.revision_ref,
                minimum_value: number(&self.minimum_value)?,
                denied: true,
                reason: self.reason,
            },
            _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        })
    }
}
#[derive(Template)]
#[template(
    source = r#"<!doctype html><html lang="{{ locale.0 }}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{{ self.text("staff") }} · Oteryn</title><link rel="stylesheet" href="/static/css/tokens.css"><link rel="stylesheet" href="/static/css/components.css"></head><body class="op-theme"><main class="op-stack"><h1>{{ self.text("staff") }}</h1><p>{{ self.text("invalidation-help") }}</p><p>{{ self.text("policy-help") }}</p>
<h2>{{ self.text("pending") }}</h2><ul class="op-stack">{% for r in reports %}<li class="op-panel"><p>{{ r.id }}</p><p>{{ r.listing_id }}</p><p>{{ self.text(&r.category) }}</p><p>{{ r.detail }}</p><form method="post"><input type="hidden" name="_csrf" value="{{ csrf }}"><input type="hidden" name="action" value="review"><input type="hidden" name="id" value="{{ r.id }}"><label for="review-{{ r.id }}">{{ self.text("review") }}</label><select id="review-{{ r.id }}" name="accepted" class="op-input"><option value="true">{{ self.text("accepted") }}</option><option value="false">{{ self.text("dismissed") }}</option></select><button class="op-button" type="submit">{{ self.text("submit") }}</button></form></li>{% endfor %}</ul>
{% for section in sections %}<section class="op-stack"><h2>{{ self.text(section.key) }}</h2><ul>{% for row in section.rows %}<li>{{ row }}</li>{% endfor %}</ul></section>{% endfor %}
<section><h2>{{ self.text("fees") }}</h2><ul class="op-stack">{% for fee in fees %}<li class="op-panel"><p>{{ fee.id }}</p><p>{{ self.text("coin-bps") }}: {{ fee.coin_bps }}</p><p>{{ self.text("gold-bps") }}: {{ fee.gold_bps }}</p><p>{{ self.text("listing-fee") }}: {{ fee.listing_fee }}</p><p>{{ self.text("promotion-price") }}: {{ fee.promotion_price }}</p></li>{% endfor %}</ul></section>
<section><h2>{{ self.text("restrict") }}</h2><p>{{ self.text("catalog-unavailable") }}</p><ul class="op-stack">{% for item in restrictions %}<li class="op-panel"><p>{{ self.text("world") }}: {{ item.world_id }}</p><p>{{ self.text("family") }}: {{ item.family }}</p><p>{{ self.text("production-key") }}: {{ item.production_key }}</p><p>{{ self.text("revision-ref") }}: {{ item.revision_ref }}</p><p>{{ self.text("minimum-value") }}: {{ item.minimum_value }}</p><p>{{ self.text("denied") }}: {% if item.denied %}{{ self.text("yes") }}{% else %}{{ self.text("no") }}{% endif %}</p></li>{% endfor %}</ul></section>
{% for form in forms %}<section><h2>{{ self.text(form.action) }}</h2><form method="post" class="op-panel op-stack"><input type="hidden" name="_csrf" value="{{ csrf }}"><input type="hidden" name="action" value="{{ form.action }}">{% if form.action != "approve" %}<input type="hidden" name="id" value="{{ form.id }}">{% endif %}{% for field in form.fields %}<label for="{{ form.action }}-{{ field.name }}">{{ self.text(field.label) }}</label><input class="op-input" id="{{ form.action }}-{{ field.name }}" name="{{ field.name }}" type="text" value="{{ field.value }}" maxlength="500" required>{% endfor %}<button type="submit" class="op-button">{{ self.text("submit") }}</button></form></section>{% endfor %}</main></body></html>"#,
    ext = "html"
)]
struct Page {
    locale: Locale,
    csrf: String,
    reports: Vec<moderation::PendingReport>,
    sections: Vec<Section>,
    forms: Vec<PanelForm>,
    fees: Vec<FeeRow>,
    restrictions: Vec<RestrictionRow>,
}
impl Page {
    fn text(&self, key: &str) -> String {
        self.locale.text(key)
    }
}
#[derive(sqlx::FromRow)]
struct FeeRow {
    id: Uuid,
    coin_bps: i32,
    gold_bps: i32,
    listing_fee: i64,
    promotion_price: i64,
}
#[derive(sqlx::FromRow)]
struct RestrictionRow {
    world_id: Uuid,
    family: String,
    production_key: String,
    revision_ref: String,
    minimum_value: i64,
    denied: bool,
}
struct Section {
    key: &'static str,
    rows: Vec<String>,
}
struct Field {
    name: &'static str,
    label: &'static str,
    value: &'static str,
}
struct PanelForm {
    action: &'static str,
    id: Uuid,
    fields: Vec<Field>,
}
fn form(action: &'static str, fields: &[(&'static str, &'static str, &'static str)]) -> PanelForm {
    PanelForm {
        action,
        id: Uuid::now_v7(),
        fields: fields
            .iter()
            .map(|(name, label, value)| Field { name, label, value })
            .collect(),
    }
}
async fn index<A: AccountStateReader>(
    State(api): State<Arc<StaffBazaar<A>>>,
    Path(lang): Path<String>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<oteryn_web::session::BrowserSession>,
) -> Result<Response, StatusCode> {
    let locale = Locale::parse(&lang)?;
    api.check(context).await.map_err(status)?;
    let csrf = session
        .csrf_token()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut connection = api
        .pool
        .acquire()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let reports = moderation::pending_reports(&mut connection)
        .await
        .map_err(moderation::web::status)?;
    let holds = sqlx::query_scalar(
        "SELECT listing_id::text FROM bazaar_moderation_holds ORDER BY updated_at DESC LIMIT 100",
    )
    .fetch_all(&mut *connection)
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let invalidations=sqlx::query_scalar("SELECT id::text FROM bazaar_invalidation_requests WHERE state='pending' ORDER BY created_at LIMIT 100").fetch_all(&mut *connection).await.map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
    let fees=sqlx::query_as("SELECT id,coin_bps,gold_bps,listing_fee,promotion_price FROM bazaar_fee_revisions ORDER BY created_at DESC,id DESC LIMIT 100").fetch_all(&mut *connection).await.map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
    let restrictions=sqlx::query_as("SELECT world_id,family,production_key,revision_ref,minimum_value,denied FROM bazaar_item_restrictions ORDER BY created_at DESC,id DESC LIMIT 100").fetch_all(&mut *connection).await.map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
    let forms = vec![
        form(
            "suspend",
            &[("listing", "listing", ""), ("reason", "reason", "")],
        ),
        form(
            "request",
            &[("listing", "listing", ""), ("reason", "reason", "")],
        ),
        form("approve", &[("id", "identifier", "")]),
        form(
            "fees",
            &[
                ("coin_bps", "coin-bps", "1000"),
                ("gold_bps", "gold-bps", "250"),
                ("listing_fee", "listing-fee", "0"),
                ("promotion_price", "promotion-price", ""),
                ("reason", "reason", ""),
            ],
        ),
        form(
            "restrict",
            &[
                ("world", "world", ""),
                ("family", "family", ""),
                ("production_key", "production-key", ""),
                ("revision_ref", "revision-ref", ""),
                ("minimum_value", "minimum-value", ""),
                ("reason", "reason", ""),
            ],
        ),
    ];
    let page = Page {
        locale,
        csrf,
        reports,
        forms,
        fees,
        restrictions,
        sections: vec![
            Section {
                key: "holds",
                rows: holds,
            },
            Section {
                key: "invalidations",
                rows: invalidations,
            },
        ],
    };
    page.render()
        .map(|s| Html(s).into_response())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
async fn store<A: AccountStateReader>(
    State(api): State<Arc<StaffBazaar<A>>>,
    Path(lang): Path<String>,
    Extension(context): Extension<CheckContext>,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    Locale::parse(&lang)?;
    api.execute(context, input.command()?)
        .await
        .map_err(status)?;
    Ok(Redirect::to(&format!("/admin/{lang}/bazaar")).into_response())
}
