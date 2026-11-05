//! SSR forms behind the existing admin host, MFA, RBAC and session CSRF guard.
use super::*;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{
        Path, Query, State,
        rejection::{FormRejection, PathRejection},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use std::sync::Arc;

pub fn router<
    A: Authority,
    T: TotpVerifier,
    I: crate::web::StaffSession,
    R: rbac::AccountStateReader + 'static,
>(
    service: Controls<A, T>,
    config: crate::web::PanelConfig,
    identity: I,
    accounts: R,
) -> Router {
    if !service.enabled {
        return Router::new();
    }
    let pool = service.pool.clone();
    crate::web::guarded(
        routes(Arc::new(service)),
        pool,
        config,
        identity,
        accounts,
        rbac::Permission::Access,
    )
}
fn routes<A: Authority, T: TotpVerifier>(s: Arc<Controls<A, T>>) -> Router {
    Router::new()
        .route(
            "/admin/control/step-up",
            get(step_page::<A, T>).post(step_submit::<A, T>),
        )
        .route(
            "/admin/control/approvals/{id}",
            get(approval_page::<A, T>).post(approval_submit::<A, T>),
        )
        .with_state(s)
}
/// Catalog/RBAC authority remains fail-closed until wired by the application.
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let enabled = std::env::var("OTERYN_ADMIN_CONTROL").as_deref() == Ok("true");
    if !enabled {
        return Ok(Router::new());
    }
    let s = Arc::new(Controls::new(
        pool.clone(),
        Unavailable,
        oteryn_identity::mfa::Mfa::configured(pool.clone())?,
        enabled,
    ));
    crate::web::configured_guard(routes(s), pool, rbac::Permission::Access)
}
#[derive(Clone, Copy)]
struct Locale(&'static str);
impl Locale {
    fn from(h: &HeaderMap) -> Self {
        Self(
            if h.get("accept-language")
                .and_then(|s| s.to_str().ok())
                .is_some_and(|s| s.starts_with("en"))
            {
                "en"
            } else {
                "pl"
            },
        )
    }
    fn text(self, key: &str) -> String {
        let source = if self.0 == "en" { EN } else { PL };
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
const EN: &str = "step = Confirm with a new TOTP code\ncode = Six-digit authenticator code\nsubmit = Confirm\napproval = Review dry run and approve\nreview = Check the operation, reason and exact preview before approving. A different administrator must approve after their own step-up.\ndigest = Preview digest\noperation = Operation\nreason = Reason\nback = Administration\ndone = Confirmation recorded. Return to the original operation and submit it again.\ninvalid = Invalid input\ndenied = This action is refused\nunavailable = This operation is unavailable\nlimited = Too many attempts. Try again in one minute.\n";
const PL: &str = "step = Potwierdź nowym kodem TOTP\ncode = Sześciocyfrowy kod z aplikacji uwierzytelniającej\nsubmit = Potwierdź\napproval = Sprawdź podgląd i zatwierdź\nreview = Sprawdź operację, powód i dokładny podgląd. Zatwierdza inny administrator po własnym potwierdzeniu TOTP.\ndigest = Skrót podglądu\noperation = Operacja\nreason = Powód\nback = Administracja\ndone = Potwierdzenie zapisane. Wróć do pierwotnej operacji i wyślij ją ponownie.\ninvalid = Nieprawidłowe dane\ndenied = Odmowa wykonania operacji\nunavailable = Ta operacja jest niedostępna\nlimited = Zbyt wiele prób. Spróbuj ponownie za minutę.\n";
#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{{ title }}</title><link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><link rel=\"stylesheet\" href=\"/static/css/components.css\"></head><body class=\"op-theme\"><main class=\"op-panel op-stack\"><h1>{{ title }}</h1>{% if step %}<form class=\"op-stack\" method=\"post\"><input type=\"hidden\" name=\"_csrf\" value=\"{{ csrf }}\"><label for=\"code\">{{ code_label }}</label><input class=\"op-input\" id=\"code\" name=\"code\" type=\"text\" size=\"6\" inputmode=\"numeric\" autocomplete=\"one-time-code\" pattern=\"[0-9]{6}\" minlength=\"6\" maxlength=\"6\" required><button class=\"op-button\" type=\"submit\">{{ submit }}</button></form>{% else %}<p>{{ review }}</p><dl><dt>{{ operation_label }}</dt><dd>{{ operation }}</dd><dt>{{ reason_label }}</dt><dd>{{ reason }}</dd><dt>{{ digest_label }}</dt><dd><code>{% for part in digest_parts %}{{ part }}<wbr>{% endfor %}</code></dd></dl><details class=\"op-disclosure\" open><summary>{{ review }}</summary>{% for line in preview_lines %}<p>{{ line }}</p>{% endfor %}</details><form class=\"op-stack\" method=\"post\"><input type=\"hidden\" name=\"_csrf\" value=\"{{ csrf }}\"><button class=\"op-button\" type=\"submit\">{{ submit }}</button></form>{% endif %}<p><a href=\"/admin\">{{ back }}</a></p></main></body></html>",
    ext = "html"
)]
struct Page {
    lang: &'static str,
    title: String,
    step: bool,
    csrf: String,
    code_label: String,
    submit: String,
    review: String,
    operation_label: String,
    operation: String,
    reason_label: String,
    reason: String,
    digest_label: String,
    digest_parts: Vec<String>,
    preview_lines: Vec<String>,
    back: String,
}
impl Page {
    fn new(locale: Locale, csrf: String, step: bool) -> Self {
        Self {
            lang: locale.0,
            title: locale.text(if step { "step" } else { "approval" }),
            step,
            csrf,
            code_label: locale.text("code"),
            submit: locale.text("submit"),
            review: locale.text("review"),
            operation_label: locale.text("operation"),
            operation: String::new(),
            reason_label: locale.text("reason"),
            reason: String::new(),
            digest_label: locale.text("digest"),
            digest_parts: vec![],
            preview_lines: vec![],
            back: locale.text("back"),
        }
    }
}
async fn step_page<A: Authority, T: TotpVerifier>(
    State(s): State<Arc<Controls<A, T>>>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
) -> Response {
    let locale = Locale::from(&headers);
    if let Err(e) = s.authority.staff(&session).await {
        return refusal(locale, e);
    }
    match session.csrf_token().await {
        Ok(csrf) => oteryn_web::render(&Page::new(locale, csrf, true)),
        Err(_) => refusal(locale, Refusal::Unavailable),
    }
}
#[derive(Deserialize)]
struct Code {
    code: String,
}
#[derive(Deserialize)]
struct StepRequest {
    request_id: Uuid,
}
async fn step_submit<A: Authority, T: TotpVerifier>(
    State(s): State<Arc<Controls<A, T>>>,
    Extension(session): Extension<BrowserSession>,
    query: Result<Query<StepRequest>, axum::extract::rejection::QueryRejection>,
    headers: HeaderMap,
    form: Result<Form<Code>, FormRejection>,
) -> Response {
    let locale = Locale::from(&headers);
    let Ok(Form(form)) = form else {
        return refusal(locale, Refusal::InvalidInput);
    };
    let Ok(Query(query)) = query else {
        return refusal(locale, Refusal::InvalidInput);
    };
    match s
        .step_up_for_request(&session, query.request_id, &form.code)
        .await
    {
        Ok(_) => message(locale, StatusCode::OK, "done"),
        Err(e) => refusal(locale, e),
    }
}
async fn approval_page<A: Authority, T: TotpVerifier>(
    State(s): State<Arc<Controls<A, T>>>,
    Extension(session): Extension<BrowserSession>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> Response {
    let locale = Locale::from(&headers);
    let Ok(Path(id)) = path else {
        return refusal(locale, Refusal::InvalidInput);
    };
    let result = async {
        let p = s.preview(id).await?;
        let (actor, op) = s.authorized(&session, &p.operation).await?;
        if op.name != p.operation
            || op.permission != p.permission
            || op.risk != RiskClass::R4
            || actor.admin_id == p.requester
        {
            return Err(Refusal::Denied);
        }
        let mut page = Page::new(locale, session.csrf_token().await?, false);
        page.operation = p.operation;
        page.reason = format!(
            "{}: {} {}",
            p.reason.code,
            p.reason.text,
            p.reason.ticket_ref.unwrap_or_default()
        );
        page.digest_parts = p
            .preview_digest
            .as_bytes()
            .chunks(8)
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect();
        page.preview_lines = serde_json::to_string_pretty(&p.preview)
            .map_err(|_| Refusal::Unavailable)?
            .lines()
            .map(str::to_owned)
            .collect();
        Ok(page)
    }
    .await;
    match result {
        Ok(page) => oteryn_web::render(&page),
        Err(e) => refusal(locale, e),
    }
}
async fn approval_submit<A: Authority, T: TotpVerifier>(
    State(s): State<Arc<Controls<A, T>>>,
    Extension(session): Extension<BrowserSession>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> Response {
    let locale = Locale::from(&headers);
    let Ok(Path(id)) = path else {
        return refusal(locale, Refusal::InvalidInput);
    };
    match s.approve_request(&session, id).await {
        Ok(_) => message(locale, StatusCode::OK, "done"),
        Err(e) => refusal_for_request(&headers, e, id),
    }
}
/// Preserve the target request while obtaining a new proof for a submission.
pub fn refusal_for_request(headers: &HeaderMap, error: Refusal, request_id: Uuid) -> Response {
    if error == Refusal::StepUpRequired {
        return Redirect::to(&format!("/admin/control/step-up?request_id={request_id}"))
            .into_response();
    }
    refusal_response(headers, error)
}
/// Submission handlers reuse this mapping for typed policy refusals.
pub fn refusal_response(headers: &HeaderMap, error: Refusal) -> Response {
    refusal(Locale::from(headers), error)
}
fn refusal(locale: Locale, e: Refusal) -> Response {
    if e == Refusal::StepUpRequired {
        return Redirect::to("/admin/control/step-up").into_response();
    }
    let (status, key) = match e {
        Refusal::Disabled => (StatusCode::NOT_FOUND, "unavailable"),
        Refusal::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
        Refusal::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "limited"),
        Refusal::InvalidInput => (StatusCode::UNPROCESSABLE_ENTITY, "invalid"),
        _ => (StatusCode::FORBIDDEN, "denied"),
    };
    message(locale, status, key)
}
fn message(locale: Locale, status: StatusCode, key: &str) -> Response {
    (status,axum::response::Html(format!("<!doctype html><html lang=\"{}\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><main><h1>{}</h1><a href=\"/admin\">{}</a></main></html>",locale.0,locale.text(key),locale.text("back")))).into_response()
}
