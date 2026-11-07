//! Fresh T45/T09c policy for statistics; source qualification is a separate gate.
use crate::store::{PermissionFuture, Permissions, Visibility};
use sqlx::PgPool;
use uuid::Uuid;
#[derive(Clone, Debug)]
pub struct Authority {
    inner: oteryn_characters::profile::authority::Authority,
    enabled: bool,
}
impl Authority {
    pub fn new(inner: oteryn_characters::profile::authority::Authority) -> Self {
        Self {
            inner,
            enabled: true,
        }
    }
    pub fn configured(db: PgPool) -> Self {
        Self {
            inner: oteryn_characters::profile::authority::Authority::configured(db),
            enabled: std::env::var("OTERYN_CHARACTER_PROFILES_ENABLED").as_deref() == Ok("true"),
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
}
impl Visibility for Authority {
    fn current_permissions(&self, id: Uuid) -> PermissionFuture<'_> {
        Box::pin(async move {
            let Some(p) = self
                .inner
                .current(id, time::OffsetDateTime::now_utc().unix_timestamp())
                .await
                .ok()
                .flatten()
                .filter(|p| !p.hidden)
            else {
                return Permissions::default();
            };
            Permissions {
                visible: true,
                skills: p.show_skills,
                online: p.show_online_time,
                guild: p.show_guild,
                deaths: p.show_deaths,
            }
        })
    }
}

/// Coordinator update: no accepted statistics contract/producer. This source
/// qualification gate is independent of T45 privacy and cannot be enabled by flags.
pub const GAME_STATISTICS_ACCEPTED: bool = false;

#[derive(Clone, Debug)]
pub struct GuildSource {
    db: PgPool,
    privacy: Authority,
}
impl GuildSource {
    pub fn configured(db: PgPool) -> Self {
        Self {
            privacy: Authority::configured(db.clone()),
            db,
        }
    }
}
impl Visibility for GuildSource {
    fn current_permissions(&self, id: Uuid) -> PermissionFuture<'_> {
        self.privacy.current_permissions(id)
    }
}
impl crate::guilds::Source for GuildSource {
    fn contract_accepted(&self) -> bool {
        GAME_STATISTICS_ACCEPTED
    }
    fn current_guilds(&self) -> crate::guilds::GuildFuture<'_> {
        Box::pin(async move {
            use crate::guilds::{Error, Guild, Member};
            // Existing push ingest only. Require qualified current same-epoch
            // guild and profile completeness before deriving a visible cohort.
            let rows:Vec<(String,serde_json::Value)>=sqlx::query_as("SELECT s.subject,s.payload FROM public_snapshots s WHERE s.family='guild' AND NOT s.is_watermark AND s.epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) AND EXISTS(SELECT 1 FROM public_snapshots w WHERE w.family=s.family AND w.subject='v2' AND w.is_watermark AND w.epoch=s.epoch AND w.watermark BETWEEN floor(extract(epoch FROM now()))::bigint-29 AND floor(extract(epoch FROM now()))::bigint+1) LIMIT 1001").fetch_all(&self.db).await.map_err(|_|Error::Unavailable)?;
            if rows.len() > 1000 {
                return Err(Error::Unavailable);
            }
            let fresh:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public_snapshots WHERE family='guild' AND subject='v2' AND is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) AND watermark BETWEEN floor(extract(epoch FROM now()))::bigint-29 AND floor(extract(epoch FROM now()))::bigint+1)").fetch_one(&self.db).await.map_err(|_|Error::Unavailable)?;
            if !fresh {
                return Err(Error::Unavailable);
            }
            let mut result = Vec::new();
            for (id, payload) in rows {
                let Some(g) = payload["guild"]
                    .as_array()
                    .filter(|v| v.len() == 1)
                    .and_then(|v| v.first())
                else {
                    continue;
                };
                let world = g["world_id"].as_str().ok_or(Error::Unavailable)?;
                let mut members = Vec::new();
                for m in g["members"].as_array().ok_or(Error::Unavailable)? {
                    let subject = m["character_id"].as_str().ok_or(Error::Unavailable)?;
                    let p:Option<serde_json::Value>=sqlx::query_scalar("SELECT s.payload->'profile'->0 FROM public_snapshots s WHERE s.family='character_profile' AND NOT s.is_watermark AND s.subject=$1 AND s.epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) AND jsonb_array_length(s.payload->'profile')=1 AND EXISTS(SELECT 1 FROM public_snapshots w WHERE w.family=s.family AND w.is_watermark AND w.epoch=s.epoch AND w.watermark BETWEEN floor(extract(epoch FROM now()))::bigint-29 AND floor(extract(epoch FROM now()))::bigint+1)").bind(subject).fetch_optional(&self.db).await.map_err(|_|Error::Unavailable)?;
                    let Some(p) = p else {
                        continue;
                    };
                    if p["world_id"].as_str() != Some(world)
                        || p["name"] != m["name"]
                        || p["guild_visible"] != true
                        || !p["guild"].as_array().is_some_and(|v| {
                            v.len() == 1
                                && v[0]["guild_id"].as_str() == Some(&id)
                                && v[0]["rank_level"] == m["rank_level"]
                        })
                    {
                        continue;
                    }
                    members.push(Member {
                        id: Uuid::parse_str(subject).map_err(|_| Error::Unavailable)?,
                        rank: m["rank_level"]
                            .as_u64()
                            .and_then(|v| u8::try_from(v).ok())
                            .ok_or(Error::Unavailable)?,
                        name: m["name"].as_str().ok_or(Error::Unavailable)?.into(),
                        level: p["level"].as_str().and_then(|s| s.parse().ok()),
                        experience: p["experience"].as_str().and_then(|s| s.parse().ok()),
                    });
                }
                result.push(Guild {
                    id: Uuid::parse_str(&id).map_err(|_| Error::Unavailable)?,
                    world: Uuid::parse_str(world).map_err(|_| Error::Unavailable)?,
                    name: g["name"].as_str().ok_or(Error::Unavailable)?.into(),
                    founded_at: g["founded_at"]
                        .as_str()
                        .and_then(|s| s.parse().ok())
                        .ok_or(Error::Unavailable)?,
                    members,
                });
            }
            Ok(result)
        })
    }
}

/// Narrows a qualified ownership-history provider with current T45/T09c floors.
/// The provider must still establish owned_since and retained-set coverage; LCFA
/// alone has no ownership-history boundary and cannot fabricate one.
#[derive(Debug)]
pub struct OnlineOwner<P> {
    pub ownership: P,
    pub privacy: Authority,
}
impl<P> OnlineOwner<P> {
    pub fn configured(db: PgPool, ownership: P) -> Self {
        Self {
            ownership,
            privacy: Authority::configured(db),
        }
    }
}
impl<P: crate::rankings::online::OwnerPolicy> crate::rankings::online::OwnerPolicy
    for OnlineOwner<P>
{
    fn characters(&self, account: Uuid, now: i64) -> crate::rankings::online::OwnerFuture<'_> {
        Box::pin(async move {
            let mut result = Vec::new();
            for c in self.ownership.characters(account, now).await? {
                let p = self.privacy.current_permissions(c.character).await;
                if p.visible && p.online {
                    result.push(c);
                }
            }
            Ok(result)
        })
    }
}
