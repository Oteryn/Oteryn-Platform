//! Event publication: only an explicit published state grants public visibility.
mod repository;
pub mod web;
pub use repository::Events;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Draft,
    Scheduled,
    Published,
    Archived,
}
impl Status {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(Self::Draft),
            "scheduled" => Some(Self::Scheduled),
            "published" => Some(Self::Published),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Scheduled => "scheduled",
            Self::Published => "published",
            Self::Archived => "archived",
        }
    }
    pub fn visible(value: &str) -> bool {
        Self::parse(value) == Some(Self::Published)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Translation {
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub body: String,
}
impl Translation {
    fn normalize(&mut self) -> Result<(), Error> {
        self.title = self.title.trim().to_owned();
        self.slug = self.slug.trim().to_owned();
        self.summary = self.summary.trim().to_owned();
        self.body = self.body.trim().to_owned();
        if !valid_slug(&self.slug)
            || !bounded(&self.title, 200)
            || !bounded(&self.summary, 500)
            || !bounded(&self.body, 100_000)
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
pub fn valid_slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 160
        && s.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}
fn bounded(s: &str, limit: usize) -> bool {
    !s.is_empty() && s.chars().count() <= limit
}
#[derive(Debug, Clone)]
pub struct Draft {
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub featured: bool,
    pub en: Translation,
    pub pl: Option<Translation>,
}
impl Draft {
    fn validate(&mut self) -> Result<(), Error> {
        if self.ends_at <= self.starts_at {
            return Err(Error::Invalid);
        }
        self.en.normalize()?;
        if let Some(pl) = &mut self.pl {
            pl.normalize()?;
        }
        Ok(())
    }
}
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Event {
    pub id: Uuid,
    pub status: String,
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub featured: bool,
    pub lock_version: i64,
}
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PublicEvent {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub body: String,
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid event input")]
    Invalid,
    #[error("event not found")]
    NotFound,
    #[error("event version or publication conflict")]
    Conflict,
    #[error(transparent)]
    Authorization(#[from] oteryn_admin::rbac::Error),
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
}
#[cfg(test)]
mod tests;
