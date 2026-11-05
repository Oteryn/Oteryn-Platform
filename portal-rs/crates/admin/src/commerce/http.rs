use super::*;
use crate::web::{self, PanelConfig, StaffSession};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{
        Path, State,
        rejection::{FormRejection, PathRejection},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_web::session::BrowserSession;
use std::sync::Arc;

/// The application injects trusted session and domain authority adapters here.
pub fn router<I, A, E>(
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
    effects: E,
    flags: Flags,
) -> Router
where
    I: StaffSession,
    A: rbac::AccountStateReader + Clone + 'static,
    E: Effects + 'static,
{
    if !flags.enabled {
        return Router::new();
    }
    let service = Arc::new(Commerce::new(
        pool.clone(),
        accounts.clone(),
        effects,
        flags,
    ));
    web::guarded(
        routes(service),
        pool,
        config,
        identity,
        accounts,
        rbac::Permission::Access,
    )
}
fn routes<A: rbac::AccountStateReader + 'static, E: Effects + 'static>(
    service: Arc<Commerce<A, E>>,
) -> Router {
    Router::new()
        .route("/admin/commerce", get(page::<A, E>))
        .route("/admin/commerce/{action}", post(submit::<A, E>))
        .route("/admin/commerce/approvals/{id}", post(approve::<A, E>))
        .with_state(service)
}
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let flags = Flags {
        enabled: enabled("OTERYN_ADMIN_COMMERCE"),
        premium: enabled("OTERYN_PREMIUM_ENABLED"),
        wallet: enabled("OTERYN_WALLET_ENABLED"),
        payments: enabled("OTERYN_PAYMENTS_ENABLED"),
        refunds: enabled("OTERYN_REFUNDS_ENABLED"),
    };
    if !flags.enabled {
        return Ok(Router::new());
    }
    let service = Arc::new(Commerce::new(
        pool.clone(),
        web::PendingAuthentication,
        PendingEffects,
        flags,
    ));
    web::configured_guard(routes(service), pool, rbac::Permission::Access)
}
fn enabled(name: &str) -> bool {
    std::env::var(name).as_deref() == Ok("true")
}
#[derive(Clone, Copy)]
struct Locale(&'static str);
impl Locale {
    fn from_headers(headers: &HeaderMap) -> Self {
        Self(
            if headers
                .get("accept-language")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|s| s.starts_with("en"))
            {
                "en"
            } else {
                "pl"
            },
        )
    }
    fn text(self, key: &str) -> String {
        let source = if self.0 == "en" {
            include_str!("en.ftl")
        } else {
            include_str!("pl.ftl")
        };
        let Ok(locale) = self.0.parse() else {
            return String::new();
        };
        let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) else {
            return String::new();
        };
        let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
        if bundle.add_resource(resource).is_err() {
            return String::new();
        }
        bundle
            .get_message(key)
            .and_then(|m| m.value())
            .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
            .unwrap_or_default()
    }
}
struct Action {
    key: &'static str,
    title: String,
    days: bool,
    amount: bool,
    request: Uuid,
}
struct Pending {
    id: Uuid,
    target: Uuid,
    actor: Uuid,
    command: String,
    amount: String,
    can_approve: bool,
}
#[derive(Template)]
#[template(path = "commerce/page.html", config = "src/commerce/askama.toml")]
struct Page {
    lang: &'static str,
    title: String,
    note: String,
    account: String,
    request: String,
    reason: String,
    days: String,
    amount: String,
    submit: String,
    pending_label: String,
    approvals: String,
    empty: String,
    approve: String,
    csrf: String,
    actions: Vec<Action>,
    pending: Vec<Pending>,
}
async fn page<A: rbac::AccountStateReader, E: Effects>(
    State(s): State<Arc<Commerce<A, E>>>,
    Extension(context): Extension<rbac::CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
) -> Response {
    let locale = Locale::from_headers(&headers);
    let mut actions = vec![];
    for (key, command) in [
        ("grant", Command::Grant { days: 1 }),
        ("revoke", Command::Revoke),
        ("adjust", Command::Adjust { amount: 1 }),
        ("reconcile", Command::Reconcile),
        ("refund", Command::Refund),
    ] {
        if command.validate(s.flags).is_err() {
            continue;
        }
        match s.authorize(context, &command).await {
            Ok(()) => actions.push(Action {
                key,
                title: locale.text(key),
                days: key == "grant",
                amount: key == "adjust",
                request: Uuid::now_v7(),
            }),
            Err(Error::Denied) => (),
            Err(error) => return failure(locale, error),
        }
    }
    // Apply permission predicates in SQL before exposing any queue entries.
    let rows = sqlx::query("SELECT id,target_id,actor_id,command,review FROM commerce_operations WHERE result IS NULL AND ((command->>'kind'='adjust' AND $1) OR (command->>'kind'='refund' AND $2)) ORDER BY created_at,id LIMIT 100")
        .bind(actions.iter().any(|a| a.key=="adjust")).bind(actions.iter().any(|a| a.key=="refund")).fetch_all(&s.pool).await;
    let pending = match rows.and_then(|rows| {
        rows.into_iter()
            .map(|r| {
                let command: Value = r.try_get("command")?;
                let kind = command
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let label = if let Some(amount) = command.get("amount").and_then(Value::as_i64) {
                    format!("{}: {amount}", locale.text(kind))
                } else {
                    locale.text(kind)
                };
                Ok(Pending {
                    id: r.try_get("id")?,
                    target: r.try_get("target_id")?,
                    actor: r.try_get("actor_id")?,
                    command: label,
                    can_approve: r.try_get::<Uuid, _>("actor_id")? != context.account,
                    amount: {
                        let review: Value = r.try_get("review")?;
                        format!(
                            "{} {}",
                            review
                                .get("amount")
                                .and_then(Value::as_i64)
                                .map(|n| n.to_string())
                                .unwrap_or_default(),
                            review
                                .get("currency")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                        )
                    },
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
    }) {
        Ok(value) => value,
        Err(e) => return failure(locale, Error::Storage(e)),
    };
    let csrf = match session.csrf_token().await {
        Ok(v) => v,
        Err(_) => return failure(locale, Error::Unavailable),
    };
    oteryn_web::render(&Page {
        lang: locale.0,
        title: locale.text("title"),
        note: locale.text("note"),
        account: locale.text("account"),
        request: locale.text("request"),
        reason: locale.text("reason"),
        days: locale.text("days"),
        amount: locale.text("amount"),
        submit: locale.text("submit"),
        pending_label: locale.text("pending"),
        approvals: locale.text("approvals"),
        empty: locale.text("empty"),
        approve: locale.text("approve"),
        csrf,
        actions,
        pending,
    })
}
#[derive(Deserialize)]
struct Input {
    request_id: Uuid,
    target: Uuid,
    reason: String,
    days: Option<u16>,
    amount: Option<i64>,
}
async fn submit<A: rbac::AccountStateReader, E: Effects>(
    State(s): State<Arc<Commerce<A, E>>>,
    Extension(context): Extension<rbac::CheckContext>,
    Path(action): Path<String>,
    headers: HeaderMap,
    form: Result<Form<Input>, FormRejection>,
) -> Response {
    let locale = Locale::from_headers(&headers);
    let Ok(Form(form)) = form else {
        return failure(locale, Error::Invalid);
    };
    let command = match (action.as_str(), form.days, form.amount) {
        ("grant", Some(days), None) => Command::Grant { days },
        ("revoke", None, None) => Command::Revoke,
        ("adjust", None, Some(amount)) => Command::Adjust { amount },
        ("reconcile", None, None) => Command::Reconcile,
        ("refund", None, None) => Command::Refund,
        _ => return failure(locale, Error::Invalid),
    };
    match s
        .submit(
            context,
            Request {
                request_id: form.request_id,
                target: form.target,
                reason: form.reason,
                command,
            },
        )
        .await
    {
        Ok(_) => Redirect::to("/admin/commerce").into_response(),
        Err(e) => failure(locale, e),
    }
}
async fn approve<A: rbac::AccountStateReader, E: Effects>(
    State(s): State<Arc<Commerce<A, E>>>,
    Extension(context): Extension<rbac::CheckContext>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> Response {
    let Ok(Path(id)) = path else {
        return failure(Locale::from_headers(&headers), Error::Invalid);
    };
    match s.approve(context, id).await {
        Ok(_) => Redirect::to("/admin/commerce").into_response(),
        Err(e) => failure(Locale::from_headers(&headers), e),
    }
}
fn failure(locale: Locale, error: Error) -> Response {
    let (status, key) = match error {
        Error::Invalid => (StatusCode::UNPROCESSABLE_ENTITY, "invalid"),
        Error::Denied => (StatusCode::FORBIDDEN, "denied"),
        Error::Conflict => (StatusCode::CONFLICT, "conflict"),
        Error::Disabled => (StatusCode::NOT_FOUND, "unavailable"),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
    };
    // Only repository-owned Fluent resources are interpolated.
    (status,axum::response::Html(format!("<!doctype html><html lang=\"{}\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><main><h1>{}</h1><a href=\"/admin/commerce\">{}</a></main></html>",locale.0,locale.text(key),locale.text("back")))).into_response()
}
