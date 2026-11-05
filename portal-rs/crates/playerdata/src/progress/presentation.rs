use super::*;
use askama::Template;
use fluent_bundle::{FluentBundle, FluentResource};
use unic_langid::langid;

#[derive(Debug, Clone, Copy)]
pub enum ProgressLanguage {
    Polish,
    English,
}
impl ProgressLanguage {
    fn code(self) -> &'static str {
        match self {
            Self::Polish => "pl",
            Self::English => "en",
        }
    }
    fn bundle(self) -> Result<FluentBundle<FluentResource>> {
        let (locale, text) = match self {
            Self::Polish => (langid!("pl"), include_str!("pl.ftl")),
            Self::English => (langid!("en"), include_str!("en.ftl")),
        };
        let resource = FluentResource::try_new(text.into()).map_err(|_| Error::Unavailable)?;
        let mut bundle = FluentBundle::new(vec![locale]);
        bundle
            .add_resource(resource)
            .map_err(|_| Error::Unavailable)?;
        Ok(bundle)
    }
}
fn text(b: &FluentBundle<FluentResource>, key: &str) -> Result<String> {
    let pattern = b
        .get_message(key)
        .and_then(|m| m.value())
        .ok_or(Error::Unavailable)?;
    let mut errors = Vec::new();
    let result = b.format_pattern(pattern, None, &mut errors).into_owned();
    if errors.is_empty() {
        Ok(result)
    } else {
        Err(Error::Unavailable)
    }
}
#[derive(Debug)]
struct Section {
    title: String,
    rows: Vec<DisplayRow>,
    notice: String,
}
#[derive(Debug)]
struct DisplayRow {
    key: String,
    details: Vec<String>,
}
#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{{ title }}</title></head><body><main id=\"main\"><h1>{{ title }}</h1><p>{{ observation }}</p>{% for section in sections %}<section aria-labelledby=\"section-{{ loop.index }}\"><h2 id=\"section-{{ loop.index }}\">{{ section.title }}</h2>{% if section.rows.is_empty() %}<p>{{ section.notice }}</p>{% else %}<ul>{% for row in section.rows %}<li><strong>{{ row.key }}</strong><ul>{% for detail in row.details %}<li>{{ detail }}</li>{% endfor %}</ul></li>{% endfor %}</ul>{% endif %}</section>{% endfor %}</main></body></html>",
    ext = "html"
)]
struct Page {
    lang: &'static str,
    title: String,
    observation: String,
    sections: Vec<Section>,
}
fn wrap_key(key: &str) -> String {
    key.chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if i > 0 && i % 20 == 0 {
                vec!['\u{200b}', c]
            } else {
                vec![c]
            }
        })
        .collect()
}
fn row(p: Payload, b: &FluentBundle<FluentResource>, now: u64) -> Result<DisplayRow> {
    let mut details = Vec::new();
    let entity = wrap_key(p.entity());
    match p {
        Payload::Bestiary(p) => {
            details.push(format!("{}: {}", text(b, "count")?, p.kill_count));
            details.push(format!(
                "{}: {}",
                text(b, "threshold")?,
                p.final_kill_threshold
            ));
        }
        Payload::Boss(p) => details.push(format!(
            "{}: {}",
            text(b, "cooldown")?,
            p.cooldown_remaining_ms(now)
                .map(|n| n.to_string())
                .unwrap_or(text(b, "unknown")?)
        )),
        Payload::Quest(p) => {
            details.push(text(
                b,
                if p.completed {
                    "completed"
                } else {
                    "in-progress"
                },
            )?);
            for t in p.tracks {
                details.push(format!("{}: {}", wrap_key(&t.track_key), t.value));
            }
        }
        Payload::Achievement(p) => details.push(if p.granted_at_unix_ms.is_some() {
            text(b, "granted")?
        } else {
            format!(
                "{} / {}",
                p.progress.ok_or(Error::Unavailable)?,
                p.target.ok_or(Error::Unavailable)?
            )
        }),
        Payload::Skills(p) => {
            for s in p.skills {
                details.push(format!(
                    "{}: {} ({}) → {} ({})",
                    text(b, &s.skill)?,
                    s.level_before,
                    s.progress_before,
                    s.level_after,
                    s.progress_after
                ));
            }
        }
    }
    Ok(DisplayRow {
        key: entity,
        details,
    })
}
impl ProgressStore {
    /// SSR seam for the authenticated router. The future handler must set
    /// Cache-Control: no-store and use its trusted session actor and authority.
    /// Denial aborts the whole page; unavailable families show no previous payload.
    pub async fn page(
        &self,
        actor: Actor,
        account: Uuid,
        character: Uuid,
        authority: &impl Authority,
        language: ProgressLanguage,
        now_ms: u64,
    ) -> Result<String> {
        actor.check(account)?;
        let bundle = language.bundle()?;
        let mut sections = Vec::new();
        for family in [
            "bestiary",
            "boss_cooldown",
            "quest",
            "achievement",
            "skill_history",
        ] {
            let scope = ReadScope {
                account,
                character: Some(character),
                family,
            };
            let (rows, notice) = match self.read(actor, &scope, authority, now_ms).await {
                Ok(payloads) => (
                    payloads
                        .into_iter()
                        .map(|p| row(p, &bundle, now_ms))
                        .collect::<Result<Vec<_>>>()?,
                    text(&bundle, "progress-empty")?,
                ),
                Err(Error::Disabled | Error::Unavailable) => {
                    (Vec::new(), text(&bundle, "progress-unavailable")?)
                }
                Err(e) => return Err(e),
            };
            sections.push(Section {
                title: text(&bundle, family)?,
                rows,
                notice,
            });
        }
        Page {
            lang: language.code(),
            title: text(&bundle, "progress-title")?,
            observation: text(&bundle, "observed")?,
            sections,
        }
        .render()
        .map_err(|_| Error::Unavailable)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fluent_catalogs_and_template_escape_untrusted_keys() -> Result<()> {
        for language in [ProgressLanguage::Polish, ProgressLanguage::English] {
            let b = language.bundle()?;
            let html = Page {
                lang: language.code(),
                title: text(&b, "progress-title")?,
                observation: text(&b, "observed")?,
                sections: vec![Section {
                    title: text(&b, "quest")?,
                    rows: vec![DisplayRow {
                        key: "<script>".into(),
                        details: vec![],
                    }],
                    notice: String::new(),
                }],
            }
            .render()
            .map_err(|_| Error::Unavailable)?;
            assert!(html.contains("&#60;script&#62;"));
            assert!(!html.contains("<script>"));
            assert!(html.contains("width=device-width"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy)]
pub struct PresentationContext {
    pub language: ProgressLanguage,
    pub now_ms: u64,
}
impl ProgressStore {
    /// Separate private goals page. Missing pace never invents a completion date.
    pub async fn goals_page(
        &self,
        actor: Actor,
        scope: &Goal,
        authority: &impl Authority,
        pace: &impl PrivatePace,
        context: PresentationContext,
    ) -> Result<String> {
        let saved = self.goals(actor, scope, authority).await?;
        let bundle = context.language.bundle()?;
        let mut rows = Vec::new();
        for (_, kind, key, target) in saved {
            let goal = Goal {
                account: scope.account,
                character: scope.character,
                epoch: scope.epoch.clone(),
                recovery: scope.recovery.clone(),
                projection: scope.projection.clone(),
                character_revision: scope.character_revision.clone(),
                kind,
                entity_key: key,
                target: target as u64,
            };
            let date = match self
                .forecast_goal(actor, &goal, authority, pace, context.now_ms)
                .await
            {
                Ok(Some(ms)) => {
                    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1000000)
                        .map_err(|_| Error::Unavailable)?
                        .format(&time::format_description::well_known::Rfc3339)
                        .map_err(|_| Error::Unavailable)?
                }
                Ok(None) | Err(Error::Unavailable) => text(&bundle, "unknown")?,
                Err(e) => return Err(e),
            };
            rows.push(DisplayRow {
                key: wrap_key(&goal.entity_key),
                details: vec![
                    format!("{}: {}", text(&bundle, "target")?, target),
                    format!("{}: {}", text(&bundle, "forecast")?, date),
                ],
            });
        }
        Page {
            lang: context.language.code(),
            title: text(&bundle, "goals")?,
            observation: text(&bundle, "observed")?,
            sections: vec![Section {
                title: text(&bundle, "goals")?,
                rows,
                notice: text(&bundle, "progress-empty")?,
            }],
        }
        .render()
        .map_err(|_| Error::Unavailable)
    }
}
