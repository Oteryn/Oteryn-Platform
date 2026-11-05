use super::{Error, Report, Result};
use askama::Template;
use fluent_bundle::{FluentBundle, FluentResource};

#[derive(Debug, Clone, Copy)]
pub enum Locale {
    Pl,
    En,
}
impl Locale {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "pl" => Ok(Self::Pl),
            "en" => Ok(Self::En),
            _ => Err(Error::Invalid),
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Self::Pl => "pl",
            Self::En => "en",
        }
    }
}
pub fn message(locale: Locale, key: &str) -> Result<String> {
    let source = match locale {
        Locale::Pl => include_str!("../reports_pl.ftl"),
        Locale::En => include_str!("../reports_en.ftl"),
    };
    let resource = FluentResource::try_new(source.into()).map_err(|_| Error::Unavailable)?;
    let mut bundle =
        FluentBundle::new(vec![locale.code().parse().map_err(|_| Error::Unavailable)?]);
    bundle
        .add_resource(resource)
        .map_err(|_| Error::Unavailable)?;
    let pattern = bundle
        .get_message(key)
        .and_then(|m| m.value())
        .ok_or(Error::Unavailable)?;
    let mut errors = Vec::new();
    let text = bundle
        .format_pattern(pattern, None, &mut errors)
        .into_owned();
    if !errors.is_empty() {
        return Err(Error::Unavailable);
    }
    Ok(text)
}
#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{{ title }}</title><link rel=\"stylesheet\" href=\"/static/css/app.css\"><link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><link rel=\"stylesheet\" href=\"/static/css/components.css\"><body class=\"op-theme\"><main class=\"op-stack\"><h1>{{ title }}</h1><p>{{ start_date }} – {{ end_date }} UTC</p><dl><dt>{{ sessions }}</dt><dd>{{ count }}</dd><dt>{{ duration }}</dt><dd>{{ seconds }}</dd><dt>{{ gained }}</dt><dd>{{ gain }}</dd><dt>{{ lost }}</dt><dd>{{ loss }}</dd></dl><nav aria-label=\"{{ title }}\"><a href=\"?week={{ start }}&amp;format=json\">JSON</a> <a href=\"?week={{ start }}&amp;format=csv&amp;dataset=sessions\">CSV ({{ sessions }})</a> <a href=\"?week={{ start }}&amp;format=csv&amp;dataset=drops\">CSV ({{ drops }})</a></nav><ul>{% for session in report.sessions %}<li>{{ session.id }} <a href=\"reports/notes?kind=session&amp;id={{ session.id }}\">{{ note_label }}</a> <a href=\"reports/notes?kind=character&amp;id={{ session.character_id }}\">{{ character_note }}</a></li>{% endfor %}</ul><form class=\"op-stack\" method=\"post\" action=\"reports/email\"><input type=\"hidden\" name=\"csrf\" value=\"{{ csrf }}\"><label><input type=\"checkbox\" name=\"enabled\" value=\"true\"{% if email_enabled %} checked{% endif %}>{{ optin }}</label><button class=\"op-button\">{{ preferences }}</button></form></main></body></html>",
    ext = "html"
)]
struct Weekly<'a> {
    report: &'a Report,
    email_enabled: bool,
    note_label: String,
    character_note: String,
    lang: &'a str,
    title: String,
    sessions: String,
    drops: String,
    duration: String,
    gained: String,
    lost: String,
    start: i64,
    start_date: String,
    end_date: String,
    count: usize,
    seconds: &'a str,
    gain: &'a str,
    loss: &'a str,
    csrf: &'a str,
    optin: String,
    preferences: String,
}
pub fn weekly(locale: Locale, report: &Report, csrf: &str, email_enabled: bool) -> Result<String> {
    Weekly {
        report,
        email_enabled,
        note_label: message(locale, "notes")?,
        character_note: message(locale, "character-note")?,
        lang: locale.code(),
        title: message(locale, "title")?,
        sessions: message(locale, "sessions")?,
        drops: message(locale, "drops")?,
        duration: message(locale, "duration")?,
        gained: message(locale, "gained")?,
        lost: message(locale, "lost")?,
        start: report.week.start,
        start_date: oteryn_db::outbox::OffsetDateTime::from_unix_timestamp(report.week.start)
            .map_err(|_| Error::Invalid)?
            .date()
            .to_string(),
        end_date: oteryn_db::outbox::OffsetDateTime::from_unix_timestamp(report.week.end)
            .map_err(|_| Error::Invalid)?
            .date()
            .to_string(),
        count: report.session_count,
        seconds: &report.duration_seconds,
        gain: &report.experience_gained,
        loss: &report.experience_lost,
        csrf,
        optin: message(locale, "optin")?,
        preferences: message(locale, "preferences")?,
    }
    .render()
    .map_err(|_| Error::Unavailable)
}
#[derive(Template)]
#[template(
    source = "<!doctype html><html lang=\"{{ lang }}\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{{ title }}</title><link rel=\"stylesheet\" href=\"/static/css/app.css\"><link rel=\"stylesheet\" href=\"/static/css/tokens.css\"><link rel=\"stylesheet\" href=\"/static/css/components.css\"><body class=\"op-theme\"><main class=\"op-stack\"><h1>{{ title }}</h1><form class=\"op-stack\" method=\"post\"><input type=\"hidden\" name=\"csrf\" value=\"{{ csrf }}\"><label for=\"note\">{{ title }}</label><p id=\"hint\">{{ hint }}</p><textarea class=\"op-input\" id=\"note\" name=\"body\" rows=\"8\" cols=\"24\" maxlength=\"4000\" aria-describedby=\"hint\">{{ body }}</textarea><button class=\"op-button\">{{ save }}</button></form></main></body></html>",
    ext = "html"
)]
struct Note<'a> {
    lang: &'a str,
    title: String,
    hint: String,
    save: String,
    csrf: &'a str,
    body: &'a str,
}
pub fn note(locale: Locale, csrf: &str, body: &str) -> Result<String> {
    Note {
        lang: locale.code(),
        title: message(locale, "notes")?,
        hint: message(locale, "hint")?,
        save: message(locale, "save")?,
        csrf,
        body,
    }
    .render()
    .map_err(|_| Error::Unavailable)
}
