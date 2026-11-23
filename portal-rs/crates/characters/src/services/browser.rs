use super::*;
use askama::Template;
use axum::{
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
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
    remaining: String,
    lifecycle: &'static str,
    sudo: bool,
    price: String,
    credit: bool,
    operations: Vec<Operation>,
}
impl Page {
    fn t(&self, key: &str) -> String {
        text(&self.lang, key)
    }
}
pub(super) fn text(lang: &str, key: &str) -> String {
    localized(lang, key, None)
}

fn localized(lang: &str, key: &str, args: Option<&FluentArgs<'_>>) -> String {
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
        .map(|p| bundle.format_pattern(p, args, &mut vec![]).into_owned())
        .unwrap_or_default()
}
pub(super) fn private(mut response: Response) -> Response {
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
pub(super) fn error(lang: &str, e: Error) -> Response {
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
    let result = async {
        let character = wire::identifier(character)?;
        let mut tx = pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await?;
        if Authority.state(&mut tx, actor).await != oteryn_wallet::AccountState::Active {
            return Err(Error::Denied);
        }
        let state = services.owner_state(&mut tx, actor, character).await?;
        let current = services.latest_game_time(&mut tx).await?;
        let waiting = awaiting_projection(&mut tx, character, state.character_revision).await?;
        let blocked = waiting || has_conflicting_operation(&mut tx, character).await?;
        let credit: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM character_service_orders WHERE account_id=$1 AND character_id=$2
                   AND financial_state='RECOVERY_REQUIRED' AND credit_used_by IS NULL)",
        )
        .bind(actor)
        .bind(character)
        .fetch_one(&mut *tx)
        .await?;
        let operations = history(&mut tx, actor, character).await?;
        let (deletion_id, effective) = state.deletion
            .map(|(id, _, effective)| (id, Some(effective)))
            .unwrap_or_default();
        let cancel_action = Action::Cancel { deletion_operation_id: deletion_id };
        let rename_action = Action::Rename { requested_name: String::new() };
        let permits = |action: &Action| {
            !blocked && services.settings.permits(action) && state.permits(action, current).is_ok()
        };
        let price = services.settings.rename_price.as_ref().filter(|p| p.valid());
        let page = Page {
            operations,
            lang: lang.into(),
            character: character.to_string(),
            name: state.name.clone(),
            csrf,
            schedule_id: Uuid::now_v7().to_string(),
            cancel_id: Uuid::now_v7().to_string(),
            rename_id: Uuid::now_v7().to_string(),
            deletion_id: deletion_id.to_string(),
            schedule: permits(&Action::Schedule),
            cancel: permits(&cancel_action),
            rename: permits(&rename_action) && (credit || price.is_some()),
            deadline: effective.filter(|_| !waiting).map(deadline).transpose()?.unwrap_or_default(),
            remaining: effective.filter(|_| !waiting).map(|date| remaining(lang, current, date)).unwrap_or_default(),
            lifecycle: if waiting {
                "awaiting-projection"
            } else if let Some(date) = effective {
                if current < date { "deletion-pending" } else { "deletion-deadline-reached" }
            } else {
                "active"
            },
            price: price.map(|p| p.coins.to_string()).unwrap_or_default(),
            credit,
            sudo: oteryn_identity::navigation::sudo_enabled(),
        };
        page.render().map_err(|_| Error::Unavailable)
    }
    .await;
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
    recent: Option<SudoProof>,
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
    recent: Option<SudoProof>,
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
        let proof = recent.ok_or(Error::Denied)?;
        if proof.account != actor {
            return Err(Error::Denied);
        }
        let request = parse_form(character, fields)?;
        let command = services
            .request_in_session(actor, request, Some(&proof))
            .await?;
        Ok(Submitted {
            lang,
            character,
            operation: command.operation_id,
        })
    }
    .await;
    match result {
        Ok(page) => match page.render() {
            Ok(html) => private((StatusCode::ACCEPTED, Html(html)).into_response()),
            Err(_) => error(lang, Error::Unavailable),
        },
        Err(e) => error(lang, e),
    }
}
#[derive(Template)]
#[template(path = "services/submitted.html", config = "src/services/askama.toml")]
struct Submitted<'a> {
    lang: &'a str,
    character: &'a str,
    operation: Uuid,
}

impl Submitted<'_> {
    fn t(&self, key: &str) -> String {
        text(self.lang, key)
    }
}

struct Operation {
    id: Uuid,
    action: &'static str,
    outcome: &'static str,
    detail: Option<String>,
    financial: Option<&'static str>,
    amount: Option<i64>,
}

async fn history(
    tx: &mut Transaction<'_, Postgres>,
    account: Uuid,
    character: Uuid,
) -> Result<Vec<Operation>, Error> {
    sqlx::query(
        "SELECT c.id,c.command,c.state,c.receipt_json,o.financial_state,o.amount,
                o.credit_from IS NOT NULL AS replacement,o.credit_used_by IS NOT NULL AS claimed,
                (c.state='PENDING' AND c.updated_at <= now()-interval '300 seconds')
                OR EXISTS (SELECT 1 FROM character_restore_reopens r
                           WHERE r.operation_id=c.id AND r.new_receipt_id IS NULL)
                OR EXISTS (SELECT 1 FROM character_restore_queue q
                           WHERE q.operation_id=c.id AND q.resolved_at IS NULL) AS review
         FROM character_service_orders o JOIN character_commands c ON c.id=o.operation_id
         WHERE o.account_id=$1 AND o.character_id=$2
         ORDER BY o.created_at DESC,o.operation_id DESC LIMIT 20",
    )
    .bind(account)
    .bind(character)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|row| {
        let command: String = row.try_get("command")?;
        let action = match command.as_str() {
            wire::RENAME => "rename",
            wire::SCHEDULE => "schedule",
            wire::CANCEL => "cancel",
            _ => return Err(Error::Unavailable),
        };
        let (outcome, detail) = if row.try_get("review")? {
            ("reconciling", None)
        } else if row.try_get::<String, _>("state")? == "PENDING" {
            ("pending-state", None)
        } else {
            let raw: String = row.try_get("receipt_json")?;
            let receipt = wire::receipt(raw.as_bytes())?;
            if wire::string(&receipt, "outcome")? == "COMMITTED" {
                let detail = match command.as_str() {
                    wire::RENAME => Some(wire::string(&receipt, "name")?.to_owned()),
                    wire::SCHEDULE => Some(deadline(wire::number(
                        wire::string(&receipt, "deletion_effective_at")?,
                        false,
                    )?)?),
                    _ => None,
                };
                ("committed", detail)
            } else {
                ("rejected", None)
            }
        };
        let financial = match row.try_get::<String, _>("financial_state")?.as_str() {
            "RESERVED" => Some("coins-reserved"),
            "SETTLED" => Some("coins-settled"),
            "RELEASED" => Some("coins-released"),
            "RECOVERY_REQUIRED" if row.try_get("claimed")? => Some("credit-claimed"),
            "RECOVERY_REQUIRED" => Some("free-restore"),
            "FREE" if row.try_get("replacement")? => Some("replacement"),
            "FREE" => None,
            _ => return Err(Error::Unavailable),
        };
        Ok(Operation {
            id: row.try_get("id")?,
            action,
            outcome,
            detail,
            financial,
            amount: if matches!(
                financial,
                Some("coins-reserved" | "coins-settled" | "coins-released")
            ) {
                row.try_get("amount")?
            } else {
                None
            },
        })
    })
    .collect()
}

fn remaining(lang: &str, current: u64, effective: u64) -> String {
    let remaining = effective.saturating_sub(current);
    if remaining == 0 {
        return String::new();
    }
    if remaining < 60 {
        return text(lang, "grace-final-minute");
    }
    let mut args = FluentArgs::new();
    args.set("days", remaining / 86400);
    args.set("hours", (remaining / 3600) % 24);
    args.set("minutes", (remaining / 60) % 60);
    localized(lang, "grace-remaining", Some(&args))
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
pub struct SudoProof {
    id_hash: Vec<u8>,
    account: Uuid,
}

impl std::fmt::Debug for SudoProof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SudoProof").finish_non_exhaustive()
    }
}

impl SudoProof {
    pub(super) async fn revalidate(&self, tx: &mut PgConnection) -> Result<(), Error> {
        sqlx::query(
            "SELECT 1 FROM web_sessions WHERE id_hash=$1 AND account_id=$2
            AND sudo_until>clock_timestamp() AND idle_expires_at>clock_timestamp()
            AND absolute_expires_at>clock_timestamp() FOR SHARE",
        )
        .bind(&self.id_hash)
        .bind(self.account)
        .fetch_optional(tx)
        .await?
        .ok_or(Error::Denied)?;
        Ok(())
    }
}

/// Checks the live session selected by web, including idle/absolute expiry and
/// sudo. Duplicate cookies and mismatched account state fail closed.
pub async fn recent(
    pool: &PgPool,
    actor: Option<Uuid>,
    headers: &axum::http::HeaderMap,
) -> Option<SudoProof> {
    let actor = actor?;
    let mut cookies = headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|h| h.split(';'))
        .filter_map(|s| s.trim().split_once('='))
        .filter(|(k, _)| *k == "__Host-oteryn_session");
    let (_, token) = cookies.next()?;
    if cookies.next().is_some()
        || token.len() != 64
        || !token
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return None;
    }
    let id_hash = Sha256::digest(token.as_bytes()).to_vec();
    sqlx::query(
        "SELECT 1 FROM web_sessions WHERE id_hash=$1 AND account_id=$2
        AND sudo_until>now() AND idle_expires_at>now() AND absolute_expires_at>now()",
    )
    .bind(&id_hash)
    .bind(actor)
    .fetch_optional(pool)
    .await
    .ok()??;
    Some(SudoProof {
        id_hash,
        account: actor,
    })
}
pub(crate) fn browser_enabled() -> bool {
    std::env::var("OTERYN_CHARACTER_SERVICES_ENABLED").as_deref() == Ok("true")
}

pub fn configured() -> Settings {
    let yes = |key| std::env::var(key).as_deref() == Ok("true");
    // Freshness is explicitly qualified only for testing/preproduction here.
    let qualified = matches!(
        std::env::var("OTERYN_ENV").as_deref(),
        Ok("testing" | "preprod")
    );
    Settings {
        enabled: browser_enabled(),
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
                remaining: String::new(),
                lifecycle: "active",
                sudo: true,
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
            assert!(html.contains(&page.t("channel-transfer")));
            assert!(html.contains("id=\"cancellation\""));
            assert!(html.contains("href=\"/account\""));
            assert!(!page.t("unavailable").is_empty());
        }
    }
}
