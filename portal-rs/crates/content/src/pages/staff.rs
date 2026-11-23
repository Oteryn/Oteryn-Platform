use super::*;
use crate::revisions::{self, Target};
use axum::{Extension, Form, extract::Query, response::Redirect, routing::post};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{
        AccountState, AccountStateReader, AccountStateUnavailable, Authorizer, CheckContext, Error,
        MutationAuthorization, Permission,
    },
    web::{PanelConfig, PortalIdentity, StaffSession},
};
use oteryn_identity::consents::{ConsentConfig, SignupEditionIssue, SignupEditionStatus};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    Ok(
        router(pool.clone()).merge(oteryn_admin::web::configured_guard(
            admin_routes(pool.clone()),
            pool,
            Permission::ManagePages,
        )?),
    )
}
/// Test and authentication-adapter composition through the shared T10B staff guard.
pub fn staff_router<I: StaffSession, A: AccountStateReader + 'static>(
    pool: PgPool,
    config: PanelConfig,
    identity: I,
    accounts: A,
) -> Router {
    oteryn_admin::web::guarded(
        admin_routes(pool.clone()),
        pool,
        config,
        identity,
        accounts,
        Permission::ManagePages,
    )
}
fn admin_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/admin/pages", get(index))
        .route("/admin/pages/{slug}/{locale}", get(edit).post(save))
        .route("/admin/pages/{slug}/{locale}/revisions", get(history))
        .route(
            "/admin/pages/{slug}/{locale}/revisions/{source}/restore",
            post(restore),
        )
        .with_state(pool)
}
#[derive(Template)]
#[template(path = "admin/pages/edit.html", config = "src/pages/askama.toml")]
struct Editor {
    lang: String,
    title: String,
    csrf: String,
    slug: String,
    locale: String,
    form: PageForm,
    labels: Vec<String>,
    legal: bool,
    history: String,
    history_label: String,
}
#[derive(Debug, Clone, Deserialize, Default)]
pub(super) struct PageForm {
    #[serde(default)]
    pub expected_revision: String,
    #[serde(default)]
    pub reviewed_source_revision: i64,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub published_at: String,
    #[serde(default)]
    pub legal_version: String,
    #[serde(default)]
    pub legal_effective_date: String,
    #[serde(default)]
    pub final_approved: String,
    #[serde(default)]
    pub action: String,
}
fn valid(form: &PageForm, slug: &str, locale: &str, publish_now: bool) -> bool {
    let published = publish_now || !form.published_at.is_empty();
    (form.published_at.is_empty()
        || time::OffsetDateTime::parse(
            &form.published_at,
            &time::format_description::well_known::Rfc3339,
        )
        .is_ok())
        && (form.legal_effective_date.is_empty()
            || time::Date::parse(
                &form.legal_effective_date,
                time::macros::format_description!("[year]-[month]-[day]"),
            )
            .is_ok())
        && (!publish_now
            || (form.legal_effective_date.len() == 10 && !form.body.contains("{{operator}}")))
        && valid_slug(slug)
        && matches!(locale, "en" | "pl")
        && !form.title.trim().is_empty()
        && form.title.chars().count() <= 200
        && !form.body.trim().is_empty()
        && form.body.chars().count() <= 100000
        && form.legal_version.len() <= 80
        && matches!(form.action.as_str(), "save" | "delete")
        && (form.final_approved.is_empty()
            || (form.final_approved == "yes" && legal(slug) && published && {
                let text = oteryn_identity::consents::inspect_legal_edition(
                    &form.legal_version,
                    &form.title,
                    &form.body,
                );
                text.has_visible_body && !text.needs_operator_update()
            }))
        && (!legal(slug)
            || !published
            || (!form.legal_version.trim().is_empty() && !form.legal_effective_date.is_empty()))
}
async fn index(
    State(pool): State<PgPool>,
    axum::extract::Query(query): axum::extract::Query<CreateQuery>,
    headers: axum::http::HeaderMap,
    consent: Option<Extension<ConsentConfig>>,
) -> Response {
    if let Some(slug) = query.slug {
        if !valid_slug(&slug) || !matches!(query.locale.as_deref(), Some("en" | "pl")) {
            return StatusCode::BAD_REQUEST.into_response();
        }
        return Redirect::to(&format!(
            "/admin/pages/{}/{}",
            slug,
            query.locale.unwrap_or_default()
        ))
        .into_response();
    }
    let lang = if headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("en"))
    {
        "en"
    } else {
        "pl"
    };
    const PAGE_SIZE: usize = 500;
    let rows = sqlx::query_as(
        "SELECT p.slug,e.locale FROM pages p JOIN editorial_pages e ON e.page_id=p.id
         ORDER BY p.slug,e.locale LIMIT $1 OFFSET $2",
    )
    .bind((PAGE_SIZE + 1) as i64)
    .bind(i64::from(query.page) * PAGE_SIZE as i64)
    .fetch_all(&pool)
    .await;
    match rows {
        Ok(mut rows) => {
            let more = rows.len() > PAGE_SIZE && query.page < u32::MAX;
            rows.truncate(PAGE_SIZE);
            let editions = consent
                .map(|Extension(config)| config)
                .unwrap_or_default()
                .live_readiness(&pool)
                .await;

            oteryn_web::render(&Listing {
                readiness: editions.ok(),
                next: u64::from(query.page) + 1,
                more,
                lang: lang.to_owned(),
                title: text(lang, "pages"),
                slug_label: text(lang, "slug"),
                locale_label: text(lang, "locale"),
                create_label: text(lang, "create"),
                rows,
            })
        }
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
#[derive(Deserialize)]
struct CreateQuery {
    #[serde(default)]
    page: u32,
    slug: Option<String>,
    locale: Option<String>,
}
#[derive(Template)]
#[template(path = "admin/pages/index.html", config = "src/pages/askama.toml")]
struct Listing {
    next: u64,
    more: bool,
    lang: String,
    title: String,
    slug_label: String,
    locale_label: String,
    create_label: String,
    rows: Vec<(String, String)>,
    readiness: Option<Vec<SignupEditionStatus>>,
}
impl Listing {
    fn text(&self, key: &str) -> String {
        text(&self.lang, key)
    }

    fn readiness_summary(&self) -> String {
        self.text(match &self.readiness {
            Some(editions) if editions.iter().all(|edition| edition.issue.is_none()) => {
                "signup-ready"
            }
            Some(_) => "signup-blocked",
            None => "signup-status-unavailable",
        })
    }

    fn edition_status(&self, issue: &Option<SignupEditionIssue>) -> String {
        use SignupEditionIssue::*;
        self.text(match issue {
            None => "signup-edition-ready",
            Some(InvalidConfiguration) => "signup-invalid-config",
            Some(Unpublished) => "signup-unpublished",
            Some(StaleTranslation) => "signup-stale",
            Some(EmptyBody) => "signup-empty",
            Some(Placeholder) => "signup-placeholder",
            Some(InvalidNip) => "signup-invalid-nip",
            Some(VersionMismatch) => "signup-version-mismatch",
            Some(MissingSnapshot) => "signup-missing-snapshot",
            Some(Unapproved) => "signup-unapproved",
        })
    }
}
async fn edit(
    State(pool): State<PgPool>,
    Path((slug, locale)): Path<(String, String)>,
    Extension(session): Extension<BrowserSession>,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> Response {
    if !valid_slug(&slug) || !matches!(locale.as_str(), "pl" | "en") {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let row=sqlx::query("SELECT e.*,e.updated_at::text AS expected_revision FROM pages p JOIN editorial_pages e ON e.page_id=p.id WHERE p.slug=$1 AND e.locale=$2").bind(&slug).bind(&locale).fetch_optional(&pool).await;
    let mut form = match row {
        Ok(Some(row)) => PageForm {
            expected_revision: row.get("expected_revision"),
            reviewed_source_revision: 0,
            title: row.get("title"),
            body: row.get("body"),
            published_at: row
                .get::<Option<sqlx::types::time::OffsetDateTime>, _>("published_at")
                .map(|t| {
                    t.format(&time::format_description::well_known::Rfc3339)
                        .unwrap_or_default()
                })
                .unwrap_or_default(),
            legal_version: row
                .get::<Option<String>, _>("legal_version")
                .unwrap_or_default(),
            legal_effective_date: row
                .get::<Option<sqlx::types::time::Date>, _>("legal_effective_date")
                .map(|v| v.to_string())
                .unwrap_or_default(),
            final_approved: String::new(),
            action: String::new(),
        },
        Ok(None) => PageForm::default(),
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    form.reviewed_source_revision = match sqlx::query_scalar::<_, i64>("SELECT e.revision FROM pages p JOIN editorial_pages e ON e.page_id=p.id WHERE p.slug=$1 AND e.locale='en'")
        .bind(&slug).fetch_optional(&pool).await {
            Ok(revision) => revision.unwrap_or(0),
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        };
    let Ok(csrf) = session.csrf_token().await else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let mut history = String::new();
    if !legal(&slug) {
        let target = Target::Page {
            slug: &slug,
            locale: &locale,
        };
        let exists = if form.expected_revision.is_empty() {
            let result = async {
                let mut connection = pool.acquire().await?;
                target.has_history(&mut connection).await
            }
            .await;
            match result {
                Ok(exists) => exists,
                Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
            }
        } else {
            true
        };
        if exists {
            history = target.history_path();
        }
    }
    oteryn_web::render(&Editor {
        history,
        history_label: revisions::history_label(revisions::language(chrome.as_deref())),
        title: text(&locale, "pages"),
        labels: [
            "title",
            "body",
            "published",
            "version",
            "effective",
            "save",
            "delete",
            "final-approval",
        ]
        .map(|k| text(&locale, k))
        .to_vec(),
        lang: locale.clone(),
        locale,
        legal: legal(&slug),
        slug,
        csrf,
        form,
    })
}
async fn save(
    State(pool): State<PgPool>,
    Path((slug, locale)): Path<(String, String)>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Form(form): Form<PageForm>,
) -> Response {
    match save_page(&pool, &auth, context, &slug, &locale, Change::Staff(form)).await {
        Ok(()) => Redirect::to("/admin/pages").into_response(),
        Err(error) => error.status().into_response(),
    }
}

async fn history(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Path((slug, locale)): Path<(String, String)>,
    Query(query): Query<revisions::Query>,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> Result<Response, StatusCode> {
    let target = revision_target(&slug, &locale).map_err(revisions::Error::status)?;
    revisions::history(
        &pool,
        target,
        revisions::language(chrome.as_deref()),
        query,
        &session,
    )
    .await
}

async fn restore(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Path((slug, locale, source)): Path<(String, String, i64)>,
    Form(form): Form<revisions::Restore<String>>,
) -> Result<Redirect, StatusCode> {
    save_page(
        &pool,
        &auth,
        context,
        &slug,
        &locale,
        Change::Restore { source, form },
    )
    .await
    .map_err(SaveError::status)?;
    Ok(Redirect::to(&format!("/admin/pages/{slug}/{locale}")))
}

fn revision_target<'a>(slug: &'a str, locale: &'a str) -> Result<Target<'a>, revisions::Error> {
    if !valid_slug(slug) || !matches!(locale, "en" | "pl") {
        return Err(revisions::Error::Invalid);
    }
    if legal(slug) {
        return Err(revisions::Error::Missing);
    }
    Ok(Target::Page { slug, locale })
}

#[derive(Debug)]
pub struct LegalEdition {
    pub slug: String,
    pub locale: String,
    pub title: String,
    pub body: String,
    pub version: String,
    pub effective: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("final approval requires valid page fields, visible text and real operator details")]
    InvalidForm,
    #[error("save the English source edition first")]
    MissingSource,
    #[error("use a new version to change an immutable legal edition")]
    ImmutableVersion,
    #[error(transparent)]
    Revision(#[from] revisions::Error),
    #[error(transparent)]
    Authorization(#[from] Error),
}

impl SaveError {
    fn status(self) -> StatusCode {
        match self {
            Self::InvalidForm => StatusCode::BAD_REQUEST,
            Self::Authorization(Error::Denied) => StatusCode::FORBIDDEN,
            Self::MissingSource
            | Self::ImmutableVersion
            | Self::Authorization(Error::InvalidInput) => StatusCode::CONFLICT,
            Self::Revision(error) => error.status(),
            Self::Authorization(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl From<sqlx::Error> for SaveError {
    fn from(error: sqlx::Error) -> Self {
        Error::from(error).into()
    }
}

struct HostOperator(PortalIdentity);

impl AccountStateReader for HostOperator {
    async fn state(&self, account: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        self.0.state(account).await
    }

    async fn state_in(
        &self,
        connection: &mut sqlx::PgConnection,
        account: Uuid,
    ) -> Result<AccountState, AccountStateUnavailable> {
        self.0.state_in(connection, account).await
    }

    async fn authenticated_in(
        &self,
        _connection: &mut sqlx::PgConnection,
        context: CheckContext,
    ) -> Result<bool, AccountStateUnavailable> {
        Ok(!context.require_staff_mfa && context.session_proof.is_none())
    }
}

/// Publish a final legal edition through the CMS save transaction.
/// Browser MFA cannot apply to a host-root CLI. The actor's account and both
/// required permissions are still revalidated on the mutation connection.
pub async fn publish_legal_edition(
    pool: &PgPool,
    actor: Uuid,
    edition: LegalEdition,
) -> Result<(), SaveError> {
    let identity = PortalIdentity::configured_from(pool.clone(), &|_| None);
    let auth = MutationAuthorization::new(Arc::new(Authorizer::new(
        pool.clone(),
        HostOperator(identity),
    )));
    let context = CheckContext {
        require_staff_mfa: false,
        ..CheckContext::new(actor)
    };
    let form = PageForm {
        title: edition.title,
        body: edition.body,
        legal_version: edition.version,
        legal_effective_date: edition.effective,
        final_approved: "yes".into(),
        action: "save".into(),
        ..PageForm::default()
    };
    save_page(
        pool,
        &auth,
        context,
        &edition.slug,
        &edition.locale,
        Change::Operator(form),
    )
    .await
}

#[cfg(test)]
pub(super) async fn persist(
    pool: &PgPool,
    auth: &MutationAuthorization,
    context: CheckContext,
    slug: &str,
    locale: &str,
    form: &PageForm,
) -> Result<(), Error> {
    save_page(
        pool,
        auth,
        context,
        slug,
        locale,
        Change::Staff(form.clone()),
    )
    .await
    .map_err(|error| match error {
        SaveError::Authorization(error) => error,
        _ => Error::InvalidInput,
    })
}

enum Change {
    Staff(PageForm),
    Operator(PageForm),
    Restore {
        source: i64,
        form: revisions::Restore<String>,
    },
}

async fn save_page(
    pool: &PgPool,
    auth: &MutationAuthorization,
    context: CheckContext,
    slug: &str,
    locale: &str,
    change: Change,
) -> Result<(), SaveError> {
    let publish_now = matches!(change, Change::Operator(_));
    let mut tx = pool.begin().await?;
    let resource_slug = slug.to_owned();
    oteryn_admin::rbac::authorize_mutation_with_resources(
        &mut tx,
        &[context.account],
        move |connection| {
            Box::pin(async move {
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,71102))")
                    .bind(resource_slug)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        },
        auth,
        context,
        &[Permission::Access, Permission::ManagePages],
    )
    .await?;
    let (mut form, restored_from) = match change {
        Change::Staff(form) | Change::Operator(form) => (form, None),
        Change::Restore { source, form } => {
            let target = revision_target(slug, locale)?;
            if form.expected_revision.is_empty() {
                return Err(revisions::Error::Invalid.into());
            }
            let snapshot = target.snapshot(&mut tx, source).await?;
            (
                PageForm {
                    expected_revision: form.expected_revision,
                    reviewed_source_revision: form.reviewed_source_revision,
                    title: revisions::value(&snapshot, "title"),
                    body: revisions::value(&snapshot, "body"),
                    action: "save".into(),
                    ..PageForm::default()
                },
                Some(source),
            )
        }
    };
    if !valid(&form, slug, locale, publish_now) {
        return Err(SaveError::InvalidForm);
    }
    if publish_now {
        let published_at: time::OffsetDateTime = sqlx::query_scalar("SELECT now()")
            .fetch_one(&mut *tx)
            .await?;
        form.published_at = published_at
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    }
    let id:Uuid=sqlx::query_scalar("INSERT INTO pages(id,slug) VALUES($1,$2) ON CONFLICT(slug) DO UPDATE SET updated_at=now() RETURNING id").bind(Uuid::now_v7()).bind(slug).fetch_one(&mut *tx).await?;
    let before: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT to_jsonb(e) FROM editorial_pages e WHERE page_id=$1 AND locale=$2",
    )
    .bind(id)
    .bind(locale)
    .fetch_optional(&mut *tx)
    .await?;
    let revision: Option<String> = sqlx::query_scalar(
        "SELECT updated_at::text FROM editorial_pages WHERE page_id=$1 AND locale=$2",
    )
    .bind(id)
    .bind(locale)
    .fetch_optional(&mut *tx)
    .await?;
    let revision = revision.unwrap_or_default();
    // The operator reads revisions only after the shared slug lock is held.
    if publish_now {
        form.expected_revision.clone_from(&revision);
    }
    if form.expected_revision != revision {
        return Err(Error::InvalidInput.into());
    }
    if locale != "en" {
        let source: Option<i64> = sqlx::query_scalar(
            "SELECT revision FROM editorial_pages WHERE page_id=$1 AND locale='en'",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        if publish_now {
            form.reviewed_source_revision = source.ok_or(SaveError::MissingSource)?;
        }
        if source != Some(form.reviewed_source_revision) {
            return Err(Error::InvalidInput.into());
        }
    }
    let mut final_approved = form.final_approved == "yes";
    if form.action == "delete" {
        if legal(slug) {
            return Err(Error::InvalidInput.into());
        }
        sqlx::query("DELETE FROM editorial_pages WHERE page_id=$1 AND locale=$2")
            .bind(id)
            .bind(locale)
            .execute(&mut *tx)
            .await?;
    } else {
        if locale != "en" {
            let source_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editorial_pages WHERE page_id=$1 AND locale='en' AND body <> '')")
                .bind(id).fetch_one(&mut *tx).await?;
            if !source_exists {
                return Err(SaveError::MissingSource);
            }
        }
        let mut content =
            json!({"title":form.title,"body":form.body,"effective":form.legal_effective_date});
        if legal(slug) && !form.published_at.is_empty() {
            let old:Option<serde_json::Value>=sqlx::query_scalar("SELECT content FROM page_legal_versions WHERE page_id=$1 AND locale=$2 AND version=$3").bind(id).bind(locale).bind(&form.legal_version).fetch_optional(&mut *tx).await?;
            final_approved |= old
                .as_ref()
                .is_some_and(|snapshot| snapshot["final_approved"] == true);
            if final_approved {
                content["final_approved"] = json!(true);
            }
            if old.is_some_and(|v| v != content) {
                return Err(SaveError::ImmutableVersion);
            }
            sqlx::query("INSERT INTO page_legal_versions(page_id,locale,version,content) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING").bind(id).bind(locale).bind(&form.legal_version).bind(content).execute(&mut *tx).await?;
        }
        sqlx::query("INSERT INTO editorial_pages(page_id,locale,title,body,published_at,legal_version,legal_effective_date,source_revision) VALUES($1,$2,$3,$4,NULLIF($5,'')::timestamptz,NULLIF($6,''),NULLIF($7,'')::date,COALESCE((SELECT revision FROM editorial_pages WHERE page_id=$1 AND locale='en'),1)) ON CONFLICT(page_id,locale) DO UPDATE SET title=$3,body=$4,published_at=NULLIF($5,'')::timestamptz,legal_version=NULLIF($6,''),legal_effective_date=NULLIF($7,'')::date,source_revision=EXCLUDED.source_revision,revision=CASE WHEN (editorial_pages.title,editorial_pages.body,editorial_pages.legal_version,editorial_pages.legal_effective_date) IS DISTINCT FROM (EXCLUDED.title,EXCLUDED.body,EXCLUDED.legal_version,EXCLUDED.legal_effective_date) THEN nextval('editorial_page_revision_seq') ELSE editorial_pages.revision END,updated_at=greatest(clock_timestamp(),editorial_pages.updated_at+interval '1 microsecond')")
            .bind(id).bind(locale).bind(&form.title).bind(&form.body).bind(&form.published_at).bind(if legal(slug){form.legal_version.as_str()}else{""}).bind(if legal(slug){form.legal_effective_date.as_str()}else{""}).execute(&mut *tx).await?;
    }
    if form.action == "save" && !legal(slug) {
        Target::Page { slug, locale }
            .append(&mut tx, context.account, restored_from)
            .await?;
    }
    let mut after = json!({"locale":locale,"title":form.title,"body":form.body,"action":form.action,"published_at":form.published_at,"legal_version":form.legal_version,"legal_effective_date":form.legal_effective_date,"final_approved":final_approved});
    if let Some(source) = restored_from {
        after["restored_from"] = json!(source);
    }
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action: if form.action == "delete" {
                "cms.page.deleted"
            } else if restored_from.is_some() {
                "cms.page.restored"
            } else {
                "cms.page.saved"
            },
            target: slug,
            before: before.unwrap_or(json!(null)),
            after,
        },
    )
    .await?;
    crate::revalidate(
        auth,
        &mut tx,
        context,
        &[Permission::Access, Permission::ManagePages],
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
