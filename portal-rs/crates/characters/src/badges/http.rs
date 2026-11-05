//! SSR boundary. Browser sessions and CSRF are provided by the web crate.
use super::*;
use askama::Template;
use axum::{
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
};
use fluent_bundle::{FluentBundle, FluentResource};

#[derive(Template)]
#[template(path = "badges/selection.html", config = "src/badges/askama.toml")]
struct Page {
    lang: String,
    csrf: String,
    title: String,
    introduction: String,
    badges_label: String,
    achievements_label: String,
    profile_label: String,
    loyalty_label: String,
    none: String,
    save: String,
    unavailable: String,
    badges: Vec<(String, String, String, bool, bool)>,
    characters: Vec<(String, String, bool, bool)>,
    earned: Vec<(String, String, bool)>,
}
fn text(lang: &str, key: &str) -> String {
    let source = if lang == "en" {
        include_str!("../../../../templates/badges/en.ftl")
    } else {
        include_str!("../../../../templates/badges/pl.ftl")
    };
    let Ok(locale) = lang.parse() else {
        return String::new();
    };
    let mut bundle = FluentBundle::new(vec![locale]);
    if let Ok(resource) = FluentResource::try_new(source.into()) {
        let _ = bundle.add_resource(resource);
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
fn status(error: Error) -> Response {
    match error {
        Error::Disabled => StatusCode::NOT_FOUND,
        Error::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
        Error::Denied => StatusCode::FORBIDDEN,
        Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
    .into_response()
}
fn private(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("x-robots-tag"),
        header::HeaderValue::from_static("noindex, nofollow"),
    );
    response
}
fn authorize(enabled: bool, account: Option<Uuid>, lang: &str) -> Result<Uuid, Box<Response>> {
    if !enabled {
        return Err(Box::new(status(Error::Disabled)));
    }
    if !["en", "pl"].contains(&lang) {
        return Err(Box::new(StatusCode::BAD_REQUEST.into_response()));
    }
    account.ok_or_else(|| Box::new(Redirect::to(&format!("/{lang}/login")).into_response()))
}
pub fn configured(pool: PgPool) -> Service<LocalEvidence> {
    Service {
        pool,
        enabled: std::env::var("OTERYN_ACCOUNT_BADGES_ENABLED").as_deref() == Ok("true"),
        evidence: LocalEvidence,
    }
}
pub async fn show<E: Evidence>(
    service: &Service<E>,
    account: Option<Uuid>,
    lang: &str,
    csrf: Result<String, sqlx::Error>,
) -> Response {
    private(show_inner(service, account, lang, csrf).await)
}
async fn show_inner<E: Evidence>(
    service: &Service<E>,
    account: Option<Uuid>,
    lang: &str,
    csrf: Result<String, sqlx::Error>,
) -> Response {
    let account = match authorize(service.enabled, account, lang) {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let csrf = match csrf {
        Ok(v) => v,
        Err(_) => return status(Error::Unavailable),
    };
    let view = match service.owner(account).await {
        Ok(v) => v,
        Err(e) => return status(e),
    };
    let t = |key| text(lang, key);
    let badges = ["account-age", "loyalty", "total-levels", "all-professions"]
        .into_iter()
        .map(|key| {
            let value = view
                .facts
                .badges
                .iter()
                .find(|(k, v)| k == key && *v >= 0)
                .map(|(_, v)| v.to_string());
            (
                key.into(),
                t(key),
                value.clone().unwrap_or_else(|| t("unavailable")),
                view.selection.badges.iter().any(|k| k == key),
                value.is_none(),
            )
        })
        .collect();
    let characters = view
        .facts
        .characters
        .iter()
        .filter(|c| c.public && !c.ownership.is_empty())
        .map(|c| {
            (
                c.id.to_string(),
                c.name.clone(),
                view.selection.profile == Some(c.id),
                view.selection.loyalty == Some(c.id),
            )
        })
        .collect();
    let earned = view
        .selection
        .profile
        .and_then(|id| character(&view.facts, id).ok())
        .and_then(|c| c.earned.as_ref())
        .map(|keys| {
            keys.iter()
                .filter_map(|k| {
                    view.facts.achievement_names.get(k).map(|names| {
                        (
                            k.clone(),
                            if lang == "en" {
                                names.0.clone()
                            } else {
                                names.1.clone()
                            },
                            view.selection.achievements.contains(k),
                        )
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let page = Page {
        lang: lang.into(),
        csrf,
        title: t("title"),
        introduction: t("introduction"),
        badges_label: t("badges"),
        achievements_label: t("achievements"),
        profile_label: t("profile"),
        loyalty_label: t("ranking"),
        none: t("none"),
        save: t("save"),
        unavailable: t("unavailable"),
        badges,
        characters,
        earned,
    };
    match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
fn parse(fields: Vec<(String, String)>) -> Result<Selection, Error> {
    if fields.len() > 14 {
        return Err(Error::Invalid);
    }
    let mut selection = Selection::default();
    let mut singular = BTreeSet::new();
    for (key, value) in fields {
        match key.as_str() {
            "badge" => selection.badges.push(value),
            "achievement" => selection.achievements.push(value),
            "profile" | "ranking" | "_csrf" => {
                if !singular.insert(key.clone()) {
                    return Err(Error::Invalid);
                }
                if key == "_csrf" {
                    continue;
                }
                let id = if value.is_empty() {
                    None
                } else {
                    let id = Uuid::parse_str(&value).map_err(|_| Error::Invalid)?;
                    if !valid_id(id) || id.to_string() != value {
                        return Err(Error::Invalid);
                    }
                    Some(id)
                };
                if key == "profile" {
                    selection.profile = id;
                } else {
                    selection.loyalty = id;
                }
            }
            _ => return Err(Error::Invalid),
        }
    }
    Ok(selection)
}
/// Mounted exclusively inside authenticated session/CSRF middleware.
pub async fn save<E: Evidence>(
    service: &Service<E>,
    account: Option<Uuid>,
    lang: &str,
    fields: Vec<(String, String)>,
) -> Response {
    private(save_inner(service, account, lang, fields).await)
}
async fn save_inner<E: Evidence>(
    service: &Service<E>,
    account: Option<Uuid>,
    lang: &str,
    fields: Vec<(String, String)>,
) -> Response {
    let account = match authorize(service.enabled, account, lang) {
        Ok(a) => a,
        Err(r) => return *r,
    };
    let selection = match parse(fields) {
        Ok(s) => s,
        Err(e) => return status(e),
    };
    match service.save(account, &selection).await {
        Ok(()) => Redirect::to(&format!("/{lang}/account/badges")).into_response(),
        Err(e) => status(e),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bad_forms_and_translations() {
        assert_eq!(
            parse(vec![("account_id".into(), Uuid::now_v7().to_string())]),
            Err(Error::Invalid)
        );
        assert_eq!(
            parse(vec![("profile".into(), "bad".into())]),
            Err(Error::Invalid)
        );
        assert_eq!(
            parse(vec![
                ("profile".into(), "".into()),
                ("profile".into(), "".into())
            ]),
            Err(Error::Invalid)
        );
        for lang in ["en", "pl"] {
            for key in [
                "title",
                "account-age",
                "loyalty",
                "total-levels",
                "all-professions",
                "unavailable",
                "introduction",
            ] {
                assert!(!text(lang, key).is_empty());
            }
        }
    }
}
