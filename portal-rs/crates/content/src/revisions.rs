//! Append-only editorial snapshots shared by the staff CMS editors.
use askama::Template;
use axum::{http::StatusCode, response::Response};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub(crate) enum Target<'a> {
    News(Uuid),
    Page { slug: &'a str, locale: &'a str },
    Announcement(Uuid),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid editorial revision")]
    Invalid,
    #[error("editorial revision not found")]
    Missing,
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
}

impl Error {
    pub(crate) fn status(self) -> StatusCode {
        match self {
            Self::Invalid => StatusCode::BAD_REQUEST,
            Self::Missing => StatusCode::NOT_FOUND,
            Self::Storage(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

pub(crate) enum Edit<T, V> {
    Save(T),
    Restore { source: i64, form: Restore<V> },
}

#[derive(Deserialize)]
pub(crate) struct Restore<V> {
    pub(crate) expected_revision: V,
    #[serde(default)]
    pub(crate) reviewed_source_revision: i64,
}

#[derive(sqlx::FromRow)]
struct Current {
    token: String,
    source_revision: i64,
}

impl Target<'_> {
    fn scope(&self) -> (&'static str, String, &str) {
        match self {
            Self::News(id) => ("news", id.to_string(), ""),
            Self::Page { slug, locale } => ("page", (*slug).into(), locale),
            Self::Announcement(id) => ("announcement", id.to_string(), ""),
        }
    }

    fn edit_path(&self) -> String {
        match self {
            Self::News(id) => format!("/admin/news/{id}/edit"),
            Self::Page { slug, locale } => format!("/admin/pages/{slug}/{locale}"),
            Self::Announcement(id) => format!("/admin/announcements/{id}/edit"),
        }
    }

    fn index_path(&self) -> &'static str {
        match self {
            Self::News(_) => "/admin/news",
            Self::Page { .. } => "/admin/pages",
            Self::Announcement(_) => "/admin/announcements",
        }
    }

    pub(crate) async fn has_history(
        &self,
        connection: &mut PgConnection,
    ) -> Result<bool, sqlx::Error> {
        let (kind, key, locale) = self.scope();
        sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM editorial_revisions WHERE kind=$1 AND object_key=$2 AND locale=$3)",
        )
        .bind(kind)
        .bind(key)
        .bind(locale)
        .fetch_one(connection)
        .await
    }

    pub(crate) fn history_path(&self) -> String {
        let edit = self.edit_path();
        format!("{}/revisions", edit.strip_suffix("/edit").unwrap_or(&edit))
    }

    async fn current(&self, connection: &mut PgConnection) -> Result<Option<Current>, sqlx::Error> {
        match self {
            Self::News(id) => {
                sqlx::query_as(
                    "SELECT updated_at::text AS token, 0::bigint AS source_revision
                 FROM news WHERE id=$1",
                )
                .bind(id)
                .fetch_optional(connection)
                .await
            }
            Self::Page { slug, locale } => {
                sqlx::query_as(
                    "SELECT e.updated_at::text AS token,
                        coalesce(source.revision,0) AS source_revision
                 FROM pages p JOIN editorial_pages e ON e.page_id=p.id
                 LEFT JOIN editorial_pages source ON source.page_id=p.id AND source.locale='en'
                 WHERE p.slug=$1 AND e.locale=$2",
                )
                .bind(slug)
                .bind(locale)
                .fetch_optional(connection)
                .await
            }
            Self::Announcement(id) => {
                sqlx::query_as(
                    "SELECT lock_version::text AS token, 0::bigint AS source_revision
                 FROM announcements WHERE id=$1",
                )
                .bind(id)
                .fetch_optional(connection)
                .await
            }
        }
    }

    pub(crate) async fn append(
        &self,
        connection: &mut PgConnection,
        actor: Uuid,
        restored_from: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        let snapshot: Value = match self {
            Self::News(id) => {
                sqlx::query_scalar("SELECT to_jsonb(n) FROM news n WHERE id=$1")
                    .bind(id)
                    .fetch_one(&mut *connection)
                    .await?
            }
            Self::Page { slug, locale } => {
                sqlx::query_scalar(
                    "SELECT to_jsonb(e) FROM editorial_pages e JOIN pages p ON p.id=e.page_id
                 WHERE p.slug=$1 AND e.locale=$2",
                )
                .bind(slug)
                .bind(locale)
                .fetch_one(&mut *connection)
                .await?
            }
            Self::Announcement(id) => {
                sqlx::query_scalar("SELECT to_jsonb(a) FROM announcements a WHERE id=$1")
                    .bind(id)
                    .fetch_one(&mut *connection)
                    .await?
            }
        };
        let (kind, key, locale) = self.scope();
        sqlx::query(
            "INSERT INTO editorial_revisions(kind, object_key, locale, actor, restored_from, snapshot)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(kind)
        .bind(key)
        .bind(locale)
        .bind(actor)
        .bind(restored_from)
        .bind(snapshot)
        .execute(connection)
        .await?;
        Ok(())
    }

    pub(crate) async fn snapshot(
        &self,
        connection: &mut PgConnection,
        id: i64,
    ) -> Result<Value, Error> {
        if id < 1 {
            return Err(Error::Invalid);
        }
        let (kind, key, locale) = self.scope();
        sqlx::query_scalar(
            "SELECT snapshot FROM editorial_revisions WHERE kind=$1 AND object_key=$2 AND locale=$3 AND id=$4",
        )
        .bind(kind)
        .bind(key)
        .bind(locale)
        .bind(id)
        .fetch_optional(connection)
        .await?
        .ok_or(Error::Missing)
    }

    fn fields(&self) -> &[(&str, &str)] {
        match self {
            Self::News(_) => &[
                ("slug", "slug"),
                ("is_changelog", "news-kind"),
                ("tags", "news-tags"),
                ("title", "title-en"),
                ("body", "body-en"),
                ("title_pl", "title-pl"),
                ("body_pl", "body-pl"),
                ("published_at", "published"),
                ("translation_published_at", "translation-published"),
            ],
            Self::Page { .. } => &[
                ("title", "title"),
                ("body", "body"),
                ("published_at", "published"),
            ],
            Self::Announcement(_) => &[
                ("title", "title-en"),
                ("body", "body-en"),
                ("title_pl", "title-pl"),
                ("body_pl", "body-pl"),
                ("publication_state", "publication-state"),
                ("severity", "severity"),
                ("starts_at", "starts"),
                ("ends_at", "ends"),
                ("action_url", "action-url"),
                ("action_label", "action-en"),
                ("action_label_pl", "action-pl"),
                ("translation_published_at", "translation-published"),
            ],
        }
    }
}

pub(crate) fn value(snapshot: &Value, field: &str) -> String {
    snapshot[field].as_str().unwrap_or_default().into()
}

pub(crate) fn language(chrome: Option<&oteryn_ui::Chrome>) -> &str {
    chrome.map_or("pl", |chrome| chrome.lang.as_str())
}

const EN: &str = include_str!("../../../templates/content/revisions.en.ftl");
const PL: &str = include_str!("../../../templates/content/revisions.pl.ftl");

fn bundle(lang: &str) -> fluent_bundle::FluentBundle<fluent_bundle::FluentResource> {
    let mut bundle = fluent_bundle::FluentBundle::new(lang.parse().into_iter().collect());
    let source = if lang == "pl" { PL } else { EN };
    if let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) {
        let _ = bundle.add_resource(resource);
    }
    bundle
}

pub(crate) fn history_label(lang: &str) -> String {
    translated(&bundle(lang), "history")
}

fn translated(
    bundle: &fluent_bundle::FluentBundle<fluent_bundle::FluentResource>,
    key: &str,
) -> String {
    bundle
        .get_message(key)
        .and_then(|message| message.value())
        .map(|pattern| {
            bundle
                .format_pattern(pattern, None, &mut vec![])
                .into_owned()
        })
        .unwrap_or_default()
}

#[derive(Default, Deserialize)]
pub(crate) struct Query {
    before: Option<i64>,
    from: Option<i64>,
    to: Option<i64>,
}

impl Query {
    fn validate(&self) -> Result<(), Error> {
        if [self.before, self.from, self.to]
            .into_iter()
            .flatten()
            .any(|n| n < 1)
            || self.from.is_some() != self.to.is_some()
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct Revision {
    id: i64,
    actor: Option<Uuid>,
    timestamp: String,
    saved_at: String,
    title: String,
    restored_from: Option<i64>,
    baseline: bool,
}

struct Difference {
    label: String,
    before: String,
    after: String,
}

fn differences(target: &Target<'_>, lang: &str, from: &Value, to: &Value) -> Vec<Difference> {
    let bundle = bundle(lang);
    let display = |snapshot: &Value, field: &str| {
        if field == "is_changelog" {
            return snapshot[field]
                .as_bool()
                .map_or_else(String::new, |is_changelog| {
                    translated(&bundle, if is_changelog { "changelog" } else { "news" })
                });
        }
        if field == "tags" {
            return snapshot[field].as_array().map_or_else(String::new, |tags| {
                tags.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            });
        }
        let value = value(snapshot, field);
        match (field, value.as_str()) {
            ("publication_state", "published") => translated(&bundle, "published-status"),
            ("publication_state", "draft") | ("severity", "info" | "maintenance" | "warning") => {
                translated(&bundle, &value)
            }
            _ => value,
        }
    };
    target
        .fields()
        .iter()
        .filter(|(field, _)| from[field] != to[field])
        .map(|(field, label)| Difference {
            label: translated(&bundle, label),
            before: display(from, field),
            after: display(to, field),
        })
        .collect()
}

#[derive(Template)]
#[template(path = "content/revisions.html", config = "src/pages/askama.toml")]
struct History<'a> {
    lang: &'a str,
    bundle: fluent_bundle::FluentBundle<fluent_bundle::FluentResource>,
    history_path: String,
    back_path: String,
    back_label: &'static str,
    locale: &'a str,
    csrf: String,
    restore: Option<Current>,
    missing_source: bool,
    rows: Vec<Revision>,
    older: Option<i64>,
    cursor: Option<i64>,
    compare: bool,
    from: i64,
    to: i64,
    diff: Vec<Difference>,
}

impl History<'_> {
    fn t(&self, key: &str) -> String {
        translated(&self.bundle, key)
    }
}

pub(crate) async fn history(
    pool: &PgPool,
    target: Target<'_>,
    lang: &str,
    query: Query,
    session: &oteryn_web::session::BrowserSession,
) -> Result<Response, StatusCode> {
    query.validate().map_err(Error::status)?;
    let mut connection = pool
        .acquire()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    // Read the optimistic token before the list. A concurrent edit can only
    // make the form stale, never authorize a restore against an unseen edit.
    let current = target
        .current(&mut connection)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let (kind, key, locale) = target.scope();
    let mut rows: Vec<Revision> = sqlx::query_as(
        "SELECT id,actor,restored_from,baseline,snapshot->>'title' AS title,
                to_char(saved_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS timestamp,
                to_char(saved_at AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS \"UTC\"') AS saved_at
         FROM editorial_revisions WHERE kind=$1 AND object_key=$2 AND locale=$3 AND id <= $4
         ORDER BY id DESC LIMIT 11",
    ).bind(kind).bind(&key).bind(locale).bind(query.before.map_or(i64::MAX, |id| id - 1))
     .fetch_all(&mut *connection).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if current.is_none()
        && rows.is_empty()
        && !target
            .has_history(&mut connection)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    {
        return Err(StatusCode::NOT_FOUND);
    }
    let older = (rows.len() == 11).then(|| rows[9].id);
    rows.truncate(10);
    let mut from = query
        .from
        .unwrap_or_else(|| rows.get(1).map_or(0, |row| row.id));
    let to = query
        .to
        .unwrap_or_else(|| rows.first().map_or(0, |row| row.id));
    if from == 0 {
        from = to;
    }
    let compare = query.from.is_some();
    let diff = if compare {
        let before = target
            .snapshot(&mut connection, from)
            .await
            .map_err(Error::status)?;
        let after = target
            .snapshot(&mut connection, to)
            .await
            .map_err(Error::status)?;
        differences(&target, lang, &before, &after)
    } else {
        Vec::new()
    };
    let (back_path, back_label) = if current.is_some() {
        (target.edit_path(), "back")
    } else {
        (target.index_path().into(), "back-list")
    };
    let missing_source = locale == "pl"
        && current
            .as_ref()
            .is_some_and(|edition| edition.source_revision == 0);
    let restore = current.filter(|_| !missing_source);
    drop(connection);
    let csrf = if restore.is_some() && !rows.is_empty() {
        session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    } else {
        String::new()
    };
    Ok(oteryn_web::render(&History {
        lang,
        bundle: bundle(lang),
        history_path: target.history_path(),
        back_path,
        back_label,
        locale,
        csrf,
        restore,
        missing_source,
        rows,
        older,
        cursor: query.before,
        compare,
        from,
        to,
        diff,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i18n_parity_and_comparison() {
        let keys = |source: &'static str| {
            source
                .lines()
                .filter_map(|line| line.split_once(" = ").map(|(key, _)| key))
                .collect::<std::collections::BTreeSet<_>>()
        };
        assert_eq!(keys(EN), keys(PL));
        for lang in ["en", "pl"] {
            let messages = bundle(lang);
            assert!(
                keys(EN)
                    .into_iter()
                    .all(|key| !translated(&messages, key).is_empty())
            );
            let target = Target::News(Uuid::nil());
            assert!(
                target
                    .fields()
                    .iter()
                    .all(|(_, key)| !translated(&messages, key).is_empty())
            );
            let diff = differences(
                &target,
                lang,
                &serde_json::json!({"title":"Earlier","body":"Same","updated_at":"old"}),
                &serde_json::json!({"title":"Later","body":"Same","updated_at":"new"}),
            );
            assert_eq!(diff.len(), 1);
            assert_eq!(diff[0].before, "Earlier");
            assert_eq!(diff[0].after, "Later");
        }
    }

    #[test]
    fn invalid_comparisons_are_rejected() {
        for query in [
            Query {
                from: Some(1),
                ..Query::default()
            },
            Query {
                from: Some(1),
                to: Some(-1),
                ..Query::default()
            },
            Query {
                before: Some(0),
                ..Query::default()
            },
        ] {
            assert!(matches!(query.validate(), Err(Error::Invalid)));
        }
    }
}
