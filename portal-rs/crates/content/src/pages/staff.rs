use super::*;
use crate::editorial::{self, Window};
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
    can_schedule: bool,
    schedule_at: String,
    schedule_until: String,
    translation_revision: String,
}

impl Editor {
    fn t(&self, key: &str) -> String {
        text(&self.lang, key)
    }
}
#[derive(Debug, Clone, Deserialize, Default)]
pub(super) struct PageForm {
    #[serde(default)]
    pub expected_revision: String,
    #[serde(default)]
    pub reviewed_source_revision: i64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
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
    #[serde(default)]
    pub schedule_revision: String,
    #[serde(default)]
    pub schedule_at: String,
    #[serde(default)]
    pub schedule_until: String,
    #[serde(default)]
    pub translation_revision: String,
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
    chrome: Option<Extension<oteryn_ui::Chrome>>,
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
    let lang = editorial::staff_language(chrome.as_deref(), &headers);
    let rows = sqlx::query(
        "WITH editions AS (
             SELECT p.slug,e.locale,
                    CASE WHEN e.published_at IS NOT NULL AND source.published_at IS NOT NULL
                         THEN greatest(e.published_at,source.published_at) END AS starts_at,
                    least(e.expires_at,source.expires_at) AS expires_at,
                    COALESCE(e.locale='en' OR e.source_revision=source.revision,false) AS current_translation
             FROM pages p JOIN editorial_pages e ON e.page_id=p.id
             LEFT JOIN editorial_pages source ON source.page_id=p.id AND source.locale='en'
         )
         SELECT slug,locale,to_jsonb(starts_at)#>>'{}' AS start_label,
                to_jsonb(expires_at)#>>'{}' AS end_label,
                COALESCE(starts_at>now(),false) AS pending,
                COALESCE(expires_at<=now(),false) AS expired,current_translation
         FROM editions
         ORDER BY CASE WHEN starts_at>now() THEN starts_at END NULLS LAST, slug,locale
         LIMIT 501 OFFSET $1",
    )
    .bind(i64::from(query.page) * 500)
    .fetch_all(&pool)
    .await;
    let Ok(rows) = rows else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let more = rows.len() > 500;
    let rows = rows
        .into_iter()
        .take(500)
        .map(|row| {
            let start: Option<String> = row.get("start_label");
            PageQueueRow {
                slug: row.get("slug"),
                locale: row.get("locale"),
                status: text(
                    lang,
                    editorial::status(
                        start.is_some(),
                        row.get("pending"),
                        row.get("expired"),
                        row.get("current_translation"),
                    ),
                ),
                start: start.unwrap_or_default(),
                end: row
                    .get::<Option<String>, _>("end_label")
                    .unwrap_or_default(),
            }
        })
        .collect();
    oteryn_web::render(&Listing {
        next: u64::from(query.page) + 1,
        more,
        lang: lang.to_owned(),
        title: text(lang, "pages"),
        slug_label: text(lang, "slug"),
        locale_label: text(lang, "locale"),
        create_label: text(lang, "create"),
        faq_help: text(lang, "faq-editor-help"),
        next_label: text(lang, "publication-next"),
        end_label: text(lang, "publication-end-label"),
        rows,
    })
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
    faq_help: String,
    next_label: String,
    end_label: String,
    rows: Vec<PageQueueRow>,
}

struct PageQueueRow {
    slug: String,
    locale: String,
    status: String,
    start: String,
    end: String,
}
async fn edit(
    State(pool): State<PgPool>,
    Path((slug, locale)): Path<(String, String)>,
    Extension(session): Extension<BrowserSession>,
    headers: axum::http::HeaderMap,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> Response {
    if !valid_slug(&slug) || !matches!(locale.as_str(), "pl" | "en") {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let row = sqlx::query(
        "SELECT e.title,e.body,e.legal_version,e.legal_effective_date,
                COALESCE(to_jsonb(e.published_at)#>>'{}','') AS publication_timestamp,
                COALESCE(to_jsonb(e.expires_at)#>>'{}','') AS expiry_timestamp,
                e.updated_at::text AS expected_revision,
                COALESCE(source.revision,0) AS reviewed_source_revision,
                COALESCE(translation.updated_at::text,'') AS translation_revision
         FROM pages p JOIN editorial_pages e ON e.page_id=p.id AND e.locale=$2
         LEFT JOIN editorial_pages source ON source.page_id=p.id AND source.locale='en'
         LEFT JOIN editorial_pages translation ON translation.page_id=p.id AND translation.locale='pl'
         WHERE p.slug=$1",
    )
    .bind(&slug)
    .bind(&locale)
    .fetch_optional(&pool)
    .await;
    let (mut form, schedule_until) = match row {
        Ok(Some(row)) => (
            PageForm {
                expected_revision: row.get("expected_revision"),
                reviewed_source_revision: row.get("reviewed_source_revision"),
                translation_revision: row.get("translation_revision"),
                title: row.get("title"),
                body: row.get("body"),
                published_at: row.get("publication_timestamp"),
                legal_version: row
                    .get::<Option<String>, _>("legal_version")
                    .unwrap_or_default(),
                legal_effective_date: row
                    .get::<Option<time::Date>, _>("legal_effective_date")
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                ..PageForm::default()
            },
            row.get("expiry_timestamp"),
        ),
        Ok(None) => (PageForm::default(), String::new()),
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    if form.expected_revision.is_empty() && locale == "pl" {
        form.reviewed_source_revision = match sqlx::query_scalar::<_, i64>(
            "SELECT e.revision FROM pages p JOIN editorial_pages e ON e.page_id=p.id
             WHERE p.slug=$1 AND e.locale='en'",
        )
        .bind(&slug)
        .fetch_optional(&pool)
        .await
        {
            Ok(revision) => revision.unwrap_or(0),
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        };
    }
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
    let lang = editorial::staff_language(chrome.as_deref(), &headers);
    oteryn_web::render(&Editor {
        history,
        history_label: revisions::history_label(lang),
        title: text(lang, "pages"),
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
        .map(|k| text(lang, k))
        .to_vec(),
        lang: lang.into(),
        can_schedule: locale == "en" && !legal(&slug) && !form.expected_revision.is_empty(),
        schedule_at: form.published_at.clone(),
        schedule_until,
        translation_revision: form.translation_revision.clone(),
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
    headers: axum::http::HeaderMap,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
    Form(form): Form<PageForm>,
) -> Response {
    match save_page(&pool, &auth, context, &slug, &locale, Change::Staff(form)).await {
        Ok(()) => Redirect::to("/admin/pages").into_response(),
        Err(SaveError::InvalidWindow) => editorial::form_error(
            editorial::staff_language(chrome.as_deref(), &headers),
            StatusCode::UNPROCESSABLE_ENTITY,
            "publication-invalid",
            &format!("/admin/pages/{slug}/{locale}"),
        ),
        Err(SaveError::ScheduleState) => editorial::form_error(
            editorial::staff_language(chrome.as_deref(), &headers),
            StatusCode::CONFLICT,
            "publication-state",
            &format!("/admin/pages/{slug}/{locale}"),
        ),
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
    #[error("invalid publication window")]
    InvalidWindow,
    #[error("publication requires both current editions")]
    ScheduleState,
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
            Self::InvalidWindow => StatusCode::UNPROCESSABLE_ENTITY,
            Self::ScheduleState => StatusCode::CONFLICT,
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
    if form.action == "schedule" {
        if publish_now || !valid_slug(slug) || locale != "en" || legal(slug) {
            return Err(SaveError::InvalidForm);
        }
        let window = Window::parse(&form.schedule_at, &form.schedule_until)
            .map_err(|_| SaveError::InvalidWindow)?;
        schedule_page(&mut tx, context, slug, &form, window).await?;
        crate::revalidate(
            auth,
            &mut tx,
            context,
            &[Permission::Access, Permission::ManagePages],
        )
        .await?;
        tx.commit().await?;
        return Ok(());
    }
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
        sqlx::query("INSERT INTO editorial_pages(page_id,locale,title,body,published_at,legal_version,legal_effective_date,source_revision) VALUES($1,$2,$3,$4,NULLIF($5,'')::timestamptz,NULLIF($6,''),NULLIF($7,'')::date,COALESCE((SELECT revision FROM editorial_pages WHERE page_id=$1 AND locale='en'),1)) ON CONFLICT(page_id,locale) DO UPDATE SET title=$3,body=$4,published_at=NULLIF($5,'')::timestamptz,
            expires_at=CASE WHEN EXCLUDED.published_at IS NULL THEN NULL ELSE editorial_pages.expires_at END,
            legal_version=NULLIF($6,''),legal_effective_date=NULLIF($7,'')::date,source_revision=EXCLUDED.source_revision,revision=CASE WHEN (editorial_pages.title,editorial_pages.body,editorial_pages.legal_version,editorial_pages.legal_effective_date) IS DISTINCT FROM (EXCLUDED.title,EXCLUDED.body,EXCLUDED.legal_version,EXCLUDED.legal_effective_date) THEN nextval('editorial_page_revision_seq') ELSE editorial_pages.revision END,updated_at=greatest(clock_timestamp(),editorial_pages.updated_at+interval '1 microsecond')")
            .bind(id).bind(locale).bind(&form.title).bind(&form.body).bind(&form.published_at).bind(if legal(slug){form.legal_version.as_str()}else{""}).bind(if legal(slug){form.legal_effective_date.as_str()}else{""}).execute(&mut *tx).await.map_err(|error| {
                if error.as_database_error().and_then(|error| error.constraint()) == Some("editorial_pages_publication_window") {
                    SaveError::InvalidWindow
                } else {
                    error.into()
                }
            })?;
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

async fn schedule_page(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    context: CheckContext,
    slug: &str,
    form: &PageForm,
    window: Window,
) -> Result<(), SaveError> {
    // The parent save transaction holds the slug lock shared by both editors.
    let (id, before): (Uuid, serde_json::Value) = sqlx::query_as(
        "SELECT p.id, jsonb_build_object('en',to_jsonb(en),'pl',to_jsonb(pl))
         FROM pages p
         JOIN editorial_pages en ON en.page_id=p.id AND en.locale='en'
         JOIN editorial_pages pl ON pl.page_id=p.id AND pl.locale='pl'
         WHERE p.slug=$1 AND en.updated_at::text=$2 AND pl.updated_at::text=$3
           AND en.revision=pl.source_revision",
    )
    .bind(slug)
    .bind(&form.schedule_revision)
    .bind(&form.translation_revision)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(SaveError::ScheduleState)?;

    sqlx::query(
        "UPDATE editorial_pages SET published_at=$2,expires_at=$3,
            updated_at=greatest(clock_timestamp(),updated_at+interval '1 microsecond')
         WHERE page_id=$1",
    )
    .bind(id)
    .bind(window.start)
    .bind(window.end)
    .execute(&mut **tx)
    .await?;

    audit::append(
        tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action: "cms.page.scheduled",
            target: slug,
            before,
            after: json!({"published_at":window.start,"expires_at":window.end}),
        },
    )
    .await?;
    Ok(())
}
