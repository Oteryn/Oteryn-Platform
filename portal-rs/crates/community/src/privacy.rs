//! Current unique ownership, saved T45 state and T09c bounds; never a cached allow.
use crate::{deaths, families, guilds, online, profiles, rankings};
use oteryn_characters::profile::Preferences;
use sqlx::PgPool;
use uuid::Uuid;
// A missing outer row is a family outage. A missing subject inside that row
// is an individual privacy decision. Read both at the same database instant.
macro_rules! from_current_profile_family {
    ($select:literal) => {
        concat!(
            $select,
            "
            FROM public_snapshots w
            JOIN public_projection_state f ON f.id=1 AND f.highest_epoch=w.epoch
            WHERE w.family='character_profile' AND w.subject='' AND w.is_watermark
              AND NOT public_projection_conflicted(w.family)
              AND w.watermark BETWEEN floor(extract(epoch FROM now()))::bigint-29
                                  AND floor(extract(epoch FROM now()))::bigint+1"
        )
    };
}

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
        let reference: Option<(String, String)> = sqlx::query_scalar(from_current_profile_family!(
            "SELECT (SELECT ROW(s.payload->'profile'->0->>'world_id', s.payload->'profile'->0->>'name')
                     FROM public_snapshots s
                     WHERE s.family=w.family AND s.epoch=w.epoch AND s.subject=$1
                       AND NOT s.is_watermark AND jsonb_array_length(s.payload->'profile')=1)"
        ))
        .bind(id.to_string())
        .fetch_one(&self.db)
        .await
        .map_err(|_| ())?;
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
        sqlx::query_scalar("SELECT s.payload->'profile'->0 FROM public_snapshots s JOIN public_projection_state f ON f.id=1 AND f.highest_epoch=s.epoch WHERE s.family='character_profile' AND NOT public_projection_conflicted('character_profile') AND s.subject=$1 AND NOT s.is_watermark AND s.epoch=$2::numeric AND jsonb_array_length(s.payload->'profile')=1 AND EXISTS(SELECT 1 FROM public_snapshots w WHERE w.family=s.family AND w.is_watermark AND w.epoch=s.epoch AND w.watermark BETWEEN floor(extract(epoch FROM now()))::bigint-29 AND floor(extract(epoch FROM now()))::bigint+1)").bind(id).bind(epoch).fetch_optional(&self.db).await.map_err(|_| ())
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
impl families::SkillsPrivacy for Authority {
    fn allows_skills<'a>(&'a self, id: &'a str, epoch: &'a str) -> families::PrivacyFuture<'a> {
        Box::pin(async move {
            self.privacy
                .ready()
                .await
                .map_err(|_| families::Unavailable)?;
            let Some(profile) = self
                .profile(id, epoch)
                .await
                .map_err(|_| families::Unavailable)?
            else {
                return Ok(false);
            };
            let world = profile["world_id"].as_str().ok_or(families::Unavailable)?;
            let name = profile["name"].as_str().ok_or(families::Unavailable)?;
            Ok(self
                .reference(id, world, name)
                .await
                .map_err(|_| families::Unavailable)?
                .is_some_and(|p| !p.hidden && p.show_skills))
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
                let baseline: bool = sqlx::query_scalar(from_current_profile_family!(
                    "SELECT EXISTS(SELECT 1 FROM public_snapshots s
                                   WHERE s.family='world_online' AND s.epoch=w.epoch
                                     AND NOT public_projection_conflicted(s.family)
                                     AND NOT s.is_watermark)"
                ))
                .fetch_one(&self.db)
                .await
                .map_err(|_| online::Unavailable)?;
                if !baseline {
                    return Err(online::Unavailable);
                }
            }
            let mut result = Vec::with_capacity(characters.len());
            for c in characters {
                let ids: Vec<String> = sqlx::query_scalar(from_current_profile_family!(
                    "SELECT ARRAY(SELECT s.subject FROM public_snapshots s
                                  WHERE s.family=w.family AND s.epoch=w.epoch AND NOT s.is_watermark
                                    AND jsonb_array_length(s.payload->'profile')=1
                                    AND s.payload->'profile'->0->>'name'=$1
                                    AND s.payload->'profile'->0->>'world_id'=$2
                                    AND s.epoch::text=$3 AND s.watermark<=$4 LIMIT 2)"
                ))
                .bind(&c.name)
                .bind(&c.world)
                .bind(&c.epoch)
                .bind(c.observed_at)
                .fetch_one(&self.db)
                .await
                .map_err(|_| online::Unavailable)?;
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
impl rankings::PublicVisibility for Authority {
    fn references<'a>(
        &'a self,
        characters: &'a [rankings::CharacterRef],
    ) -> rankings::VisibilityFuture<'a, Vec<bool>> {
        Box::pin(async move {
            self.privacy
                .ready()
                .await
                .map_err(|_| rankings::VisibilityUnavailable)?;
            if characters.is_empty() {
                sqlx::query(from_current_profile_family!("SELECT 1"))
                    .fetch_one(&self.db)
                    .await
                    .map_err(|_| rankings::VisibilityUnavailable)?;
            }
            let mut hidden = Vec::with_capacity(characters.len());
            for c in characters {
                let current = self
                    .current(&c.id)
                    .await
                    .map_err(|_| rankings::VisibilityUnavailable)?;
                hidden.push(
                    !current.is_some_and(|p| !p.hidden)
                        || !self
                            .reference(&c.id, &c.world, &c.name)
                            .await
                            .map_err(|_| rankings::VisibilityUnavailable)?
                            .is_some_and(|p| !p.hidden),
                );
            }
            Ok(hidden)
        })
    }
    fn is_hidden<'a>(&'a self, character_ref: &'a str) -> rankings::VisibilityFuture<'a, bool> {
        Box::pin(async move {
            self.privacy
                .ready()
                .await
                .map_err(|_| rankings::VisibilityUnavailable)?;
            Ok(!self
                .current(character_ref)
                .await
                .map_err(|_| rankings::VisibilityUnavailable)?
                .is_some_and(|p| !p.hidden))
        })
    }
}

/// Begin a public read fence. Ingest holds the projection row FOR UPDATE, so a
/// fence's shared locks wait at most one second and the whole fence 2.5 seconds
/// before the page answers unavailable.
pub(crate) async fn read_fence(
    db: &sqlx::PgPool,
) -> Result<sqlx::Transaction<'static, sqlx::Postgres>, sqlx::Error> {
    let mut fence = db.begin().await?;
    sqlx::query(
        "SELECT set_config('lock_timeout','1s',true),set_config('statement_timeout','2500ms',true)",
    )
    .execute(&mut *fence)
    .await?;
    Ok(fence)
}

/// Revision evidence spans every asynchronous permission lookup in a response.
/// Validation holds a shared revision fence until the caller's final commit.
#[derive(Debug)]
pub(crate) struct Cut(i64);
impl Cut {
    pub(crate) async fn read(db: &PgPool) -> Result<Self, sqlx::Error> {
        sqlx::query_scalar("SELECT revision FROM community_privacy_revision WHERE id=1")
            .fetch_one(db)
            .await
            .map(Self)
    }
    pub(crate) async fn validate(&self, db: &mut sqlx::PgConnection) -> Result<bool, sqlx::Error> {
        let current: i64 = sqlx::query_scalar(
            "SELECT revision FROM community_privacy_revision WHERE id=1 FOR SHARE",
        )
        .fetch_one(db)
        .await?;
        Ok(current == self.0)
    }
}
