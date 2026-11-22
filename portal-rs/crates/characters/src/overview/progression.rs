use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

pub(super) struct Facts {
    pub name: String,
    pub world: Uuid,
    pub level: u64,
    pub vocation: u8,
}

/// Read only the two public progression fields, joined to the viewer's own
/// character IDs. The profile projection has its own epoch and freshness fence.
pub(super) async fn load(pool: &PgPool, account: Uuid) -> HashMap<Uuid, Facts> {
    if !crate::navigation::flag("PORTAL_PUBLIC_PROFILES_ENABLED") {
        return HashMap::new();
    }
    let rows = sqlx::query_as::<_, (Uuid, String, Uuid, String, String)>(
        r#"SELECT c.character_id, c.name, c.world_id,
                  p.payload->'profile'->0->>'level', p.payload->'profile'->0->>'vocation'
           FROM account_characters c
           JOIN accounts a ON a.id = c.account_id AND a.status = 'active'
           JOIN public_snapshots p ON p.subject = c.character_id::text
             AND p.family = 'character_profile' AND NOT p.is_watermark
           JOIN public_projection_state s ON s.id = 1 AND s.highest_epoch = p.epoch
           WHERE c.account_id = $1
             AND p.payload->'profile'->0->>'name' = c.name
             AND p.payload->'profile'->0->>'world_id' = c.world_id::text
             AND NOT public_projection_conflicted('character_profile')
             AND EXISTS (
                 SELECT 1 FROM public_snapshots w
                 WHERE w.family = p.family AND w.is_watermark AND w.epoch = p.epoch
                   AND w.watermark BETWEEN floor(extract(epoch FROM statement_timestamp()))::bigint - 29
                                       AND floor(extract(epoch FROM statement_timestamp()))::bigint + 1
             )"#,
    )
    .bind(account)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .filter_map(|(id, name, world, level, vocation)| {
            let level = level.parse().ok()?;
            let vocation = vocation.parse().ok()?;
            Some((
                id,
                Facts {
                    name,
                    world,
                    level,
                    vocation,
                },
            ))
        })
        .collect()
}
