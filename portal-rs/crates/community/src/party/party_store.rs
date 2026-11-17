use super::*;
use oteryn_admin::audit::{self, AuditContext, Entry};
use serde_json::json;

pub(super) const REPORT_BATCH_SIZE: usize = 50;

#[derive(Debug)]
pub(super) struct Listing {
    pub id: Uuid,
    pub name: String,
    pub world: Uuid,
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
    pub(super) async fn list(&self, filter: &Filter) -> Result<Vec<Listing>, Error> {
        if !filter.valid() {
            return Err(Error::Invalid);
        }
        let rows = sqlx::query("SELECT *, starts_at::text AS starts, expires_at::text AS expires FROM pd_party_posts p WHERE status='active' AND expires_at>clock_timestamp() AND EXISTS (SELECT 1 FROM account_characters c WHERE c.account_id=p.account_id AND c.character_id=p.character_id AND c.world_id=p.world_id AND c.availability='AVAILABLE') AND ($1::uuid IS NULL OR world_id=$1) AND ($2::text IS NULL OR purpose=$2) AND ($3::int IS NULL OR $3 BETWEEN min_level AND max_level) AND ($4::text IS NULL OR profession='any' OR profession=$4) ORDER BY starts_at,id LIMIT 100")
            .bind(filter.world).bind(&filter.purpose).bind(filter.level).bind(&filter.profession).fetch_all(&self.db).await?;
        rows.into_iter().map(listing).collect()
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
        if character.id != draft.character
            || character.world.is_nil()
            || !text(&character.name, 1, 64)
            || !(1..=100000).contains(&character.level)
            || !profession(&character.profession)
            || character.profession == "any"
        {
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
            "SELECT p.*, p.starts_at::text AS starts, p.expires_at::text AS expires,
                    batch.reasons, batch.report_ids, batch.more_reports
             FROM (
                 SELECT p.* FROM pd_party_posts p
                 WHERE EXISTS (
                     SELECT 1 FROM pd_party_reports r WHERE r.post_id=p.id AND r.status='open'
                 )
                 ORDER BY p.created_at, p.id LIMIT 100
             ) p
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
        sqlx::query("UPDATE pd_party_reports SET status='resolved',updated_at=now() WHERE post_id=$1 AND status='open' AND id=ANY($2)")
            .bind(id).bind(report_ids).execute(&mut *tx).await?;
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
    Ok(Listing {
        id: row.try_get("id")?,
        name: row.try_get("character_name")?,
        world: row.try_get("world_id")?,
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
