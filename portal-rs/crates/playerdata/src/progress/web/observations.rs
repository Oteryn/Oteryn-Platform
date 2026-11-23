use super::*;
use crate::{
    authority::{
        CollectionEvidence, DbClock,
        progress_read::{Observation, read_character_progress},
    },
    deaths::{Justification, Pvp, Role, Skull, SkullColor},
    store::{Error, Freshness, http::Clock},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const SECTIONS: &[&str] = &[
    "bestiary",
    "boss_cooldown",
    "quest",
    "achievement",
    "skill_history",
    "stamina",
    "death_recap",
    "pvp",
    "skull",
    "premium_end",
    "house_rent",
];

pub(super) struct Section {
    pub family: &'static str,
    pub state: &'static str,
    pub rows: Vec<Row>,
}

pub(super) struct Row {
    pub when: String,
    pub details: Vec<(String, String)>,
}

fn date(milliseconds: i64) -> Result<String, Error> {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(milliseconds) * 1_000_000)
        .map_err(|_| Error::Unavailable)?
        .format(&Rfc3339)
        .map_err(|_| Error::Unavailable)
}

fn deadline(value: Option<String>, locale: &Locale) -> Result<String, Error> {
    match value {
        Some(ms) => date(ms.parse().map_err(|_| Error::Unavailable)?),
        None => Ok(locale.label("unknown")),
    }
}

fn row(locale: &Locale, family: &str, observation: Observation) -> Result<Row, Error> {
    let value = serde_json::to_value(observation.payload).map_err(|_| Error::Unavailable)?;
    let mut details = Vec::new();
    let mut field = |key: &str, content: String| details.push((locale.label(key), content));
    match family {
        "skill_history" => {
            let super::super::Payload::Skills(skills) =
                super::super::Payload::decode(family, value, false)
                    .map_err(|_| Error::Unavailable)?
            else {
                return Err(Error::Unavailable);
            };
            for s in skills.skills {
                field(
                    &s.skill,
                    format!(
                        "{} ({}) → {} ({})",
                        s.level_before, s.progress_before, s.level_after, s.progress_after
                    ),
                );
            }
        }
        "pvp" => {
            let event: Pvp = serde_json::from_value(value).map_err(|_| Error::Unavailable)?;
            field(
                "journal-role",
                locale.label(match event.role {
                    Role::Kill => "journal-kill",
                    Role::Assist => "journal-assist",
                    Role::Victim => "journal-victim",
                }),
            );
            field(
                "journal-justification",
                locale.label(match event.justification {
                    Justification::Justified => "journal-justified",
                    Justification::Unjustified => "journal-unjustified",
                    Justification::Unknown => "unknown",
                }),
            );
            if let Some(points) = event.points_milli {
                field("journal-points", points.to_string());
            }
            if let Some(counterpart) = event.counterpart_character_id {
                field("journal-counterpart", counterpart);
            }
        }
        "skull" => {
            let state: Skull = serde_json::from_value(value).map_err(|_| Error::Unavailable)?;
            field(
                "skull",
                locale.label(match state.skull {
                    SkullColor::None => "journal-skull-none",
                    SkullColor::Red => "journal-skull-red",
                    SkullColor::Black => "journal-skull-black",
                }),
            );
            field(
                "journal-skull-until",
                deadline(state.skull_until_unix_ms, locale)?,
            );
            field(
                "journal-kill-block-until",
                deadline(state.kill_block_until_unix_ms, locale)?,
            );
        }
        "stamina" => {
            let minutes = value
                .get("stamina_minutes")
                .and_then(serde_json::Value::as_u64)
                .ok_or(Error::Unavailable)?;
            field("journal-stamina-minutes", minutes.to_string());
        }
        _ => return Err(Error::Unavailable),
    }
    Ok(Row {
        when: date(observation.occurred_ms)?,
        details,
    })
}

type GameSections = (Vec<(Uuid, String)>, Vec<Section>);

pub(super) async fn sections(
    journal: &Journal,
    locale: &Locale,
    account: Uuid,
    selected: Option<Uuid>,
) -> Result<GameSections, StatusCode> {
    let mut views: Vec<_> = SECTIONS
        .iter()
        .map(|&family| Section {
            family,
            state: "progress-unavailable",
            rows: Vec::new(),
        })
        .collect();
    let Some(source) = journal.source().filter(|_| journal.read_enabled) else {
        return if selected.is_some() {
            Err(StatusCode::NOT_FOUND)
        } else {
            Ok((Vec::new(), views))
        };
    };
    let characters = match journal.pool.begin().await {
        Ok(tx) => CollectionEvidence::characters(tx, account)
            .await
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    let Some(character) = selected else {
        return Ok((characters, views));
    };
    if !characters.iter().any(|(id, _)| *id == character) {
        return Err(StatusCode::NOT_FOUND);
    }
    let Ok(now) = DbClock::new(journal.pool.clone()).trusted_time().await else {
        return Ok((characters, views));
    };
    for view in &mut views {
        if !journal.read_family(view.family) {
            continue;
        }
        let observation =
            read_character_progress(&journal.pool, account, character, source, view.family, now)
                .await;
        if matches!(observation, Err(Error::Denied)) {
            return Err(StatusCode::NOT_FOUND);
        }
        let Ok((freshness, rows)) = observation else {
            continue;
        };
        let Ok(rows) = rows
            .into_iter()
            .map(|observation| row(locale, view.family, observation))
            .collect::<Result<Vec<_>, _>>()
        else {
            continue;
        };
        view.state = if matches!(freshness, Freshness::Stale) {
            "progress-stale"
        } else if rows.is_empty() {
            "progress-empty"
        } else {
            "observed"
        };
        view.rows = rows;
    }
    Ok((characters, views))
}
