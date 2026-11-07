//! Current unique ownership, saved T45 state and T09c bounds; never a cached allow.
use crate::{deaths, guilds, online, profiles};
use oteryn_characters::profile::Preferences;
use sqlx::PgPool;
use uuid::Uuid;
#[derive(Clone, Debug)]
pub struct Authority {
    db: PgPool,
    privacy: oteryn_characters::profile::authority::Authority,
}
impl Authority {
    pub fn new(db: PgPool, privacy: oteryn_characters::profile::authority::Authority) -> Self {
        Self { db, privacy }
    }
    pub fn configured(db: PgPool) -> Self {
        Self {
            privacy: oteryn_characters::profile::authority::Authority::configured(db.clone()),
            db,
        }
    }
    async fn current(&self, id: &str) -> Result<Option<Preferences>, ()> {
        let Ok(id) = Uuid::parse_str(id) else {
            return Ok(None);
        };
        let reference: Option<(String,String)> = sqlx::query_as("SELECT s.payload->'profile'->0->>'world_id',s.payload->'profile'->0->>'name' FROM public_snapshots s WHERE s.family='character_profile' AND s.subject=$1 AND NOT s.is_watermark AND s.epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) AND jsonb_array_length(s.payload->'profile')=1 AND EXISTS(SELECT 1 FROM public_snapshots w WHERE w.family=s.family AND w.is_watermark AND w.epoch=s.epoch AND w.watermark BETWEEN floor(extract(epoch FROM now()))::bigint-29 AND floor(extract(epoch FROM now()))::bigint+1)").bind(id.to_string()).fetch_optional(&self.db).await.map_err(|_|())?;
        let Some((world, name)) = reference else {
            return Ok(None);
        };
        self.privacy
            .reference(
                id,
                &world,
                &name,
                time::OffsetDateTime::now_utc().unix_timestamp(),
            )
            .await
            .map_err(|_| ())
    }
    async fn reference(
        &self,
        id: &str,
        world: &str,
        name: &str,
    ) -> Result<Option<Preferences>, ()> {
        let Ok(id) = Uuid::parse_str(id) else {
            return Ok(None);
        };
        self.privacy
            .reference(
                id,
                world,
                name,
                time::OffsetDateTime::now_utc().unix_timestamp(),
            )
            .await
            .map_err(|_| ())
    }
    async fn profile(&self, id: &str, epoch: &str) -> Result<Option<serde_json::Value>, ()> {
        sqlx::query_scalar("SELECT s.payload->'profile'->0 FROM public_snapshots s JOIN public_projection_state f ON f.id=1 AND f.highest_epoch=s.epoch WHERE s.family='character_profile' AND s.subject=$1 AND NOT s.is_watermark AND s.epoch=$2::numeric AND jsonb_array_length(s.payload->'profile')=1 AND EXISTS(SELECT 1 FROM public_snapshots w WHERE w.family=s.family AND w.is_watermark AND w.epoch=s.epoch AND w.watermark BETWEEN floor(extract(epoch FROM now()))::bigint-29 AND floor(extract(epoch FROM now()))::bigint+1)").bind(id).bind(epoch).fetch_optional(&self.db).await.map_err(|_| ())
    }
}
impl profiles::Privacy for Authority {
    fn visible<'a>(&'a self, id: &'a str) -> profiles::DecisionFuture<'a> {
        Box::pin(async move {
            self.privacy
                .ready()
                .await
                .map_err(|_| profiles::Unavailable)?;
            Ok(self
                .current(id)
                .await
                .map_err(|_| profiles::Unavailable)?
                .is_some_and(|p| !p.hidden))
        })
    }
    fn fields<'a>(
        &'a self,
        id: &'a str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<profiles::details::Visibility, profiles::Unavailable>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let Some(p) = self
                .current(id)
                .await
                .map_err(|_| profiles::Unavailable)?
                .filter(|p| !p.hidden)
            else {
                return Ok(profiles::details::Visibility::default());
            };
            Ok(profiles::details::Visibility {
                guild: p.show_guild,
                house: p.show_house,
                skills: p.show_skills,
                deaths: p.show_deaths,
                comment: (!p.comment.is_empty()).then_some(p.comment),
            })
        })
    }
}
impl deaths::Privacy for Authority {
    fn ready(&self) -> deaths::PrivacyFuture<'_> {
        Box::pin(async move {
            self.privacy
                .ready()
                .await
                .map(|()| true)
                .map_err(|_| deaths::Unavailable)
        })
    }
    fn allows_deaths<'a>(&'a self, id: &'a str, epoch: &'a str) -> deaths::PrivacyFuture<'a> {
        Box::pin(async move {
            let Some(profile) = self
                .profile(id, epoch)
                .await
                .map_err(|_| deaths::Unavailable)?
            else {
                return Ok(false);
            };
            let world = profile["world_id"].as_str().ok_or(deaths::Unavailable)?;
            let name = profile["name"].as_str().ok_or(deaths::Unavailable)?;
            Ok(self
                .reference(id, world, name)
                .await
                .map_err(|_| deaths::Unavailable)?
                .is_some_and(|p| !p.hidden && p.show_deaths))
        })
    }
}
impl online::PublicVisibility for Authority {
    fn hidden_batch<'a>(
        &'a self,
        characters: &'a [online::CharacterRef],
    ) -> online::VisibilityResult<'a> {
        Box::pin(async move {
            self.privacy
                .ready()
                .await
                .map_err(|_| online::Unavailable)?;
            if characters.is_empty() {
                let baseline: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public_snapshots WHERE family='world_online' AND NOT is_watermark AND epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1))").fetch_one(&self.db).await.map_err(|_|online::Unavailable)?;
                if !baseline {
                    return Err(online::Unavailable);
                }
            }
            let mut result = Vec::with_capacity(characters.len());
            for c in characters {
                let ids: Vec<String> = sqlx::query_scalar("SELECT s.subject FROM public_snapshots s WHERE s.family='character_profile' AND NOT s.is_watermark AND s.epoch=(SELECT highest_epoch FROM public_projection_state WHERE id=1) AND jsonb_array_length(s.payload->'profile')=1 AND s.payload->'profile'->0->>'name'=$1 AND s.payload->'profile'->0->>'world_id'=$2 AND s.epoch::text=$3 AND s.watermark<=$4 AND EXISTS(SELECT 1 FROM public_snapshots f WHERE f.family=s.family AND f.is_watermark AND f.epoch=s.epoch AND f.watermark BETWEEN floor(extract(epoch FROM now()))::bigint-29 AND floor(extract(epoch FROM now()))::bigint+1) LIMIT 2").bind(&c.name).bind(&c.world).bind(&c.epoch).bind(c.observed_at).fetch_all(&self.db).await.map_err(|_| online::Unavailable)?;
                result.push(if ids.len() == 1 {
                    !self
                        .reference(&ids[0], &c.world, &c.name)
                        .await
                        .map_err(|_| online::Unavailable)?
                        .is_some_and(|p| !p.hidden && p.show_status)
                } else {
                    true
                });
            }
            Ok(result)
        })
    }
}
impl guilds::PublicVisibility for Authority {
    fn visible_batch<'a>(
        &'a self,
        epoch: &'a str,
        members: &'a [guilds::MemberRef],
    ) -> guilds::VisibilityResult<'a> {
        Box::pin(async move {
            self.privacy
                .ready()
                .await
                .map_err(|_| guilds::Unavailable::Evidence)?;
            let mut result = Vec::with_capacity(members.len());
            for m in members {
                let profile = self
                    .profile(&m.character_id, epoch)
                    .await
                    .map_err(|_| guilds::Unavailable::Evidence)?;
                let eligible = profile.is_some_and(|p| {
                    p["world_id"].as_str() == Some(&m.world_id)
                        && p["name"].as_str() == Some(&m.name)
                        && p["guild_visible"] == true
                        && p["guild"].as_array().is_some_and(|g| {
                            g.len() == 1 && g[0]["guild_id"].as_str() == Some(&m.guild_id)
                        })
                });
                result.push(
                    eligible
                        && self
                            .reference(&m.character_id, &m.world_id, &m.name)
                            .await
                            .map_err(|_| guilds::Unavailable::Evidence)?
                            .is_some_and(|p| !p.hidden && p.show_guild),
                );
            }
            Ok(result)
        })
    }
}
