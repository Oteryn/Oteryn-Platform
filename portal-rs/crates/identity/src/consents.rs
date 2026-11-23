//! Versioned legal acceptance recorded in the registration transaction.
use crate::accounts::AccountId;
use sqlx::{PgExecutor, PgTransaction};
use std::net::IpAddr;
use std::sync::LazyLock;
use uuid::Uuid;
/// Version of the repository's bilingual placeholder package.
pub const PLACEHOLDER_LEGAL_VERSION: &str = "placeholder-2026-11-22.2";

static INVISIBLE_LEGAL_TEXT: LazyLock<Result<regex::Regex, regex::Error>> = LazyLock::new(|| {
    regex::Regex::new(r"[[\p{Cc}\p{Cf}\p{Default_Ignorable_Code_Point}\u{2800}]&&\S]+")
});
static LEGAL_NIP: LazyLock<Result<regex::Regex, regex::Error>> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?i)\bnip[\s\p{P}−]*(?:pl[\s\p{P}−]*)?([0-9](?:[\p{Zs}\t().·\p{Pd}−]*[0-9])*)",
    )
});
static LEGAL_TEMPLATE_FIELD: LazyLock<Result<regex::Regex, regex::Error>> =
    LazyLock::new(|| regex::Regex::new(r"\{\{\s*[^{}\s][^{}]*\}\}"));

/// Properties of the text the public renderer exposes.
#[derive(Debug)]
pub struct LegalEditionText {
    pub has_visible_body: bool,
    pub has_placeholder: bool,
    pub has_invalid_nip: bool,
}

impl LegalEditionText {
    pub fn needs_operator_update(&self) -> bool {
        self.has_placeholder || self.has_invalid_nip
    }
}

pub fn inspect_legal_edition(version: &str, title: &str, body: &str) -> LegalEditionText {
    use html5ever::tokenizer::{
        BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer,
    };
    use std::cell::RefCell;
    use unicode_normalization::UnicodeNormalization;

    #[derive(Default)]
    struct Text {
        body: String,
        with_image_alternatives: String,
        tooltips: Vec<String>,
    }
    struct Sink(RefCell<Text>);
    impl TokenSink for Sink {
        type Handle = ();

        fn process_token(&self, token: Token, _: u64) -> TokenSinkResult<()> {
            let mut text = self.0.borrow_mut();
            match token {
                Token::CharacterTokens(value) => {
                    text.body.push_str(&value);
                    text.with_image_alternatives.push_str(&value);
                }
                Token::TagToken(tag) => {
                    let name = tag.name.as_ref();
                    if matches!(name, "br" | "p" | "div" | "li" | "td" | "th" | "tr") {
                        text.body.push('\n');
                        text.with_image_alternatives.push('\n');
                    }
                    if tag.kind == TagKind::StartTag {
                        for attr in tag.attrs {
                            match attr.name.local.as_ref() {
                                "alt" if name == "img" => {
                                    text.with_image_alternatives.push_str(&attr.value);
                                }
                                "title" => text.tooltips.push(attr.value.to_string()),
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
            TokenSinkResult::Continue
        }
    }

    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new(body));
    let input = BufferQueue::default();
    input.push_back(ammonia::clean(&html).into());
    let tokenizer = Tokenizer::new(Sink(RefCell::new(Text::default())), Default::default());
    let _ = tokenizer.feed(&input);
    tokenizer.end();
    let text = tokenizer.sink.0.into_inner();

    let (Ok(invisible), Ok(nips), Ok(fields)) =
        (&*INVISIBLE_LEGAL_TEXT, &*LEGAL_NIP, &*LEGAL_TEMPLATE_FIELD)
    else {
        return LegalEditionText {
            has_visible_body: false,
            has_placeholder: true,
            has_invalid_nip: true,
        };
    };
    let mut result = LegalEditionText {
        // An image's fallback text may disappear when the image loads.
        has_visible_body: !invisible.replace_all(&text.body, "").trim().is_empty(),
        has_placeholder: version
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("placeholder"),
        has_invalid_nip: false,
    };
    for value in [title, text.body.as_str()]
        .into_iter()
        .chain(
            (text.body != text.with_image_alternatives)
                .then_some(text.with_image_alternatives.as_str()),
        )
        .chain(text.tooltips.iter().map(String::as_str))
    {
        let normalized: String = invisible.replace_all(value, "").nfkc().collect();
        result.has_placeholder |=
            fields.is_match(&normalized) || contains_legal_placeholder(&normalized);
        result.has_invalid_nip |= nips
            .captures_iter(&normalized)
            .any(|field| !valid_nip(&field[1]));
    }
    result
}

fn valid_nip(field: &str) -> bool {
    let digits: Vec<u32> = field.chars().filter_map(|c| c.to_digit(10)).collect();
    let [first_nine @ .., check] = digits.as_slice() else {
        return false;
    };
    first_nine.len() == 9
        && first_nine.iter().any(|&digit| digit != 0)
        && first_nine
            .iter()
            .zip([6, 5, 7, 2, 3, 4, 5, 6, 7])
            .map(|(digit, weight)| digit * weight)
            .sum::<u32>()
            % 11
            == *check
}

fn contains_legal_placeholder(text: &str) -> bool {
    use unicode_general_category::{GeneralCategory, get_general_category};
    let compact: String = text
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|&c| {
            !c.is_whitespace()
                && !matches!(
                    get_general_category(c),
                    GeneralCategory::ConnectorPunctuation
                        | GeneralCategory::DashPunctuation
                        | GeneralCategory::OpenPunctuation
                        | GeneralCategory::ClosePunctuation
                        | GeneralCategory::InitialPunctuation
                        | GeneralCategory::FinalPunctuation
                        | GeneralCategory::OtherPunctuation
                        | GeneralCategory::Format
                )
                && c != '−'
        })
        .collect();
    [
        "draftoperatordetailsareplaceholders",
        "wersjaroboczadaneoperatorasątymczasowe",
        "danetymczasowe",
        "placeholderdetails",
        "przykładowafirma",
        "przykładowa1",
        "00000warszawa",
        "0000000000",
    ]
    .iter()
    .any(|marker| compact.contains(marker))
}

/// Existing production defaults, also used when no web configuration is installed.
pub const DEFAULT_LEGAL_VERSION: &str = "2026-11-05";

#[derive(Debug, Clone)]
pub struct ConsentConfig {
    pub terms_version: String,
    pub privacy_version: String,
    publication: PublicationPolicy,
}
#[derive(Debug, Clone, Copy)]
enum PublicationPolicy {
    AllowPlaceholders,
    FinalOnly,
}

#[derive(Debug, sqlx::FromRow)]
struct PublishedDocument {
    slug: String,
    version: String,
    title: String,
    body: String,
    page_id: Option<Uuid>,
    final_approved: bool,
}

#[derive(Debug)]
struct EditionKey {
    page_id: Uuid,
    version: String,
}

/// Edition keys verified against the published locale and immutable snapshots.
#[derive(Debug)]
pub struct SignupEditions {
    locale: String,
    terms: EditionKey,
    privacy: EditionKey,
}

impl Default for ConsentConfig {
    fn default() -> Self {
        Self {
            terms_version: DEFAULT_LEGAL_VERSION.into(),
            privacy_version: DEFAULT_LEGAL_VERSION.into(),
            publication: PublicationPolicy::FinalOnly,
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
            publication: if allow_placeholders {
                PublicationPolicy::AllowPlaceholders
            } else {
                PublicationPolicy::FinalOnly
            },
        }
    }

    /// In a registration transaction the row locks hold through consent recording.
    pub async fn check_publication<'e>(
        &self,
        db: impl PgExecutor<'e>,
        locale: &str,
    ) -> Result<SignupEditions, sqlx::Error> {
        if !self.validate() {
            tracing::error!(locale, "sign-up blocked: invalid configured legal versions");
            return Err(sqlx::Error::Protocol(
                "invalid configured signup legal versions".into(),
            ));
        }
        let documents: Vec<PublishedDocument> = sqlx::query_as(
            "SELECT p.slug,e.legal_version AS version,e.title,e.body,v.page_id,
                    COALESCE(v.content @> '{\"final_approved\":true}'::jsonb,false) AS final_approved
             FROM pages p
             JOIN editorial_pages e ON e.page_id=p.id
             JOIN editorial_pages source ON source.page_id=p.id AND source.locale='en'
             LEFT JOIN page_legal_versions v
                ON v.page_id=e.page_id AND v.locale=e.locale AND v.version=e.legal_version
                AND v.content @> jsonb_build_object(
                    'title',e.title,'body',e.body,'effective',e.legal_effective_date::text
                )
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
            tracing::error!(locale, "sign-up blocked: cannot read published legal documents")
        })?;

        let checked = |slug: &str, expected: &str| {
            let document = documents.iter().find(|row| row.slug == slug);
            let reason = match document.map(|row| {
                (
                    row,
                    inspect_legal_edition(&row.version, &row.title, &row.body),
                )
            }) {
                None => "published legal document missing for signup locale",
                Some((_, text)) if !text.has_visible_body => {
                    "published legal document has no visible body text"
                }
                Some((_, text))
                    if matches!(self.publication, PublicationPolicy::FinalOnly)
                        && text.has_placeholder =>
                {
                    "placeholder legal document must be replaced before live signup"
                }
                Some((_, text))
                    if matches!(self.publication, PublicationPolicy::FinalOnly)
                        && text.has_invalid_nip =>
                {
                    "published legal edition contains an invalid operator NIP"
                }
                Some((row, _))
                    if matches!(self.publication, PublicationPolicy::FinalOnly)
                        && !row.final_approved =>
                {
                    "published legal edition requires explicit final approval"
                }
                Some((row, _)) if row.version != expected => {
                    "configured consent version does not match published legal document"
                }
                Some((row, _)) => {
                    if let Some(page_id) = row.page_id {
                        return Ok(EditionKey {
                            page_id,
                            version: row.version.clone(),
                        });
                    }
                    "published legal edition has no matching immutable snapshot"
                }
            };
            tracing::error!(locale, slug, expected, published = ?document.map(|row| &row.version), reason, "sign-up blocked by legal document configuration");
            Err(sqlx::Error::Protocol(reason.into()))
        };

        Ok(SignupEditions {
            locale: locale.into(),
            terms: checked("terms-of-service", &self.terms_version)?,
            privacy: checked("privacy-policy", &self.privacy_version)?,
        })
    }

    pub fn validate(&self) -> bool {
        [&self.terms_version, &self.privacy_version]
            .into_iter()
            .all(|v| !v.trim().is_empty() && v.chars().count() <= 128)
    }
}
pub(crate) async fn record(
    tx: &mut PgTransaction<'_>,
    id: AccountId,
    editions: &SignupEditions,
    ip: Option<IpAddr>,
) -> Result<(), sqlx::Error> {
    for (kind, edition) in [("terms", &editions.terms), ("privacy", &editions.privacy)] {
        sqlx::query(
            "INSERT INTO account_consents
                (account_id,document_kind,document_version,ip,legal_page_id,legal_locale)
             VALUES ($1,$2,$3,$4::text::inet,$5,$6)",
        )
        .bind(id)
        .bind(kind)
        .bind(&edition.version)
        .bind(ip.map(|v| v.to_string()))
        .bind(edition.page_id)
        .bind(&editions.locale)
        .execute(&mut **tx)
        .await?;
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
