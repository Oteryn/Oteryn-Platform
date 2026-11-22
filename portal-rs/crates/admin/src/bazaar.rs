//! Staff Bazaar surface, guarded by the panel's host, session, CSRF and MFA policy.
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{AccountStateReader, Authorizer, CheckContext, Permission},
    web::{self, PanelConfig, StaffSession},
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State},
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
    async fn check_in(
        &self,
        db: &mut sqlx::PgConnection,
        context: CheckContext,
    ) -> Result<(), Error> {
        if !context.mfa_verified || !context.require_staff_mfa {
            return Err(crate::rbac::Error::Denied.into());
        }
        if !self.enabled {
            return Err(Error::Disabled);
        }
        self.auth.check_in(db, context, Permission::Access).await?;
        self.auth
            .check_in(db, context, Permission::ManageMarketplace)
            .await?;
        Ok(())
    }
    pub async fn execute(&self, context: CheckContext, command: StaffCommand) -> Result<(), Error> {
        self.check(context).await?;
        let mut tx = self.pool.begin().await?;
        crate::rbac::mutation_budget(&mut tx).await?;
        // Bazaar globally locks its exclusive staff moderation gate/target before accounts.
        // Start the shared budget before that first domain lock; the canonical
        // helper retains it while locking ordered accounts and then RBAC.
        let resource_command = command.clone();
        crate::rbac::lock_mutation_resources_with::<Error, _>(&mut tx, move |connection| {
            Box::pin(async move {
                moderation::lock_staff_target(connection, &resource_command).await?;
                Ok(())
            })
        })
        .await?;
        crate::rbac::authorize_mutation(
            &mut tx,
            &[context.account],
            &self.auth,
            context,
            &[Permission::Access, Permission::ManageMarketplace],
        )
        .await?;
        self.check_in(&mut tx, context).await?;
        let before = snapshot(&mut tx, command.target()).await?;
        let outcome = moderation::execute_staff(&mut tx, context.account, &command).await?;
        self.check_in(&mut tx, context).await?;
        if outcome == moderation::StaffOutcome::Replay {
            tx.commit().await?;
            return Ok(());
        }
        let mut after = snapshot(&mut tx, command.target()).await?;
        after["reason"] = Value::String(command.reason().to_owned());
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
        self.check_in(&mut tx, context).await?;
        tx.commit().await?;
        Ok(())
    }
}
async fn snapshot(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
) -> Result<Value, sqlx::Error> {
    sqlx::query_scalar("SELECT jsonb_build_object('hold',(SELECT to_jsonb(h) FROM bazaar_moderation_holds h WHERE listing_id=$1),'invalidation',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY r.hold_revision,r.id),'[]'::jsonb) FROM bazaar_invalidation_requests r WHERE id=$1 OR (listing_id=$1 AND hold_revision=(SELECT revision FROM bazaar_moderation_holds WHERE listing_id=$1))),'report',(SELECT jsonb_build_object('state',state,'reviewed_by',reviewed_by) FROM bazaar_reports WHERE id=$1),'fees',(SELECT to_jsonb(f) FROM bazaar_fee_revisions f WHERE id=$1),'restriction',(SELECT to_jsonb(p) FROM bazaar_item_restrictions p WHERE id=$1))")
        .bind(id).fetch_one(&mut **tx).await
}
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let enabled = std::env::var("OTERYN_BAZAAR_MODERATION").as_deref() == Ok("true");
    if !enabled {
        return Ok(Router::new());
    }
    web::configured_guard(
        routes(
            pool.clone(),
            web::PortalIdentity::configured(pool.clone()),
            enabled,
        ),
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
                reason: self.reason,
            },
            "review" => StaffCommand::ReviewReport {
                id: uuid(&self.id)?,
                reason: self.reason,
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
<h2>{{ self.text("pending") }}</h2><ul class="op-stack">{% for r in reports %}<li class="op-panel"><p>{{ r.id }}</p><p>{{ r.listing_id }}</p><p>{{ self.text(&r.category) }}</p><p>{{ r.detail }}</p><form method="post"><input type="hidden" name="_csrf" value="{{ csrf }}"><input type="hidden" name="action" value="review"><input type="hidden" name="id" value="{{ r.id }}"><label for="review-{{ r.id }}">{{ self.text("review") }}</label><select id="review-{{ r.id }}" name="accepted" class="op-input"><option value="true">{{ self.text("accepted") }}</option><option value="false">{{ self.text("dismissed") }}</option></select><label for="reason-{{ r.id }}">{{ self.text("reason") }}</label><input class="op-input" id="reason-{{ r.id }}" name="reason" minlength="10" maxlength="500" required><button class="op-button" type="submit">{{ self.text("submit") }}</button></form></li>{% endfor %}</ul>
<section class="op-stack"><h2>{{ self.text("holds") }}</h2><ul>{% for id in holds %}<li>{{ id }}</li>{% endfor %}</ul></section>
{% for section in invalidations %}<section class="op-stack"><h2>{{ self.text(section.label) }}</h2><ul>
{% for row in section.rows %}<li data-invalidation="{{ row.id }}" data-request-state="{{ row.request_state }}" data-listing-state="{{ row.listing_state }}">{{ row.id }} · {{ self.text("listing") }} {{ row.listing }} · {{ self.text(&row.request_label()) }} · {{ self.text("listing-state") }} · {{ self.text(&row.listing_label()) }}</li>{% endfor %}
</ul>{% if let Some(next) = section.next %}<a href="/admin/{{ locale.0 }}/bazaar?pending_after={{ next }}">{{ self.text("invalidation-next") }}</a>{% endif %}</section>{% endfor %}
<section><h2>{{ self.text("fees") }}</h2><ul class="op-stack">{% for fee in fees %}<li class="op-panel"><p>{{ fee.id }}</p><p>{{ self.text("coin-bps") }}: {{ fee.coin_bps }}</p><p>{{ self.text("gold-bps") }}: {{ fee.gold_bps }}</p><p>{{ self.text("listing-fee") }}: {{ fee.listing_fee }}</p><p>{{ self.text("promotion-price") }}: {{ fee.promotion_price }}</p></li>{% endfor %}</ul></section>
<section><h2>{{ self.text("restrict") }}</h2><p>{{ self.text("catalog-unavailable") }}</p><ul class="op-stack">{% for item in restrictions %}<li class="op-panel"><p>{{ self.text("world") }}: {{ item.world_id }}</p><p>{{ self.text("family") }}: {{ item.family }}</p><p>{{ self.text("production-key") }}: {{ item.production_key }}</p><p>{{ self.text("revision-ref") }}: {{ item.revision_ref }}</p><p>{{ self.text("minimum-value") }}: {{ item.minimum_value }}</p><p>{{ self.text("denied") }}: {% if item.denied %}{{ self.text("yes") }}{% else %}{{ self.text("no") }}{% endif %}</p></li>{% endfor %}</ul></section>
{% for form in forms %}<section><h2>{{ self.text(form.action) }}</h2><form method="post" class="op-panel op-stack"><input type="hidden" name="_csrf" value="{{ csrf }}"><input type="hidden" name="action" value="{{ form.action }}">{% if form.action != "approve" %}<input type="hidden" name="id" value="{{ form.id }}">{% endif %}{% for field in form.fields %}<label for="{{ form.action }}-{{ field.name }}">{{ self.text(field.label) }}</label><input class="op-input" id="{{ form.action }}-{{ field.name }}" name="{{ field.name }}" type="text" value="{{ field.value }}" maxlength="500" required>{% endfor %}<button type="submit" class="op-button">{{ self.text("submit") }}</button></form></section>{% endfor %}</main></body></html>"#,
    ext = "html"
)]
struct Page {
    locale: Locale,
    csrf: String,
    reports: Vec<moderation::PendingReport>,
    holds: Vec<String>,
    invalidations: [InvalidationSection; 2],
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
const INVALIDATION_PAGE_SIZE: usize = 100;

#[derive(Deserialize)]
struct InvalidationPage {
    pending_after: Option<Uuid>,
}
struct InvalidationSection {
    label: &'static str,
    rows: Vec<InvalidationRow>,
    next: Option<Uuid>,
}

async fn invalidation_sections(
    db: &mut sqlx::PgConnection,
    pending_after: Option<Uuid>,
) -> Result<[InvalidationSection; 2], sqlx::Error> {
    let mut pending: Vec<InvalidationRow> = sqlx::query_as(
        "SELECT request.id, listing.id AS listing, request.state AS request_state,
                listing.state::text AS listing_state
         FROM bazaar_invalidation_requests request
         JOIN bazaar_listings listing ON listing.id = request.listing_id
         WHERE request.state='pending' AND ($1::uuid IS NULL OR request.id > $1)
         ORDER BY request.id LIMIT $2",
    )
    .bind(pending_after)
    .bind(INVALIDATION_PAGE_SIZE as i64 + 1)
    .fetch_all(&mut *db)
    .await?;
    let next = if pending.len() > INVALIDATION_PAGE_SIZE {
        pending.truncate(INVALIDATION_PAGE_SIZE);
        pending.last().map(|row| row.id)
    } else {
        None
    };
    let approved = sqlx::query_as(
        "SELECT request.id, listing.id AS listing, request.state AS request_state,
                listing.state::text AS listing_state
         FROM bazaar_invalidation_requests request
         JOIN bazaar_listings listing ON listing.id = request.listing_id
         WHERE request.state='approved'
         ORDER BY request.updated_at DESC, request.id DESC LIMIT $1",
    )
    .bind(INVALIDATION_PAGE_SIZE as i64)
    .fetch_all(db)
    .await?;
    Ok([
        InvalidationSection {
            label: "invalidations",
            rows: pending,
            next,
        },
        InvalidationSection {
            label: "invalidation-history",
            rows: approved,
            next: None,
        },
    ])
}

#[derive(sqlx::FromRow)]
struct InvalidationRow {
    id: Uuid,
    listing: Uuid,
    request_state: String,
    listing_state: String,
}
impl InvalidationRow {
    fn listing_label(&self) -> String {
        format!("listing-state-{}", self.listing_state.replace('_', "-"))
    }

    fn request_label(&self) -> &str {
        match self.request_state.as_str() {
            "pending" => "invalidation-pending",
            "approved" => "invalidation-approved",
            _ => "invalidation-unknown",
        }
    }
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
    Query(page): Query<InvalidationPage>,
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
    let invalidations = invalidation_sections(&mut connection, page.pending_after)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
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
        form(
            "approve",
            &[("id", "identifier", ""), ("reason", "reason", "")],
        ),
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
        holds,
        invalidations,
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
