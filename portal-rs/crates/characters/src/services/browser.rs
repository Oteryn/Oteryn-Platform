use super::*;
use askama::Template;
use axum::{
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
};
use fluent_bundle::{FluentBundle, FluentResource};
#[derive(Template)]
#[template(path = "services/manage.html", config = "src/services/askama.toml")]
struct Page {
    lang: String,
    character: String,
    name: String,
    csrf: String,
    schedule_id: String,
    cancel_id: String,
    rename_id: String,
    deletion_id: String,
    schedule: bool,
    cancel: bool,
    rename: bool,
    deadline: String,
    price: String,
    credit: bool,
    operations: Vec<(String, String)>,
}
impl Page {
    fn t(&self, key: &str) -> String {
        text(&self.lang, key)
    }
}
fn text(lang: &str, key: &str) -> String {
    let source = if lang == "en" {
        include_str!("../../../../templates/services/en.ftl")
    } else {
        include_str!("../../../../templates/services/pl.ftl")
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
fn error(lang: &str, e: Error) -> Response {
    private(
        (
            super::transport::status(&e),
            text(
                lang,
                match e {
                    Error::BadInput => "bad-input",
                    Error::Denied | Error::Unauthorized => "denied",
                    Error::Conflict => "conflict",
                    _ => "unavailable",
                },
            ),
        )
            .into_response(),
    )
}
/// Called from the web router after session authentication; no account is read from input.
pub async fn show(
    pool: &PgPool,
    actor: Option<Uuid>,
    character: &str,
    lang: &str,
    csrf: String,
) -> Response {
    show_with(
        &Services::new(pool.clone(), configured()),
        actor,
        character,
        lang,
        csrf,
    )
    .await
}
pub(super) async fn show_with(
    services: &Services,
    actor: Option<Uuid>,
    character: &str,
    lang: &str,
    csrf: String,
) -> Response {
    let pool = &services.pool;
    if !services.settings.enabled {
        return private(StatusCode::NOT_FOUND.into_response());
    }
    if !matches!(lang, "pl" | "en") {
        return error("en", Error::BadInput);
    }
    let Some(actor) = actor else {
        return private(Redirect::to(&format!("/{lang}/login")).into_response());
    };
    let result=async {
        let character=wire::identifier(character)?;
        let mut tx=pool.begin().await?;
        if Authority.state(&mut tx,actor).await!=oteryn_wallet::AccountState::Active {return Err(Error::Denied);}
        let state=services.owner_state(&mut tx,actor,character).await?;
        let current=now(&mut tx).await?;
        let credit:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM character_service_orders WHERE account_id=$1 AND character_id=$2 AND financial_state='RECOVERY_REQUIRED' AND credit_used_by IS NULL)").bind(actor).bind(character).fetch_one(&mut *tx).await?;
        let cancel_action=Action::Cancel{deletion_operation_id:state.deletion.map(|d|d.0).unwrap_or_default()};
        let operations = sqlx::query("SELECT c.id,c.state,c.receipt_json,o.financial_state FROM character_service_orders o JOIN character_commands c ON c.id=o.operation_id WHERE o.account_id=$1 AND o.character_id=$2 ORDER BY o.created_at DESC LIMIT 20").bind(actor).bind(character).fetch_all(&mut *tx).await?.into_iter().map(|r| -> Result<_, Error> {
            let key = if r.try_get::<String,_>("state")? == "PENDING" { "pending-state" } else if r.try_get::<String,_>("financial_state")? == "RECOVERY_REQUIRED" { "free-restore" } else {
                let raw: String = r.try_get("receipt_json")?;
                let v = wire::decode_value(raw.as_bytes(),1024)?;
                if wire::string(&v,"outcome")? == "COMMITTED" { "committed" } else { "rejected" }
            };
            Ok((r.try_get::<Uuid,_>("id")?.to_string(), text(lang,key)))
        }).collect::<Result<Vec<_>,_>>()?;
        let page=Page { operations,
            lang:lang.into(),character:character.to_string(),name:state.name.clone(),csrf,
            schedule_id:Uuid::now_v7().to_string(),cancel_id:Uuid::now_v7().to_string(),rename_id:Uuid::now_v7().to_string(),
            deletion_id:state.deletion.map(|d|d.0.to_string()).unwrap_or_default(),
            schedule:services.settings.permits(&Action::Schedule)&&state.permits(&Action::Schedule,current).is_ok(),
            cancel:services.settings.permits(&cancel_action)&&state.permits(&cancel_action,current).is_ok(),
            rename:services.settings.permits(&Action::Rename{requested_name:String::new()})&&state.lifecycle=="ACTIVE"&&(credit||services.settings.rename_price.as_ref().is_some_and(Price::valid)),
            deadline:state.deletion.map(|d|deadline(d.2)).transpose()?.unwrap_or_default(),
            price:services.settings.rename_price.as_ref().filter(|p|p.valid()).map(|p|p.coins.to_string()).unwrap_or_default(),credit,
        };
        page.render().map_err(|_|Error::Unavailable)
    }.await;
    match result {
        Ok(html) => private(Html(html).into_response()),
        Err(e) => error(lang, e),
    }
}
/// CSRF is enforced by web's session middleware before form extraction.
/// The recent-auth proof is derived from the same live server session, never form input.
pub async fn submit(
    pool: &PgPool,
    actor: Option<Uuid>,
    character: &str,
    lang: &str,
    recent: bool,
    fields: Vec<(String, String)>,
) -> Response {
    submit_with(
        &Services::new(pool.clone(), configured()),
        actor,
        character,
        lang,
        recent,
        fields,
    )
    .await
}
pub(super) async fn submit_with(
    services: &Services,
    actor: Option<Uuid>,
    character: &str,
    lang: &str,
    recent: bool,
    fields: Vec<(String, String)>,
) -> Response {
    if !services.settings.enabled {
        return private(StatusCode::NOT_FOUND.into_response());
    }
    if !matches!(lang, "pl" | "en") {
        return error("en", Error::BadInput);
    }
    let result = async {
        let actor = actor.ok_or(Error::Denied)?;
        if !recent {
            return Err(Error::Denied);
        }
        let request = parse_form(character, fields)?;
        let command = services.request(actor, request).await?;
        Ok(command.operation_id)
    }
    .await;
    match result {
        Ok(_) => private((StatusCode::ACCEPTED, text(lang, "pending")).into_response()),
        Err(e) => error(lang, e),
    }
}
fn parse_form(character: &str, fields: Vec<(String, String)>) -> Result<Request, Error> {
    let mut map = std::collections::BTreeMap::new();
    for (k, v) in fields {
        if map.insert(k, v).is_some() {
            return Err(Error::BadInput);
        }
    }
    let action = map.remove("action").ok_or(Error::BadInput)?;
    let request_id = wire::identifier(&map.remove("request_id").ok_or(Error::BadInput)?)?;
    let csrf = map.remove("_csrf").ok_or(Error::BadInput)?;
    if csrf.len() != 64 {
        return Err(Error::BadInput);
    }
    if map.remove("confirm").as_deref() != Some("yes") {
        return Err(Error::BadInput);
    }
    let accepted_price = if action == "rename" {
        Some(
            map.remove("accepted_price")
                .ok_or(Error::BadInput)?
                .parse::<i64>()
                .map_err(|_| Error::BadInput)?,
        )
    } else {
        None
    };
    let action = match action.as_str() {
        "schedule" => Action::Schedule,
        "cancel" => Action::Cancel {
            deletion_operation_id: wire::identifier(
                &map.remove("deletion_operation_id").ok_or(Error::BadInput)?,
            )?,
        },
        "rename" => Action::Rename {
            requested_name: map.remove("requested_name").ok_or(Error::BadInput)?,
        },
        _ => return Err(Error::BadInput),
    };
    if !map.is_empty() {
        return Err(Error::BadInput);
    }
    action_json(&action)?;
    Ok(Request {
        accepted_price,
        request_id,
        character_id: wire::identifier(character)?,
        action,
    })
}
/// Checks the live session selected by web, including idle/absolute expiry and
/// sudo. Duplicate cookies and mismatched account state fail closed.
pub async fn recent(pool: &PgPool, actor: Option<Uuid>, headers: &axum::http::HeaderMap) -> bool {
    let Some(actor) = actor else {
        return false;
    };
    let mut cookies = headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|h| h.split(';'))
        .filter_map(|s| s.trim().split_once('='))
        .filter(|(k, _)| *k == "__Host-oteryn_session");
    let Some((_, token)) = cookies.next() else {
        return false;
    };
    if cookies.next().is_some()
        || token.len() != 64
        || !token
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return false;
    }
    sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM web_sessions WHERE id_hash=$1 AND account_id=$2 AND sudo_until>now() AND idle_expires_at>now() AND absolute_expires_at>now())").bind(Sha256::digest(token.as_bytes()).to_vec()).bind(actor).fetch_one(pool).await.unwrap_or(false)
}
pub fn configured() -> Settings {
    let yes = |key| std::env::var(key).as_deref() == Ok("true");
    // Freshness is explicitly qualified only for testing/preproduction here.
    let qualified = matches!(
        std::env::var("OTERYN_ENV").as_deref(),
        Ok("testing" | "preprod")
    );
    Settings {
        enabled: yes("OTERYN_CHARACTER_SERVICES_ENABLED"),
        schedule_deletion: yes("OTERYN_CHARACTER_SERVICES_SCHEDULE_ENABLED"),
        cancel_deletion: yes("OTERYN_CHARACTER_SERVICES_CANCEL_ENABLED"),
        rename: yes("OTERYN_CHARACTER_SERVICES_RENAME_ENABLED"),
        projection_enabled: yes("OTERYN_CHARACTER_SERVICES_PROJECTION_ENABLED"),
        exchange_enabled: yes("OTERYN_CHARACTER_SERVICES_EXCHANGE_ENABLED"),
        wallet_enabled: yes("OTERYN_WALLET_ENABLED"),
        command_authority: std::env::var("OTERYN_CHARACTER_SERVICES_COMMAND_AUTHORITY")
            .unwrap_or_default(),
        projection_authority: std::env::var("OTERYN_CHARACTER_SERVICES_PROJECTION_AUTHORITY")
            .unwrap_or_default(),
        clock_uncertainty_seconds: if qualified {
            std::env::var("OTERYN_CHARACTER_SERVICES_CLOCK_UNCERTAINTY_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
        } else {
            None
        },
        freshness_seconds: qualified.then_some(30),
        rename_price: price(),
    }
}
fn price() -> Option<Price> {
    Some(Price {
        product_id: std::env::var("OTERYN_CHARACTER_RENAME_PRODUCT_ID").ok()?,
        version: std::env::var("OTERYN_CHARACTER_RENAME_PRODUCT_VERSION")
            .ok()?
            .parse()
            .ok()?,
        coins: std::env::var("OTERYN_CHARACTER_RENAME_PRICE")
            .ok()?
            .parse()
            .ok()?,
    })
}

fn deadline(value: u64) -> Result<String, Error> {
    let value = i64::try_from(value).map_err(|_| Error::Unavailable)?;
    let date = sqlx::types::time::OffsetDateTime::from_unix_timestamp(value)
        .map_err(|_| Error::Unavailable)?;
    Ok(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        date.year(),
        u8::from(date.month()),
        date.day(),
        date.hour(),
        date.minute(),
        date.second()
    ))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn form_rejects_duplicates_unknown_actions_and_bad_names() {
        let c = Uuid::now_v7().to_string();
        let fields = || {
            vec![
                ("_csrf".into(), "a".repeat(64)),
                ("action".into(), "rename".into()),
                ("request_id".into(), Uuid::now_v7().to_string()),
                ("confirm".into(), "yes".into()),
                ("requested_name".into(), "Aldric Vale".into()),
                ("accepted_price".into(), "70".into()),
            ]
        };
        assert!(parse_form(&c, fields()).is_ok());
        let mut f = fields();
        f.push(("action".into(), "schedule".into()));
        assert!(parse_form(&c, f).is_err());
        let mut f = fields();
        f[1].1 = "sex".into();
        assert!(parse_form(&c, f).is_err());
        let mut f = fields();
        f[4].1 = " A1".into();
        assert!(parse_form(&c, f).is_err());
    }
    #[test]
    fn both_locales_render_accessible_forms_and_unavailable_services() {
        for lang in ["pl", "en"] {
            let page = Page {
                lang: lang.into(),
                character: Uuid::now_v7().to_string(),
                name: "Aldric".into(),
                csrf: "a".repeat(64),
                schedule_id: Uuid::now_v7().to_string(),
                cancel_id: Uuid::now_v7().to_string(),
                rename_id: Uuid::now_v7().to_string(),
                deletion_id: String::new(),
                schedule: true,
                cancel: false,
                rename: true,
                deadline: String::new(),
                price: "100".into(),
                credit: false,
                operations: vec![],
            };
            if let Ok(dir) = std::env::var("PORTAL_T46_RENDER_DIR") {
                std::fs::write(
                    std::path::Path::new(&dir).join(format!("{lang}.html")),
                    page.render().unwrap(),
                )
                .unwrap();
            }
            let html = page.render().unwrap();
            assert!(html.contains("name=\"_csrf\""));
            assert!(html.contains("name=\"confirm\""));
            assert!(html.contains("disabled"));
            assert!(!page.t("unavailable").is_empty());
        }
    }
}
