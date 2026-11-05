//! Transactional editorial storage. Every staff operation rechecks current grants.
use crate::domain::{self, Action, Error, Translation};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{AccountStateReader, Authorizer, CheckContext, Permission},
};
use serde_json::json;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Guide {
    pub id: Uuid,
    pub status: String,
    pub version: i64,
    pub revision_version: i64,
    pub entity_key: Option<String>,
}
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Revision {
    pub version: i64,
    pub locale: String,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub markdown: String,
    pub editor: Uuid,
    pub change_note: String,
    pub source_version: Option<i64>,
}
#[derive(Debug)]
pub struct Repository<A> {
    pool: PgPool,
    auth: Authorizer<A>,
}
impl<A: AccountStateReader> Repository<A> {
    pub fn new(pool: PgPool, accounts: A) -> Self {
        Self {
            auth: Authorizer::new(pool.clone(), accounts),
            pool,
        }
    }
    pub async fn authorize(
        &self,
        context: CheckContext,
        permission: Permission,
    ) -> Result<(), Error> {
        if !context.require_staff_mfa || !context.mfa_verified {
            return Err(Error::Denied);
        }
        for required in [Permission::Access, Permission::WikiAccess, permission] {
            if let Err(error) = self.auth.check(context, required).await {
                if matches!(error, oteryn_admin::rbac::Error::Denied) && !context.account.is_nil() {
                    let mut tx = self.pool.begin().await?;
                    audit::append(
                        &mut tx,
                        Entry {
                            context: AuditContext {
                                actor: Some(context.account),
                                request_id: Uuid::now_v7(),
                            },
                            action: "wiki.access.denied",
                            target: required.as_str(),
                            before: json!(null),
                            after: json!({"allowed":false}),
                        },
                    )
                    .await?;
                    tx.commit().await?;
                }
                return Err(error.into());
            }
        }
        Ok(())
    }
    pub async fn list(&self, context: CheckContext) -> Result<Vec<Guide>, Error> {
        self.authorize(context, Permission::WikiAccess).await?;
        Ok(sqlx::query_as("SELECT id,status,version,revision_version,entity_key FROM wiki_guides ORDER BY updated_at DESC,id LIMIT 100").fetch_all(&self.pool).await?)
    }
    pub async fn get(
        &self,
        context: CheckContext,
        id: Uuid,
    ) -> Result<(Guide, Vec<Translation>), Error> {
        self.authorize(context, Permission::WikiAccess).await?;
        let mut tx = self.pool.begin().await?;
        let guide = lock(&mut tx, id).await?;
        let values = values(&mut tx, id, guide.revision_version).await?;
        tx.commit().await?;
        Ok((guide, values))
    }
    pub async fn history(&self, context: CheckContext, id: Uuid) -> Result<Vec<Revision>, Error> {
        self.authorize(context, Permission::ManageWikiArticles)
            .await?;
        Ok(sqlx::query_as("SELECT version,locale,title,slug,summary,markdown,editor,change_note,source_version FROM wiki_revisions WHERE guide_id=$1 ORDER BY version DESC,locale").bind(id).fetch_all(&self.pool).await?)
    }
    pub async fn create(
        &self,
        context: CheckContext,
        values: &[Translation],
        entity: Option<&str>,
        note: &str,
    ) -> Result<Uuid, Error> {
        self.authorize(context, Permission::ManageWikiArticles)
            .await?;
        domain::translations(values, false)?;
        if note.chars().count() > 500 || entity.is_some_and(|v| v.is_empty() || v.len() > 256) {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        reserve(&mut tx, values, None).await?;
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO wiki_guides(id,entity_key) VALUES($1,$2)")
            .bind(id)
            .bind(entity)
            .execute(&mut *tx)
            .await?;
        append(&mut tx, id, 1, values, context.account, note, None).await?;
        record(
            &mut tx,
            context,
            id,
            "wiki.created",
            json!(null),
            json!({"version":1}),
        )
        .await?;
        tx.commit().await?;
        Ok(id)
    }
    pub async fn edit(
        &self,
        context: CheckContext,
        id: Uuid,
        expected: i64,
        input: &[Translation],
        note: &str,
    ) -> Result<(), Error> {
        self.authorize(context, Permission::ManageWikiArticles)
            .await?;
        domain::translations(input, false)?;
        if note.chars().count() > 500 {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        // Same lock ordering for every slug reservation, including restore.
        slug_lock(&mut tx).await?;
        let guide = lock(&mut tx, id).await?;
        editable(&guide, expected)?;
        reserve(&mut tx, input, Some(id)).await?;
        let next = guide.version.checked_add(1).ok_or(Error::Conflict)?;
        append(&mut tx, id, next, input, context.account, note, None).await?;
        update_revision(&mut tx, id, next).await?;
        record(
            &mut tx,
            context,
            id,
            "wiki.updated",
            json!({"version":expected}),
            json!({"version":next}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
    /// Restore appends a new snapshot and invalidates review; source rows never change.
    pub async fn restore(
        &self,
        context: CheckContext,
        id: Uuid,
        expected: i64,
        source: i64,
    ) -> Result<(), Error> {
        self.authorize(context, Permission::PublishWiki).await?;
        let mut tx = self.pool.begin().await?;
        slug_lock(&mut tx).await?;
        let guide = lock(&mut tx, id).await?;
        editable(&guide, expected)?;
        let input = values(&mut tx, id, source).await?;
        domain::translations(&input, false)?;
        reserve(&mut tx, &input, Some(id)).await?;
        let next = expected.checked_add(1).ok_or(Error::Conflict)?;
        append(&mut tx, id, next, &input, context.account, "", Some(source)).await?;
        update_revision(&mut tx, id, next).await?;
        record(
            &mut tx,
            context,
            id,
            "wiki.restored",
            json!({"version":expected}),
            json!({"version":next,"source":source}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn transition(
        &self,
        context: CheckContext,
        id: Uuid,
        expected: i64,
        action: Action,
    ) -> Result<(), Error> {
        self.authorize(
            context,
            if action.publisher() {
                Permission::PublishWiki
            } else {
                Permission::ManageWikiArticles
            },
        )
        .await?;
        let mut tx = self.pool.begin().await?;
        let guide = lock(&mut tx, id).await?;
        if guide.version != expected {
            return Err(Error::Conflict);
        }
        let target = action.target(&guide.status)?;
        if action == Action::Publish {
            domain::translations(&values(&mut tx, id, guide.revision_version).await?, true)?;
            let submitted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM wiki_reviews WHERE guide_id=$1 AND revision_version=$2 AND action='submit-review')").bind(id).bind(guide.revision_version).fetch_one(&mut *tx).await?;
            if !submitted {
                return Err(Error::Conflict);
            }
        }
        sqlx::query("INSERT INTO wiki_reviews(id,guide_id,revision_version,actor,action) VALUES($1,$2,$3,$4,$5)").bind(Uuid::now_v7()).bind(id).bind(guide.revision_version).bind(context.account).bind(action.name()).execute(&mut *tx).await?;
        sqlx::query("UPDATE wiki_guides SET status=$2,version=version+1,published_at=CASE WHEN $2='published' THEN now() ELSE NULL END,updated_at=now() WHERE id=$1").bind(id).bind(target).execute(&mut *tx).await?;
        record(
            &mut tx,
            context,
            id,
            &format!("wiki.{}", action.name()),
            json!({"status":guide.status,"version":guide.version}),
            json!({"status":target,"version":guide.version+1}),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
fn editable(guide: &Guide, expected: i64) -> Result<(), Error> {
    if guide.version != expected || !matches!(guide.status.as_str(), "draft" | "in_review") {
        return Err(Error::Conflict);
    }
    Ok(())
}
async fn lock(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<Guide, Error> {
    sqlx::query_as("SELECT id,status,version,revision_version,entity_key FROM wiki_guides WHERE id=$1 FOR UPDATE").bind(id).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)
}
async fn values(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    version: i64,
) -> Result<Vec<Translation>, Error> {
    Ok(sqlx::query_as("SELECT locale,title,slug,summary,markdown FROM wiki_revisions WHERE guide_id=$1 AND version=$2 ORDER BY locale").bind(id).bind(version).fetch_all(&mut **tx).await?)
}
async fn slug_lock(tx: &mut Transaction<'_, Postgres>) -> Result<(), Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(20261105220206)")
        .execute(&mut **tx)
        .await?;
    Ok(())
}
async fn reserve(
    tx: &mut Transaction<'_, Postgres>,
    input: &[Translation],
    id: Option<Uuid>,
) -> Result<(), Error> {
    slug_lock(tx).await?;
    for value in input {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM wiki_revisions WHERE locale=$1 AND slug=$2 AND ($3::uuid IS NULL OR guide_id<>$3))").bind(&value.locale).bind(&value.slug).bind(id).fetch_one(&mut **tx).await?;
        if exists {
            return Err(Error::Conflict);
        }
    }
    Ok(())
}
async fn append(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    version: i64,
    input: &[Translation],
    editor: Uuid,
    note: &str,
    source: Option<i64>,
) -> Result<(), Error> {
    for value in input {
        sqlx::query("INSERT INTO wiki_revisions(id,guide_id,version,locale,title,slug,summary,markdown,editor,change_note,source_version) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)").bind(Uuid::now_v7()).bind(id).bind(version).bind(&value.locale).bind(&value.title).bind(&value.slug).bind(&value.summary).bind(&value.markdown).bind(editor).bind(note).bind(source).execute(&mut **tx).await?;
    }
    Ok(())
}
async fn update_revision(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    next: i64,
) -> Result<(), Error> {
    sqlx::query("UPDATE wiki_guides SET version=$2,revision_version=$2,status='draft',published_at=NULL,updated_at=now() WHERE id=$1").bind(id).bind(next).execute(&mut **tx).await?;
    Ok(())
}
async fn record(
    tx: &mut Transaction<'_, Postgres>,
    context: CheckContext,
    id: Uuid,
    action: &str,
    before: serde_json::Value,
    after: serde_json::Value,
) -> Result<(), Error> {
    audit::append(
        tx,
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
    .await?;
    Ok(())
}
/// Public queries share the exact publication boundary and never expose draft source.
#[derive(Debug, Clone)]
pub struct PublicRepository {
    pool: PgPool,
}
impl PublicRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn article(&self, locale: &str, slug: &str) -> Result<Translation, Error> {
        if !matches!(locale, "pl" | "en") || !domain::slug(slug) {
            return Err(Error::NotFound);
        }
        sqlx::query_as("SELECT r.locale,r.title,r.slug,r.summary,r.markdown FROM wiki_revisions r JOIN wiki_guides g ON g.id=r.guide_id AND g.revision_version=r.version WHERE g.status='published' AND g.published_at<=now() AND r.locale=$1 AND r.slug=$2").bind(locale).bind(slug).fetch_optional(&self.pool).await?.ok_or(Error::NotFound)
    }
    pub async fn search(
        &self,
        locale: &str,
        query: &str,
        page: i64,
        browse: bool,
    ) -> Result<Vec<Translation>, Error> {
        if !matches!(locale, "pl" | "en") {
            return Err(Error::Invalid);
        }
        let query = domain::search(query, page)?;
        if query.is_empty() && !browse {
            return Ok(Vec::new());
        }
        Ok(sqlx::query_as("SELECT r.locale,r.title,r.slug,r.summary,''::text AS markdown FROM wiki_revisions r JOIN wiki_guides g ON g.id=r.guide_id AND g.revision_version=r.version WHERE g.status='published' AND g.published_at<=now() AND r.locale=$1 AND ($2='' OR r.search_document @@ plainto_tsquery('simple',$2)) ORDER BY ts_rank(r.search_document,plainto_tsquery('simple',$2)) DESC,r.title,g.id LIMIT 12 OFFSET $3").bind(locale).bind(query).bind((page-1)*12).fetch_all(&self.pool).await?)
    }
}
