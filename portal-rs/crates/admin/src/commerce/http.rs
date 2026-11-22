use super::*;
use crate::{
    staff_actions::{self, Action as StaffAction, AuditChange, Guard, StepUp},
    web::{self, PanelConfig, StaffSession},
};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{
        Path, Query, Request as HttpRequest, State,
        rejection::{FormRejection, PathRejection},
    },
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_payments::Amount;
use oteryn_ui::Chrome;
use oteryn_web::csrf::Verified;
use oteryn_web::session::BrowserSession;
use sqlx::Connection;
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
    let legacy_review = if service.flags.payments {
        Router::new()
            .route(
                "/admin/commerce/reconcile",
                get(panels::payments::review).post(panels::payments::record_review),
            )
            .with_state(service.pool.clone())
            .route_layer(middleware::from_fn_with_state(
                service.clone(),
                require_review::<A, E>,
            ))
    } else {
        Router::new()
    };
    Router::new()
        .route("/admin/commerce", get(page::<A, E>))
        .route("/admin/commerce/{action}", post(submit::<A, E>))
        .route("/admin/commerce/approvals/{id}", post(approve::<A, E>))
        .route("/admin/commerce/cancellations/{id}", post(cancel::<A, E>))
        .with_state(service.clone())
        .merge(legacy_review)
        .route_layer(middleware::from_fn_with_state(
            service,
            authorize_panel::<A, E>,
        ))
}

async fn require_review<A: rbac::AccountStateReader, E: Effects>(
    State(service): State<Arc<Commerce<A, E>>>,
    Extension(context): Extension<rbac::CheckContext>,
    request: HttpRequest,
    next: Next,
) -> Response {
    match service.authorize(context, &Command::Reconcile).await {
        Ok(()) => next.run(request).await,
        Err(error) => failure(Locale::from_headers(request.headers()), error),
    }
}

async fn authorize_panel<A: rbac::AccountStateReader, E: Effects>(
    State(s): State<Arc<Commerce<A, E>>>,
    Extension(context): Extension<rbac::CheckContext>,
    request: HttpRequest,
    next: Next,
) -> Response {
    let locale = Locale::from_headers(request.headers());
    for permission in [
        rbac::Permission::ManagePremiumTime,
        rbac::Permission::ManageMarketplace,
        rbac::Permission::ReconcilePayments,
    ] {
        match s.auth.check(context, permission).await {
            Ok(()) => return next.run(request).await,
            Err(rbac::Error::Denied) => (),
            Err(error) => return failure(locale, error.into()),
        }
    }
    failure(locale, Error::Denied)
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
    web::configured_guard(
        configured_routes(
            pool.clone(),
            flags,
            configured_payment_adapter(pool.clone())?,
        ),
        pool,
        rbac::Permission::Access,
    )
}
pub(super) fn configured_routes(
    pool: PgPool,
    flags: Flags,
    payments: Option<TestPaymentAdapter>,
) -> Router {
    let service = Arc::new(Commerce::new(
        pool.clone(),
        web::PortalIdentity::configured(pool.clone()),
        domain_effects(pool, flags, payments),
        flags,
    ));
    routes(service)
}

fn enabled(name: &str) -> bool {
    std::env::var(name).as_deref() == Ok("true")
}
#[derive(Clone, Copy)]
pub(super) struct Locale(pub(super) &'static str);
impl Locale {
    pub(super) fn from_headers(headers: &HeaderMap) -> Self {
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
    adjustment_amount: Option<i64>,
    refund_amount: Option<String>,
    can_approve: bool,
    refund: bool,
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
    refund_amount: String,
    submit: String,
    pending_label: String,
    approvals: String,
    empty: String,
    approve: String,
    cancel: String,
    next_label: String,
    next: Option<Uuid>,
    csrf: String,
    actions: Vec<Action>,
    pending: Vec<Pending>,
    review_link: Option<String>,
    refund_verified: String,
    refund_withdrawn: String,
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueueFilter {
    after: Option<Uuid>,
    // Site-wide locale switch parameter carried by admin navigation links.
    #[serde(rename = "lang")]
    _lang: Option<String>,
}
pub(super) fn format_refund_amount(minor_units: i64, currency: &str) -> String {
    // ISO 4217 currencies whose minor units differ from the usual hundredths.
    let exponent = match currency {
        "BIF" | "CLP" | "DJF" | "GNF" | "ISK" | "JPY" | "KMF" | "KRW" | "PYG" | "RWF" | "UGX"
        | "UYI" | "VND" | "VUV" | "XAF" | "XOF" | "XPF" => 0,
        "BHD" | "IQD" | "JOD" | "KWD" | "LYD" | "OMR" | "TND" => 3,
        "CLF" | "UYW" => 4,
        _ => 2,
    };
    oteryn_payments::coin::Price {
        minor_units,
        currency: currency.to_owned(),
        exponent,
    }
    .display()
}

async fn page<A: rbac::AccountStateReader, E: Effects>(
    State(s): State<Arc<Commerce<A, E>>>,
    Extension(context): Extension<rbac::CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    Query(filter): Query<QueueFilter>,
) -> Response {
    let locale = Locale::from_headers(&headers);
    let mut actions = vec![];
    for (key, command) in [
        ("grant", Command::Grant { days: 1 }),
        ("revoke", Command::Revoke),
        ("adjust", Command::Adjust { amount: 1 }),
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
    let review_link = if s.flags.payments {
        match s.authorize(context, &Command::Reconcile).await {
            Ok(()) => Some(locale.text("reconcile")),
            Err(Error::Denied) => None,
            Err(error) => return failure(locale, error),
        }
    } else {
        None
    };
    // Apply permission predicates in SQL before exposing any queue entries.
    let rows = sqlx::query("SELECT id,target_id,actor_id,command,review FROM commerce_operations WHERE result IS NULL AND ($3::uuid IS NULL OR (created_at,id)>(SELECT created_at,id FROM commerce_operations WHERE id=$3)) AND ((command->>'kind'='adjust' AND $1) OR (command->>'kind'='refund' AND $2) OR (command->>'kind'='grant' AND $4)) ORDER BY created_at,id LIMIT 101")
        .bind(actions.iter().any(|a| a.key=="adjust")).bind(actions.iter().any(|a| a.key=="refund")).bind(filter.after).bind(actions.iter().any(|a| a.key=="grant")).fetch_all(&s.pool).await;
    let mut pending = match rows.and_then(|rows| {
        rows.into_iter()
            .map(|r| {
                let command: Value = r.try_get("command")?;
                let kind = command
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let label = if let Some(amount) = command
                    .get("amount")
                    .or_else(|| command.get("days"))
                    .and_then(Value::as_i64)
                {
                    format!("{}: {amount}", locale.text(kind))
                } else {
                    locale.text(kind)
                };
                let review: Value = r.try_get("review")?;
                let amount = review.get("amount").and_then(Value::as_i64);
                let currency = review.get("currency").and_then(Value::as_str);
                Ok(Pending {
                    id: r.try_get("id")?,
                    target: r.try_get("target_id")?,
                    actor: r.try_get("actor_id")?,
                    command: label,
                    can_approve: r.try_get::<Uuid, _>("actor_id")? != context.account,
                    refund: kind == "refund",
                    adjustment_amount: amount.filter(|_| kind == "adjust"),
                    refund_amount: amount
                        .filter(|_| kind == "refund")
                        .zip(currency)
                        .map(|(amount, currency)| format_refund_amount(amount, currency)),
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
    }) {
        Ok(value) => value,
        Err(e) => return failure(locale, Error::Storage(e)),
    };
    let next = if pending.len() > 100 {
        pending.truncate(100);
        pending.last().map(|row| row.id)
    } else {
        None
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
        refund_amount: locale.text("refund-amount"),
        submit: locale.text("submit"),
        pending_label: locale.text("pending"),
        approvals: locale.text("approvals"),
        empty: locale.text("empty"),
        approve: locale.text("approve"),
        cancel: locale.text("cancel"),
        next_label: locale.text("next"),
        next,
        csrf,
        actions,
        pending,
        review_link,
        refund_verified: oteryn_ui::text(locale.0, "commerce-confirm-approve"),
        refund_withdrawn: oteryn_ui::text(locale.0, "commerce-confirm-cancel"),
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
#[derive(Deserialize)]
struct ApprovalForm {
    #[serde(default)]
    reason: String,
}

async fn approve<A: rbac::AccountStateReader + 'static, E: Effects + 'static>(
    State(s): State<Arc<Commerce<A, E>>>,
    Extension(context): Extension<rbac::CheckContext>,
    Extension(authority): Extension<rbac::MutationAuthorization>,
    chrome: Option<Extension<Chrome>>,
    csrf: Option<Extension<Verified>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    form: Result<Form<ApprovalForm>, FormRejection>,
) -> Response {
    decide(
        s,
        context,
        authority,
        chrome,
        csrf.map(|value| value.0),
        path,
        headers,
        form,
        false,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn decide<A: rbac::AccountStateReader + 'static, E: Effects + 'static>(
    service: Arc<Commerce<A, E>>,
    context: rbac::CheckContext,
    authority: rbac::MutationAuthorization,
    chrome: Option<Extension<Chrome>>,
    csrf: Option<Verified>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    form: Result<Form<ApprovalForm>, FormRejection>,
    cancel: bool,
) -> Response {
    let locale = Locale::from_headers(&headers);
    let Ok(Path(id)) = path else {
        return failure(locale, Error::Invalid);
    };
    let target =
        match async { approval_target(&mut *service.pool.acquire().await?, id).await }.await {
            Ok(target) => target,
            Err(error) => return failure(locale, error),
        };
    if !matches!(target.command, Command::Refund) {
        let result = if cancel {
            service.cancel(context, id).await
        } else {
            service.approve(context, id).await.map(|_| ())
        };
        return match result {
            Ok(()) => Redirect::to("/admin/commerce").into_response(),
            Err(error) => failure(locale, error),
        };
    }
    let Ok(Form(form)) = form else {
        return failure(locale, Error::Invalid);
    };
    let chrome = chrome.map(|value| value.0).unwrap_or_else(|| Chrome {
        lang: locale.0.to_owned(),
        ..Chrome::default()
    });
    let pool = service.pool.clone();
    let flags = service.flags;
    let result = staff_actions::run(
        &pool,
        Guard {
            actor: context,
            authority: &authority,
            csrf,
        },
        StaffAction {
            permission: rbac::Permission::ReconcilePayments,
            reason: &form.reason,
            reason_codes: if cancel {
                &["refund_withdrawn"]
            } else {
                &["refund_verified"]
            },
            step_up: StepUp::SessionMfa,
        },
        || {
            target.command.validate(flags).map_err(approval_error)?;
            Ok((
                vec![target.original, target.owner],
                (target.original, target.owner),
            ))
        },
        move |conn, (proposer, owner)| {
            Box::pin(async move {
                // Re-arm the outer transaction's deadline before the ledger's
                // savepoint acquires additional domain locks.
                let mut tx = conn.begin().await?;
                rbac::mutation_budget(&mut tx).await?;
                let audit = if cancel {
                    service
                        .cancel_locked(&mut tx, context, id, proposer, owner)
                        .await
                        .map_err(approval_error)?
                        .map(|request| AuditChange::RefundCancellation {
                            id,
                            intent: request.target,
                            proposer,
                        })
                } else if let Some((proposer, request, _, result)) = service
                    .approve_locked(&mut tx, context, id, owner)
                    .await
                    .map_err(approval_error)?
                {
                    let amount = Amount {
                        minor_units: result
                            .get("amount")
                            .and_then(Value::as_i64)
                            .ok_or(staff_actions::Error::Unavailable)?,
                        currency: result
                            .get("currency")
                            .and_then(Value::as_str)
                            .ok_or(staff_actions::Error::Unavailable)?
                            .to_owned(),
                    };
                    amount
                        .validate()
                        .map_err(|_| staff_actions::Error::Unavailable)?;
                    let queued = result
                        .get("queued")
                        .and_then(Value::as_bool)
                        .ok_or(staff_actions::Error::Unavailable)?;
                    Some(AuditChange::RefundApproval {
                        id,
                        intent: request.target,
                        proposer,
                        amount,
                        queued,
                    })
                } else {
                    None
                };
                tx.commit().await?;
                Ok(audit)
            })
        },
    )
    .await;
    match result {
        Ok(()) => Redirect::to("/admin/commerce").into_response(),
        Err(error) => error.response(&chrome),
    }
}

fn approval_error(error: Error) -> staff_actions::Error {
    match error {
        Error::Denied => staff_actions::Error::Forbidden,
        Error::Invalid => staff_actions::Error::Invalid,
        Error::Conflict => staff_actions::Error::Conflict,
        Error::Disabled | Error::Unavailable | Error::Storage(_) => {
            staff_actions::Error::Unavailable
        }
    }
}
// Cancellation shares the approval authority and locking order, but never applies
// a domain effect. A second authorized reviewer can retire obsolete intents even
// after the original requester has lost authority.
impl<A: rbac::AccountStateReader, E: Effects> Commerce<A, E> {
    async fn cancel(&self, context: rbac::CheckContext, id: Uuid) -> Result<(), Error> {
        if !self.flags.enabled {
            return Err(Error::Disabled);
        }
        if !valid_id(id) {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        crate::rbac::mutation_budget(&mut tx).await?;
        let (original, target, saved): (Uuid, Uuid, Value) = sqlx::query_as(
            "SELECT actor_id,target_id,command FROM commerce_operations WHERE id=$1",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(Error::Invalid)?;
        let saved: Command = serde_json::from_value(saved).map_err(|_| Error::Unavailable)?;
        let owner = payment_owner(&mut tx, &saved, target).await?;
        rbac::lock_accounts_and_mutation(&mut tx, &[context.account, original, owner]).await?;
        if let Some(request) = self
            .cancel_locked(&mut tx, context, id, original, owner)
            .await?
        {
            record(
                &mut tx,
                context.account,
                &request,
                "commerce.cancelled",
                json!({"pending":true}),
                json!({"requested_by":original,"pending":false,"status":"cancelled"}),
            )
            .await?;
            self.authorize_in(&mut tx, context, &request.command)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn cancel_locked(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        context: rbac::CheckContext,
        id: Uuid,
        original: Uuid,
        owner: Uuid,
    ) -> Result<Option<Request>, Error> {
        let row = sqlx::query("SELECT target_id,command,requires_approval,result,approved_by FROM commerce_operations WHERE id=$1 FOR UPDATE")
            .bind(id).fetch_one(&mut **tx).await?;
        let command: Command =
            serde_json::from_value(row.try_get("command")?).map_err(|_| Error::Unavailable)?;
        command.validate(self.flags)?;
        self.authorize_in(tx, context, &command).await?;
        if original == context.account
            || owner == context.account
            || !row.try_get::<bool, _>("requires_approval")?
        {
            return Err(Error::Denied);
        }
        let cancelled = json!({"status":"cancelled"});
        if let Some(result) = row.try_get::<Option<Value>, _>("result")? {
            if result != cancelled
                || row.try_get::<Option<Uuid>, _>("approved_by")? != Some(context.account)
            {
                return Err(Error::Conflict);
            }
            return Ok(None);
        }
        complete(tx, id, Some(context.account), cancelled).await?;
        Ok(Some(Request {
            request_id: id,
            target: row.try_get("target_id")?,
            reason: String::new(),
            command,
        }))
    }
}
async fn cancel<A: rbac::AccountStateReader + 'static, E: Effects + 'static>(
    State(service): State<Arc<Commerce<A, E>>>,
    Extension(context): Extension<rbac::CheckContext>,
    Extension(authority): Extension<rbac::MutationAuthorization>,
    chrome: Option<Extension<Chrome>>,
    csrf: Option<Extension<Verified>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    form: Result<Form<ApprovalForm>, FormRejection>,
) -> Response {
    decide(
        service,
        context,
        authority,
        chrome,
        csrf.map(|value| value.0),
        path,
        headers,
        form,
        true,
    )
    .await
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
