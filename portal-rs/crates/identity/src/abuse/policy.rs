//! Settings shared by the browser registration gate and the staff policy editor.

use serde::{Deserialize, Serialize};
use sqlx::{Executor, PgConnection, Postgres, query_as};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct Policy {
    pub enabled: bool,
    pub reason_pl: String,
    pub reason_en: String,
    pub work_bits: i16,
    pub work_ttl_seconds: i32,
    pub challenge_limit: i16,
    pub attempt_limit: i16,
    pub rate_window_seconds: i32,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid registration policy")]
pub struct InvalidPolicy;

impl Policy {
    pub fn validated(mut self) -> Result<Self, InvalidPolicy> {
        self.reason_pl = self.reason_pl.trim().replace("\r\n", "\n");
        self.reason_en = self.reason_en.trim().replace("\r\n", "\n");
        if (!self.enabled && (self.reason_pl.is_empty() || self.reason_en.is_empty()))
            || [&self.reason_pl, &self.reason_en].iter().any(|reason| {
                reason.chars().count() > 500 || reason.chars().any(|c| c.is_control() && c != '\n')
            })
            || !(16..=18).contains(&self.work_bits)
            || !(300..=900).contains(&self.work_ttl_seconds)
            || !(1..=5).contains(&self.challenge_limit)
            || !(1..=5).contains(&self.attempt_limit)
            || !(60..=3600).contains(&self.rate_window_seconds)
        {
            return Err(InvalidPolicy);
        }
        Ok(self)
    }

    pub(crate) fn reason(&self, language: &str) -> &str {
        if language == "en" {
            &self.reason_en
        } else {
            &self.reason_pl
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct StoredPolicy {
    #[sqlx(flatten)]
    pub values: Policy,
    pub revision: i64,
}

const SELECT: &str = "SELECT enabled,reason_pl,reason_en,work_bits,work_ttl_seconds,
    challenge_limit,attempt_limit,rate_window_seconds,revision
    FROM registration_abuse_policy WHERE singleton";

pub async fn read<'e>(
    database: impl Executor<'e, Database = Postgres>,
) -> Result<StoredPolicy, sqlx::Error> {
    query_as(SELECT).fetch_one(database).await
}

pub async fn update(connection: &mut PgConnection, values: &Policy) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE registration_abuse_policy SET enabled=$1,reason_pl=$2,reason_en=$3,
         work_bits=$4,work_ttl_seconds=$5,challenge_limit=$6,attempt_limit=$7,
         rate_window_seconds=$8,revision=revision+1,updated_at=clock_timestamp()
         WHERE singleton",
    )
    .bind(values.enabled)
    .bind(&values.reason_pl)
    .bind(&values.reason_en)
    .bind(values.work_bits)
    .bind(values.work_ttl_seconds)
    .bind(values.challenge_limit)
    .bind(values.attempt_limit)
    .bind(values.rate_window_seconds)
    .execute(connection)
    .await?;
    Ok(())
}
