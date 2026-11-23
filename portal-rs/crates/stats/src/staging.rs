//! Public statistics for the disposable staging projection.
use crate::{character, privacy::Authority, rankings, store::Visibility};
use sqlx::PgPool;
use uuid::Uuid;

pub(crate) fn enabled() -> bool {
    std::env::var("OTERYN_ENV").as_deref() == Ok("testing")
        && std::env::var("PORTAL_STAGING_SAMPLE_DATA").as_deref() == Ok("true")
        && std::env::var("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY").as_deref() == Ok("staging-sample")
}

#[derive(Debug)]
pub(crate) struct Samples {
    db: PgPool,
    privacy: Authority,
}

impl Samples {
    pub(crate) fn new(db: PgPool) -> Self {
        Self {
            privacy: Authority::configured(db.clone()),
            db,
        }
    }

    // Every row is from the current synthetic publication. Current Platform
    // character privacy is checked separately for each subject.
    const PUBLIC: &'static str = "
        SELECT c.subject::uuid, c.payload->'profile'->0, w.name, w.pvp_type
        FROM public_snapshots c
        JOIN public_projection_state f ON f.id=1 AND f.highest_epoch=c.epoch
        JOIN world_status w ON w.id::text=c.payload->'profile'->0->>'world_id'
        WHERE c.family='character_profile' AND NOT c.is_watermark
          AND c.subject LIKE '019a0000-0000-7002-8000-%'
          AND NOT public_projection_conflicted(c.family)
          AND EXISTS (
              SELECT 1 FROM public_snapshots p
              WHERE p.family=c.family AND p.epoch=c.epoch AND p.is_watermark AND p.subject=''
                AND p.payload->>'source_authority'='staging-sample'
                AND p.watermark BETWEEN floor(extract(epoch FROM now()))::bigint-29
                                    AND floor(extract(epoch FROM now()))::bigint+1
          )
          AND ($1::text IS NULL OR c.subject=$1)
        ORDER BY c.subject LIMIT 51";
}

impl character::Current for Samples {
    fn character(&self, id: Uuid) -> character::CurrentFuture<'_> {
        Box::pin(async move {
            let row: Option<(Uuid, serde_json::Value, String, Option<String>)> =
                sqlx::query_as(Self::PUBLIC)
                    .bind(id.to_string())
                    .fetch_optional(&self.db)
                    .await
                    .map_err(|_| character::Error::Unavailable)?;
            let (_, profile, _, _) = row.ok_or(character::Error::NotFound)?;
            Ok(character::PublicCharacter {
                name: profile["name"]
                    .as_str()
                    .ok_or(character::Error::Unavailable)?
                    .to_owned(),
                permissions: self.privacy.current_permissions(id).await,
                ranks: None,
            })
        })
    }
}

impl rankings::PublicCatalog for Samples {
    fn characters(&self, _: rankings::Kind) -> rankings::CatalogFuture<'_> {
        Box::pin(async move {
            let rows: Vec<(Uuid, serde_json::Value, String, Option<String>)> =
                sqlx::query_as(Self::PUBLIC)
                    .bind(Option::<String>::None)
                    .fetch_all(&self.db)
                    .await
                    .map_err(|_| rankings::Error::Unavailable)?;
            if rows.is_empty() || rows.len() > 50 {
                return Err(rankings::Error::Unavailable);
            }
            let mut characters = Vec::with_capacity(rows.len());
            for (id, profile, world_name, pvp) in rows {
                if !self.privacy.current_permissions(id).await.visible {
                    continue;
                }
                let pvp = match pvp.as_deref() {
                    Some("open") => rankings::Pvp::Open,
                    Some("optional") => rankings::Pvp::Optional,
                    Some("hardcore") => rankings::Pvp::Hardcore,
                    _ => return Err(rankings::Error::Unavailable),
                };
                let created = profile["created_at"]
                    .as_str()
                    .and_then(|raw| raw.parse().ok())
                    .and_then(|t| time::OffsetDateTime::from_unix_timestamp(t).ok())
                    .map(|t| t.date());
                characters.push(rankings::Character {
                    id,
                    name: profile["name"]
                        .as_str()
                        .ok_or(rankings::Error::Unavailable)?
                        .into(),
                    world: profile["world_id"]
                        .as_str()
                        .and_then(|s| s.parse().ok())
                        .ok_or(rankings::Error::Unavailable)?,
                    world_name,
                    vocation: profile["vocation"]
                        .as_u64()
                        .and_then(|v| u8::try_from(v).ok())
                        .ok_or(rankings::Error::Unavailable)?,
                    pvp,
                    created,
                });
            }
            Ok(characters)
        })
    }
}
