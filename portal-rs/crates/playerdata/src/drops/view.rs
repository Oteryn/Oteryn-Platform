use super::{Error, Presentation};
use askama::Template;
use fluent_bundle::{FluentBundle, FluentResource};
use unic_langid::langid;

#[derive(Debug, Clone, Copy)]
pub enum Language {
    Polish,
    English,
}

#[derive(Template)]
#[template(
    source = r#"<!doctype html>
<html lang="{{ lang }}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{{ title }}</title></head>
<body><main><h1>{{ title }}</h1>
<h2>{{ records }}</h2><dl>
<dt>{{ experience }}</dt><dd>{{ exp_value }}</dd>
<dt>{{ duration }}</dt><dd>{{ duration_value }}</dd>
<dt>{{ boss }}</dt><dd>{{ unavailable }}</dd>
</dl>
<h3>{{ profit }}</h3>{% if data.best_profit_per_hour_by_basis.is_empty() %}<p>{{ unavailable }}</p>{% endif %}
{% for (basis, record) in data.best_profit_per_hour_by_basis %}
<p>{{ basis_label }}: {% for ch in basis.chars() %}{{ ch }}<wbr>{% endfor %}</p><p>{{ record.value.numerator }} / {{ record.value.seconds }}</p>
{% endfor %}
<h3>{{ loot }}</h3>{% if data.largest_loot_by_basis.is_empty() %}<p>{{ unavailable }}</p>{% endif %}
{% for (basis, record) in data.largest_loot_by_basis %}
<p>{{ basis_label }}: {% for ch in basis.chars() %}{{ ch }}<wbr>{% endfor %}</p><p>{{ record.value }}</p>
{% endfor %}
<h2>{{ drops }}</h2>{% if data.drops.is_empty() %}<p>{{ empty }}</p>{% endif %}<ul>
{% for drop in data.drops %}<li><p>{% for ch in drop.item_key.chars() %}{{ ch }}<wbr>{% endfor %}</p><dl><dt>{{ quantity }}</dt><dd>{{ drop.quantity }}</dd><dt>{{ occurred }}</dt><dd>{{ drop.occurred_at }}</dd></dl></li>{% endfor %}
</ul><h2>{{ chronicle }}</h2>{% if data.chronicle.is_empty() %}<p>{{ empty }}</p>{% endif %}<ol>
{% for session in data.chronicle %}<li><dl><dt>{{ started }}</dt><dd>{{ session.started_at }}</dd><dt>{{ ended }}</dt><dd>{{ session.ended_at }}</dd><dt>{{ seconds }}</dt><dd>{{ session.active_seconds }}</dd></dl></li>{% endfor %}
</ol></main></body></html>"#,
    ext = "html"
)]
struct Page<'a> {
    data: &'a Presentation,
    lang: &'a str,
    title: String,
    records: String,
    experience: String,
    duration: String,
    boss: String,
    unavailable: String,
    profit: String,
    loot: String,
    basis_label: String,
    drops: String,
    empty: String,
    quantity: String,
    occurred: String,
    chronicle: String,
    started: String,
    ended: String,
    seconds: String,
    exp_value: String,
    duration_value: String,
}

pub(super) fn render(data: &Presentation, language: Language) -> Result<String, Error> {
    let (lang, id, resource) = match language {
        Language::Polish => ("pl", langid!("pl"), include_str!("pl.ftl")),
        Language::English => ("en", langid!("en"), include_str!("en.ftl")),
    };
    let resource = FluentResource::try_new(resource.to_owned()).map_err(|_| Error::Unavailable)?;
    let mut bundle = FluentBundle::new(vec![id]);
    bundle
        .add_resource(resource)
        .map_err(|_| Error::Unavailable)?;
    let text = |key: &str| -> Result<String, Error> {
        let pattern = bundle
            .get_message(key)
            .and_then(|m| m.value())
            .ok_or(Error::Unavailable)?;
        let mut errors = Vec::new();
        let value = bundle
            .format_pattern(pattern, None, &mut errors)
            .into_owned();
        if errors.is_empty() {
            Ok(value)
        } else {
            Err(Error::Unavailable)
        }
    };
    Page {
        data,
        lang,
        title: text("title")?,
        records: text("records")?,
        experience: text("experience")?,
        duration: text("duration")?,
        boss: text("boss")?,
        unavailable: text("unavailable")?,
        profit: text("profit")?,
        loot: text("loot")?,
        basis_label: text("basis")?,
        drops: text("drops")?,
        empty: text("empty")?,
        quantity: text("quantity")?,
        occurred: text("occurred")?,
        chronicle: text("chronicle")?,
        started: text("started")?,
        ended: text("ended")?,
        seconds: text("seconds")?,
        exp_value: data
            .best_experience_per_hour
            .as_ref()
            .map(|r| format!("{} / {}", r.value.numerator, r.value.seconds))
            .unwrap_or(text("unavailable")?),
        duration_value: data
            .longest_session
            .as_ref()
            .map(|r| r.value.to_string())
            .unwrap_or(text("unavailable")?),
    }
    .render()
    .map_err(|_| Error::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_localized_mobile_document() -> Result<(), Error> {
        let data = Presentation::default();
        let pl = render(&data, Language::Polish)?;
        let en = render(&data, Language::English)?;
        assert!(pl.contains("Dziennik postaci"));
        assert!(en.contains("Character journal"));
        assert!(en.contains("name=\"viewport\""));
        assert!(en.contains("<main>"));
        assert!(!en.contains("<script"));
        Ok(())
    }
}
