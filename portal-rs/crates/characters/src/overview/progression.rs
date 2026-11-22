use crate::ownership::{self, Ownership, Unavailable};
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
    load_owned(pool, account).await.unwrap_or_default()
}

async fn load_owned(pool: &PgPool, account: Uuid) -> Result<HashMap<Uuid, Facts>, Unavailable> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL statement_timeout = '2500ms'")
        .execute(&mut *tx)
        .await?;
    let active: Option<String> =
        sqlx::query_scalar("SELECT status FROM accounts WHERE id=$1 FOR SHARE")
            .bind(account)
            .fetch_optional(&mut *tx)
            .await?;
    if active.as_deref() != Some("active") {
        return Ok(HashMap::new());
    }
    // Match the journal's lock order and keep transfers and portfolio updates
    // fenced from the facts query through the final ownership proof.
    ownership::shared_fences(&mut tx).await?;
    let feed = ownership::live_feed(&mut tx).await?;
    let rows = sqlx::query_as::<_, (Uuid, String, Uuid, String, String)>(
        r#"SELECT c.character_id, c.name, c.world_id,
                  p.payload->'profile'->0->>'level', p.payload->'profile'->0->>'vocation'
           FROM account_characters c
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
    .fetch_all(&mut *tx)
    .await?;

    let mut facts = HashMap::new();
    for (id, name, world, level, vocation) in rows {
        let (Ok(level), Ok(vocation)) = (level.parse(), vocation.parse()) else {
            continue;
        };
        if let Ok(Ownership::Owner(cut)) =
            ownership::ownership_with_feed(&mut tx, account, id, &feed).await
            && cut.world == world
        {
            facts.insert(
                id,
                Facts {
                    name,
                    world,
                    level,
                    vocation,
                },
            );
        }
    }
    tx.commit().await?;
    Ok(facts)
}
