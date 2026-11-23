use super::*;
use crate::web::{self, PortalIdentity};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use oteryn_web::{
    Language,
    ratelimit::{ClientSource, Denial, Keys, NamedPolicy, RateLimiter},
    session::BrowserSession,
};
use std::sync::Arc;

#[derive(Clone)]
struct Panel {
    service: Newsletters,
    auth: Arc<rbac::Authorizer<PortalIdentity>>,
    limiter: RateLimiter,
}

pub(crate) fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let panel = Panel {
        limiter: RateLimiter::new(pool.clone()),
        service: Newsletters {
            pool: pool.clone(),
            mailer: Mailer::configured(),
        },
        auth: Arc::new(rbac::Authorizer::new(
            pool.clone(),
            PortalIdentity::configured(pool.clone()),
        )),
    };
    let mut routes = Router::new();
    for (router, permission) in [
        (
            Router::new()
                .route("/admin/newsletter", get(index))
                .route("/admin/newsletter/{id}", get(preview)),
            Permission::NewsletterView,
        ),
        (
            Router::new().route("/admin/newsletter", post(create)),
            Permission::NewsletterCompose,
        ),
        (
            Router::new().route("/admin/newsletter/{id}/test", post(test)),
            Permission::NewsletterTest,
        ),
        (
            Router::new().route("/admin/newsletter/{id}/send", post(send)),
            Permission::NewsletterSend,
        ),
    ] {
        routes = routes.merge(web::configured_guard(
            oteryn_web::ratelimit::finish_routes(router.with_state(panel.clone())),
            pool.clone(),
            permission,
        )?);
    }
    Ok(routes.layer(DefaultBodyLimit::max(64 * 1024)))
}

pub(crate) fn label(lang: &str, key: &str) -> String {
    let source = if lang == "en" {
        include_str!("en.ftl")
    } else {
        include_str!("pl.ftl")
    };
    let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) else {
        return String::new();
    };
    let Ok(locale) = lang.parse() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    bundle
        .get_message(key)
        .and_then(|message| message.value())
        .map(|pattern| {
            bundle
                .format_pattern(pattern, None, &mut Vec::new())
                .into_owned()
        })
        .unwrap_or_default()
}

#[derive(Template)]
#[template(path = "admin/newsletter/index.html", config = "src/web_askama.toml")]
struct Index {
    lang: &'static str,
    csrf: String,
    compose: bool,
    campaigns: Vec<(Uuid, String, String, String)>,
    next: Option<Uuid>,
}

#[derive(Template)]
#[template(path = "admin/newsletter/preview.html", config = "src/web_askama.toml")]
struct Preview {
    lang: &'static str,
    csrf: String,
    campaign: Campaign,
    en: Copy,
    pl: Copy,
    can_test: bool,
    can_send: bool,
    delivery_available: bool,
}

#[derive(Template)]
#[template(path = "admin/newsletter/error.html", config = "src/web_askama.toml")]
struct Failure {
    lang: &'static str,
    description: &'static str,
    campaign: Option<Uuid>,
}

#[derive(serde::Deserialize)]
struct PageQuery {
    after: Option<Uuid>,
}

async fn index(
    State(panel): State<Panel>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Extension(language): Extension<Language>,
    Query(query): Query<PageQuery>,
) -> Response {
    let lang = language.code();
    let result = async {
        let mut campaigns: Vec<(Uuid, String, String, String)> = sqlx::query_as(
            "SELECT id, subject_pl, subject_en,
                CASE WHEN queued_at IS NOT NULL THEN 'queued'
                     WHEN started_at IS NOT NULL THEN 'queuing' ELSE 'draft' END
             FROM newsletter_campaigns WHERE ($1::uuid IS NULL OR id < $1)
             ORDER BY id DESC LIMIT 21",
        )
        .bind(query.after)
        .fetch_all(&panel.service.pool)
        .await?;
        let next = if campaigns.len() > 20 {
            campaigns.pop();
            campaigns.last().map(|row| row.0)
        } else {
            None
        };
        let csrf = session.csrf_token().await.map_err(|_| Error::Unavailable)?;
        Ok(oteryn_web::render(&Index {
            lang,
            csrf,
            compose: panel
                .auth
                .check(context, Permission::NewsletterCompose)
                .await
                .is_ok(),
            campaigns,
            next,
        }))
    }
    .await;
    outcome(lang, None, result)
}

async fn preview(
    State(panel): State<Panel>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Extension(language): Extension<Language>,
    Path(id): Path<Uuid>,
) -> Response {
    let lang = language.code();
    let result = async {
        let campaign =
            Newsletters::load(&mut *panel.service.pool.acquire().await?, id, false).await?;
        let pl = campaign.copy(MailLanguage::Pl)?;
        let en = campaign.copy(MailLanguage::En)?;
        let csrf = session.csrf_token().await.map_err(|_| Error::Unavailable)?;
        Ok(oteryn_web::render(&Preview {
            lang,
            csrf,
            can_test: campaign.started_at.is_none()
                && panel
                    .auth
                    .check(context, Permission::NewsletterTest)
                    .await
                    .is_ok(),
            can_send: campaign.queued_at.is_none()
                && campaign.tested
                && panel
                    .auth
                    .check(context, Permission::NewsletterSend)
                    .await
                    .is_ok(),
            delivery_available: panel.service.mailer.is_some(),
            campaign,
            en,
            pl,
        }))
    }
    .await;
    outcome(lang, Some(id), result)
}

async fn create(
    State(panel): State<Panel>,
    Extension(context): Extension<CheckContext>,
    Extension(authorization): Extension<MutationAuthorization>,
    Extension(language): Extension<Language>,
    source: ClientSource,
    Form(draft): Form<Draft>,
) -> Response {
    if let Err(denial) = panel.check_quota(context, source).await {
        return denial.response(language, false);
    }
    let staff = Staff {
        context,
        authorization,
        request_id: Uuid::now_v7(),
    };
    outcome(
        language.code(),
        None,
        panel.service.create(&staff, draft).await.map(back_to),
    )
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TestForm {
    _csrf: String,
    locale: MailLanguage,
}

async fn test(
    State(panel): State<Panel>,
    Extension(context): Extension<CheckContext>,
    Extension(authorization): Extension<MutationAuthorization>,
    Extension(language): Extension<Language>,
    source: ClientSource,
    Path(id): Path<Uuid>,
    Form(input): Form<TestForm>,
) -> Response {
    if let Err(denial) = panel.check_quota(context, source).await {
        return denial.response(language, false);
    }
    let staff = Staff {
        context,
        authorization,
        request_id: Uuid::now_v7(),
    };
    outcome(
        language.code(),
        Some(id),
        panel
            .service
            .test(&staff, id, input.locale)
            .await
            .map(|()| back_to(id)),
    )
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SendForm {
    _csrf: String,
    #[serde(default)]
    current_password: String,
}

async fn send(
    State(panel): State<Panel>,
    Extension(context): Extension<CheckContext>,
    Extension(authorization): Extension<MutationAuthorization>,
    Extension(language): Extension<Language>,
    source: ClientSource,
    Path(id): Path<Uuid>,
    Form(input): Form<SendForm>,
) -> Response {
    if let Err(denial) = panel.check_quota(context, source).await {
        return denial.response(language, false);
    }
    let staff = Staff {
        context,
        authorization,
        request_id: Uuid::now_v7(),
    };
    outcome(
        language.code(),
        Some(id),
        panel
            .service
            .send(&staff, id, input.current_password)
            .await
            .map(|()| back_to(id)),
    )
}

impl Panel {
    async fn check_quota(
        &self,
        context: CheckContext,
        ClientSource(source): ClientSource,
    ) -> Result<(), Denial> {
        let mut keys = Keys::source(source);
        keys.account = Some(context.account.to_string());
        self.limiter
            .check(NamedPolicy::IdentitySecurityMutation, &[keys])
            .await
    }
}

fn back_to(id: Uuid) -> Response {
    Redirect::to(&format!("/admin/newsletter/{id}")).into_response()
}

fn outcome(
    lang: &'static str,
    campaign: Option<Uuid>,
    result: Result<Response, Error>,
) -> Response {
    let mut response = match result {
        Ok(response) => response,
        Err(error) => {
            let (status, description) = match error {
                Error::Invalid => (StatusCode::UNPROCESSABLE_ENTITY, "invalid"),
                Error::Missing => (StatusCode::NOT_FOUND, "missing"),
                Error::State => (StatusCode::CONFLICT, "state"),
                Error::RecentAuth => (StatusCode::FORBIDDEN, "recent-auth"),
                Error::SelfOptIn => (StatusCode::CONFLICT, "self-opt-in"),
                Error::Authorization(rbac::Error::Denied) => (StatusCode::FORBIDDEN, "denied"),
                _ => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
            };
            let mut response = oteryn_web::render(&Failure {
                lang,
                description,
                campaign,
            });
            *response.status_mut() = status;
            response
        }
    };
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}

impl Index {
    fn text(&self, key: &str) -> String {
        label(self.lang, key)
    }
}

impl Preview {
    fn text(&self, key: &str) -> String {
        label(self.lang, key)
    }

    fn audience_label(&self) -> String {
        self.text(match self.campaign.audience_locale.as_deref() {
            Some("pl") => "polish",
            Some("en") => "english",
            _ => "all-languages",
        })
    }
}

impl Failure {
    fn text(&self, key: &str) -> String {
        label(self.lang, key)
    }
}
