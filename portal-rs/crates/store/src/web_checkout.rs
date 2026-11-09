//! Browser checkout delegates all settlement to the verified provider boundary.
use super::*;

#[derive(Debug)]
pub struct Checkout {
    pub package: Package,
    /// Fluent status key, derived from persisted payment state.
    pub status: String,
    pub pending: bool,
}
#[derive(Default, Deserialize)]
pub(super) struct CheckoutStatus {
    pub checkout: Option<Uuid>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckoutForm {
    #[serde(rename = "_csrf")]
    _csrf: String,
    action: String,
}
pub(super) async fn checkout_page(
    State(s): State<Shop>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, intent)): Path<(String, Uuid)>,
) -> Response {
    let l = match Locale::parse(&lang) {
        Ok(l) => l,
        Err(e) => return e.into_response(),
    };
    let a = match account(&s, &session, l).await {
        Ok(a) => a,
        Err(r) => return r,
    };
    let mut p = Page::new(l, "checkout");
    p.csrf = match session.csrf_token().await {
        Ok(token) => token,
        Err(_) => return failure(l, Error::Unavailable),
    };
    match s.backend.checkout(a, intent, None).await {
        Ok(Some(c)) => {
            p.message = l.text(&c.status);
            p.enabled = c.pending;
            p.checkout_price = l.currency(c.package.minor_units, &c.package.currency);
            p.checkout = Some(c);
            p.operation = intent;
            oteryn_web::render(&p)
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => failure(l, e),
    }
}
pub(super) async fn checkout_post(
    State(s): State<Shop>,
    Extension(session): Extension<BrowserSession>,
    Path((lang, intent)): Path<(String, Uuid)>,
    request: Request,
) -> Response {
    let (a, l) = match mutation(&s, &session, &lang, s.flags.top_ups).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let form: CheckoutForm = match parse(request).await {
        Ok(v) => v,
        Err(e) => return failure(l, e),
    };
    if !matches!(form.action.as_str(), "pay" | "reject" | "cancel") {
        return failure(l, Error::Invalid);
    }
    match s.backend.checkout(a, intent, Some(form.action)).await {
        Ok(Some(_)) => {
            Redirect::to(&format!("/{}/shop/coins?checkout={intent}", l.0)).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => failure(l, e),
    }
}
