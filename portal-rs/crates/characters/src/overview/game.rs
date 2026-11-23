//! Read only Game facts present in both the private roster and the public profile cut.
use crate::lcfa::Settings;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub(super) struct Profile {
    pub character_id: Uuid,
    pub world_id: Uuid,
    pub name: String,
    pub level: Option<String>,
    pub vocation: Option<String>,
    pub observed: String,
}

pub(super) fn configured_profiles() -> Option<String> {
    (std::env::var("PORTAL_PUBLIC_PROFILES_ENABLED").as_deref() == Ok("true"))
        .then(|| std::env::var("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY").ok())
        .flatten()
        .filter(|authority| !authority.is_empty())
}

pub(super) async fn profiles(
    pool: &PgPool,
    account: Uuid,
    settings: &Settings,
    authority: &str,
) -> Result<HashMap<Uuid, Profile>, sqlx::Error> {
    let rows = sqlx::query_as::<_, Profile>(
        "WITH clock AS (SELECT floor(extract(epoch FROM statement_timestamp()))::bigint AS now)
         SELECT c.character_id,c.world_id,c.name,
                p.payload->'profile'->0->>'level' AS level,
                p.payload->'profile'->0->>'vocation' AS vocation,
                to_char(to_timestamp(p.watermark) AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') || ' UTC' AS observed
         FROM account_characters c
         JOIN accounts a ON a.id=c.account_id AND a.status='active'
         JOIN account_character_snapshots s USING (account_id)
         JOIN account_character_feed f ON f.id=1 AND f.highest_epoch=f.watermark_epoch
             AND s.projection_epoch=f.highest_epoch AND s.source_authority=f.source_authority
         CROSS JOIN clock
         JOIN public_snapshots p ON p.family='character_profile' AND NOT p.is_watermark
             AND p.subject=c.character_id::text
         JOIN public_projection_state e ON e.id=1 AND p.epoch=e.highest_epoch
         JOIN public_snapshots w ON w.family=p.family AND w.is_watermark AND w.subject=''
             AND w.epoch=p.epoch
         WHERE c.account_id=$1 AND NOT s.invalid AND f.source_authority=$2
           AND f.complete_through BETWEEN clock.now-$3+$4 AND clock.now+$4
           AND p.payload->>'source_authority'=$5 AND w.payload->>'source_authority'=$5
           AND w.watermark BETWEEN clock.now-29 AND clock.now+1
           AND p.watermark BETWEEN clock.now-29 AND LEAST(clock.now,w.watermark)
           AND NOT public_projection_conflicted('character_profile')
           AND p.payload->>'character_id'=c.character_id::text
           AND p.payload->'profile'->0->>'world_id'=c.world_id::text
           AND p.payload->'profile'->0->>'name'=c.name
           AND NOT EXISTS (
               SELECT FROM account_characters other
               JOIN accounts owner ON owner.id=other.account_id AND owner.status='active'
               WHERE other.character_id=c.character_id AND other.account_id<>c.account_id
           )",
    )
    .bind(account)
    .bind(&settings.source_authority)
    .bind(i64::from(settings.freshness_seconds))
    .bind(i64::from(settings.clock_uncertainty_seconds))
    .bind(authority)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.character_id, row))
        .collect())
}

pub(super) fn vocation_key(value: &str) -> Option<&'static str> {
    match value {
        "0" => Some("vocation-0"),
        "1" => Some("vocation-1"),
        "2" => Some("vocation-2"),
        "3" => Some("vocation-3"),
        "4" => Some("vocation-4"),
        "5" => Some("vocation-5"),
        _ => None,
    }
}
