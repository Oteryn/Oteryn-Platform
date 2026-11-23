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
#[template(path = "community/referrals.html", config = "src/guilds_askama.toml")]
struct Page {
    lang: String,
    title: String,
    notice: String,
    csrf: String,
    landing: bool,
    accept_hint: String,
    accept: String,
    link_label: String,
    create_link: String,
    link: String,
    share_title: String,
    expiry_hint: String,
    game_unavailable: String,
    attribution: Option<String>,
    hint: String,
    rewards_hint: String,
    email: String,
    send: String,
    buddy: String,
    status: String,
    rewards_title: String,
    empty: String,
    invitations: Vec<String>,
    rewards: Vec<(&'static str, String)>,
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
            create_link: t("create-link")?,
            link: String::new(),
            share_title: t("share-title")?,
            expiry_hint: t("expiry-hint")?,
            game_unavailable: t("game-unavailable")?,
            attribution: None,
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
    message(lang, status, key)
}
fn message(lang: &str, status: StatusCode, key: &str) -> Response {
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
    let operations = super::operations::configured_router(&worker)?;
    Ok(router(worker.campaign).merge(operations))
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
                    HeaderValue::from_static("same-origin"),
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
    let result = account_page(&campaign, &session, actor, &lang).await;
    match result {
        Ok(page) => page.response(StatusCode::OK),
        Err(error) => failure(&lang, error),
    }
}
async fn account_page(
    campaign: &Campaign,
    session: &BrowserSession,
    actor: Uuid,
    lang: &str,
) -> Result<Page, Error> {
    let code = campaign.existing_link(actor).await?;
    let mut page = Page::new(
        lang,
        session.csrf_token().await.map_err(|_| Error::Unavailable)?,
    )?;
    if let Some(code) = code {
        page.link = format!("{}/{lang}/referrals/{code}", campaign.origin);
    }
    if session
        .get(SessionKey::<bool>::new("referral.notice"))
        .await
        .map_err(|_| Error::Unavailable)?
        .unwrap_or(false)
    {
        page.notice = translate(lang, "notice")?;
        session
            .set(SessionKey::new("referral.notice"), &false)
            .await
            .map_err(|_| Error::Unavailable)?;
    }
    let attribution: Option<(String, bool)> = sqlx::query_as(
        "SELECT i.kind,i.first_paid_at IS NOT NULL OR premium.id IS NOT NULL
         FROM referral_campaign_invitations i
         LEFT JOIN referral_campaign_rewards premium ON premium.operation_key='buddy-premium:'||i.id
         WHERE i.invitee_id=$1",
    )
    .bind(actor)
    .fetch_optional(&campaign.pool)
    .await?;
    if let Some((kind, paid)) = attribution {
        let key = if paid {
            "qualified"
        } else if kind == "returning" {
            "returning-accepted"
        } else {
            "invite-accepted"
        };
        page.attribution = Some(translate(lang, key)?);
    }
    let invitations: Vec<(String, bool)> = sqlx::query_as(
        "SELECT i.kind,i.first_paid_at IS NOT NULL OR premium.id IS NOT NULL
         FROM referral_campaign_invitations i
         LEFT JOIN referral_campaign_rewards premium ON premium.operation_key='buddy-premium:'||i.id
         WHERE i.inviter_id=$1 AND (i.kind='new' OR premium.id IS NOT NULL OR EXISTS(
             SELECT 1 FROM referral_campaign_rewards r WHERE r.operation_key='welcome:'||i.id))
         ORDER BY i.created_at DESC,i.id DESC LIMIT 100",
    )
    .bind(actor)
    .fetch_all(&campaign.pool)
    .await?;
    for (kind, paid) in invitations {
        let status = if paid { "qualified" } else { "waiting" };
        page.invitations.push(format!(
            "{} — {}",
            translate(lang, &kind)?,
            translate(lang, status)?
        ));
    }
    let rewards: Vec<(String, i32, bool, bool)> = sqlx::query_as(
        "SELECT kind,amount,applied_at IS NOT NULL,COALESCE(expires_at<=statement_timestamp(),false)
         FROM referral_campaign_rewards WHERE account_id=$1 ORDER BY created_at DESC,id DESC LIMIT 100",
    ).bind(actor).fetch_all(&campaign.pool).await?;
    for (kind, amount, applied, expired) in rewards {
        let status = reward_status(&kind, applied, expired);
        let label = translate(lang, &kind)?;
        let state = translate(lang, status)?;
        page.rewards
            .push((status, format!("{label}: {amount} — {state}")));
    }
    page.loyalty = sqlx::query_scalar(
        "SELECT COALESCE(sum(points),0)::bigint FROM referral_loyalty_credits WHERE account_id=$1",
    )
    .bind(actor)
    .fetch_one(&campaign.pool)
    .await?;
    Ok(page)
}

fn reward_status(kind: &str, applied: bool, expired: bool) -> &'static str {
    if applied {
        "applied"
    } else if expired {
        "expired"
    } else if kind == "premium" {
        "premium-pending"
    } else {
        "game-unavailable"
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct InvitationForm {
    #[serde(default)]
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
        "link" => {
            return match campaign.link(actor).await {
                Ok(_) => Redirect::to(&format!("/{lang}/account/referrals")).into_response(),
                Err(error) => failure(&lang, error),
            };
        }
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
    Path((lang, code)): Path<(String, String)>,
) -> Response {
    let code = match invitation_code(&lang, &code) {
        Ok(code) => code,
        Err(error) => return code_failure(&lang, error),
    };
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
    Path((lang, code)): Path<(String, String)>,
    Form(_form): Form<AcceptForm>,
) -> Response {
    let code = match invitation_code(&lang, &code) {
        Ok(code) => code,
        Err(error) => return code_failure(&lang, error),
    };
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

fn invitation_code(lang: &str, raw: &str) -> Result<Uuid, Error> {
    if !matches!(lang, "pl" | "en") {
        return Err(Error::Disabled);
    }
    if raw.len() != 36 {
        return Err(Error::Invalid);
    }
    let code = Uuid::parse_str(raw).map_err(|_| Error::Invalid)?;
    valid(code)?;
    Ok(code)
}

fn code_failure(lang: &str, error: Error) -> Response {
    if error == Error::Invalid {
        message(lang, StatusCode::BAD_REQUEST, "invalid-code")
    } else {
        failure(lang, error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn referral_reward_status_matches_platform_records() {
        assert_eq!(reward_status("starter", false, false), "game-unavailable");
        assert_eq!(reward_status("starter", false, true), "expired");
        assert_eq!(reward_status("outfit", false, false), "game-unavailable");
        assert_eq!(reward_status("premium", false, false), "premium-pending");
        assert_eq!(reward_status("premium", true, false), "applied");
    }

    #[test]
    fn referral_codes_require_supported_locale_and_uuidv7() {
        let valid = Uuid::now_v7();
        assert_eq!(invitation_code("pl", &valid.to_string()), Ok(valid));
        assert_eq!(
            invitation_code("en", &valid.simple().to_string()),
            Err(Error::Invalid)
        );
        assert_eq!(
            invitation_code("en", &Uuid::nil().to_string()),
            Err(Error::Invalid)
        );
        assert_eq!(
            invitation_code("fr", &valid.to_string()),
            Err(Error::Disabled)
        );
    }
}
