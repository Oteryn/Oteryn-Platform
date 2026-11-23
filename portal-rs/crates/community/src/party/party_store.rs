use super::*;
use oteryn_admin::audit::{self, AuditContext, Entry};
use serde_json::json;
use std::collections::HashMap;

pub(super) const REPORT_BATCH_SIZE: usize = 50;
const DISCOVERY_PAGE_SIZE: usize = 100;

pub(super) struct Discovery {
    pub rows: Vec<Listing>,
    pub has_next: bool,
}

#[derive(Debug)]
pub(super) struct Listing {
    pub id: Uuid,
    pub name: String,
    pub owner: Uuid,
    pub character: Uuid,
    pub world_id: Uuid,
    pub world: Option<String>,
    pub status: String,
    pub active: bool,
    pub purpose: String,
    pub min: i32,
    pub max: i32,
    pub profession: String,
    pub note: String,
    pub starts: String,
    pub expires: String,
    pub reports: String,
    pub report_ids: String,
    pub more_reports: i64,
}
impl Board {
    /// Expiry is enforced by reads and mutations even if the worker is stopped.
    pub async fn expire(&self) -> Result<u64, Error> {
        Ok(sqlx::query("UPDATE pd_party_posts SET status='expired', updated_at=now() WHERE status='active' AND expires_at<=now()")
            .execute(&self.db).await?.rows_affected())
    }
    pub(super) async fn list(&self, filter: &Filter) -> Result<Discovery, Error> {
        if !filter.valid() {
            return Err(Error::Invalid);
        }
        let page = filter.page.unwrap_or(1);
        let rows = sqlx::query(
            "SELECT p.*, w.name AS world_name, p.status AS effective_status,
                    to_char(p.starts_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI') AS starts,
                    to_char(p.expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI') AS expires
             FROM pd_party_posts p LEFT JOIN world_status w ON w.id=p.world_id
             WHERE p.status='active' AND p.expires_at>clock_timestamp()
               AND EXISTS (
                   SELECT 1 FROM account_characters c WHERE c.account_id=p.account_id
                     AND c.character_id=p.character_id AND c.world_id=p.world_id
                     AND c.availability='AVAILABLE' AND c.name=p.character_name
               )
               AND ($1::uuid IS NULL OR p.world_id=$1)
               AND ($2::text IS NULL OR p.purpose=$2)
               AND ($3::int IS NULL OR $3 BETWEEN p.min_level AND p.max_level)
               AND ($4::text IS NULL OR $4='any' OR p.profession='any' OR p.profession=$4)
             ORDER BY p.starts_at,p.id LIMIT $5 OFFSET $6",
        )
        .bind(filter.world)
        .bind(&filter.purpose)
        .bind(filter.level)
        .bind(&filter.profession)
        .bind((DISCOVERY_PAGE_SIZE + 1) as i64)
        .bind(i64::from(page - 1) * DISCOVERY_PAGE_SIZE as i64)
        .fetch_all(&self.db)
        .await?;
        let has_next = page < MAX_DISCOVERY_PAGE && rows.len() > DISCOVERY_PAGE_SIZE;
        let mut visible = Vec::with_capacity(rows.len().min(DISCOVERY_PAGE_SIZE));
        let mut proofs = HashMap::new();
        for row in rows.into_iter().take(DISCOVERY_PAGE_SIZE) {
            let row = listing(row)?;
            let key = (row.owner, row.character);
            let proof = match proofs.entry(key) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(self.authority.character(row.owner, row.character).await)
                }
            };
            if proof.as_ref().is_some_and(|character| {
                character.id == row.character
                    && character.world == row.world_id
                    && character.name == row.name
                    && valid_character(character)
            }) {
                visible.push(row);
            }
        }
        Ok(Discovery {
            rows: visible,
            has_next,
        })
    }

    pub(super) async fn worlds(&self) -> Result<Vec<(Uuid, String)>, Error> {
        Ok(
            sqlx::query_as("SELECT id,name FROM world_status ORDER BY name,id LIMIT 256")
                .fetch_all(&self.db)
                .await?,
        )
    }

    pub(super) async fn characters(&self, actor: Uuid) -> Result<Vec<Character>, Error> {
        let ids = sqlx::query_scalar(
            "SELECT character_id FROM account_characters
             WHERE account_id=$1 AND availability='AVAILABLE' ORDER BY name,character_id LIMIT 100",
        )
        .bind(actor)
        .fetch_all(&self.db)
        .await?;
        let mut qualified = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(character) = self.authority.character(actor, id).await
                && character.id == id
                && valid_character(&character)
            {
                qualified.push(character);
            }
        }
        Ok(qualified)
    }

    pub(super) async fn owned(&self, actor: Uuid) -> Result<Vec<Listing>, Error> {
        let rows = sqlx::query(
            "SELECT p.*, w.name AS world_name, to_char(p.starts_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI') AS starts,
                    to_char(p.expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI') AS expires,
                    CASE WHEN p.status='active' AND p.expires_at<=clock_timestamp()
                         THEN 'expired' ELSE p.status END AS effective_status
             FROM pd_party_posts p LEFT JOIN world_status w ON w.id=p.world_id
             WHERE p.account_id=$1 ORDER BY (p.status='active' AND p.expires_at>clock_timestamp()) DESC,
             p.created_at DESC,p.id DESC LIMIT 100",
        )
        .bind(actor)
        .fetch_all(&self.db)
        .await?;
        rows.into_iter().map(listing).collect()
    }

    /// Feed freshness is a diagnostic. Character eligibility still needs the
    /// account snapshot, ownership cut and a matching per-character profile.
    pub(super) async fn feed_status(&self) -> Result<(bool, bool), Error> {
        Ok(sqlx::query_as(
            "WITH stamp AS (SELECT floor(extract(epoch FROM statement_timestamp()))::bigint AS now)
             SELECT EXISTS (
                 SELECT 1 FROM account_character_feed f, stamp
                 WHERE f.id=1 AND f.highest_epoch=f.watermark_epoch
                   AND f.source_authority<>'' AND f.complete_through BETWEEN now-29 AND now+1
             ), EXISTS (
                 SELECT 1 FROM public_snapshots w
                 JOIN public_projection_state p ON p.id=1 AND p.highest_epoch=w.epoch
                 CROSS JOIN stamp
                 WHERE w.family='character_profile' AND w.is_watermark
                   AND NOT public_projection_conflicted('character_profile')
                   AND w.watermark BETWEEN now-29 AND now+1
             )",
        )
        .fetch_one(&self.db)
        .await?)
    }

    pub async fn create(&self, owner: Uuid, draft: Draft) -> Result<Uuid, Error> {
        if owner.is_nil() {
            return Err(Error::Denied);
        }
        self.authority.active(owner).await.ok_or(Error::Denied)?;
        let admitted_at: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&self.db)
                .await?;
        if !draft.valid(admitted_at) {
            return Err(Error::Invalid);
        }
        let mut tx = self.db.begin().await?;
        // The shared acquisition budget bounds every wait in this transaction, the request lock included.
        oteryn_db::mutation_budget(&mut tx).await?;
        // Serialize per-account caps across serving processes; hash collisions only reduce throughput.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,83))")
            .bind(owner.to_string())
            .execute(&mut *tx)
            .await?;
        let character = self
            .authority
            .character_in(&mut tx, owner, draft.character)
            .await
            .ok_or(Error::Denied)?;
        if character.id != draft.character || !valid_character(&character) {
            return Err(Error::Denied);
        }
        self.authority
            .active_in(&mut tx, owner)
            .await
            .ok_or(Error::Denied)?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *tx)
                .await?;
        if !draft.valid(now) {
            return Err(Error::Invalid);
        }
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_party_posts WHERE account_id=$1 AND status='active' AND expires_at>clock_timestamp()")
            .bind(owner).fetch_one(&mut *tx).await?;
        let recent: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_party_posts WHERE account_id=$1 AND created_at>clock_timestamp()-interval '1 hour'")
            .bind(owner).fetch_one(&mut *tx).await?;
        if count >= 5 || recent >= 10 {
            return Err(Error::Limited);
        }
        let id = sqlx::query_scalar("INSERT INTO pd_party_posts(account_id,character_id,character_name,world_id,purpose,min_level,max_level,profession,note,starts_at,expires_at,created_at,updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,to_timestamp($10),to_timestamp($11),clock_timestamp(),clock_timestamp()) RETURNING id")
            .bind(owner).bind(character.id).bind(character.name).bind(character.world).bind(draft.purpose)
            .bind(draft.min_level).bind(draft.max_level).bind(draft.profession).bind(draft.note.trim())
            .bind(draft.starts_at as f64).bind(draft.expires_at as f64).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(id)
    }
    pub async fn report(&self, actor: Uuid, id: Uuid, reason: &str) -> Result<(), Error> {
        self.authority.active(actor).await.ok_or(Error::Denied)?;
        self.authority
            .reporting_allowed(actor)
            .await
            .ok_or(Error::Denied)?;
        if actor.is_nil() || id.is_nil() || !text(reason, 1, 500) {
            return Err(Error::Invalid);
        }
        let mut tx = self.db.begin().await?;
        oteryn_db::mutation_budget(&mut tx).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,83))")
            .bind(actor.to_string())
            .execute(&mut *tx)
            .await?;
        self.authority
            .active_in(&mut tx, actor)
            .await
            .ok_or(Error::Denied)?;
        self.authority
            .reporting_allowed_in(&mut tx, actor)
            .await
            .ok_or(Error::Denied)?;
        let owner: Option<Uuid> = sqlx::query_scalar(
            "SELECT account_id FROM pd_party_posts WHERE id=$1 AND status='active' FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        let owner = owner.ok_or(Error::Missing)?;
        let current: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pd_party_posts WHERE id=$1 AND expires_at>clock_timestamp())").bind(id).fetch_one(&mut *tx).await?;
        if !current {
            return Err(Error::Missing);
        }

        if owner == actor {
            return Err(Error::Denied);
        }
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pd_party_reports WHERE post_id=$1 AND account_id=$2)",
        )
        .bind(id)
        .bind(actor)
        .fetch_one(&mut *tx)
        .await?;
        if exists {
            return Ok(());
        }
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_party_reports WHERE account_id=$1 AND created_at>clock_timestamp()-interval '1 hour'").bind(actor).fetch_one(&mut *tx).await?;
        let pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pd_party_reports WHERE account_id=$1 AND status='open'",
        )
        .bind(actor)
        .fetch_one(&mut *tx)
        .await?;
        if count >= 20 || pending >= 20 {
            return Err(Error::Limited);
        }
        // A repeated report is idempotent and never overwrites the original evidence.
        sqlx::query("INSERT INTO pd_party_reports(post_id,account_id,reason,created_at,updated_at) VALUES($1,$2,$3,clock_timestamp(),clock_timestamp()) ON CONFLICT(post_id,account_id) DO NOTHING")
            .bind(id).bind(actor).bind(reason.trim()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn remove(&self, owner: Uuid, id: Uuid) -> Result<(), Error> {
        self.authority.active(owner).await.ok_or(Error::Denied)?;
        let mut tx = self.db.begin().await?;
        self.authority
            .active_in(&mut tx, owner)
            .await
            .ok_or(Error::Denied)?;
        if owner.is_nil() || id.is_nil() {
            return Err(Error::Denied);
        }
        let result = sqlx::query("UPDATE pd_party_posts SET status='removed', updated_at=now() WHERE id=$1 AND account_id=$2 AND status='active' AND expires_at>clock_timestamp()")
            .bind(id).bind(owner).execute(&mut *tx).await?;
        if result.rows_affected() != 1 {
            return Err(Error::Missing);
        }
        tx.commit().await?;
        Ok(())
    }
    pub(super) async fn reports(&self) -> Result<Vec<Listing>, Error> {
        let rows = sqlx::query(
            "SELECT p.*, w.name AS world_name, to_char(p.starts_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI') AS starts,
                    to_char(p.expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI') AS expires,
                    CASE WHEN p.status='active' AND p.expires_at<=clock_timestamp()
                         THEN 'expired' ELSE p.status END AS effective_status,
                    batch.reasons, batch.report_ids, batch.more_reports
             FROM (
                 SELECT p.* FROM pd_party_posts p
                 WHERE EXISTS (
                     SELECT 1 FROM pd_party_reports r WHERE r.post_id=p.id AND r.status='open'
                 )
                 ORDER BY p.created_at, p.id LIMIT 100
             ) p
             LEFT JOIN world_status w ON w.id=p.world_id
             CROSS JOIN LATERAL (
                 SELECT string_agg(reason, ' | ' ORDER BY created_at, id) AS reasons,
                        string_agg(id::text, ',' ORDER BY created_at, id) AS report_ids,
                        (SELECT count(*) FROM pd_party_reports r
                         WHERE r.post_id=p.id AND r.status='open') - count(*) AS more_reports
                 FROM (
                     SELECT r.id, r.reason, r.created_at
                     FROM pd_party_reports r
                     WHERE r.post_id=p.id AND r.status='open'
                     ORDER BY r.created_at, r.id
                     LIMIT $1
                 ) shown
             ) batch
             ORDER BY p.created_at, p.id",
        )
        .bind(REPORT_BATCH_SIZE as i64)
        .fetch_all(&self.db)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(Listing {
                    reports: row.try_get("reasons")?,
                    report_ids: row.try_get("report_ids")?,
                    more_reports: row.try_get("more_reports")?,
                    ..listing(row)?
                })
            })
            .collect()
    }
    pub async fn moderate(
        &self,
        context: oteryn_admin::rbac::CheckContext,
        authority: &oteryn_admin::rbac::MutationAuthorization,
        id: Uuid,
        action: &str,
        reason: &str,
        report_ids: &[Uuid],
    ) -> Result<(), Error> {
        let actor = context.account;
        if actor.is_nil() || id.is_nil() || !context.require_staff_mfa || !context.mfa_verified {
            return Err(Error::Denied);
        }
        self.authority.active(actor).await.ok_or(Error::Denied)?;
        if !matches!(action, "remove" | "dismiss")
            || !text(reason, 1, 500)
            || report_ids.len() > REPORT_BATCH_SIZE
        {
            return Err(Error::Invalid);
        }
        let mut tx = self.db.begin().await?;
        oteryn_admin::rbac::authorize_mutation_with_resources_domain::<Error, _>(
            &mut tx,
            &[actor],
            move |connection| {
                Box::pin(async move {
                    sqlx::query("SELECT id FROM pd_party_posts WHERE id=$1 FOR UPDATE")
                        .bind(id)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            },
            authority,
            context,
            &[
                oteryn_admin::rbac::Permission::Access,
                oteryn_admin::rbac::Permission::ManageSupportReports,
            ],
        )
        .await?;
        let before: String = sqlx::query_scalar("SELECT status FROM pd_party_posts WHERE id=$1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::Missing)?;
        let after = if action == "remove" {
            "removed"
        } else {
            &before
        };
        sqlx::query("UPDATE pd_party_posts SET status=$2,updated_at=now() WHERE id=$1")
            .bind(id)
            .bind(after)
            .execute(&mut *tx)
            .await?;
        let resolved = sqlx::query("UPDATE pd_party_reports SET status='resolved',updated_at=now() WHERE post_id=$1 AND status='open' AND id=ANY($2)")
            .bind(id).bind(report_ids).execute(&mut *tx).await?;
        if resolved.rows_affected() != report_ids.len() as u64 {
            return Err(Error::Missing);
        }
        audit::append(
            &mut tx,
            Entry {
                context: AuditContext {
                    actor: Some(actor),
                    request_id: Uuid::now_v7(),
                },
                action: "party.moderate",
                target: &id.to_string(),
                before: json!({"status":before}),
                after: json!({"status":after,"decision":action,"reason":reason}),
            },
        )
        .await
        .map_err(|_| Error::Denied)?;
        authority
            .check_in(
                &mut tx,
                context,
                oteryn_admin::rbac::Permission::ManageSupportReports,
            )
            .await
            .map_err(|_| Error::Denied)?;
        tx.commit().await?;
        Ok(())
    }
}
fn listing(row: sqlx::postgres::PgRow) -> Result<Listing, Error> {
    let status: String = row.try_get("effective_status")?;
    Ok(Listing {
        id: row.try_get("id")?,
        name: row.try_get("character_name")?,
        owner: row.try_get("account_id")?,
        character: row.try_get("character_id")?,
        world_id: row.try_get("world_id")?,
        world: row.try_get("world_name")?,
        active: status == "active",
        status,
        purpose: row.try_get("purpose")?,
        min: row.try_get("min_level")?,
        max: row.try_get("max_level")?,
        profession: row.try_get("profession")?,
        note: row.try_get("note")?,
        starts: row.try_get("starts")?,
        expires: row.try_get("expires")?,
        reports: String::new(),
        report_ids: String::new(),
        more_reports: 0,
    })
}
