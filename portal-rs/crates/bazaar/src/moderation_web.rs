//! SSR public and account surfaces; identity and projection adapters are injected.
use super::*;
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    middleware,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_web::session::{BrowserSession, SessionLayer};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct Locale(pub &'static str);
impl Locale {
    pub fn parse(s: &str) -> Result<Self, StatusCode> {
        match s {
            "en" => Ok(Self("en")),
            "pl" => Ok(Self("pl")),
            _ => Err(StatusCode::NOT_FOUND),
        }
    }
    pub fn text(self, key: &str) -> String {
        let source = if self.0 == "pl" {
            include_str!("moderation_pl.ftl")
        } else {
            include_str!("moderation_en.ftl")
        };
        let Ok(resource) = FluentResource::try_new(source.to_owned()) else {
            return String::new();
        };
        let Ok(lang) = self.0.parse() else {
            return String::new();
        };
        let mut bundle = FluentBundle::new(vec![lang]);
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
pub fn status(error: Error) -> StatusCode {
    match error {
        Error::Disabled | Error::NotFound => StatusCode::NOT_FOUND,
        Error::InvalidInput | Error::Overflow => StatusCode::UNPROCESSABLE_ENTITY,
        Error::Denied => StatusCode::FORBIDDEN,
        Error::IllegalTransition | Error::IdempotencyConflict | Error::InsufficientFunds => {
            StatusCode::CONFLICT
        }
        Error::Database(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}
pub fn configured_router(pool: PgPool) -> Router {
    // Proposed Game producers are not shipped. No runtime adapter grants authority.
    router(
        pool,
        PendingIntegration,
        PendingIntegration,
        PendingIntegration,
        Flags {
            enabled: std::env::var("OTERYN_BAZAAR_MODERATION").as_deref() == Ok("true"),
            public_profiles: std::env::var("OTERYN_BAZAAR_PROFILES").as_deref() == Ok("true"),
            promotions: std::env::var("OTERYN_BAZAAR_PROMOTIONS").as_deref() == Ok("true"),
        },
    )
}
pub fn router<A: Accounts + 'static, V: PublicViews + 'static, L: PromotionLedger + 'static>(
    pool: PgPool,
    accounts: A,
    views: V,
    ledger: L,
    flags: Flags,
) -> Router {
    if !flags.enabled {
        return Router::new();
    }
    Router::new()
        .route("/{lang}/bazaar/sellers/{seller}", get(seller::<A, V, L>))
        .route(
            "/{lang}/account/bazaar/{listing}/report",
            get(report_form).post(report::<A, V, L>),
        )
        .route(
            "/{lang}/account/bazaar/{listing}/promote",
            get(promotion_form::<A, V, L>).post(promote::<A, V, L>),
        )
        .route_layer(middleware::from_fn_with_state(
            SessionLayer::new(pool.clone()),
            oteryn_web::session::middleware,
        ))
        .layer(axum::extract::DefaultBodyLimit::max(8192))
        .layer(middleware::from_fn(no_store))
        .with_state(Arc::new(Moderation::new(
            pool, accounts, views, ledger, flags,
        )))
}
async fn no_store(req: axum::extract::Request, next: middleware::Next) -> Response {
    let mut response = next.run(req).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    response
}
#[derive(Template)]
#[template(
    source = r#"<!doctype html><html lang="{{ locale.0 }}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{{ self.text("bazaar") }} · Oteryn</title><link rel="stylesheet" href="/static/css/tokens.css"><link rel="stylesheet" href="/static/css/components.css"></head><body class="op-theme"><main class="op-stack"><h1>{{ self.text("bazaar") }}</h1>
{% if let Some(p) = profile %}<h2>{{ p.character_name }}</h2><p>{{ self.text("joined") }}: {{ p.joined }}</p><p>{{ self.text("completed") }}: {{ p.completed_transactions }}</p><h2>{{ self.text("active") }}</h2><ul class="op-stack">{% for l in p.listings %}<li><a href="/{{ locale.0 }}/bazaar/{{ l.id }}">{{ l.title }}</a>{% if l.featured %} <span class="op-badge">{{ self.text("featured") }}</span>{% endif %}</li>{% endfor %}</ul>{% else %}
<h2>{{ self.text(action) }}</h2><form method="post" class="op-panel op-stack"><input type="hidden" name="_csrf" value="{{ csrf }}"><input type="hidden" name="id" value="{{ id }}">
{% if action == "report" %}<label for="category">{{ self.text("category") }}</label><select class="op-input" id="category" name="category" required><option value="fraud">{{ self.text("fraud") }}</option><option value="inappropriate">{{ self.text("inappropriate") }}</option><option value="other">{{ self.text("other") }}</option></select><label for="detail">{{ self.text("detail") }}</label><textarea class="op-input" id="detail" name="detail" minlength="10" maxlength="1000" required></textarea>{% else %}<p>{{ self.text("promotion-help") }}</p>{% if let Some(q) = quote %}<p>{{ self.text("promotion-price") }}: {{ q.amount }}</p><input type="hidden" name="fee_revision" value="{{ q.revision }}"><input type="hidden" name="amount" value="{{ q.amount }}">{% endif %}{% endif %}<button type="submit" class="op-button">{{ self.text(action) }}</button></form>{% endif %}</main></body></html>"#,
    ext = "html"
)]
struct Page {
    locale: Locale,
    profile: Option<SellerProfile>,
    quote: Option<PromotionQuote>,
    csrf: String,
    id: Uuid,
    action: &'static str,
}
impl Page {
    fn text(&self, key: &str) -> String {
        self.locale.text(key)
    }
    fn response(self) -> Result<Response, StatusCode> {
        self.render()
            .map(|s| Html(s).into_response())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}
async fn seller<A: Accounts, V: PublicViews, L: PromotionLedger>(
    State(api): State<Arc<Moderation<A, V, L>>>,
    Path((lang, id)): Path<(String, Uuid)>,
) -> Result<Response, StatusCode> {
    let locale = Locale::parse(&lang)?;
    let profile = api.seller(id).await.map_err(status)?;
    Page {
        locale,
        profile: Some(profile),
        quote: None,
        csrf: String::new(),
        id: Uuid::nil(),
        action: "",
    }
    .response()
}
async fn form(
    lang: String,
    session: BrowserSession,
    action: &'static str,
    quote: Option<PromotionQuote>,
) -> Result<Response, StatusCode> {
    let locale = Locale::parse(&lang)?;
    if session.account_id().await.is_none() {
        return Ok(Redirect::to(&format!("/{lang}/login")).into_response());
    }
    let csrf = session
        .csrf_token()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Page {
        locale,
        profile: None,
        quote,
        csrf,
        id: Uuid::now_v7(),
        action,
    }
    .response()
}
async fn report_form(
    Path((lang, _)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    form(lang, session, "report", None).await
}
async fn promotion_form<A: Accounts, V: PublicViews, L: PromotionLedger>(
    State(api): State<Arc<Moderation<A, V, L>>>,
    Path((lang, _)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
) -> Result<Response, StatusCode> {
    let quote = api.promotion_quote().await.map_err(status)?;
    form(lang, session, "promote", Some(quote)).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    id: Uuid,
    #[serde(default)]
    fee_revision: Option<Uuid>,
    #[serde(default)]
    amount: Option<i64>,
    #[serde(default)]
    category: String,
    #[serde(default)]
    detail: String,
    #[serde(rename = "_csrf")]
    _csrf: String,
}
async fn report<A: Accounts, V: PublicViews, L: PromotionLedger>(
    State(api): State<Arc<Moderation<A, V, L>>>,
    Path((lang, listing)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    Locale::parse(&lang)?;
    let actor = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    api.report(
        actor,
        ReportInput {
            id: input.id,
            listing,
            category: input.category,
            detail: input.detail,
        },
    )
    .await
    .map_err(status)?;
    Ok(Redirect::to(&format!("/{lang}/bazaar/{listing}")).into_response())
}
async fn promote<A: Accounts, V: PublicViews, L: PromotionLedger>(
    State(api): State<Arc<Moderation<A, V, L>>>,
    Path((lang, listing)): Path<(String, Uuid)>,
    Extension(session): Extension<BrowserSession>,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    Locale::parse(&lang)?;
    let actor = session.account_id().await.ok_or(StatusCode::FORBIDDEN)?;
    api.promote(
        actor,
        listing,
        input.id,
        PromotionQuote {
            revision: input.fee_revision.ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
            amount: input.amount.ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
        },
    )
    .await
    .map_err(status)?;
    Ok(Redirect::to(&format!("/{lang}/bazaar/{listing}")).into_response())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    #[test]
    fn public_pages_are_localized_escaped_and_labelled() {
        for lang in ["pl", "en"] {
            let locale = Locale::parse(lang).unwrap();
            for action in ["report", "promote", ""] {
                let profile = (action.is_empty()).then(|| SellerProfile {
                    character_name: "<script>name</script>".into(),
                    joined: "2026-11-05".into(),
                    completed_transactions: 3,
                    listings: vec![PublicListing {
                        id: Uuid::now_v7(),
                        title: "<unsafe>".into(),
                        featured: false,
                    }],
                });
                let html = Page {
                    locale,
                    profile,
                    quote: Some(PromotionQuote {
                        revision: Uuid::now_v7(),
                        amount: 25,
                    }),
                    csrf: "fixture-token".into(),
                    id: Uuid::now_v7(),
                    action,
                }
                .render()
                .unwrap();
                assert!(html.contains("width=device-width"));
                assert!(!html.contains("<script>name"));
                assert!(!html.contains("<unsafe>"));
                if action == "report" {
                    assert!(html.contains("for=\"detail\""));
                    assert!(html.contains("for=\"category\""));
                }
                if action == "promote" {
                    assert!(html.contains("name=\"fee_revision\""));
                    assert!(html.contains("25"));
                }
                std::fs::write(format!("/tmp/t64-public-{lang}-{action}.html"), html).unwrap();
            }
        }
    }
}
