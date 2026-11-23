use axum::http::StatusCode;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(crate) enum Action {
    WeeklyNoteSave,
    WeeklyNoteExport,
    PlacesRead,
    PlacesParticipation,
}

impl Action {
    pub(crate) async fn charge(self, pool: &PgPool, account: Uuid) -> Result<(), StatusCode> {
        let (name, limit) = match self {
            Self::WeeklyNoteSave => ("journal-weekly-note-save", 30_i64),
            Self::WeeklyNoteExport => ("journal-weekly-note-export", 10),
            Self::PlacesRead => ("atlas-hunting-places-read", 30),
            Self::PlacesParticipation => ("atlas-hunting-places-participation", 10),
        };
        let digest = Sha256::new()
            .chain_update(name.as_bytes())
            .chain_update(account.as_bytes())
            .finalize();
        // Reuse the shared counter table and its worker cleanup.
        let count: i64 = sqlx::query_scalar(
            "INSERT INTO web_rate_limits(limiter_name,key_hash,window_start,count)
             VALUES ($1,$2,date_trunc('minute',statement_timestamp()),1)
             ON CONFLICT(limiter_name,key_hash,window_start) DO UPDATE
             SET count=least(web_rate_limits.count+1,$3+1)
             RETURNING count",
        )
        .bind(name)
        .bind(&digest[..])
        .bind(limit)
        .fetch_one(pool)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        if count > limit {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        Ok(())
    }
}
