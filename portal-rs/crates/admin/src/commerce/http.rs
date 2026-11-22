use super::{
    audit_change::{CommerceAudit, ValueDecision},
    *,
};
use crate::{
    staff_actions::{self, Action as StaffAction, Guard, StepUp},
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

pub(super) type StaffMutation = (
    Extension<rbac::CheckContext>,
    Extension<rbac::MutationAuthorization>,
    Option<Extension<Verified>>,
);

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
    refund: bool,
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
    refund_requested: String,
    value_verified: String,
    value_withdrawn: String,
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
                refund: key == "refund",
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
        refund_requested: oteryn_ui::text(locale.0, "commerce-request-refund-reason"),
        value_verified: oteryn_ui::text(locale.0, "commerce-value-approve-reason"),
        value_withdrawn: oteryn_ui::text(locale.0, "commerce-value-cancel-reason"),
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
async fn submit<A: rbac::AccountStateReader + 'static, E: Effects + 'static>(
    State(s): State<Arc<Commerce<A, E>>>,
    (Extension(context), Extension(authority), csrf): StaffMutation,
    chrome: Option<Extension<Chrome>>,
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
    let request = Request {
        request_id: form.request_id,
        target: form.target,
        reason: form.reason,
        command,
    };
    if request.command == Command::Refund {
        let chrome = decision_chrome(chrome, locale);
        let guard = Guard {
            actor: context,
            authority: &authority,
            csrf: csrf.map(|value| value.0),
        };
        return match request_refund(&s, guard, request).await {
            Ok(()) => Redirect::to("/admin/commerce").into_response(),
            Err(error) => error.response(&chrome),
        };
    }
    match s.submit(context, request).await {
        Ok(_) => Redirect::to("/admin/commerce").into_response(),
        Err(error) => failure(locale, error),
    }
}

async fn request_refund<A: rbac::AccountStateReader + 'static, E: Effects + 'static>(
    service: &Arc<Commerce<A, E>>,
    guard: Guard<'_>,
    mut request: Request,
) -> Result<(), staff_actions::Error> {
    const REASON: &str = "refund_requested";
    let reason = std::mem::take(&mut request.reason);
    let actor = guard.actor;
    let pool = &service.pool;
    if !valid_id(request.request_id) || !valid_id(request.target) {
        return Err(staff_actions::Error::Invalid);
    }
    service
        .authorize(actor, &request.command)
        .await
        .map_err(approval_error)?;
    let owner = payment_owner(
        &mut *pool.acquire().await?,
        &request.command,
        request.target,
    )
    .await
    .map_err(approval_error)?;
    let service = service.clone();
    staff_actions::run(
        pool,
        guard,
        StaffAction {
            permission: rbac::Permission::ReconcilePayments,
            reason: &reason,
            reason_codes: &[REASON],
            step_up: StepUp::SessionMfa,
        },
        || {
            request
                .command
                .validate(service.flags)
                .map_err(approval_error)?;
            if owner == actor.account {
                return Err(staff_actions::Error::Forbidden);
            }
            Ok((vec![owner], request))
        },
        {
            let service = service.clone();
            move |conn, request| {
                Box::pin(async move {
                    let mut tx = conn.begin().await?;
                    rbac::mutation_budget(&mut tx).await?;
                    let hash = format!("{:x}", Sha256::digest(REASON.as_bytes()));
                    let submission = service
                        .submit_locked(&mut tx, actor, &request, &hash)
                        .await
                        .map_err(approval_error)?;
                    let audit = match submission {
                        Submission::Replay { .. } => None,
                        Submission::Created { review, .. } => Some(
                            CommerceAudit::RefundRequest {
                                id: request.request_id,
                                intent: request.target,
                                amount: refund_amount(&review)?,
                            }
                            .into(),
                        ),
                    };
                    tx.commit().await?;
                    Ok(audit)
                })
            }
        },
    )
    .await
}

#[derive(Deserialize)]
struct ApprovalForm {
    #[serde(default)]
    reason: String,
}

async fn approve<A: rbac::AccountStateReader + 'static, E: Effects + 'static>(
    State(s): State<Arc<Commerce<A, E>>>,
    (Extension(context), Extension(authority), csrf): StaffMutation,
    chrome: Option<Extension<Chrome>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    form: Result<Form<ApprovalForm>, FormRejection>,
) -> Response {
    decide(
        s,
        Guard {
            actor: context,
            authority: &authority,
            csrf: csrf.map(|value| value.0),
        },
        chrome,
        path,
        headers,
        form,
        false,
    )
    .await
}

async fn decide<A: rbac::AccountStateReader + 'static, E: Effects + 'static>(
    service: Arc<Commerce<A, E>>,
    guard: Guard<'_>,
    chrome: Option<Extension<Chrome>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    form: Result<Form<ApprovalForm>, FormRejection>,
    cancel: bool,
) -> Response {
    let context = guard.actor;
    let locale = Locale::from_headers(&headers);
    let Ok(Path(id)) = path else {
        return failure(locale, Error::Invalid);
    };
    let target =
        match async { approval_target(&mut *service.pool.acquire().await?, id).await }.await {
            Ok(target) => target,
            Err(error) => return failure(locale, error),
        };
    let Ok(Form(form)) = form else {
        return failure(locale, Error::Invalid);
    };
    let chrome = decision_chrome(chrome, locale);
    let reason_codes: &[_] = match (&target.command, cancel) {
        (Command::Refund, false) => &["refund_verified"],
        (Command::Refund, true) => &["refund_withdrawn"],
        (Command::Grant { .. } | Command::Adjust { .. }, false) => &["value_verified"],
        (Command::Grant { .. } | Command::Adjust { .. }, true) => &["value_withdrawn"],
        _ => return failure(locale, Error::Denied),
    };
    let pool = service.pool.clone();
    let flags = service.flags;
    let result = staff_actions::run(
        &pool,
        guard,
        StaffAction {
            permission: target.command.permission(),
            reason: &form.reason,
            reason_codes,
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
                        .map(|request| cancellation_audit(request, proposer))
                        .transpose()?
                } else {
                    service
                        .approve_locked(&mut tx, context, id, owner)
                        .await
                        .map_err(approval_error)?
                        .map(|(proposer, request, _, result)| {
                            approval_audit(request, proposer, &result)
                        })
                        .transpose()?
                };
                tx.commit().await?;
                Ok(audit.map(Into::into))
            })
        },
    )
    .await;
    match result {
        Ok(()) => Redirect::to("/admin/commerce").into_response(),
        Err(error) => error.response(&chrome),
    }
}

fn decision_chrome(chrome: Option<Extension<Chrome>>, locale: Locale) -> Chrome {
    chrome.map(|value| value.0).unwrap_or_else(|| Chrome {
        lang: locale.0.to_owned(),
        ..Chrome::default()
    })
}

fn refund_amount(result: &Value) -> Result<Amount, staff_actions::Error> {
    let missing = staff_actions::Error::Unavailable;
    let amount = Amount {
        minor_units: result
            .get("amount")
            .and_then(Value::as_i64)
            .ok_or(missing)?,
        currency: result
            .get("currency")
            .and_then(Value::as_str)
            .ok_or(staff_actions::Error::Unavailable)?
            .to_owned(),
    };
    amount
        .validate()
        .map_err(|_| staff_actions::Error::Unavailable)?;
    Ok(amount)
}

fn approval_audit(
    request: Request,
    proposer: Uuid,
    result: &Value,
) -> Result<CommerceAudit, staff_actions::Error> {
    let Request {
        request_id: id,
        target,
        command,
        ..
    } = request;
    let read_id = |key: &str| {
        result
            .get(key)
            .and_then(Value::as_str)
            .and_then(|value| value.parse().ok())
            .ok_or(staff_actions::Error::Unavailable)
    };
    let change = match command {
        Command::Refund => {
            return Ok(CommerceAudit::RefundApproval {
                id,
                intent: target,
                proposer,
                amount: refund_amount(result)?,
                queued: result
                    .get("queued")
                    .and_then(Value::as_bool)
                    .ok_or(staff_actions::Error::Unavailable)?,
            });
        }
        Command::Adjust { amount } => ValueDecision::WalletApproved {
            coins: amount,
            ledger: read_id("operation")?,
        },
        Command::Grant { days } => ValueDecision::PremiumApproved {
            days,
            entitlement: read_id("entitlement")?,
            revision: result
                .get("revision")
                .and_then(Value::as_i64)
                .ok_or(staff_actions::Error::Unavailable)?,
        },
        _ => return Err(staff_actions::Error::Forbidden),
    };
    Ok(CommerceAudit::ValueDecision {
        id,
        account: target,
        proposer,
        change,
    })
}

fn cancellation_audit(
    request: Request,
    proposer: Uuid,
) -> Result<CommerceAudit, staff_actions::Error> {
    let Request {
        request_id: id,
        target,
        command,
        ..
    } = request;
    let change = match command {
        Command::Refund => {
            return Ok(CommerceAudit::RefundCancellation {
                id,
                intent: target,
                proposer,
            });
        }
        Command::Adjust { amount } => ValueDecision::WalletCancelled { coins: amount },
        Command::Grant { days } => ValueDecision::PremiumCancelled { days },
        _ => return Err(staff_actions::Error::Forbidden),
    };
    Ok(CommerceAudit::ValueDecision {
        id,
        account: target,
        proposer,
        change,
    })
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
    (Extension(context), Extension(authority), csrf): StaffMutation,
    chrome: Option<Extension<Chrome>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    form: Result<Form<ApprovalForm>, FormRejection>,
) -> Response {
    decide(
        service,
        Guard {
            actor: context,
            authority: &authority,
            csrf: csrf.map(|value| value.0),
        },
        chrome,
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
