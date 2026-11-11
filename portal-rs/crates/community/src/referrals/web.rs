use super::*;
use askama::Template;
use axum::{
    Form, Router,
    extract::{Extension, Path, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_web::session::{BrowserSession, SessionKey};

pub fn translate(lang: &str, key: &str) -> Result<String, Error> {
    let source = match lang {
        "pl" => include_str!("pl.ftl"),
        "en" => include_str!("en.ftl"),
        _ => return Err(Error::Invalid),
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![
        lang.parse::<unic_langid::LanguageIdentifier>()
            .map_err(|_| Error::Unavailable)?,
    ]);
    bundle
        .add_resource(
            fluent_bundle::FluentResource::try_new(source.into())
                .map_err(|_| Error::Unavailable)?,
        )
        .map_err(|_| Error::Unavailable)?;
    let pattern = bundle
        .get_message(key)
        .and_then(|m| m.value())
        .ok_or(Error::Unavailable)?;
    let mut errors = Vec::new();
    let text = bundle
        .format_pattern(pattern, None, &mut errors)
        .into_owned();
    if errors.is_empty() {
        Ok(text)
    } else {
        Err(Error::Unavailable)
    }
}
#[derive(Template)]
#[template(
    source = r#"<!doctype html><html lang="{{ lang }}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{{ title }}</title><link rel="stylesheet" href="/static/css/tokens.css"><link rel="stylesheet" href="/static/css/components.css"><link rel="stylesheet" href="/static/css/app.css"></head><body class="op-theme"><main class="op-stack" id="main"><h1>{{ title }}</h1>{% if !notice.is_empty() %}<p role="status">{{ notice }}</p>{% endif %}
{% if landing %}<p>{{ accept_hint }}</p>{% if !csrf.is_empty() %}<form method="post" class="op-stack"><input type="hidden" name="_csrf" value="{{ csrf }}"><button class="op-button" type="submit">{{ accept }}</button></form>{% endif %}
{% else %}<label class="op-field" for="invite-link">{{ link_label }}</label><input class="op-input" id="invite-link" type="url" readonly value="{{ link }}"><p>{{ hint }}</p><p>{{ rewards_hint }}</p>
<form class="op-stack" method="post"><input type="hidden" name="_csrf" value="{{ csrf }}"><input type="hidden" name="kind" value="new"><label class="op-field" for="new-email">{{ email }}</label><input class="op-input" id="new-email" name="email" type="email" maxlength="254" autocomplete="off" required><button class="op-button" type="submit">{{ send }}</button></form>
<form class="op-stack" method="post"><input type="hidden" name="_csrf" value="{{ csrf }}"><input type="hidden" name="kind" value="returning"><label class="op-field" for="buddy-email">{{ email }}</label><input class="op-input" id="buddy-email" name="email" type="email" maxlength="254" autocomplete="off" required><button class="op-button" type="submit">{{ buddy }}</button></form>
<section aria-labelledby="invitations"><h2 id="invitations">{{ status }}</h2><ul>{% for entry in invitations %}<li>{{ entry }}</li>{% else %}<li>{{ empty }}</li>{% endfor %}</ul></section>
<section aria-labelledby="rewards"><h2 id="rewards">{{ rewards_title }}</h2><p>{{ loyalty_label }}: {{ loyalty }}</p><ul>{% for entry in rewards %}<li>{{ entry }}</li>{% else %}<li>{{ empty }}</li>{% endfor %}</ul></section>{% endif %}</main></body></html>"#,
    ext = "html"
)]
struct Page {
    lang: String,
    title: String,
    notice: String,
    csrf: String,
    landing: bool,
    accept_hint: String,
    accept: String,
    link_label: String,
    link: String,
    hint: String,
    rewards_hint: String,
    email: String,
    send: String,
    buddy: String,
    status: String,
    rewards_title: String,
    empty: String,
    invitations: Vec<String>,
    rewards: Vec<String>,
    loyalty_label: String,
    loyalty: i64,
}
impl Page {
    fn new(lang: &str, csrf: String) -> Result<Self, Error> {
        let t = |key| translate(lang, key);
        Ok(Self {
            lang: lang.into(),
            title: t("title")?,
            notice: String::new(),
            csrf,
            landing: false,
            accept_hint: t("accept-hint")?,
            accept: t("accept")?,
            link_label: t("link")?,
            link: String::new(),
            hint: t("hint")?,
            rewards_hint: t("rewards-hint")?,
            email: t("email")?,
            send: t("send")?,
            buddy: t("buddy")?,
            status: t("status")?,
            rewards_title: t("rewards")?,
            empty: t("empty")?,
            invitations: vec![],
            rewards: vec![],
            loyalty_label: t("loyalty")?,
            loyalty: 0,
        })
    }
    fn response(self, status: StatusCode) -> Response {
        match self.render() {
            Ok(html) => (status, axum::response::Html(html)).into_response(),
            Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    }
}
fn failure(lang: &str, error: Error) -> Response {
    let (status, key) = match error {
        Error::Disabled => (StatusCode::NOT_FOUND, "unavailable"),
        Error::Invalid => (StatusCode::BAD_REQUEST, "invalid"),
        Error::Denied | Error::Conflict => (StatusCode::FORBIDDEN, "denied"),
        Error::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
    };
    match Page::new(lang, String::new()) {
        Ok(mut page) => {
            page.landing = true;
            page.notice = translate(lang, key).unwrap_or_default();
            page.response(status)
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let worker = Worker::configured(pool)?;
    Ok(router(worker.campaign))
}
pub fn routes(campaign: Campaign) -> Router {
    if !campaign.flags.enabled {
        return Router::new();
    }
    Router::new()
        .route("/{lang}/account/referrals", get(page).post(submit))
        .route("/{lang}/referrals/{code}", get(landing).post(accept))
        .with_state(campaign)
        .layer(axum::extract::DefaultBodyLimit::max(4096))
        .layer(axum::middleware::from_fn(
            |request: axum::extract::Request, next: axum::middleware::Next| async move {
                let mut response = next.run(request).await;
            for (name,value) in [
                (header::CONTENT_SECURITY_POLICY,"default-src 'none'; style-src 'self'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'"),
                (header::X_CONTENT_TYPE_OPTIONS,"nosniff"),
                (header::STRICT_TRANSPORT_SECURITY,"max-age=31536000; includeSubDomains"),
            ] { response.headers_mut().insert(name,HeaderValue::from_static(value)); }

                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                response.headers_mut().insert(
                    header::REFERRER_POLICY,
                    HeaderValue::from_static("no-referrer"),
                );
                response
            },
        ))
}
pub fn router(campaign: Campaign) -> Router {
    let mut layer = oteryn_web::session::SessionLayer::new(campaign.pool.clone());
    layer.csrf.trusted_origins = vec![campaign.origin.clone()];
    routes(campaign).layer(axum::middleware::from_fn_with_state(
        layer,
        oteryn_web::session::middleware,
    ))
}
async fn page(
    State(campaign): State<Campaign>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
) -> Response {
    if !matches!(lang.as_str(), "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(actor) = session.account_id().await else {
        return Redirect::to(&format!("/{lang}/login")).into_response();
    };
    let result=async {
        let code=campaign.link(actor).await?;
        let mut page=Page::new(&lang,session.csrf_token().await.map_err(|_|Error::Unavailable)?)?;
        page.link=format!("{}/{lang}/referrals/{code}",campaign.origin);
        if session.get(SessionKey::<bool>::new("referral.notice")).await.map_err(|_|Error::Unavailable)?.unwrap_or(false) {
            page.notice=translate(&lang,"notice")?;
            session.set(SessionKey::new("referral.notice"),&false).await.map_err(|_|Error::Unavailable)?;
        }
        let rows:Vec<(String,bool)>=sqlx::query_as("SELECT kind,first_paid_at IS NOT NULL FROM referral_campaign_invitations WHERE inviter_id=$1 AND (kind='new' OR EXISTS(SELECT 1 FROM referral_campaign_rewards r WHERE r.operation_key='welcome:'||referral_campaign_invitations.id)) ORDER BY created_at DESC LIMIT 100").bind(actor).fetch_all(&campaign.pool).await?;
        for (kind,paid) in rows {page.invitations.push(format!("{} — {}",translate(&lang,&kind)?,translate(&lang,if paid {"qualified"} else {"waiting"})?));}
        let rows:Vec<(String,i32,bool)>=sqlx::query_as("SELECT kind,amount,applied_at IS NOT NULL FROM referral_campaign_rewards WHERE account_id=$1 ORDER BY created_at DESC LIMIT 100").bind(actor).fetch_all(&campaign.pool).await?;
        for (kind,amount,applied) in rows {page.rewards.push(format!("{}: {amount} — {}",translate(&lang,&kind)?,translate(&lang,if applied {"applied"} else {"pending"})?));}
        page.loyalty=sqlx::query_scalar("SELECT COALESCE(sum(points),0)::bigint FROM referral_loyalty_credits WHERE account_id=$1").bind(actor).fetch_one(&campaign.pool).await?;
        Ok::<_,Error>(page)
    }.await;
    match result {
        Ok(page) => page.response(StatusCode::OK),
        Err(error) => failure(&lang, error),
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct InvitationForm {
    email: String,
    kind: String,
    #[serde(rename = "_csrf")]
    _csrf: String,
}
async fn submit(
    State(campaign): State<Campaign>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
    Form(form): Form<InvitationForm>,
) -> Response {
    if !matches!(lang.as_str(), "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(actor) = session.account_id().await else {
        return Redirect::to(&format!("/{lang}/login")).into_response();
    };
    let kind = match form.kind.as_str() {
        "new" => Kind::New,
        "returning" => Kind::Returning,
        _ => return failure(&lang, Error::Invalid),
    };
    match campaign.email(actor, &form.email, kind).await {
        Ok(()) | Err(Error::Denied) => {}
        Err(error) => return failure(&lang, error),
    }
    if session
        .set(SessionKey::new("referral.notice"), &true)
        .await
        .is_err()
    {
        return failure(&lang, Error::Unavailable);
    }
    Redirect::to(&format!("/{lang}/account/referrals")).into_response()
}
async fn landing(
    State(campaign): State<Campaign>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, code)): Path<(String, Uuid)>,
) -> Response {
    let result = async {
        campaign.invitation_kind(code).await?;
        let mut page = Page::new(
            &lang,
            session.csrf_token().await.map_err(|_| Error::Unavailable)?,
        )?;
        page.landing = true;
        Ok::<_, Error>(page)
    }
    .await;
    match result {
        Ok(page) => page.response(StatusCode::OK),
        Err(error) => failure(&lang, error),
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptForm {
    #[serde(rename = "_csrf")]
    _csrf: String,
}
async fn accept(
    State(campaign): State<Campaign>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, code)): Path<(String, Uuid)>,
    Form(_form): Form<AcceptForm>,
) -> Response {
    if !matches!(lang.as_str(), "pl" | "en") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let kind = match campaign.invitation_kind(code).await {
        Ok(kind) => kind,
        Err(error) => return failure(&lang, error),
    };
    if let Some(actor) = session.account_id().await {
        match campaign.accept(actor, code).await {
            Ok(()) => return Redirect::to(&format!("/{lang}/account/referrals")).into_response(),
            Err(error) => return failure(&lang, error),
        }
    }
    if session
        .set(SessionKey::new("referral.code"), &code.to_string())
        .await
        .is_err()
    {
        return failure(&lang, Error::Unavailable);
    }
    let destination = if kind == Kind::Returning {
        "login"
    } else {
        "register"
    };
    Redirect::to(&format!("/{lang}/{destination}")).into_response()
}
