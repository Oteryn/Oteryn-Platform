use crate::{
    lcfa::CharacterSummary,
    ownership::{self, Ownership, Unavailable},
};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

pub(super) struct Facts {
    pub level: u64,
    pub vocation: u8,
}

#[derive(Default)]
pub(super) struct Visibility {
    pub rows: HashMap<Uuid, Option<Facts>>,
    pub unproven: bool,
}

/// Verify which portfolio rows the actor may see and attach optional public
/// progression facts under the same receipt and portfolio fences.
pub(super) async fn load(
    pool: &PgPool,
    account: Uuid,
    candidates: &[CharacterSummary],
    include_progression: bool,
) -> Visibility {
    if candidates.is_empty() {
        return Visibility::default();
    }
    load_owned(pool, account, candidates, include_progression)
        .await
        .unwrap_or(Visibility {
            rows: HashMap::new(),
            unproven: true,
        })
}

async fn load_owned(
    pool: &PgPool,
    account: Uuid,
    candidates: &[CharacterSummary],
    include_progression: bool,
) -> Result<Visibility, Unavailable> {
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
        return Err(Unavailable);
    }
    // Match the journal's lock order and keep transfers and portfolio updates
    // fenced from the facts query through the final ownership proof.
    ownership::shared_fences(&mut tx).await?;
    let feed = ownership::live_feed(&mut tx).await?;
    // A portfolio refresh can occur between list_characters and this read.
    // Require the exact same row before rendering any of its fields.
    let current: HashMap<Uuid, _> =
        sqlx::query_as::<_, (Uuid, Uuid, String, String)>(
            "SELECT character_id, world_id, name, availability FROM account_characters WHERE account_id=$1",
        )
        .bind(account)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|(id, world, name, availability)| (id, (world, name, availability)))
        .collect();
    let mut visible = Visibility::default();
    for candidate in candidates {
        let id = candidate.character_id.as_uuid();
        let world_id = candidate.world_id.as_uuid();
        if !current.get(&id).is_some_and(|(world, name, availability)| {
            *world == world_id
                && *name == candidate.name
                && availability == candidate.availability.as_str()
        }) {
            visible.unproven = true;
            continue;
        }
        match ownership::ownership_with_feed(&mut tx, account, id, &feed).await {
            Ok(Ownership::Owner(cut)) if cut.world == world_id => {
                visible.rows.insert(id, None);
            }
            Ok(Ownership::Former | Ownership::Stranger) => {}
            _ => visible.unproven = true,
        }
    }
    if include_progression && !visible.rows.is_empty() {
        let rows = sqlx::query_as::<_, (Uuid, String, String)>(
        r#"SELECT c.character_id,
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

        for (id, level, vocation) in rows {
            if let Some(facts) = visible.rows.get_mut(&id)
                && let (Ok(level), Ok(vocation)) = (level.parse(), vocation.parse())
            {
                *facts = Some(Facts { level, vocation });
            }
        }
    }
    tx.commit().await?;
    Ok(visible)
}
