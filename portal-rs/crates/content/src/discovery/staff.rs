use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{self, CheckContext, MutationAuthorization, MutationCheck, Permission},
};
use oteryn_web::session::BrowserSession;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Pages,
    News,
    Events,
    Announcements,
}
impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Pages => "pages",
            Self::News => "news",
            Self::Events => "events",
            Self::Announcements => "announcements",
        }
    }
    fn permissions(self) -> &'static [Permission] {
        match self {
            Self::Pages => &[Permission::Access, Permission::ManagePages],
            Self::News => &[Permission::Access, Permission::ManageNews],
            Self::Events => &[
                Permission::Access,
                Permission::ManageEvents,
                Permission::PublishEvents,
            ],
            Self::Announcements => &[Permission::Access, Permission::ManagePortalAnnouncements],
        }
    }
    fn lock_query(self) -> &'static str {
        match self {
            Self::Pages => {
                "SELECT pg_advisory_xact_lock(hashtextextended(slug,71102)) FROM pages WHERE id=$1 AND slug<>'getting-started'"
            }
            Self::News => "SELECT 1 FROM news WHERE id=$1 FOR UPDATE",
            Self::Events => "SELECT 1 FROM events WHERE id=$1 FOR UPDATE",
            Self::Announcements => "SELECT 1 FROM announcements WHERE id=$1 FOR UPDATE",
        }
    }
    fn document_query(self) -> &'static str {
        match self {
            Self::Pages => "SELECT slug, coalesce(
                (SELECT title FROM editorial_pages WHERE page_id=p.id ORDER BY (locale=$2) DESC LIMIT 1), slug)
                FROM pages p WHERE id=$1 AND slug<>'getting-started'",
            Self::News => "SELECT '', CASE WHEN $2='pl' THEN coalesce(title_pl, title) ELSE title END FROM news WHERE id=$1",
            Self::Events => "SELECT '', title FROM event_translations WHERE event_id=$1 ORDER BY (locale=$2) DESC LIMIT 1",
            Self::Announcements => "SELECT '', CASE WHEN $2='pl' THEN coalesce(title_pl, title) ELSE title END FROM announcements WHERE id=$1",
        }
    }
    fn path(self, id: Uuid) -> String {
        format!("/admin/discovery/{}/{id}", self.name())
    }
    fn back(self, id: Uuid, slug: &str, lang: &str) -> String {
        if matches!(self, Self::Pages) {
            format!("/admin/pages/{slug}/{lang}")
        } else {
            format!("/admin/{}/{id}/edit?lang={lang}", self.name())
        }
    }
}

pub(crate) fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let mut router = Router::new();
    for kind in [Kind::Pages, Kind::News, Kind::Events, Kind::Announcements] {
        let routes = Router::new()
            .route(
                &format!("/admin/discovery/{}/{{id}}", kind.name()),
                get(edit).post(update),
            )
            .layer(Extension(kind))
            .layer(axum::extract::DefaultBodyLimit::max(16 * 1024))
            .with_state(pool.clone());
        router = router.merge(oteryn_admin::web::configured_guard(
            routes,
            pool.clone(),
            kind.permissions()[1],
        )?);
    }
    Ok(router)
}

#[derive(Clone, Debug, Deserialize, Serialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
pub(super) struct Settings {
    #[serde(default, rename = "_csrf", skip_serializing)]
    #[sqlx(skip)]
    pub(super) _csrf: Option<String>,
    pub revision: i64,
    pub indexing: String,
    pub placement: String,
    pub menu_key: String,
    pub menu_position: i16,
    pub menu_label_en: String,
    pub menu_label_pl: String,
    pub title_en: String,
    pub title_pl: String,
    pub description_en: String,
    pub description_pl: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            _csrf: None,
            revision: 0,
            indexing: "index".into(),
            placement: String::new(),
            menu_key: String::new(),
            menu_position: 1,
            menu_label_en: String::new(),
            menu_label_pl: String::new(),
            title_en: String::new(),
            title_pl: String::new(),
            description_en: String::new(),
            description_pl: String::new(),
        }
    }
}
impl Settings {
    fn has_menu(&self) -> bool {
        !self.placement.is_empty()
    }
    pub(super) fn valid(&self) -> bool {
        let within =
            |text: &str, max| text.chars().count() <= max && !text.chars().any(char::is_control);
        self.revision >= 0
            && matches!(self.indexing.as_str(), "index" | "noindex")
            && matches!(self.placement.as_str(), "" | "world" | "community")
            && (1..=8).contains(&self.menu_position)
            && (self.menu_key.is_empty()
                || (self.menu_key.len() <= 64 && crate::pages::valid_slug(&self.menu_key)))
            && within(&self.menu_label_en, 80)
            && within(&self.menu_label_pl, 80)
            && within(&self.title_en, 200)
            && within(&self.title_pl, 200)
            && within(&self.description_en, 320)
            && within(&self.description_pl, 320)
            && (!self.has_menu()
                || (!self.menu_key.is_empty()
                    && !self.menu_label_en.trim().is_empty()
                    && !self.menu_label_pl.trim().is_empty()))
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("invalid discovery settings")]
    Invalid,
    #[error("document not found")]
    Missing,
    #[error("discovery settings have changed")]
    Stale,
    #[error("placement requires a published document in both languages")]
    Unpublished,
    #[error("menu key or position already assigned")]
    Duplicate,
    #[error(transparent)]
    Authorization(#[from] rbac::Error),
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
}
impl Error {
    fn status(&self) -> StatusCode {
        match self {
            Self::Invalid => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Missing => StatusCode::NOT_FOUND,
            Self::Stale | Self::Unpublished | Self::Duplicate => StatusCode::CONFLICT,
            Self::Authorization(rbac::Error::Denied) => StatusCode::FORBIDDEN,
            Self::Authorization(_) | Self::Storage(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
    fn message(&self) -> Option<&'static str> {
        match self {
            Self::Invalid => Some("discovery-invalid"),
            Self::Stale => Some("discovery-stale"),
            Self::Unpublished => Some("discovery-unpublished"),
            Self::Duplicate => Some("discovery-duplicate"),
            _ => None,
        }
    }
}

async fn load(pool: &PgPool, kind: Kind, id: Uuid) -> Result<Settings, sqlx::Error> {
    Ok(sqlx::query_as(
        "SELECT revision, CASE WHEN indexing THEN 'index' ELSE 'noindex' END AS indexing,
                title_en,title_pl,description_en,description_pl,coalesce(placement,'') AS placement,
                coalesce(menu_key,'') AS menu_key,coalesce(menu_position,1::smallint) AS menu_position,
                coalesce(menu_label_en,'') AS menu_label_en,coalesce(menu_label_pl,'') AS menu_label_pl
         FROM editorial_discovery WHERE kind=$1 AND document_id=$2",
    )
    .bind(kind.name())
    .bind(id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_default())
}

pub(super) async fn save(
    pool: &PgPool,
    auth: &dyn MutationCheck,
    context: CheckContext,
    kind: Kind,
    id: Uuid,
    settings: &Settings,
) -> Result<(), Error> {
    if !settings.valid() {
        return Err(Error::Invalid);
    }
    let mut tx = pool.begin().await?;
    rbac::authorize_mutation_with_resources_domain(
        &mut tx,
        &[context.account],
        move |connection| {
            Box::pin(async move {
                sqlx::query(kind.lock_query())
                    .bind(id)
                    .fetch_optional(connection)
                    .await?
                    .ok_or(Error::Missing)?;
                Ok::<(), Error>(())
            })
        },
        auth,
        context,
        kind.permissions(),
    )
    .await?;
    let before: Option<(i64, serde_json::Value)> = sqlx::query_as(
        "SELECT revision,to_jsonb(d) FROM editorial_discovery d WHERE kind=$1 AND document_id=$2",
    )
    .bind(kind.name())
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    if before.as_ref().map_or(0, |(revision, _)| *revision) != settings.revision {
        return Err(Error::Stale);
    }
    if settings.has_menu() {
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM editorial_publication WHERE kind=$1 AND document_id=$2",
        )
        .bind(kind.name())
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if count != 2 {
            return Err(Error::Unpublished);
        }
    }
    let menu = settings.has_menu();
    let after: serde_json::Value = sqlx::query_scalar(
        "INSERT INTO editorial_discovery AS d
            (page_id,news_id,event_id,announcement_id,indexing,title_en,title_pl,description_en,description_pl,
             placement,menu_key,menu_position,menu_label_en,menu_label_pl)
         VALUES (CASE WHEN $1='pages' THEN $2::uuid END, CASE WHEN $1='news' THEN $2::uuid END,
                 CASE WHEN $1='events' THEN $2::uuid END, CASE WHEN $1='announcements' THEN $2::uuid END,
                 $3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
         ON CONFLICT (kind, document_id) DO UPDATE
             SET revision=d.revision+1, updated_at=clock_timestamp(), indexing=excluded.indexing,
                 title_en=excluded.title_en,title_pl=excluded.title_pl,
                 description_en=excluded.description_en,description_pl=excluded.description_pl,
                 placement=excluded.placement,menu_key=excluded.menu_key,menu_position=excluded.menu_position,
                 menu_label_en=excluded.menu_label_en,menu_label_pl=excluded.menu_label_pl
         RETURNING to_jsonb(d)",
    )
    .bind(kind.name())
    .bind(id)
    .bind(settings.indexing == "index")
    .bind(settings.title_en.trim())
    .bind(settings.title_pl.trim())
    .bind(settings.description_en.trim())
    .bind(settings.description_pl.trim())
    .bind(menu.then_some(&settings.placement))
    .bind(menu.then_some(&settings.menu_key))
    .bind(menu.then_some(settings.menu_position))
    .bind(menu.then_some(settings.menu_label_en.trim()))
    .bind(menu.then_some(settings.menu_label_pl.trim()))
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| {
        if error.as_database_error().is_some_and(sqlx::error::DatabaseError::is_unique_violation) {
            Error::Duplicate
        } else {
            Error::Storage(error)
        }
    })?;
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action: "cms.discovery_saved",
            target: &id.to_string(),
            before: before.map_or_else(|| json!({}), |(_, data)| data),
            after,
        },
    )
    .await?;
    crate::revalidate(auth, &mut tx, context, kind.permissions()).await?;
    tx.commit().await?;
    Ok(())
}

#[derive(Deserialize)]
struct Locale {
    lang: Option<String>,
    saved: Option<String>,
}
impl Locale {
    fn language(&self, chrome: &oteryn_ui::Chrome) -> Result<&'static str, StatusCode> {
        match self.lang.as_deref() {
            Some("pl") => Ok("pl"),
            Some("en") => Ok("en"),
            Some(_) => Err(StatusCode::BAD_REQUEST),
            None => Ok(if chrome.lang == "en" { "en" } else { "pl" }),
        }
    }
}
#[derive(Template)]
#[template(
    path = "content/discovery-editor.html",
    config = "src/pages/askama.toml"
)]
struct Editor<'a> {
    lang: &'a str,
    action: String,
    back: String,
    document: String,
    csrf: String,
    saved: bool,
    error: Option<&'static str>,
    settings: Settings,
}
impl Editor<'_> {
    fn text(&self, key: &str) -> String {
        oteryn_ui::text(self.lang, key)
    }
}
struct Target {
    kind: Kind,
    id: Uuid,
    lang: &'static str,
}
impl Target {
    async fn render(
        &self,
        pool: &PgPool,
        session: &BrowserSession,
        settings: Settings,
        saved: bool,
        error: Option<&'static str>,
    ) -> Result<Response, StatusCode> {
        let (slug, document): (String, String) = sqlx::query_as(self.kind.document_query())
            .bind(self.id)
            .bind(self.lang)
            .fetch_optional(pool)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .ok_or(StatusCode::NOT_FOUND)?;
        let mut response = oteryn_web::render(&Editor {
            lang: self.lang,
            action: format!("{}?lang={}", self.kind.path(self.id), self.lang),
            back: self.kind.back(self.id, &slug, self.lang),
            document,
            csrf: session
                .csrf_token()
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            saved,
            error,
            settings,
        });
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-store"),
        );
        Ok(response)
    }
}
async fn edit(
    State(pool): State<PgPool>,
    Extension(kind): Extension<Kind>,
    (Extension(context), Extension(auth), Extension(session)): (
        Extension<CheckContext>,
        Extension<MutationAuthorization>,
        Extension<BrowserSession>,
    ),
    Path(id): Path<Uuid>,
    Query(query): Query<Locale>,
    Extension(chrome): Extension<oteryn_ui::Chrome>,
) -> Result<Response, StatusCode> {
    if matches!(kind, Kind::Events) {
        let mut connection = pool
            .acquire()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        auth.check(&mut connection, context, Permission::PublishEvents)
            .await
            .map_err(|error| Error::Authorization(error).status())?;
    }
    let target = Target {
        kind,
        id,
        lang: query.language(&chrome)?,
    };
    let settings = load(&pool, kind, id)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    target
        .render(
            &pool,
            &session,
            settings,
            query.saved.as_deref() == Some("1"),
            None,
        )
        .await
}
async fn update(
    State(pool): State<PgPool>,
    Extension(kind): Extension<Kind>,
    (Extension(context), Extension(auth), Extension(session)): (
        Extension<CheckContext>,
        Extension<MutationAuthorization>,
        Extension<BrowserSession>,
    ),
    Path(id): Path<Uuid>,
    Query(query): Query<Locale>,
    Extension(chrome): Extension<oteryn_ui::Chrome>,
    Form(settings): Form<Settings>,
) -> Result<Response, StatusCode> {
    let target = Target {
        kind,
        id,
        lang: query.language(&chrome)?,
    };
    match save(&pool, &auth, context, kind, id, &settings).await {
        Ok(()) => Ok(
            Redirect::to(&format!("{}?lang={}&saved=1", kind.path(id), target.lang))
                .into_response(),
        ),
        Err(error) => {
            let message = error.message().ok_or_else(|| error.status())?;
            let mut response = target
                .render(&pool, &session, settings, false, Some(message))
                .await?;
            *response.status_mut() = error.status();
            Ok(response)
        }
    }
}
