//! Account-only sandbox checkout. Quotes and orders use the native coin ledger.
use super::*;
use crate::account::Locale;
use crate::coin::{CoinOrder, CoinOrderRequest, CoinPackage, CoinSettings, PackageDef};

pub(super) const PATH: &str = "/{lang}/panel/payments/coin-checkout";

pub(crate) fn enabled() -> bool {
    let settings = CoinSettings::from_lookup(|key| std::env::var(key).ok());
    settings.enabled && !settings.packages.is_empty()
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Input {
    Quote {
        package: String,
        _csrf: Option<String>,
    },
    Order(Buy),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Buy {
    pub(super) package: String,
    pub(super) quote: String,
    pub(super) request: String,
    pub(super) _csrf: Option<String>,
}

impl Buy {
    pub(super) fn order_request(&self) -> Result<CoinOrderRequest, CoinError> {
        Ok(CoinOrderRequest {
            request_id: canonical_id(&self.request).ok_or(CoinError::Invalid)?,
            package_id: canonical_id(&self.package).ok_or(CoinError::Invalid)?,
            quote_id: canonical_id(&self.quote).ok_or(CoinError::Invalid)?,
            character_id: None,
        })
    }
}

#[derive(Template)]
#[template(path = "coin/browser.html")]
struct Page<'a> {
    locale: Locale,
    history: bool,
    csrf: String,
    packages: &'a [PackageDef],
    orders: Vec<CoinOrder>,
    quote: Option<CoinPackage>,
    request: Uuid,
    notice: Option<&'static str>,
}

async fn render_page(
    service: &CoinPurchases,
    session: &BrowserSession,
    payer: Uuid,
    locale: Locale,
    quote: Option<CoinPackage>,
    notice: Option<&'static str>,
) -> Result<Response, CoinError> {
    let orders = service.browser_orders(payer).await?;
    let csrf = session
        .csrf_token()
        .await
        .map_err(|_| CoinError::Unavailable)?;
    Ok(oteryn_web::render(&Page {
        locale,
        history: std::env::var("PANEL_PREMIUM_PAYMENTS").as_deref() == Ok("true"),
        csrf,
        packages: service.browser_packages(),
        orders,
        quote,
        request: oteryn_core::new_id(),
        notice,
    }))
}

pub(super) async fn page(
    State(service): State<Arc<CoinPurchases>>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
) -> Response {
    let Ok(locale) = Locale::parse(&lang) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(payer) = session.account_id().await else {
        return Redirect::to(&format!("/{lang}/login")).into_response();
    };
    let empty = service.browser_packages().is_empty();
    let response = render_page(
        &service,
        &session,
        payer,
        locale,
        None,
        empty.then_some("sandbox-unavailable"),
    )
    .await;
    match response {
        Ok(response) if empty => (StatusCode::SERVICE_UNAVAILABLE, response).into_response(),
        Ok(response) => response,
        Err(error) => refusal(error),
    }
}

pub(super) async fn submit(
    State(service): State<Arc<CoinPurchases>>,
    Extension(session): Extension<BrowserSession>,
    Path(lang): Path<String>,
    form: Result<Form<Input>, axum::extract::rejection::FormRejection>,
) -> Response {
    let Ok(locale) = Locale::parse(&lang) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(payer) = session.account_id().await else {
        return Redirect::to(&format!("/{lang}/login")).into_response();
    };
    let result = match form {
        Ok(Form(Input::Quote { package, .. })) => match canonical_id(&package) {
            Some(id) => match service.browser_quote(payer, id).await {
                Ok(quote) => {
                    return render_page(&service, &session, payer, locale, Some(quote), None)
                        .await
                        .unwrap_or_else(refusal);
                }
                Err(error) => error,
            },
            None => CoinError::Invalid,
        },
        Ok(Form(Input::Order(form))) => match form.order_request() {
            Ok(request) => match service.create_order(payer, &request).await {
                Ok(order) => {
                    return Redirect::to(&format!("{}?lang={lang}", checkout_path(order.order_id)))
                        .into_response();
                }
                Err(error) => error,
            },
            Err(error) => error,
        },
        Err(_) => CoinError::Invalid,
    };
    let notice = match result {
        CoinError::Invalid => "sandbox-invalid",
        CoinError::Conflict => "sandbox-conflict",
        _ => "sandbox-unavailable",
    };
    match render_page(&service, &session, payer, locale, None, Some(notice)).await {
        Ok(response) => (refusal(result).status(), response).into_response(),
        Err(error) => refusal(error),
    }
}
