use super::{Config, Error, schema};
use sqlx::PgPool;
use time::{Date, OffsetDateTime, macros::format_description};
use uuid::Uuid;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    world: Option<String>,
    lang: Option<String>,
    date: Option<String>,
}

#[derive(Default)]
pub(super) struct Options {
    world: Option<Uuid>,
    pub(super) lang: Option<String>,
    pub(super) date: Option<Date>,
}

impl Options {
    pub(super) fn parse(raw: Option<&str>) -> Result<Self, Error> {
        let query: Query =
            serde_urlencoded::from_str(raw.unwrap_or_default()).map_err(|_| Error::BadInput)?;
        if query
            .lang
            .as_deref()
            .is_some_and(|s| !matches!(s, "pl" | "en"))
        {
            return Err(Error::BadInput);
        }
        Ok(Self {
            world: query.world.as_deref().map(schema::world_id).transpose()?,
            date: query.date.as_deref().map(parse_date).transpose()?,
            lang: query.lang,
        })
    }

    pub(super) fn world(&self, config: &Config) -> Result<Uuid, Error> {
        let world = self.world.or_else(|| {
            if config.worlds.len() == 1 {
                config.worlds.first().copied()
            } else {
                None
            }
        });
        world
            .filter(|id| config.worlds.contains(id))
            .ok_or(Error::Unavailable)
    }
}

fn parse_date(input: &str) -> Result<Date, Error> {
    let date = Date::parse(input, format_description!("[year]-[month]-[day]"))
        .map_err(|_| Error::BadInput)?;
    if input.len() != 10 || date.year() < 1 || date.to_string() != input {
        return Err(Error::BadInput);
    }
    Ok(date)
}

pub(super) fn day(timestamp: i64) -> Option<Date> {
    OffsetDateTime::from_unix_timestamp(timestamp)
        .ok()
        .map(|time| time.date())
}

pub(super) struct WorldChoice {
    pub(super) id: Uuid,
    pub(super) name: String,
    pub(super) selected: bool,
}

pub(super) async fn choices(
    db: &PgPool,
    config: &Config,
    selected: Option<Uuid>,
) -> Vec<WorldChoice> {
    if config.worlds.is_empty() || config.worlds.len() > 256 {
        return Vec::new();
    }
    let ids: Vec<_> = config.worlds.iter().copied().collect();
    let query = sqlx::query_as::<_, (Uuid, String)>(
        r#"SELECT id, name FROM world_status WHERE id = ANY($1) ORDER BY name COLLATE "C", id"#,
    )
    .bind(ids)
    .fetch_all(db);
    let Ok(Ok(rows)) = tokio::time::timeout(std::time::Duration::from_secs(1), query).await else {
        return Vec::new();
    };
    rows.into_iter()
        .map(|(id, name)| WorldChoice {
            id,
            name,
            selected: selected == Some(id),
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn world_must_be_canonical_and_within_the_configured_scope() {
        let first = "01934f10-7c02-7001-805b-3b1122334401";
        let other = "01934f10-7c02-7001-805b-3b1122334402";
        let id = schema::world_id(first).unwrap();
        let mut config = Config {
            worlds: [id].into(),
            ..Config::default()
        };
        assert_eq!(Options::parse(None).unwrap().world(&config), Ok(id));
        assert_eq!(
            Options::parse(Some(&format!("world={other}")))
                .unwrap()
                .world(&config),
            Err(Error::Unavailable),
        );
        assert!(matches!(
            Options::parse(Some(&format!("world={}", first.to_uppercase()))),
            Err(Error::BadInput)
        ));

        config.worlds.insert(schema::world_id(other).unwrap());
        assert_eq!(
            Options::parse(None).unwrap().world(&config),
            Err(Error::Unavailable)
        );
        assert_eq!(
            Options::parse(Some(&format!("world={first}")))
                .unwrap()
                .world(&config),
            Ok(id)
        );
    }
}
