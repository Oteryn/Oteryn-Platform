use super::{Error, Preferences, Service};
use askama::Template;
use axum::{
    body::Bytes,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
};
use fluent_bundle::{FluentBundle, FluentResource};
use sqlx::PgPool;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
#[derive(Template)]
#[template(path = "profile/settings.html", config = "src/profile/askama.toml")]
struct Page<'a> {
    lang: &'a str,
    title: String,
    name: String,
    csrf: &'a str,
    guild: String,
    main: String,
    save: String,
    value: Preferences,
    labels: Vec<String>,
}
fn labels(lang: &str) -> Vec<String> {
    [
        "hidden",
        "hide-all",
        "account",
        "status",
        "house",
        "skills",
        "deaths",
        "kills",
        "online-time",
        "comment",
    ]
    .iter()
    .map(|key| text(lang, key))
    .collect()
}
fn text(lang: &str, key: &str) -> String {
    let source = if lang == "en" {
        include_str!("../../../../templates/profile/en.ftl")
    } else {
        include_str!("../../../../templates/profile/pl.ftl")
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
fn now() -> Result<i64, Error> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .ok_or(Error::Unavailable)
}
fn service(pool: &PgPool) -> Service {
    Service::new(
        pool.clone(),
        crate::overview::configured_projection(),
        std::env::var("OTERYN_CHARACTER_PROFILES_ENABLED").as_deref() == Ok("true"),
    )
}
fn failure(error: Error, lang: &str) -> Response {
    let (status, key) = match error {
        Error::NotFound => (StatusCode::NOT_FOUND, "missing"),
        Error::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
        Error::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "invalid"),
        Error::BadInput => (StatusCode::UNPROCESSABLE_ENTITY, "invalid"),
    };
    (status, text(lang, key)).into_response()
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
fn preflight(account: Option<Uuid>, lang: &str) -> Result<Uuid, Box<Response>> {
    if !matches!(lang, "pl" | "en") {
        return Err(Box::new(StatusCode::BAD_REQUEST.into_response()));
    }
    if std::env::var("OTERYN_CHARACTER_PROFILES_ENABLED").as_deref() != Ok("true") {
        return Err(Box::new(StatusCode::NOT_FOUND.into_response()));
    }
    account.ok_or_else(|| Box::new(Redirect::to(&format!("/{lang}/login")).into_response()))
}
/// The mounting router supplies the session account and CSRF token, never form fields.
pub async fn edit(
    pool: &PgPool,
    account: Option<Uuid>,
    lang: &str,
    name: &str,
    csrf: &str,
) -> Response {
    private(edit_inner(pool, account, lang, name, csrf).await)
}
async fn edit_inner(
    pool: &PgPool,
    account: Option<Uuid>,
    lang: &str,
    name: &str,
    csrf: &str,
) -> Response {
    let account = match preflight(account, lang) {
        Ok(v) => v,
        Err(r) => return *r,
    };
    let result = match now() {
        Ok(now) => service(pool).edit(account, name, now).await,
        Err(e) => Err(e),
    };
    match result {
        Ok((name, value)) => match (Page {
            lang,
            title: text(lang, "title"),
            name,
            csrf,
            guild: text(lang, "guild"),
            main: text(lang, "main"),
            save: text(lang, "save"),
            value,
            labels: labels(lang),
        })
        .render()
        {
            Ok(html) => Html(html).into_response(),
            Err(_) => failure(Error::Unavailable, lang),
        },
        Err(e) => failure(e, lang),
    }
}
fn parse(body: &[u8]) -> Result<Preferences, Error> {
    if body.len() > 4096 {
        return Err(Error::BadInput);
    }
    let fields: Vec<(String, String)> =
        serde_urlencoded::from_bytes(body).map_err(|_| Error::BadInput)?;
    let mut p = Preferences {
        show_guild: false,
        hidden: false,
        ..Preferences::default()
    };
    let mut seen = std::collections::HashSet::new();
    for (key, value) in fields {
        if !seen.insert(key.clone()) {
            return Err(Error::BadInput);
        }
        match key.as_str() {
            "_csrf" => {}
            "comment" => {
                p.comment = value.replace("\r\n", "\n");
            }
            "show_guild"
            | "is_main_character"
            | "hidden"
            | "hide_all_characters"
            | "show_account"
            | "show_status"
            | "show_house"
            | "show_skills"
            | "show_deaths"
            | "show_kills"
            | "show_online_time" => {
                let value = match value.as_str() {
                    "1" | "true" => true,
                    "0" | "false" => false,
                    _ => return Err(Error::BadInput),
                };
                match key.as_str() {
                    "show_guild" => p.show_guild = value,
                    "is_main_character" => p.is_main_character = value,
                    "hidden" => p.hidden = value,
                    "hide_all_characters" => p.hide_all_characters = value,
                    "show_account" => p.show_account = value,
                    "show_status" => p.show_status = value,
                    "show_house" => p.show_house = value,
                    "show_skills" => p.show_skills = value,
                    "show_deaths" => p.show_deaths = value,
                    "show_kills" => p.show_kills = value,
                    "show_online_time" => p.show_online_time = value,
                    _ => return Err(Error::BadInput),
                }
            }
            _ => return Err(Error::BadInput),
        }
    }
    if p.comment.chars().count() > 500 || p.comment.chars().any(|c| c.is_control() && c != '\n') {
        return Err(Error::BadInput);
    }
    Ok(p)
}
/// The mounting router must enforce session CSRF before this browser mutation.
pub async fn update(
    pool: &PgPool,
    account: Option<Uuid>,
    lang: &str,
    name: &str,
    body: Bytes,
) -> Response {
    let result = async {
        let account = preflight(account, lang).map_err(|r| *r)?;
        let p = parse(&body).map_err(|e| failure(e, lang))?;
        let time = now().map_err(|e| failure(e, lang))?;
        service(pool)
            .update(account, name, p, time)
            .await
            .map_err(|e| failure(e, lang))?;
        let segment: String = name
            .bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                    char::from(b).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect();
        Ok::<_, Response>(
            Redirect::to(&format!("/{lang}/account/characters/{segment}/profile")).into_response(),
        )
    }
    .await;
    private(match result {
        Ok(r) | Err(r) => r,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn form_validation_and_translations() {
        assert_eq!(
            parse(b"show_guild=1&is_main_character=true").ok(),
            Some(Preferences {
                show_guild: true,
                is_main_character: true,
                hidden: false,
                ..Preferences::default()
            })
        );
        assert!(parse(b"show_guild=banana").is_err());
        assert!(parse(format!("comment={}", "a".repeat(501)).as_bytes()).is_err());
        assert!(parse(b"hidden=true&hidden=false").is_err());
        assert!(parse(b"comment=%00").is_err());
        assert_eq!(
            parse(b"comment=first%0D%0Asecond").ok().map(|p| p.comment),
            Some("first\nsecond".into())
        );
        assert!(parse("comment=żółć".as_bytes()).is_ok());
        assert!(parse(b"show_guild=1&show_guild=0").is_err());
        assert!(parse(b"account_id=foreign").is_err());
        assert_eq!(
            parse(b"").ok(),
            Some(Preferences {
                show_guild: false,
                is_main_character: false,
                hidden: false,
                ..Preferences::default()
            })
        );
        for lang in ["pl", "en"] {
            for key in [
                "title",
                "guild",
                "main",
                "save",
                "missing",
                "unavailable",
                "invalid",
                "updated",
            ] {
                assert!(!text(lang, key).is_empty());
            }
        }
    }
    #[test]
    fn escaped_accessible_form() -> Result<(), Box<dyn std::error::Error>> {
        let html = Page {
            lang: "en",
            title: text("en", "title"),
            name: "<script>".into(),
            csrf: "secret",
            guild: text("en", "guild"),
            main: text("en", "main"),
            save: text("en", "save"),
            value: Preferences {
                comment: "<script>comment</script>".into(),
                ..Preferences::default()
            },
            labels: labels("en"),
        }
        .render()?;
        assert!(!html.contains("<script>"));
        assert!(html.contains("script"));
        assert!(html.contains("name=\"_csrf\""));
        assert!(html.contains("for=\"show_guild\""));
        assert!(html.contains("width=device-width"));
        Ok(())
    }
}
