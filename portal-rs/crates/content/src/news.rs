//! News storage and public and staff routes.
use crate::revisions::{self, Edit, Target};
use axum::{
    Extension, Form, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{CheckContext, MutationAuthorization, Permission},
};
use oteryn_web::ratelimit::{self, PublicGameData, RateLimit, RateLimiter};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

mod public;

fn unavailable(_: impl std::fmt::Display) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn label(lang: &str, key: &str) -> String {
    use fluent_bundle::{FluentBundle, FluentResource};
    let source = if lang == "pl" {
        include_str!("../../../templates/content/pl.ftl")
    } else {
        include_str!("../../../templates/content/en.ftl")
    };
    let Ok(locale) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = FluentBundle::new(vec![locale]);
    let Ok(resource) = FluentResource::try_new(source.to_owned()) else {
        return String::new();
    };
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
#[derive(askama::Template)]
#[template(
    source = "<!doctype html><html class=\"op-theme\" lang=\"{{ lang }}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><link rel=\"stylesheet\" href=\"/static/css/components.css\"><link rel=\"stylesheet\" href=\"/static/css/app.css\"><title>{{ title }}</title></head><body class=\"op-theme\"><main class=\"op-stack\"><h1>{{ title }}</h1>{{ body|safe }}</main></body></html>",
    ext = "html"
)]
struct NewsPage<'a> {
    lang: &'a str,
    title: &'a str,
    body: String,
}
fn page(lang: &str, title: &str, body: String) -> Html<String> {
    use askama::Template;
    Html(NewsPage { lang, title, body }.render().unwrap_or_default())
}
const MAX_PAGE: i64 = 100000;

#[derive(Debug, Default, Deserialize)]
struct Options {
    lang: Option<String>,
    page: Option<i64>,
    #[serde(default)]
    year: String,
    #[serde(default)]
    month: String,
    #[serde(default)]
    tag: String,
    #[serde(default)]
    q: String,
}
impl Options {
    fn lang(&self) -> &str {
        if self.lang.as_deref() == Some("en") {
            "en"
        } else {
            "pl"
        }
    }
    fn offset(&self, size: i64) -> Result<i64, StatusCode> {
        let n = self.page.unwrap_or(1);
        if !(1..=MAX_PAGE).contains(&n) {
            Err(StatusCode::UNPROCESSABLE_ENTITY)
        } else {
            Ok((n - 1) * size)
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Kind {
    #[default]
    News,
    Changelog,
}

#[derive(Debug, Default, Deserialize)]
struct Input {
    #[serde(default)]
    kind: Option<Kind>,
    #[serde(default)]
    tags: Option<String>,
    #[serde(default)]
    expected_revision: String,
    slug: String,
    title: String,
    body: String,
    published_at: String,
    #[serde(default)]
    title_pl: String,
    #[serde(default)]
    body_pl: String,
    #[serde(default)]
    translation_published_at: String,
}
fn slug_valid(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 160
        && s.split('-').all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}
const MAX_TAGS: usize = 10;
const MAX_TAG_LENGTH: usize = 48;

fn tag_valid(tag: &str) -> bool {
    tag.len() <= MAX_TAG_LENGTH && slug_valid(tag)
}

fn parse_tags(value: &str) -> Result<Vec<&str>, StatusCode> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut tags: Vec<_> = value.split(',').take(MAX_TAGS + 1).map(str::trim).collect();
    if tags.len() > MAX_TAGS || tags.iter().any(|tag| !tag_valid(tag)) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    tags.sort_unstable();
    tags.dedup();
    Ok(tags)
}

impl Input {
    fn valid(&self) -> bool {
        slug_valid(&self.slug)
            && !self.title.trim().is_empty()
            && self.title.chars().count() <= 200
            && !self.body.trim().is_empty()
            && self.body.chars().count() <= 100000
            && self.title_pl.chars().count() <= 200
            && self.body_pl.chars().count() <= 100000
            && (self.translation_published_at.is_empty()
                || (!self.title_pl.trim().is_empty() && !self.body_pl.trim().is_empty()))
    }
}
pub fn public_router(pool: PgPool) -> Router {
    public_router_at(pool, "http://localhost:3000")
}

fn public_router_at(pool: PgPool, origin: &str) -> Router {
    let archive = Router::new()
        .route("/news", get(public::index))
        .route("/{lang}/news", get(public::local_index))
        .route("/{lang}/news/rss.xml", get(public::rss))
        .route("/{lang}/changelog", get(public::changelog))
        .route_layer(axum::middleware::from_fn(
            |_: RateLimit<PublicGameData>, request, next: axum::middleware::Next| async move {
                next.run(request).await
            },
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            RateLimiter::new(pool.clone()),
            ratelimit::supply,
        ));
    archive
        .route("/news/{slug}", get(show))
        .route("/{lang}/news/{slug}", get(local_show))
        .with_state(pool)
        .layer(Extension(public::FeedOrigin(
            origin.trim_end_matches('/').to_owned(),
        )))
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
        .layer(axum::middleware::from_fn(headers))
}
pub fn admin_router(pool: PgPool) -> Router {
    Router::new()
        .route("/admin/news", get(admin_index).post(create))
        .route("/admin/news/create", get(new))
        .route("/admin/news/{id}/edit", get(edit))
        .route("/admin/news/{id}/revisions", get(history))
        .route("/admin/news/{id}/revisions/{source}/restore", post(restore))
        .route("/admin/news/{id}", post(update).put(update).delete(delete))
        .route("/admin/news/{id}/delete", post(delete))
        .with_state(pool)
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
        .layer(axum::middleware::from_fn(headers))
}
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let origin =
        std::env::var("OTERYN_PUBLIC_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".into());
    oteryn_web::seo::Seo::new(&origin, false)?;
    Ok(
        public_router_at(pool.clone(), &origin).merge(oteryn_admin::web::configured_guard(
            admin_router(pool.clone()),
            pool,
            Permission::ManageNews,
        )?),
    )
}
fn language(lang: &str) -> Result<(), StatusCode> {
    if matches!(lang, "pl" | "en") {
        Ok(())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
/// Newest published entries for the home page, under the same language rule
/// as the public listing. The excerpt is plain text; templates escape it.
pub async fn headlines(
    pool: &PgPool,
    lang: oteryn_web::Language,
    limit: i64,
) -> Result<Vec<oteryn_ui::HomeNews>, sqlx::Error> {
    let lang = lang.code();
    let rows = sqlx::query("SELECT slug,CASE WHEN $1='pl' THEN title_pl ELSE title END AS title,CASE WHEN $1='pl' THEN body_pl ELSE body END AS body,to_char(published_at AT TIME ZONE 'UTC','YYYY-MM-DD') AS day FROM news WHERE NOT is_changelog AND published_at <= now() AND ($1='en' OR (translation_published_at <= now() AND translation_source_updated_at >= updated_at AND length(trim(title_pl))>0 AND length(trim(body_pl))>0)) ORDER BY published_at DESC,id DESC LIMIT $2").bind(lang).bind(limit.clamp(0, 10)).fetch_all(pool).await?;
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let slug: String = row.try_get("slug")?;
        if !slug_valid(&slug) {
            continue;
        }
        let day: String = row.try_get("day")?;
        let body: String = row.try_get("body")?;
        entries.push(oteryn_ui::HomeNews {
            url: format!("/{lang}/news/{slug}"),
            title: row.try_get("title")?,
            display_date: display_date(lang, &day),
            date: day,
            excerpt: excerpt(&body, 160),
        });
    }
    Ok(entries)
}
fn display_date(lang: &str, day: &str) -> String {
    match (lang, day.split('-').collect::<Vec<_>>().as_slice()) {
        ("pl", [year, month, date]) => format!("{date}.{month}.{year}"),
        _ => day.to_owned(),
    }
}
/// First `limit` characters of the body's words, ending with an ellipsis when cut.
fn excerpt(body: &str, limit: usize) -> String {
    let text = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= limit {
        return text;
    }
    let cut: String = text.chars().take(limit).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{}…", cut.trim_end_matches([',', '.', ';', ':', '-', '–']))
}
async fn show(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    Query(q): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    article(&pool, q.lang(), &slug).await
}
async fn local_show(
    State(pool): State<PgPool>,
    Path((lang, slug)): Path<(String, String)>,
) -> Result<Html<String>, StatusCode> {
    language(&lang)?;
    article(&pool, &lang, &slug).await
}
async fn article(pool: &PgPool, lang: &str, slug: &str) -> Result<Html<String>, StatusCode> {
    if !slug_valid(slug) {
        return Err(StatusCode::NOT_FOUND);
    }
    let row = sqlx::query(
        "SELECT is_changelog,
                CASE WHEN $1='pl' THEN title_pl ELSE title END AS title,
                CASE WHEN $1='pl' THEN body_pl ELSE body END AS body
           FROM news
          WHERE published_at <= now() AND slug=$2
            AND ($1='en' OR (translation_published_at <= now()
                 AND translation_source_updated_at >= updated_at
                 AND length(trim(title_pl)) > 0 AND length(trim(body_pl)) > 0))",
    )
    .bind(lang)
    .bind(slug)
    .fetch_optional(pool)
    .await
    .map_err(unavailable)?
    .ok_or(StatusCode::NOT_FOUND)?;
    let section = if row.get("is_changelog") {
        "changelog"
    } else {
        "news"
    };
    Ok(page(
        lang,
        row.get("title"),
        format!(
            "<p><a href=\"/{lang}/{section}\">{}</a></p><article><p>{}</p></article>",
            label(lang, section),
            escape(row.get("body")).replace('\n', "<br>")
        ),
    ))
}

async fn admin_index(
    State(pool): State<PgPool>,
    Extension(_): Extension<CheckContext>,
    Query(q): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    let rows = sqlx::query(
        "SELECT id,title,is_changelog FROM news ORDER BY updated_at DESC,id DESC LIMIT 26 OFFSET $1",
    )
    .bind(q.offset(25)?)
    .fetch_all(&pool)
    .await
    .map_err(unavailable)?;
    let lang = q.lang();
    let mut html = format!(
        "<a href=\"/admin/news/create?lang={lang}\">{}</a>",
        label(lang, "new")
    );
    let has_next = rows.len() > 25 && q.page.unwrap_or(1) < MAX_PAGE;
    for row in rows.into_iter().take(25) {
        let id: Uuid = row.get("id");
        let kind = if row.get("is_changelog") {
            "changelog"
        } else {
            "news"
        };
        html.push_str(&format!(
            "<p><a href=\"/admin/news/{id}/edit?lang={lang}\">{}</a> · {}</p>",
            escape(row.get("title")),
            label(lang, kind)
        ));
    }
    if has_next {
        html.push_str(&format!(
            "<a href=\"?page={}&amp;lang={lang}\">{}</a>",
            q.page.unwrap_or(1) + 1,
            label(lang, "next")
        ));
    }
    Ok(page(lang, &label(lang, "news"), html))
}
async fn new(
    Extension(_): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Query(q): Query<Options>,
) -> Result<Html<String>, StatusCode> {
    Ok(form(
        q.lang(),
        q.lang(),
        None,
        Input::default(),
        session.csrf_token().await.map_err(unavailable)?,
    ))
}
async fn edit(
    State(pool): State<PgPool>,
    Extension(_): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    Path(id): Path<Uuid>,
    Query(q): Query<Options>,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> Result<Html<String>, StatusCode> {
    let row=sqlx::query("SELECT updated_at::text AS expected_revision,slug,title,body,is_changelog,tags,coalesce(published_at::text,'') AS published_at,coalesce(title_pl,'') AS title_pl,coalesce(body_pl,'') AS body_pl,coalesce(translation_published_at::text,'') AS translation_published_at FROM news WHERE id=$1").bind(id).fetch_optional(&pool).await.map_err(unavailable)?.ok_or(StatusCode::NOT_FOUND)?;
    let history_lang = revisions::language(chrome.as_deref());
    let lang = q
        .lang
        .as_deref()
        .filter(|lang| matches!(*lang, "en" | "pl"))
        .unwrap_or(history_lang);
    Ok(form(
        lang,
        history_lang,
        Some(id),
        Input {
            kind: Some(if row.get("is_changelog") {
                Kind::Changelog
            } else {
                Kind::News
            }),
            tags: Some(row.get::<Vec<String>, _>("tags").join(", ")),
            expected_revision: row.get("expected_revision"),
            slug: row.get("slug"),
            title: row.get("title"),
            body: row.get("body"),
            published_at: row.get("published_at"),
            title_pl: row.get("title_pl"),
            body_pl: row.get("body_pl"),
            translation_published_at: row.get("translation_published_at"),
        },
        session.csrf_token().await.map_err(unavailable)?,
    ))
}
fn form(
    lang: &str,
    history_lang: &str,
    id: Option<Uuid>,
    input: Input,
    csrf: String,
) -> Html<String> {
    let action = id.map_or_else(
        || "/admin/news".to_owned(),
        |id| format!("/admin/news/{id}"),
    );
    let mut html = id.map_or_else(String::new, |id| {
        format!(
            "<a href=\"{}\">{}</a>",
            Target::News(id).history_path(),
            escape(&revisions::history_label(history_lang)),
        )
    });
    html.push_str(&format!(
        "<form class=\"op-stack\" method=\"post\" action=\"{action}?lang={lang}\"><input type=\"hidden\" name=\"_csrf\" value=\"{}\">",
        escape(&csrf)
    ));
    html.push_str(&format!(
        "<input type=\"hidden\" name=\"expected_revision\" value=\"{}\">",
        escape(&input.expected_revision)
    ));
    let (news_selected, changelog_selected) = match input.kind.unwrap_or_default() {
        Kind::News => (" selected", ""),
        Kind::Changelog => ("", " selected"),
    };
    html.push_str(&format!(
        "<label for=\"kind\">{}</label><select class=\"op-input\" id=\"kind\" name=\"kind\">\
         <option value=\"news\"{news_selected}>{}</option>\
         <option value=\"changelog\"{changelog_selected}>{}</option></select>\
         <label for=\"tags\">{}</label><input class=\"op-input\" id=\"tags\" name=\"tags\" value=\"{}\" aria-describedby=\"tags-help\">\
         <small id=\"tags-help\">{}</small>",
        label(lang, "kind"),
        label(lang, "news"),
        label(lang, "changelog"),
        label(lang, "tags"),
        escape(input.tags.as_deref().unwrap_or_default()),
        label(lang, "tags-help"),
    ));
    let expected_revision = input.expected_revision.clone();
    for (name, key, value, multi) in [
        ("slug", "slug", input.slug, false),
        ("title", "title", input.title, false),
        ("body", "body", input.body, true),
        ("published_at", "publication", input.published_at, false),
        ("title_pl", "title", input.title_pl, false),
        ("body_pl", "body", input.body_pl, true),
        (
            "translation_published_at",
            "publication",
            input.translation_published_at,
            false,
        ),
    ] {
        let locale = if name.ends_with("_pl") || name.starts_with("translation") {
            "pl"
        } else {
            "en"
        };
        html.push_str(&format!(
            "<label for=\"{name}\">{} ({locale})</label>",
            label(lang, key)
        ));
        html.push_str(&if multi {
            format!(
                "<textarea class=\"op-input\" id=\"{name}\" name=\"{name}\" rows=\"10\">{}</textarea>",
                escape(&value)
            )
        } else {
            format!(
                "<input class=\"op-input\" id=\"{name}\" name=\"{name}\" value=\"{}\">",
                escape(&value)
            )
        });
    }
    html.push_str(&format!(
        "<button class=\"op-button\">{}</button></form>",
        label(lang, "save")
    ));
    if let Some(id) = id {
        html.push_str(&format!("<form class=\"op-stack\" method=\"post\" action=\"/admin/news/{id}/delete\"><input type=\"hidden\" name=\"_csrf\" value=\"{}\"><input type=\"hidden\" name=\"expected_revision\" value=\"{}\"><button class=\"op-button\">{}</button></form>",escape(&csrf),escape(&expected_revision),label(lang,"delete")));
    }
    page(lang, &label(lang, "news"), html)
}
async fn history(
    State(pool): State<PgPool>,
    Extension(session): Extension<BrowserSession>,
    Path(id): Path<Uuid>,
    Query(query): Query<revisions::Query>,
    chrome: Option<Extension<oteryn_ui::Chrome>>,
) -> Result<Response, StatusCode> {
    revisions::history(
        &pool,
        Target::News(id),
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
    Path((id, source)): Path<(Uuid, i64)>,
    Form(form): Form<revisions::Restore<String>>,
) -> Result<Response, StatusCode> {
    save_change(
        &pool,
        &auth,
        context,
        Some(id),
        Edit::Restore { source, form },
    )
    .await
}

async fn create(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    save(&pool, &auth, context, None, input).await
}
async fn update(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Path(id): Path<Uuid>,
    Form(input): Form<Input>,
) -> Result<Response, StatusCode> {
    save(&pool, &auth, context, Some(id), input).await
}
fn storage(e: sqlx::Error) -> StatusCode {
    if e.as_database_error().is_some_and(|e| {
        e.is_unique_violation()
            || e.code()
                .is_some_and(|c| c.starts_with("22") || c.starts_with("23"))
    }) {
        StatusCode::UNPROCESSABLE_ENTITY
    } else {
        unavailable(e)
    }
}
async fn save(
    pool: &PgPool,
    auth: &MutationAuthorization,
    context: CheckContext,
    id: Option<Uuid>,
    input: Input,
) -> Result<Response, StatusCode> {
    save_change(pool, auth, context, id, Edit::Save(input)).await
}

async fn save_change(
    pool: &PgPool,
    auth: &MutationAuthorization,
    context: CheckContext,
    id: Option<Uuid>,
    change: Edit<Input, String>,
) -> Result<Response, StatusCode> {
    let mut tx = pool.begin().await.map_err(unavailable)?;
    oteryn_admin::rbac::authorize_mutation_with_resources(
        &mut tx,
        &[context.account],
        move |connection| {
            Box::pin(async move {
                sqlx::query("SELECT id FROM news WHERE id=$1 FOR UPDATE")
                    .bind(id)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        },
        auth,
        context,
        &[Permission::Access, Permission::ManageNews],
    )
    .await
    .map_err(|error| match error {
        oteryn_admin::rbac::Error::Denied => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    })?;
    let before: serde_json::Value = if let Some(id) = id {
        sqlx::query_scalar("SELECT to_jsonb(news) FROM news WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?
            .ok_or(StatusCode::NOT_FOUND)?
    } else {
        json!({})
    };
    auth.check_in(&mut tx, context, Permission::ManageNews)
        .await
        .map_err(|e| {
            if matches!(e, oteryn_admin::rbac::Error::Denied) {
                StatusCode::FORBIDDEN
            } else {
                unavailable(e)
            }
        })?;
    let (input, restored_from) = match change {
        Edit::Save(input) => {
            if let Some(id) = id {
                verify_revision(&mut tx, id, &input.expected_revision).await?;
            }
            (input, None)
        }
        Edit::Restore { source, form } => {
            if form.expected_revision.is_empty() {
                return Err(StatusCode::BAD_REQUEST);
            }
            let id = id.ok_or(StatusCode::BAD_REQUEST)?;
            verify_revision(&mut tx, id, &form.expected_revision).await?;
            let snapshot = Target::News(id)
                .snapshot(&mut tx, source)
                .await
                .map_err(revisions::Error::status)?;
            let value = |field| revisions::value(&snapshot, field);
            (
                Input {
                    slug: value("slug"),
                    title: value("title"),
                    body: value("body"),
                    title_pl: value("title_pl"),
                    body_pl: value("body_pl"),
                    kind: snapshot["is_changelog"].as_bool().map(|is_changelog| {
                        if is_changelog {
                            Kind::Changelog
                        } else {
                            Kind::News
                        }
                    }),
                    tags: snapshot["tags"].as_array().map(|tags| {
                        tags.iter()
                            .filter_map(serde_json::Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    }),
                    ..Input::default()
                },
                Some(source),
            )
        }
    };
    if !input.valid() {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let tags = input.tags.as_deref().map(parse_tags).transpose()?;
    let is_changelog = input.kind.map(|kind| matches!(kind, Kind::Changelog));
    let action = if restored_from.is_some() {
        "cms.news_restored"
    } else if id.is_some() {
        "cms.news_updated"
    } else {
        "cms.news_created"
    };
    let (id, is_changelog, tags): (Uuid, bool, Vec<String>) = if let Some(id) = id {
        sqlx::query_as(
            "WITH revision AS MATERIALIZED (
                SELECT greatest(clock_timestamp(), updated_at + interval '1 microsecond') AS stamp
                  FROM news WHERE id=$1
             )
             UPDATE news SET slug=$2, title=$3, body=$4, published_at=NULLIF($5,'')::timestamptz,
                   title_pl=NULLIF($6,''), body_pl=NULLIF($7,''),
                   translation_published_at=NULLIF($8,'')::timestamptz,
                   translation_source_updated_at=CASE
                       WHEN translation_published_at IS DISTINCT FROM NULLIF($8,'')::timestamptz
                           AND NULLIF($8,'')::timestamptz IS NOT NULL THEN (SELECT stamp FROM revision)
                       WHEN (title,body) IS DISTINCT FROM ($3,$4)
                           AND (title_pl,body_pl) IS NOT DISTINCT FROM (NULLIF($6,''),NULLIF($7,'')) THEN NULL
                       WHEN (title_pl,body_pl,translation_published_at) IS DISTINCT FROM
                           (NULLIF($6,''),NULLIF($7,''),NULLIF($8,'')::timestamptz) THEN (SELECT stamp FROM revision)
                       WHEN translation_source_updated_at>=updated_at THEN (SELECT stamp FROM revision)
                       ELSE NULL END,
                   is_changelog=coalesce($9,is_changelog), tags=coalesce($10,tags),
                   updated_at=(SELECT stamp FROM revision)
             WHERE id=$1 RETURNING id,is_changelog,tags",
        )
        .bind(id)
        .bind(&input.slug)
        .bind(&input.title)
        .bind(&input.body)
        .bind(&input.published_at)
        .bind(&input.title_pl)
        .bind(&input.body_pl)
        .bind(&input.translation_published_at)
        .bind(is_changelog)
        .bind(&tags)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| {
            if restored_from.is_some()
                && error.as_database_error().is_some_and(|db| db.is_unique_violation())
            {
                StatusCode::CONFLICT
            } else {
                storage(error)
            }
        })?
        .ok_or(StatusCode::NOT_FOUND)?
    } else {
        sqlx::query_as(
            "INSERT INTO news(slug,title,body,published_at,title_pl,body_pl,translation_published_at,
                              translation_source_updated_at,is_changelog,tags)
             VALUES($1,$2,$3,NULLIF($4,'')::timestamptz,NULLIF($5,''),NULLIF($6,''),
                    NULLIF($7,'')::timestamptz,now(),$8,$9) RETURNING id,is_changelog,tags",
        )
        .bind(&input.slug)
        .bind(&input.title)
        .bind(&input.body)
        .bind(&input.published_at)
        .bind(&input.title_pl)
        .bind(&input.body_pl)
        .bind(&input.translation_published_at)
        .bind(is_changelog.unwrap_or_default())
        .bind(tags.unwrap_or_default())
        .fetch_one(&mut *tx)
        .await
        .map_err(storage)?
    };
    Target::News(id)
        .append(&mut tx, context.account, restored_from)
        .await
        .map_err(unavailable)?;
    let mut after = json!({"slug":input.slug,"title":input.title,"body":input.body,"published_at":input.published_at,"title_pl":input.title_pl,"body_pl":input.body_pl,"translation_published_at":input.translation_published_at,"is_changelog":is_changelog,"tags":tags});
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
            action,
            target: &id.to_string(),
            before,
            after,
        },
    )
    .await
    .map_err(unavailable)?;
    crate::revalidate(
        auth,
        &mut tx,
        context,
        &[Permission::Access, Permission::ManageNews],
    )
    .await
    .map_err(|error| match error {
        oteryn_admin::rbac::Error::Denied => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    })?;
    tx.commit().await.map_err(unavailable)?;
    Ok((
        StatusCode::FOUND,
        [(
            axum::http::header::LOCATION,
            format!("/admin/news/{id}/edit"),
        )],
    )
        .into_response())
}
async fn delete(
    State(pool): State<PgPool>,
    Extension(context): Extension<CheckContext>,
    Extension(auth): Extension<MutationAuthorization>,
    Path(id): Path<Uuid>,
    Form(revision): Form<Revision>,
) -> Result<Response, StatusCode> {
    let mut tx = pool.begin().await.map_err(unavailable)?;
    oteryn_admin::rbac::authorize_mutation_with_resources(
        &mut tx,
        &[context.account],
        move |connection| {
            Box::pin(async move {
                sqlx::query("SELECT id FROM news WHERE id=$1 FOR UPDATE")
                    .bind(id)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        },
        &auth,
        context,
        &[Permission::Access, Permission::ManageNews],
    )
    .await
    .map_err(|error| match error {
        oteryn_admin::rbac::Error::Denied => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    })?;
    let before: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT to_jsonb(news) FROM news WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?;
    let before = before.ok_or(StatusCode::NOT_FOUND)?;
    auth.check_in(&mut tx, context, Permission::ManageNews)
        .await
        .map_err(|e| {
            if matches!(e, oteryn_admin::rbac::Error::Denied) {
                StatusCode::FORBIDDEN
            } else {
                unavailable(e)
            }
        })?;
    verify_revision(&mut tx, id, &revision.expected_revision).await?;
    sqlx::query("DELETE FROM news WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action: "cms.news_deleted",
            target: &id.to_string(),
            before,
            after: json!({}),
        },
    )
    .await
    .map_err(unavailable)?;
    crate::revalidate(
        &auth,
        &mut tx,
        context,
        &[Permission::Access, Permission::ManageNews],
    )
    .await
    .map_err(|error| match error {
        oteryn_admin::rbac::Error::Denied => StatusCode::FORBIDDEN,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    })?;
    tx.commit().await.map_err(unavailable)?;
    Ok((
        StatusCode::FOUND,
        [(axum::http::header::LOCATION, "/admin/news")],
    )
        .into_response())
}
#[cfg(test)]
#[path = "news_tests.rs"]
mod tests;

async fn headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
        ),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
        (
            "strict-transport-security",
            "max-age=63072000; includeSubDomains",
        ),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        response.headers_mut().insert(
            axum::http::HeaderName::from_static(name),
            axum::http::HeaderValue::from_static(value),
        );
    }
    response
}

#[derive(Deserialize)]
struct Revision {
    #[serde(default)]
    expected_revision: String,
}
async fn verify_revision(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    expected: &str,
) -> Result<(), StatusCode> {
    // Compare the exact opaque value emitted by the edit form; no client date
    // parsing or timestamp precision loss participates in concurrency control.
    let current: String = sqlx::query_scalar("SELECT updated_at::text FROM news WHERE id=$1")
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(unavailable)?;
    if current != expected {
        return Err(StatusCode::CONFLICT);
    }
    Ok(())
}
