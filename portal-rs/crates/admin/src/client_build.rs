//! Read-only qualification of the proposed minimum client build policy.
use crate::{
    rbac::AccountStateReader,
    web::{Panel, StaffSession, language, message, messages},
};
use askama::Template;
use axum::{
    extract::{RawQuery, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use fluent_bundle::{FluentBundle, FluentResource};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;

pub(crate) const PATH: &str = "/admin/client-build";
const MAX_SEQUENCE: i64 = (1 << 53) - 1;

#[derive(Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Channel {
    #[default]
    Stable,
    Beta,
}

impl Channel {
    const ALL: [Self; 2] = [Self::Stable, Self::Beta];

    fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Open,
    #[default]
    Enforcing,
    Maintenance,
}

impl Mode {
    const ALL: [Self; 3] = [Self::Open, Self::Enforcing, Self::Maintenance];

    fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Enforcing => "enforcing",
            Self::Maintenance => "maintenance",
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Unknown {
    #[default]
    Admit,
    Refuse,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Locale {
    Pl,
    En,
}

impl Locale {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pl => "pl",
            Self::En => "en",
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Form {
    lang: Option<Locale>,
    channel: Channel,
    release: String,
    sequence: String,
    minimum: String,
    mode: Mode,
    unknown: Unknown,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid client build qualification input")]
struct Invalid;

struct Offer<'a> {
    release: &'a str,
    sequence: i64,
}

struct Scenario<'a> {
    offer: Option<Offer<'a>>,
    minimum: Option<i64>,
    mode: Mode,
    unknown: Unknown,
}

fn sequence(value: &str) -> Result<Option<i64>, Invalid> {
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 16 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Invalid);
    }
    value
        .parse::<i64>()
        .ok()
        .filter(|value| (1..=MAX_SEQUENCE).contains(value))
        .map(Some)
        .ok_or(Invalid)
}

fn valid_release(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && !matches!(value, "." | "..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

impl Form {
    fn parse(query: &str) -> Result<Self, Invalid> {
        if query.len() > 1024 {
            return Err(Invalid);
        }
        serde_urlencoded::from_str(query).map_err(|_| Invalid)
    }

    fn scenario(&self) -> Result<Scenario<'_>, Invalid> {
        let offer = match (self.release.as_str(), sequence(&self.sequence)?) {
            ("", None) => None,
            (release, Some(sequence)) if valid_release(release) => {
                Some(Offer { release, sequence })
            }
            _ => return Err(Invalid),
        };
        Ok(Scenario {
            offer,
            minimum: sequence(&self.minimum)?,
            mode: self.mode,
            unknown: self.unknown,
        })
    }

    fn refuses_unknown(&self) -> bool {
        self.unknown == Unknown::Refuse
    }
}

// Decode the fields needed from the immutable TUF-verified Downloads policy.
#[derive(Deserialize)]
struct Policy {
    schema_version: u32,
    policy_revision: i64,
    channel: Channel,
    current_release_id: String,
    current_release_sequence: i64,
    minimum_supported_release_sequence: i64,
    revoked_release_ids: Vec<String>,
    artifacts: Vec<Artifact>,
}

#[derive(Deserialize)]
struct Artifact {
    platform: String,
    architecture: String,
}

struct Head {
    policy: Policy,
    fresh: bool,
    registered: bool,
}

impl Head {
    fn current(&self) -> Option<i64> {
        self.fresh.then_some(self.policy.current_release_sequence)
    }

    fn lists_installer(&self) -> bool {
        self.policy
            .artifacts
            .iter()
            .any(|artifact| artifact.platform == "windows" && artifact.architecture == "x86_64")
    }

    fn state(&self) -> &'static str {
        if !self.fresh {
            "build-expired"
        } else if self.lists_installer() {
            "build-installable"
        } else {
            "build-no-installer"
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ReadError {
    #[error("release publications are unavailable")]
    Storage(#[from] sqlx::Error),
    #[error("the published release does not match its policy")]
    Policy,
}

struct Publication {
    channel: Channel,
    head: Option<Head>,
}

type PublicationRow = (String, Vec<u8>, String, i64, i64, bool, bool);

async fn publications(
    pool: &PgPool,
    channel: Channel,
    offer: Option<&Offer<'_>>,
) -> Result<Vec<Publication>, ReadError> {
    let rows: Vec<PublicationRow> = sqlx::query_as(
        "SELECT c.name, r.policy, r.release_id, r.release_sequence, r.policy_revision,
            r.expires_at > now(),
            c.name = $1 AND EXISTS (
                SELECT 1 FROM client_releases history
                WHERE history.channel_id = c.id
                  AND history.release_id = $2 AND history.release_sequence = $3
            )
         FROM client_channels c JOIN client_releases r
           ON r.channel_id = c.id AND r.id = c.active_publication_id
         WHERE c.name IN ('stable', 'beta')",
    )
    .bind(channel.as_str())
    .bind(offer.map(|offer| offer.release))
    .bind(offer.map(|offer| offer.sequence))
    .fetch_all(pool)
    .await?;
    Channel::ALL
        .into_iter()
        .map(|channel| {
            let head = rows
                .iter()
                .find(|(name, ..)| name == channel.as_str())
                .map(|(_, bytes, id, sequence, revision, fresh, registered)| {
                    decode_head(
                        channel,
                        (bytes, id, *sequence, *revision, *fresh, *registered),
                    )
                })
                .transpose()?;
            Ok(Publication { channel, head })
        })
        .collect()
}

fn decode_head(
    channel: Channel,
    (bytes, id, sequence, revision, fresh, registered): (&[u8], &str, i64, i64, bool, bool),
) -> Result<Head, ReadError> {
    let policy: Policy = serde_json::from_slice(bytes).map_err(|_| ReadError::Policy)?;
    if policy.schema_version != 1
        || policy.policy_revision != revision
        || policy.channel != channel
        || policy.current_release_id != id
        || policy.current_release_sequence != sequence
        || !(1..=MAX_SEQUENCE).contains(&sequence)
        || !(1..=sequence).contains(&policy.minimum_supported_release_sequence)
    {
        return Err(ReadError::Policy);
    }
    Ok(Head {
        policy,
        fresh,
        registered,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Eligible,
    Update,
    LegacyUpdate,
    Maintenance,
    LegacyMaintenance,
    Unverified,
    Unavailable,
}

impl Outcome {
    fn label(self) -> &'static str {
        match self {
            Self::Eligible => "build-eligible",
            Self::Update => "build-update",
            Self::LegacyUpdate => "build-legacy-update",
            Self::Maintenance | Self::LegacyMaintenance => "build-maintenance",
            Self::Unverified => "build-unverified",
            Self::Unavailable => "build-unavailable",
        }
    }

    fn code(self) -> Option<&'static str> {
        match self {
            Self::Update => Some("NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED"),
            Self::LegacyUpdate => Some("NATIVE_LOGIN_OFFER_UNSUPPORTED"),
            Self::Maintenance => Some("NATIVE_LOGIN_MAINTENANCE"),
            Self::LegacyMaintenance => Some("NATIVE_LOGIN_ROUTE_UNAVAILABLE"),
            Self::Eligible | Self::Unverified | Self::Unavailable => None,
        }
    }
}

struct Preview {
    outcome: Outcome,
    minimum: Option<i64>,
    current: Option<i64>,
    missing_update: bool,
}

fn preview(scenario: &Scenario<'_>, head: Option<&Head>) -> Preview {
    let current = head.and_then(Head::current);
    let published_minimum = head.map(|head| head.policy.minimum_supported_release_sequence);
    let minimum = match scenario.mode {
        Mode::Open => published_minimum,
        Mode::Enforcing | Mode::Maintenance => published_minimum.max(scenario.minimum),
    };
    let missing_update = scenario.mode == Mode::Enforcing
        && minimum.is_some_and(|minimum| {
            !head.is_some_and(|head| {
                head.lists_installer() && current.is_some_and(|current| current >= minimum)
            })
        });
    let outcome = match (&scenario.offer, head) {
        (None, _) => match (scenario.mode, scenario.unknown) {
            (Mode::Maintenance, _) => Outcome::LegacyMaintenance,
            (Mode::Enforcing, Unknown::Refuse) => Outcome::LegacyUpdate,
            _ => Outcome::Eligible,
        },
        (Some(offer), _) if scenario.mode == Mode::Maintenance => {
            if current.is_some_and(|current| current > offer.sequence) {
                Outcome::Update
            } else {
                Outcome::Maintenance
            }
        }
        (Some(_), None) => Outcome::Unavailable,
        (Some(offer), Some(head))
            if head
                .policy
                .revoked_release_ids
                .iter()
                .any(|id| id == offer.release) =>
        {
            Outcome::Update
        }
        (Some(_), Some(head)) if !head.registered => Outcome::Unverified,
        (Some(offer), Some(_)) if minimum.is_some_and(|minimum| offer.sequence < minimum) => {
            Outcome::Update
        }
        (Some(_), Some(_)) => Outcome::Eligible,
    };
    Preview {
        outcome,
        minimum,
        current,
        missing_update,
    }
}

#[derive(Template)]
#[template(path = "admin/client_build.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    messages: FluentBundle<FluentResource>,
    form: Form,
    publications: Vec<Publication>,
    result: Option<Preview>,
    error: Option<&'static str>,
}

impl Page {
    fn tr(&self, key: &str) -> String {
        message(&self.messages, key)
    }

    fn channels(&self) -> &'static [Channel] {
        &Channel::ALL
    }

    fn modes(&self) -> &'static [Mode] {
        &Mode::ALL
    }

    fn mode_label(&self, mode: &Mode) -> String {
        self.tr(match mode {
            Mode::Open => "build-open",
            Mode::Enforcing => "build-enforcing",
            Mode::Maintenance => "build-maintenance",
        })
    }
}

pub(crate) async fn page<I: StaffSession, A: AccountStateReader>(
    State(state): State<Arc<Panel<I, A>>>,
    headers: HeaderMap,
    RawQuery(raw): RawQuery,
) -> Response {
    let form = Form::parse(raw.as_deref().unwrap_or_default());
    let scenario = form.as_ref().ok().and_then(|form| form.scenario().ok());
    let channel = form
        .as_ref()
        .map_or(Channel::default(), |form| form.channel);
    let publications = publications(
        &state.pool,
        channel,
        scenario
            .as_ref()
            .and_then(|scenario| scenario.offer.as_ref()),
    )
    .await;
    let lang = form
        .as_ref()
        .ok()
        .and_then(|form| form.lang)
        .map_or_else(|| language(&headers), Locale::as_str);
    let Some(messages) = messages(lang) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut page = Page {
        lang,
        messages,
        form: Form::default(),
        publications: Vec::new(),
        result: None,
        error: None,
    };
    if let Ok(publications) = publications {
        page.publications = publications;
    } else {
        page.error = Some("build-policy-unavailable");
        return render(page, StatusCode::SERVICE_UNAVAILABLE);
    }
    let Some(scenario) = scenario else {
        page.error = Some("build-invalid");
        page.form = form.unwrap_or_default();
        return render(page, StatusCode::BAD_REQUEST);
    };
    if raw.is_some_and(|query| {
        query
            .split('&')
            .any(|member| !member.is_empty() && !member.starts_with("lang="))
    }) {
        let head = page
            .publications
            .iter()
            .find(|publication| publication.channel == channel)
            .and_then(|publication| publication.head.as_ref());
        page.result = Some(preview(&scenario, head));
    }
    if let Ok(form) = form {
        page.form = form;
    }
    render(page, StatusCode::OK)
}

fn render(page: Page, status: StatusCode) -> Response {
    let mut response = oteryn_web::render(&page);
    if response.status().is_success() {
        *response.status_mut() = status;
    }
    response
}

#[cfg(test)]
mod tests;
