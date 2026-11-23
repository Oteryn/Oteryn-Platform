//! Private account overview. Missing producer facts remain explicitly unavailable.
use crate::lcfa::{AccountId, AccountState, AccountStateLookup, Projection, ReadState, Settings};
mod display;
mod game;
mod wallet;
use askama::Template;
use axum::{
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
};
pub use display::{DisplaySession, display_routes};
use fluent_bundle::{FluentBundle, FluentResource};
use sqlx::PgPool;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

/// Identity account state for LCFA reads: only `active` reads. Callers that
/// also need sanctions apply them before and after the read (player API).
#[derive(Debug, Clone)]
pub struct Accounts(pub PgPool);
impl AccountStateLookup for Accounts {
    fn state(&self, account: AccountId) -> Pin<Box<dyn Future<Output = AccountState> + Send + '_>> {
        Box::pin(async move {
            match sqlx::query_scalar::<_, String>("SELECT status FROM accounts WHERE id=$1")
                .bind(account.as_uuid())
                .fetch_optional(&self.0)
                .await
            {
                Ok(Some(status)) if status == "active" => AccountState::Active,
                Ok(_) => AccountState::Denied,
                Err(_) => AccountState::Unavailable,
            }
        })
    }
}

#[derive(Template)]
#[template(path = "account/overview.html", config = "src/overview/askama.toml")]
struct Page {
    lang: &'static str,
    title: String,
    facts: Vec<(String, String)>,
    characters_label: String,
    main_label: String,
    state: String,
    characters: Vec<Character>,
    verified_through: Option<String>,
    verified_label: String,
    observed_label: String,
    empty: Option<String>,
    game_history: Option<String>,
    game_login: String,
    unavailable: String,
    world: String,
    availability: String,
    level: String,
    profession: String,
    shortcuts: String,
    security: String,
    security_shortcut: Option<String>,
    security_shortcut_label: String,
    security_actions: Vec<oteryn_ui::NavLink>,
    links: Vec<Link>,
}
struct Character {
    name: String,
    world: String,
    availability: String,
    main: bool,
    profile: Option<String>,
    level: String,
    vocation: String,
    observed: Option<String>,
    links: Vec<Link>,
}

struct Link {
    url: String,
    label: String,
}

fn text(lang: &str, key: &str) -> String {
    let source = if lang == "en" {
        include_str!("../../../../templates/account/overview.en.ftl")
    } else {
        include_str!("../../../../templates/account/overview.pl.ftl")
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
fn segment(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
/// Called only with the account resolved by the server-side browser session.
/// No account identifier is accepted from the URL or request body.
pub async fn show(
    pool: &PgPool,
    account: Option<Uuid>,
    lang: &str,
    enabled: bool,
    settings: Settings,
) -> Response {
    let profile_authority = game::configured_profiles();
    private(
        load(
            pool,
            account,
            lang,
            enabled,
            settings,
            profile_authority.as_deref(),
        )
        .await,
    )
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
async fn load(
    pool: &PgPool,
    account: Option<Uuid>,
    lang: &str,
    enabled: bool,
    settings: Settings,
    profile_authority: Option<&str>,
) -> Response {
    if !enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    let lang = match lang {
        "pl" => "pl",
        "en" => "en",
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let Some(account) = account else {
        return Redirect::to(&format!("/{lang}/login")).into_response();
    };
    let Ok(id) = AccountId::parse(&account.to_string()) else {
        return StatusCode::FORBIDDEN.into_response();
    };
    let record = sqlx::query_as::<_, (String, String, bool, bool, String, String)>(
        "SELECT status, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') || ' UTC',
         email_verified_at IS NOT NULL, COALESCE(login_mfa_required, true), display_name, locale
         FROM accounts WHERE id=$1",
    )
    .bind(account)
    .fetch_optional(pool)
    .await;
    let (status, created, verified, mfa_required, display_name, display_locale) = match record {
        Ok(Some(row)) if row.0 == "active" => row,
        Ok(_) => return StatusCode::FORBIDDEN.into_response(),
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let enabled = |key| std::env::var(key).as_deref() == Ok("true");
    let profiles_enabled = enabled("OTERYN_CHARACTER_PROFILES_ENABLED");
    let profiles = crate::profile::Service::new(pool.clone(), settings.clone(), profiles_enabled);
    let projection = Projection::new(
        pool.clone(),
        settings.clone(),
        Arc::new(Accounts(pool.clone())),
    );
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|v| i64::try_from(v.as_secs()).ok());
    let view = match now {
        Some(now) => projection.overview(id, now).await.ok(),
        None => None,
    };
    if view.as_ref().is_some_and(|v| v.state == ReadState::Denied) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let read_state = view.as_ref().map(|view| view.state);
    let t = |key| text(lang, key);
    let mut facts = vec![(t("created"), created), (t("status"), t(&status))];
    facts.push((
        t("display-name"),
        if display_name.is_empty() {
            t("display-unset")
        } else {
            display_name
        },
    ));
    facts.push((
        t("display-locale"),
        t(if display_locale == "pl" {
            "display-pl"
        } else {
            "display-en"
        }),
    ));
    facts.push((
        t("web-login"),
        last_website_login(pool, account)
            .await
            .unwrap_or_else(|| t("unavailable")),
    ));
    for key in ["loyalty", "title"] {
        facts.push((t(key), t("unavailable")));
    }
    let pending = oteryn_identity::termination::pending_closure(pool, account).await;
    let closure_state = match pending {
        Ok(true) => "termination-pending",
        Ok(false) => "termination-none",
        Err(_) => "unavailable",
    };
    facts.push((t("termination"), t(closure_state)));
    let limits = configured_limits();
    facts.push((
        t("limit"),
        limits
            .map(|(active, total)| format!("{active} / {total}"))
            .unwrap_or_else(|| t("unavailable")),
    ));
    facts.push((
        t("balance"),
        wallet::balance(pool, account)
            .await
            .map(|value| value.to_string())
            .unwrap_or_else(|| t("unavailable")),
    ));
    let main = match now {
        Some(now) => profiles.main_character(account, now).await,
        None => None,
    };
    let ready = view.filter(|v| v.state == ReadState::Ready);
    let verified_through = ready
        .as_ref()
        .and_then(|view| view.verified_through_utc.clone());
    let mut game_profiles = match (&ready, profile_authority) {
        (Some(view), Some(authority)) if !view.characters.is_empty() => {
            game::profiles(pool, account, &settings, authority)
                .await
                .unwrap_or_default()
        }
        _ => Default::default(),
    };
    let mut characters = Vec::new();
    let services = crate::services::Services::new(pool.clone(), crate::services::configured());
    let public_profiles = enabled("PORTAL_PUBLIC_PROFILES_ENABLED");
    let items_source = std::env::var("PORTAL_PLAYERDATA_G10_SOURCE_AUTHORITY")
        .is_ok_and(|source| !source.is_empty());
    let items = items_source && enabled("PLAYERDATA_G10_ITEMS_ENABLED");
    let history = items_source && enabled("PLAYERDATA_G10_HISTORY_ENABLED");
    if let Some(view) = ready {
        for c in view.characters {
            let name = segment(&c.name);
            let character = c.character_id.as_uuid();
            let world = c.world_id.to_string();
            let available = c.availability == crate::lcfa::Availability::Available;
            let mut links = Vec::new();
            if profiles_enabled && available {
                links.push(Link {
                    url: format!("/{lang}/account/characters/{name}/profile"),
                    label: t("profile-preferences"),
                });
            }
            if services.has_owner_view(account, character).await {
                links.push(Link {
                    url: format!("/{lang}/account/characters/{character}/services"),
                    label: t("character-services"),
                });
            }
            if items && available {
                links.push(Link {
                    url: format!("/{lang}/panel/items/{world}/{character}"),
                    label: t("character-items"),
                });
            }
            if history && available {
                for (family, label) in [("market", "market-history"), ("trades", "trade-history")] {
                    links.push(Link {
                        url: format!("/{lang}/panel/history/{world}/{family}/{character}"),
                        label: t(label),
                    });
                }
            }
            let profile = game_profiles
                .remove(&character)
                .filter(|p| p.world_id == c.world_id.as_uuid() && p.name == c.name);
            let level = profile
                .as_ref()
                .and_then(|p| p.level.as_deref())
                .and_then(|value| value.parse::<u64>().ok())
                .filter(|&value| value > 0);
            let vocation = profile
                .as_ref()
                .and_then(|p| p.vocation.as_deref())
                .and_then(game::vocation_key);
            characters.push(Character {
                name: c.name,
                world,
                availability: t(if available {
                    "available"
                } else {
                    "unavailable"
                }),
                main: main == Some(character),
                profile: (public_profiles && available)
                    .then(|| format!("/{lang}/characters/{name}")),
                    level: level.map_or_else(|| t("unavailable"), |n| n.to_string()),
                    vocation: t(vocation.unwrap_or("unavailable")),
                    observed: profile
                        .filter(|_| level.is_some() || vocation.is_some())
                        .map(|p| p.observed),
                links,
            });
        }
    }
    let links = [
        (
            enabled("PANEL_PREMIUM_PAYMENTS"),
            "panel/premium",
            "premium",
        ),
        (enabled("OTERYN_SHOP_ENABLED"), "shop/history", "balance"),
        (true, "account/display", "display-settings"),
    ]
    .into_iter()
    .filter(|(mounted, _, _)| *mounted)
    .map(|(_, path, label)| Link {
        url: format!("/{lang}/{path}"),
        label: t(label),
    })
    .collect();
    let empty =
        (read_state == Some(ReadState::Ready) && characters.is_empty()).then(|| t("roster-empty"));
    let game_history = (std::env::var("OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED").as_deref()
        == Ok("true")
        && std::env::var("OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED").as_deref() == Ok("true")
        && std::env::var("PORTAL_PLAYERDATA_PROGRESS_SOURCE_AUTHORITY")
            .is_ok_and(|value| !value.is_empty()))
    .then(|| t("game-history"));
    let page = Page {
        lang,
        title: t("overview"),
        facts,
        characters_label: t("characters"),
        main_label: t("main-character"),
        state: t(match read_state {
            Some(ReadState::Ready) => "linked",
            Some(ReadState::Missing) => "missing",
            Some(ReadState::Stale) => "stale",
            Some(ReadState::Invalid) => "invalid",
            _ => "unavailable",
        }),
        characters,
        verified_through,
        verified_label: t("verified-through"),
        observed_label: t("profile-observed"),
        empty,
        game_history,
        game_login: t("game-login"),
        unavailable: t("unavailable"),
        world: t("world"),
        availability: t("availability"),
        level: t("level"),
        profession: t("profession"),
        shortcuts: t("shortcuts"),
        security: t("security"),
        security_shortcut: oteryn_identity::navigation::sudo_enabled()
            .then(|| oteryn_identity::navigation::SudoReturn::Security.path(lang)),
        security_shortcut_label: t("security-shortcut"),
        security_actions: oteryn_identity::navigation::overview_actions(
            lang,
            oteryn_identity::passkeys::account_eligible(&status, verified, mfa_required),
            matches!(pending, Ok(true)),
        ),
        links,
    };
    match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn last_website_login(pool: &PgPool, account: Uuid) -> Option<String> {
    sqlx::query_scalar(
        "SELECT to_char(created_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') || ' UTC'
         FROM security_events WHERE account_id=$1 AND kind='login_succeeded'
           AND created_at<=statement_timestamp()
         ORDER BY created_at DESC, id DESC LIMIT 1",
    )
    .bind(account)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

#[cfg(test)]
mod tests;

/// Projection settings retain the LCFA environment and authority restrictions.
pub fn configured_projection() -> Settings {
    Settings {
        enabled: std::env::var("OTERYN_LCFA_ENABLED").as_deref() == Ok("true"),
        environment: match std::env::var("OTERYN_ENV").as_deref() {
            Ok("testing") => oteryn_core::Environment::Testing,
            Ok("preprod") => oteryn_core::Environment::Preprod,
            _ => oteryn_core::Environment::Production,
        },
        source_authority: std::env::var("OTERYN_LCFA_SOURCE_AUTHORITY").unwrap_or_default(),
        ..Settings::default()
    }
}

/// Missing, inconsistent or malformed limits never become an unlimited allowance.
pub fn configured_limits() -> Option<(u16, u16)> {
    parse_limits(
        &std::env::var("OTERYN_ACTIVE_CHARACTER_LIMIT").ok()?,
        &std::env::var("OTERYN_TOTAL_CHARACTER_LIMIT").ok()?,
    )
}
fn parse_limits(active: &str, total: &str) -> Option<(u16, u16)> {
    let active = active.parse::<u16>().ok()?;
    let total = total.parse::<u16>().ok()?;
    (active > 0 && total >= active).then_some((active, total))
}
