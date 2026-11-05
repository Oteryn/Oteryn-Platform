use super::*;
use fluent_bundle::{FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

#[derive(Debug, Clone, Copy)]
pub enum Language {
    Polish,
    English,
}
struct Messages(FluentBundle<FluentResource>);
impl Messages {
    fn new(language: Language) -> Result<Self, Failure> {
        let (locale, text) = match language {
            Language::Polish => ("pl", include_str!("deaths.pl.ftl")),
            Language::English => ("en", include_str!("deaths.en.ftl")),
        };
        let locale: LanguageIdentifier = locale.parse().map_err(|_| Failure::Unavailable)?;
        let mut bundle = FluentBundle::new(vec![locale]);
        bundle.set_use_isolating(false);
        bundle
            .add_resource(
                FluentResource::try_new(text.to_owned()).map_err(|_| Failure::Unavailable)?,
            )
            .map_err(|_| Failure::Unavailable)?;
        Ok(Self(bundle))
    }
    fn get(&self, key: &str) -> Result<String, Failure> {
        let pattern = self
            .0
            .get_message(key)
            .and_then(|m| m.value())
            .ok_or(Failure::Unavailable)?;
        let mut errors = Vec::new();
        let text = self
            .0
            .format_pattern(pattern, None, &mut errors)
            .into_owned();
        if errors.is_empty() {
            Ok(text)
        } else {
            Err(Failure::Unavailable)
        }
    }
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn field(html: &mut String, messages: &Messages, label: &str, value: &str) -> Result<(), Failure> {
    html.push_str(&format!(
        "<dt>{}</dt><dd>{}</dd>",
        escape(&messages.get(label)?),
        break_key(value)
    ));
    Ok(())
}
/// SSR fragment for the existing layout. Semantic blocks wrap at 360px; no scripts,
/// horizontal tables or interactive controls. Caller sends private, no-store responses.
pub fn render(projection: &Projection, language: Language) -> Result<String, Failure> {
    let messages = Messages::new(language)?;
    let mut html = format!(
        "<section aria-labelledby=\"death-heading\"><h1 id=\"death-heading\">{}</h1><p>{}</p>",
        escape(&messages.get("title")?),
        escape(&messages.get(if projection.stale {
            "stale"
        } else {
            "observed"
        })?)
    );
    if projection.records.is_empty() {
        html.push_str(&format!("<p>{}</p>", escape(&messages.get("empty")?)));
    }
    for record in &projection.records {
        record.validate().map_err(|_| Failure::Unavailable)?;
        html.push_str(&format!(
            "<article><h2>{}</h2><dl>",
            escape(&messages.get(record.family.key())?)
        ));
        field(
            &mut html,
            &messages,
            "occurred",
            &timestamp(&record.occurred_at_unix_ms)?,
        )?;
        match &record.payload {
            Some(Payload::Death(d)) => {
                field(
                    &mut html,
                    &messages,
                    "level-before",
                    &d.level_before.to_string(),
                )?;
                field(
                    &mut html,
                    &messages,
                    "level-after",
                    &d.level_after.to_string(),
                )?;
                field(&mut html, &messages, "experience", &d.experience_lost)?;
                html.push_str(&format!(
                    "</dl><p>{}</p>",
                    escape(&messages.get("actions-unavailable")?)
                ));
                for (label, values) in [
                    ("blessings-before", &d.blessings_before),
                    ("blessings-after", &d.blessings_after),
                ] {
                    html.push_str(&format!("<h3>{}</h3><ul>", escape(&messages.get(label)?)));
                    if values.is_empty() {
                        html.push_str(&format!(
                            "<li>{}</li>",
                            escape(&messages.get("none-recorded")?)
                        ));
                    }
                    for value in values {
                        html.push_str(&format!("<li>{}</li>", break_key(value)));
                    }
                    html.push_str("</ul>");
                }
                html.push_str(&format!(
                    "<h3>{}</h3><ul>",
                    escape(&messages.get("lost-items")?)
                ));
                if d.lost_items.is_empty() {
                    html.push_str(&format!(
                        "<li>{}</li>",
                        escape(&messages.get("none-recorded")?)
                    ));
                }
                for item in &d.lost_items {
                    html.push_str(&format!(
                        "<li>{}: {}</li>",
                        break_key(&item.item_key),
                        item.quantity
                    ));
                }
                html.push_str("</ul>");
            }
            Some(Payload::Pvp(p)) => {
                let role = match p.role {
                    Role::Kill => "kill",
                    Role::Assist => "assist",
                    Role::Victim => "victim",
                };
                field(&mut html, &messages, "role", &messages.get(role)?)?;
                let justification = match p.justification {
                    Justification::Justified => "justified",
                    Justification::Unjustified => "unjustified",
                    Justification::Unknown => "unknown",
                };
                field(
                    &mut html,
                    &messages,
                    "justification",
                    &messages.get(justification)?,
                )?;
                field(
                    &mut html,
                    &messages,
                    "counterpart",
                    &p.counterpart_character_id
                        .clone()
                        .unwrap_or(messages.get("redacted")?),
                )?;
                field(
                    &mut html,
                    &messages,
                    "points",
                    &p.points_milli
                        .map(|v| v.to_string())
                        .unwrap_or(messages.get("unknown")?),
                )?;
                html.push_str("</dl>");
            }
            Some(Payload::Skull(s)) => {
                let color = match s.skull {
                    SkullColor::None => "skull-none",
                    SkullColor::Red => "skull-red",
                    SkullColor::Black => "skull-black",
                };
                field(&mut html, &messages, "color", &messages.get(color)?)?;
                field(
                    &mut html,
                    &messages,
                    "deadline",
                    &s.skull_until_unix_ms
                        .as_deref()
                        .map(timestamp)
                        .transpose()?
                        .unwrap_or(messages.get("unknown")?),
                )?;
                field(
                    &mut html,
                    &messages,
                    "kill-block",
                    &s.kill_block_until_unix_ms
                        .as_deref()
                        .map(timestamp)
                        .transpose()?
                        .unwrap_or(messages.get("unknown")?),
                )?;
                html.push_str(&format!(
                    "</dl><p>{}</p>",
                    escape(&messages.get("source-state")?)
                ));
            }
            None => return Err(Failure::Unavailable),
        }
        html.push_str("</article>");
    }
    html.push_str("</section>");
    Ok(html)
}
fn break_key(value: &str) -> String {
    let mut output = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index > 0 && index % 20 == 0 {
            output.push_str("<wbr>");
        }
        output.push_str(&escape(&ch.to_string()));
    }
    output
}

pub(crate) fn page_title(language: Language) -> Result<String, Failure> {
    Messages::new(language)?.get("title")
}

fn timestamp(value: &str) -> Result<String, Failure> {
    let ms = decimal(value, true, MAX_TIME)?;
    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .map_err(|_| Failure::Unavailable)?
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| Failure::Unavailable)
}
