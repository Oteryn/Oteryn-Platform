//! PostgreSQL projection seam for T72. Nothing here acknowledges game records.
use super::{Error, Filter, Flags, Permit, Session};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Repository {
    pool: PgPool,
    flags: Flags,
}
impl Repository {
    pub fn new(pool: PgPool, flags: Flags) -> Self {
        Self { pool, flags }
    }

    /// Persist a complete, reconciled projection under T72's current decision.
    /// Exact retry succeeds; changed immutable facts fail without overwriting.
    /// T72 must not call this for gaps, rare-drop mismatches, pending transfers,
    /// unknown restore generations, expired data or unaccepted party capture.
    pub async fn project(
        &self,
        permit: &Permit,
        session: &Session,
        expires_at: i64,
    ) -> Result<(), Error> {
        if !self.flags.read || (session.party.is_some() && !self.flags.party) {
            return Err(Error::Unavailable);
        }
        session.validate()?;
        if expires_at <= permit.now
            || expires_at <= session.ended_at
            || expires_at > session.ended_at + 90 * 86400
            || session.ended_at > permit.now
        {
            return Err(Error::Invalid);
        }
        let facts = serde_json::to_value(session).map_err(|_| Error::Invalid)?;
        let mut tx = self.pool.begin().await.map_err(|_| Error::Storage)?;
        let result = sqlx::query("INSERT INTO pd_hunt_sessions (account_id, session_id, stream_generation, character_revision, character_id, started_at, ended_at, expires_at, has_party, facts) VALUES ($1,$2,$3,$4::text::numeric,$5,$6,$7,$8,$9,$10) ON CONFLICT (account_id,session_id) DO NOTHING")
            .bind(permit.account).bind(session.session_id).bind(permit.generation)
            .bind(permit.character_revision.to_string()).bind(session.character_id)
            .bind(session.started_at).bind(session.ended_at).bind(expires_at)
            .bind(session.party.is_some()).bind(&facts)
            .execute(&mut *tx).await.map_err(|_| Error::Storage)?;
        if result.rows_affected() == 0 {
            let same: bool = sqlx::query_scalar("SELECT stream_generation=$3 AND character_revision=$4::text::numeric AND expires_at=$5 AND facts=$6 FROM pd_hunt_sessions WHERE account_id=$1 AND session_id=$2 FOR UPDATE")
                .bind(permit.account).bind(session.session_id).bind(permit.generation)
                .bind(permit.character_revision.to_string()).bind(expires_at).bind(&facts)
                .fetch_one(&mut *tx).await.map_err(|_| Error::Storage)?;
            if !same {
                sqlx::query(
                    "UPDATE pd_hunt_access SET allowed=false WHERE account_id=$1 AND session_id=$2",
                )
                .bind(permit.account)
                .bind(session.session_id)
                .execute(&mut *tx)
                .await
                .map_err(|_| Error::Storage)?;
                tx.commit().await.map_err(|_| Error::Storage)?;
                return Err(Error::Invalid);
            }
            // A retry must never restore a revoked or missing access decision.
        } else {
            sqlx::query("INSERT INTO pd_hunt_access (account_id, session_id, stream_generation, character_revision, complete, allowed) VALUES ($1,$2,$3,$4::text::numeric,true,true)")
                .bind(permit.account).bind(session.session_id).bind(permit.generation)
                .bind(permit.character_revision.to_string()).execute(&mut *tx)
                .await.map_err(|_| Error::Storage)?;
        }
        tx.commit().await.map_err(|_| Error::Storage)?;
        Ok(())
    }

    /// Revoke before erasure, opt-out, transfer ambiguity or restore work.
    /// Retain the projection until retention purge; retries cannot resurrect it.
    pub async fn revoke(&self, account: Uuid, session: Uuid) -> Result<(), Error> {
        sqlx::query(
            "UPDATE pd_hunt_access SET allowed=false WHERE account_id=$1 AND session_id=$2",
        )
        .bind(account)
        .bind(session)
        .execute(&self.pool)
        .await
        .map_err(|_| Error::Storage)?;
        Ok(())
    }

    pub async fn list(
        &self,
        account: Uuid,
        now: i64,
        filter: &Filter,
    ) -> Result<Vec<Session>, Error> {
        if !self.flags.read {
            return Err(Error::Unavailable);
        }
        filter.validate()?;
        if !super::v7(account) || now <= 0 {
            return Err(Error::Invalid);
        }
        let rows = sqlx::query("SELECT s.facts FROM pd_hunt_sessions s JOIN pd_hunt_access a USING (account_id, session_id) WHERE s.account_id=$1 AND a.allowed AND a.complete AND a.stream_generation=s.stream_generation AND a.character_revision=s.character_revision AND s.expires_at>$2 AND s.ended_at<=$2 AND ($3::uuid IS NULL OR s.character_id=$3) AND ($4::text IS NULL OR s.facts->'places' ? $4) AND ($5::bigint IS NULL OR s.started_at >= $5) AND ($6::bigint IS NULL OR s.started_at < $6) AND ($7::boolean IS NULL OR s.has_party=$7) AND ($8 OR NOT s.has_party) ORDER BY s.ended_at DESC, s.session_id LIMIT 50 OFFSET $9")
            .bind(account).bind(now).bind(filter.character).bind(&filter.place)
            .bind(filter.from).bind(filter.until).bind(filter.party).bind(self.flags.party)
            .bind(filter.offset.unwrap_or(0)).fetch_all(&self.pool).await.map_err(|_| Error::Storage)?;
        rows.into_iter()
            .map(|row| {
                let facts: serde_json::Value = row.try_get("facts").map_err(|_| Error::Storage)?;
                let session: Session =
                    serde_json::from_value(facts).map_err(|_| Error::Unavailable)?;
                session.validate().map_err(|_| Error::Unavailable)?;
                Ok(session)
            })
            .collect()
    }

    pub async fn detail(&self, account: Uuid, now: i64, id: Uuid) -> Result<Session, Error> {
        if !self.flags.read {
            return Err(Error::Unavailable);
        }
        if !super::v7(account) || !super::v7(id) || now <= 0 {
            return Err(Error::Invalid);
        }
        let facts: Option<serde_json::Value> = sqlx::query_scalar("SELECT s.facts FROM pd_hunt_sessions s JOIN pd_hunt_access a USING (account_id,session_id) WHERE s.account_id=$1 AND s.session_id=$2 AND a.allowed AND a.complete AND a.stream_generation=s.stream_generation AND a.character_revision=s.character_revision AND s.expires_at>$3 AND s.ended_at<=$3 AND ($4 OR NOT s.has_party)")
            .bind(account).bind(id).bind(now).bind(self.flags.party)
            .fetch_optional(&self.pool).await.map_err(|_| Error::Storage)?;
        let session: Session = serde_json::from_value(facts.ok_or(Error::NotFound)?)
            .map_err(|_| Error::Unavailable)?;
        session.validate().map_err(|_| Error::Unavailable)?;
        Ok(session)
    }
}
