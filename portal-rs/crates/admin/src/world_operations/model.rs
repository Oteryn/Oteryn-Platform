use super::Error;
use serde::Deserialize;
use time::{OffsetDateTime, PrimitiveDateTime, format_description};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Action {
    EnterMaintenance,
    ExitMaintenance,
    Drain,
    Restart,
    EventStart,
    EventStop,
    EventCancel,
    Broadcast,
}

impl Action {
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::EnterMaintenance => "enter-maintenance",
            Self::ExitMaintenance => "exit-maintenance",
            Self::Drain => "drain",
            Self::Restart => "restart",
            Self::EventStart => "event-start",
            Self::EventStop => "event-stop",
            Self::EventCancel => "event-cancel",
            Self::Broadcast => "broadcast",
        }
    }

    pub(super) fn from_key(key: &str) -> Result<Self, Error> {
        Self::deserialize(serde::de::value::StrDeserializer::<serde::de::value::Error>::new(key))
            .map_err(|_| Error::Invalid)
    }

    pub(super) fn event(self) -> bool {
        matches!(self, Self::EventStart | Self::EventStop | Self::EventCancel)
    }

    pub(super) fn broadcast(self) -> bool {
        matches!(self, Self::Broadcast)
    }

    pub(super) fn node(self) -> bool {
        matches!(self, Self::Drain | Self::Restart)
    }

    pub(super) fn requires_node(self) -> bool {
        matches!(self, Self::Restart)
    }

    pub(super) fn channel(self) -> bool {
        self.event() || self.broadcast()
    }

    pub(super) fn maintenance(self) -> bool {
        !self.event() && !self.broadcast()
    }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum MessageLocale {
    Pl,
    En,
}

impl MessageLocale {
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Pl => "pl",
            Self::En => "en",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DraftForm {
    pub(super) _csrf: String,
    pub(super) id: Uuid,
    pub(super) world: Uuid,
    pub(super) action: Action,
    pub(super) starts: String,
    pub(super) ends: String,
    pub(super) ticket: String,
    #[serde(default, deserialize_with = "optional_uuid")]
    pub(super) target: Option<Uuid>,
    #[serde(default, deserialize_with = "optional_uuid")]
    pub(super) event: Option<Uuid>,
    pub(super) locale: Option<MessageLocale>,
    pub(super) message: Option<String>,
}

fn optional_uuid<'de, D: serde::Deserializer<'de>>(data: D) -> Result<Option<Uuid>, D::Error> {
    let raw = String::deserialize(data)?;
    if raw.is_empty() {
        return Ok(None);
    }
    raw.parse().map(Some).map_err(serde::de::Error::custom)
}

const DAY: i64 = 86400;

fn parse_time(raw: &str) -> Result<i64, Error> {
    if raw.len() != 16 {
        return Err(Error::Invalid);
    }
    let format = format_description::parse_borrowed::<2>("[year]-[month]-[day]T[hour]:[minute]")
        .map_err(|_| Error::Unavailable)?;
    PrimitiveDateTime::parse(raw, &format)
        .map(|dt| dt.assume_utc().unix_timestamp())
        .map_err(|_| Error::Invalid)
}

pub(super) fn display_time(timestamp: i64) -> Result<String, Error> {
    OffsetDateTime::from_unix_timestamp(timestamp)
        .ok()
        .and_then(|dt| dt.format(&format_description::well_known::Rfc3339).ok())
        .ok_or(Error::Unavailable)
}

fn uuid_v7(id: Uuid) -> bool {
    id.get_version_num() == 7 && id.get_variant() == uuid::Variant::RFC4122
}

impl DraftForm {
    pub(super) fn validate(&self) -> Result<(i64, i64), Error> {
        let start = parse_time(&self.starts)?;
        let end = parse_time(&self.ends)?;
        let ticket = self.ticket.trim();
        if !(1..=128).contains(&ticket.len())
            || ticket.chars().any(char::is_control)
            || ![self.id, self.world].into_iter().all(uuid_v7)
            || !self.target.into_iter().chain(self.event).all(uuid_v7)
            || !(1..=DAY).contains(&(end - start))
            || (self.action.requires_node() && self.target.is_none())
            || (!self.action.node() && !self.action.channel() && self.target.is_some())
            || self.action.event() != self.event.is_some()
            || self.action.broadcast() != self.locale.is_some()
            || self.action.broadcast() != self.message.is_some()
            || self.message.as_ref().is_some_and(|text| {
                !(1..=500).contains(&text.trim().chars().count())
                    || text.chars().any(char::is_control)
            })
        {
            return Err(Error::Invalid);
        }
        Ok((start, end))
    }
}

pub(super) fn check_schedule(start: i64, now: i64) -> Result<(), Error> {
    if !(1..=90 * DAY).contains(&(start - now)) {
        return Err(Error::Conflict);
    }
    Ok(())
}

pub(super) fn check_review(
    requester: Uuid,
    reviewer: Uuid,
    starts: i64,
    now: i64,
    reviewed: bool,
) -> Result<(), Error> {
    if requester == reviewer {
        return Err(Error::Denied);
    }
    if reviewed || starts <= now {
        return Err(Error::Conflict);
    }
    Ok(())
}
