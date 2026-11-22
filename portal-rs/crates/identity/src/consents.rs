//! Versioned legal acceptance recorded in the registration transaction.
use crate::accounts::AccountId;
use sqlx::{PgConnection, PgExecutor};
use std::net::IpAddr;
/// Version of the repository's bilingual placeholder package.
pub const PLACEHOLDER_LEGAL_VERSION: &str = "placeholder-2026-11-22.2";

/// Detect the bundled operator placeholders even if staff change the version or remove a notice.
pub fn is_placeholder_edition(version: &str, body: &str) -> bool {
    let body = body.to_lowercase();
    let compact: String = body
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '-' | '–' | '—' | '*' | '_' | '`'))
        .collect();
    version
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("placeholder")
        || [
            "draft — operator details are placeholders.",
            "wersja robocza — dane operatora są tymczasowe.",
            "[dane tymczasowe",
            "placeholder details",
        ]
        .iter()
        .any(|marker| body.contains(marker))
        || [
            "przykładowafirma",
            "przykładowa1",
            "00000warszawa",
            "0000000000",
        ]
        .iter()
        .any(|marker| compact.contains(marker))
}

#[derive(Debug, Clone)]
pub struct ConsentConfig {
    pub terms_version: String,
    pub privacy_version: String,
    publication: Option<PublicationPolicy>,
}
#[derive(Debug, Clone, Copy)]
enum PublicationPolicy {
    AllowPlaceholders,
    FinalOnly,
}

impl Default for ConsentConfig {
    fn default() -> Self {
        Self {
            terms_version: PLACEHOLDER_LEGAL_VERSION.into(),
            privacy_version: PLACEHOLDER_LEGAL_VERSION.into(),
            publication: None,
        }
    }
}
impl ConsentConfig {
    /// Require the same published CMS editions for display and consent recording.
    pub fn published(
        terms_version: String,
        privacy_version: String,
        allow_placeholders: bool,
    ) -> Self {
        Self {
            terms_version,
            privacy_version,
            publication: Some(if allow_placeholders {
                PublicationPolicy::AllowPlaceholders
            } else {
                PublicationPolicy::FinalOnly
            }),
        }
    }

    /// In a registration transaction the row locks hold through consent recording.
    pub async fn check_publication<'e>(
        &self,
        db: impl PgExecutor<'e>,
        locale: &str,
    ) -> Result<(), sqlx::Error> {
        let Some(policy) = self.publication else {
            // Unconfigured adapters use an isolated registration fixture.
            return Ok(());
        };
        let documents: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT p.slug,e.legal_version,e.body
             FROM pages p
             JOIN editorial_pages e ON e.page_id=p.id
             JOIN editorial_pages source ON source.page_id=p.id AND source.locale='en'
             WHERE p.slug IN ('terms-of-service','privacy-policy') AND e.locale=$1
                AND e.published_at<=now() AND source.published_at<=now()
                AND (e.locale='en' OR e.source_revision=source.revision)
                AND e.legal_version IS NOT NULL AND e.legal_effective_date IS NOT NULL
             ORDER BY p.slug FOR SHARE OF e,source",
        )
        .bind(locale)
        .fetch_all(db)
        .await
        .inspect_err(|_| {
            tracing::error!(
                locale,
                "sign-up blocked: cannot read published legal documents"
            )
        })?;
        for (slug, expected) in [
            ("terms-of-service", &self.terms_version),
            ("privacy-policy", &self.privacy_version),
        ] {
            let document = documents.iter().find(|(name, _, _)| name == slug);
            let reason = match document {
                None => "published legal document missing for signup locale",
                Some((_, version, body))
                    if matches!(policy, PublicationPolicy::FinalOnly)
                        && is_placeholder_edition(version, body) =>
                {
                    "placeholder legal document must be replaced before live signup"
                }
                Some((_, version, _)) if version != expected => {
                    "configured consent version does not match published legal document"
                }
                Some(_) => continue,
            };
            tracing::error!(locale, slug, expected, published = ?document.map(|(_, version, _)| version), reason, "sign-up blocked by legal document configuration");
            return Err(sqlx::Error::Protocol(reason.into()));
        }
        Ok(())
    }

    pub fn validate(&self) -> bool {
        [&self.terms_version, &self.privacy_version]
            .into_iter()
            .all(|v| !v.trim().is_empty() && v.chars().count() <= 128)
    }
}
pub async fn record(
    conn: &mut PgConnection,
    id: AccountId,
    config: &ConsentConfig,
    ip: Option<IpAddr>,
) -> Result<(), sqlx::Error> {
    for (kind, version) in [
        ("terms", &config.terms_version),
        ("privacy", &config.privacy_version),
    ] {
        sqlx::query("INSERT INTO account_consents (account_id,document_kind,document_version,ip) VALUES ($1,$2,$3,$4::text::inet)")
            .bind(id).bind(kind).bind(version).bind(ip.map(|v|v.to_string())).execute(&mut *conn).await?;
    }
    Ok(())
}

/// Optional purposes are independent of mandatory legal acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    PlaySession,
    PersonalisedAds,
}
impl Kind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "play_session" => Some(Self::PlaySession),
            "personalised_ads" => Some(Self::PersonalisedAds),
            _ => None,
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Self::PlaySession => "play_session",
            Self::PersonalisedAds => "personalised_ads",
        }
    }
}

/// Text and version ship together. Bump the corresponding version when either
/// translation of a purpose changes. Collection remains disabled by default.
#[derive(Debug, Clone)]
pub struct OptionalConfig {
    pub enabled: bool,
    pub play_session_version: String,
    pub personalised_ads_version: String,
}
impl Default for OptionalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            play_session_version: "2026-11-11.1".into(),
            personalised_ads_version: "2026-11-11.1".into(),
        }
    }
}
impl OptionalConfig {
    pub fn configured() -> Self {
        Self {
            enabled: std::env::var("OTERYN_ACCOUNT_CONSENTS_ENABLED").as_deref() == Ok("true"),
            ..Self::default()
        }
    }
    pub fn version(&self, kind: Kind) -> &str {
        match kind {
            Kind::PlaySession => &self.play_session_version,
            Kind::PersonalisedAds => &self.personalised_ads_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub granted: bool,
    pub version: String,
    /// UTC ISO 8601, suitable for both display and the HTML time element.
    pub decided_at: String,
}

/// Absence is a refusal, never implicit permission.
pub async fn current(
    pool: &sqlx::PgPool,
    account: AccountId,
    kind: Kind,
) -> Result<Option<Decision>, sqlx::Error> {
    let row: Option<(bool, String, String)> = sqlx::query_as("SELECT granted,document_version,to_char(accepted_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') FROM account_consents WHERE account_id=$1 AND document_kind=$2 ORDER BY accepted_at DESC,id DESC LIMIT 1")
        .bind(account).bind(kind.code()).fetch_optional(pool).await?;
    Ok(row.map(|(granted, version, decided_at)| Decision {
        granted,
        version,
        decided_at,
    }))
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecisionError {
    Disabled,
    InvalidInput,
    OutdatedVersion,
    Refused,
    Unavailable,
}

/// Serialize decisions per account so the timestamp order follows commit order.
/// Revocation remains possible from an old form and records the current text version.
pub async fn decide(
    pool: &sqlx::PgPool,
    config: &OptionalConfig,
    account: impl Into<crate::mutation_actor::MutationActor>,
    kind: Kind,
    granted: bool,
    version: &str,
) -> Result<(), DecisionError> {
    let actor = account.into();
    let account = actor.account;
    if !config.enabled {
        return Err(DecisionError::Disabled);
    }
    let configured = config.version(kind);
    if configured.trim().is_empty()
        || configured.chars().count() > 128
        || version.trim().is_empty()
        || version.chars().count() > 128
    {
        return Err(DecisionError::InvalidInput);
    }
    if granted && version != configured {
        return Err(DecisionError::OutdatedVersion);
    }
    let mut tx = pool.begin().await.map_err(|_| DecisionError::Unavailable)?;
    let active: Option<AccountId> =
        sqlx::query_scalar("SELECT id FROM accounts WHERE id=$1 AND status='active' FOR UPDATE")
            .bind(account)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| DecisionError::Unavailable)?;

    if !actor
        .live(&mut tx)
        .await
        .map_err(|_| DecisionError::Unavailable)?
    {
        return Err(DecisionError::Refused);
    }
    if active.is_none() {
        return Err(DecisionError::Refused);
    }
    sqlx::query("INSERT INTO account_consents(account_id,document_kind,document_version,granted,actor,source,accepted_at) VALUES ($1,$2,$3,$4,$1,'web',clock_timestamp())")
        .bind(account).bind(kind.code()).bind(configured).bind(granted).execute(&mut *tx).await.map_err(|_| DecisionError::Unavailable)?;
    tx.commit().await.map_err(|_| DecisionError::Unavailable)
}

#[path = "consents_http.rs"]
pub mod http;
#[cfg(test)]
#[path = "consents_tests.rs"]
mod tests;

/// Startup-injected flags shared by browser module adapters.
pub type FeatureFlags = std::collections::BTreeSet<String>;
pub const FEATURE: &str = "account-consents";
