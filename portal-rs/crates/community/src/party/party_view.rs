use super::*;
use askama::Template;
use axum::{
    extract::Request,
    http::{HeaderValue, header},
    middleware::Next,
};
use fluent_bundle::{FluentBundle, FluentResource};
use std::borrow::Cow;
struct Labels {
    unavailable: String,
    title: String,
    world: String,
    purpose: String,
    level: String,
    profession: String,
    filter: String,
    create: String,
    character: String,
    min: String,
    max: String,
    starts: String,
    expires: String,
    note: String,
    contact: String,
    reason: String,
    report: String,
    remove: String,
    dismiss: String,
    moderation: String,
    empty: String,
    hint: String,
    hunt: String,
    boss: String,
    quest: String,
    any: String,
    knight: String,
    paladin: String,
    sorcerer: String,
    druid: String,
    monk: String,
}
fn translate(bundle: &FluentBundle<FluentResource>, key: &str) -> Result<String, Error> {
    let value = bundle
        .get_message(key)
        .and_then(|m| m.value())
        .ok_or(Error::Invalid)?;
    let mut errors = Vec::new();
    let text: Cow<'_, str> = bundle.format_pattern(value, None, &mut errors);
    if !errors.is_empty() {
        return Err(Error::Invalid);
    }
    Ok(text.into_owned())
}
#[derive(Template)]
#[template(
    source = r###"<!doctype html>
<html lang="{{ lang }}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{{ t.title }}</title>
<link rel="stylesheet" href="/static/css/tokens.css">
<link rel="stylesheet" href="/static/css/components.css">
<link rel="stylesheet" href="/static/css/app.css">
</head>
<body class="op-theme">
<main class="op-stack">
<h1>{{ t.title }}</h1>
<p>{{ t.hint }}</p>
<a href="/admin/{{ lang }}/party">{{ t.moderation }}</a>
{% if !moderation %}<form class="op-stack" method="get">
<label class="op-field">{{ t.world }}<input class="op-input" name="world" type="text"  required>
</label>
<label class="op-field">{{ t.purpose }}<select class="op-input" name="purpose">
<option value="hunt">{{ t.hunt }}</option>
<option value="boss">{{ t.boss }}</option>
<option value="quest">{{ t.quest }}</option>
</select>
</label>
<label class="op-field">{{ t.level }}<input class="op-input" name="level" type="number" min="1" max="100000" required>
</label>
<label class="op-field">{{ t.profession }}<select class="op-input" name="profession">
<option value="any">{{ t.any }}</option>
<option value="knight">{{ t.knight }}</option>
<option value="paladin">{{ t.paladin }}</option>
<option value="sorcerer">{{ t.sorcerer }}</option>
<option value="druid">{{ t.druid }}</option>
<option value="monk">{{ t.monk }}</option>
</select>
</label>
<button class="op-button">{{ t.filter }}</button>
</form>
{% if can_post %}<h2>{{ t.create }}</h2>
<form class="op-stack" method="post" action="/{{ lang }}/party">
<input class="op-input" type="hidden" name="_csrf" value="{{ token }}">
<label class="op-field">{{ t.character }}<input class="op-input" name="character" type="text"  required>
</label>
<label class="op-field">{{ t.purpose }}<select class="op-input" name="purpose">
<option value="hunt">{{ t.hunt }}</option>
<option value="boss">{{ t.boss }}</option>
<option value="quest">{{ t.quest }}</option>
</select>
</label>
<label class="op-field">{{ t.min }}<input class="op-input" name="min_level" type="number" min="1" max="100000" required>
</label>
<label class="op-field">{{ t.max }}<input class="op-input" name="max_level" type="number" min="1" max="100000" required>
</label>
<label class="op-field">{{ t.profession }}<select class="op-input" name="profession">
<option value="any">{{ t.any }}</option>
<option value="knight">{{ t.knight }}</option>
<option value="paladin">{{ t.paladin }}</option>
<option value="sorcerer">{{ t.sorcerer }}</option>
<option value="druid">{{ t.druid }}</option>
<option value="monk">{{ t.monk }}</option>
</select>
</label>
<label class="op-field">{{ t.starts }}<input class="op-input" name="starts_at" type="number" min="0" required>
</label>
<label class="op-field">{{ t.expires }}<input class="op-input" name="expires_at" type="number" min="0" required>
</label>
<label class="op-field">{{ t.note }}<textarea class="op-input" name="note" maxlength="500">
</textarea>
</label>
<button class="op-button">{{ t.create }}</button>
</form>{% else %}<p>{{ t.unavailable }}</p>{% endif %}{% endif %}
{% for row in rows %}<article class="op-panel op-stack">
<h2>{{ row.name }}</h2>
<dl>
<dt>{{ t.world }}</dt>
<dd>{{ row.world }}</dd>
<dt>{{ t.purpose }}</dt>
<dd>{{ row.purpose }}</dd>
<dt>{{ t.level }}</dt>
<dd>{{ row.min }}–{{ row.max }}</dd>
<dt>{{ t.profession }}</dt>
<dd>{{ row.profession }}</dd>
<dt>{{ t.starts }}</dt>
<dd>{{ row.starts }}</dd>
<dt>{{ t.expires }}</dt>
<dd>{{ row.expires }}</dd>
<dt>{{ t.contact }}</dt>
<dd>{{ row.name }}</dd>
</dl>
<p>{{ row.note }}</p>
{% if moderation %}<p>{{ row.reports }}</p>
<form class="op-stack" method="post" action="/admin/{{ lang }}/party/{{ row.id }}/moderate">
<input class="op-input" type="hidden" name="_csrf" value="{{ token }}">
<label class="op-field">{{ t.reason }}<input class="op-input" name="reason" type="text" maxlength="500" required>
</label>
<button class="op-button" name="action" value="remove">{{ t.remove }}</button>
<button class="op-button" name="action" value="dismiss">{{ t.dismiss }}</button>
</form>{% else %}<form class="op-stack" method="post" action="/{{ lang }}/party/{{ row.id }}/report">
<input class="op-input" type="hidden" name="_csrf" value="{{ token }}">
<label class="op-field">{{ t.reason }}<input class="op-input" name="reason" type="text" maxlength="500" required>
</label>
<button class="op-button">{{ t.report }}</button>
</form>
<form class="op-stack" method="post" action="/{{ lang }}/party/{{ row.id }}/remove">
<input class="op-input" type="hidden" name="_csrf" value="{{ token }}">
<button class="op-button">{{ t.remove }}</button>
</form>{% endif %}</article>{% else %}<p>{{ t.empty }}</p>{% endfor %}</main>
</body>
</html>"###,
    ext = "html"
)]
struct Page<'a> {
    lang: &'a str,
    token: &'a str,
    t: Labels,
    rows: Vec<party_store::Listing>,
    moderation: bool,
    can_post: bool,
}
pub(super) fn page(
    lang: &str,
    token: &str,
    mut rows: Vec<party_store::Listing>,
    moderation: bool,
    can_post: bool,
) -> Result<Response, Error> {
    let resource = FluentResource::try_new(
        if lang == "pl" {
            include_str!("party_pl.ftl")
        } else {
            include_str!("party_en.ftl")
        }
        .into(),
    )
    .map_err(|_| Error::Invalid)?;
    let mut bundle = FluentBundle::new(vec![if lang == "pl" {
        unic_langid::langid!("pl")
    } else {
        unic_langid::langid!("en")
    }]);
    bundle.add_resource(resource).map_err(|_| Error::Invalid)?;
    let t = Labels {
        unavailable: translate(&bundle, "unavailable")?,
        title: translate(&bundle, "title")?,
        world: translate(&bundle, "world")?,
        purpose: translate(&bundle, "purpose")?,
        level: translate(&bundle, "level")?,
        profession: translate(&bundle, "profession")?,
        filter: translate(&bundle, "filter")?,
        create: translate(&bundle, "create")?,
        character: translate(&bundle, "character")?,
        min: translate(&bundle, "min")?,
        max: translate(&bundle, "max")?,
        starts: translate(&bundle, "starts")?,
        expires: translate(&bundle, "expires")?,
        note: translate(&bundle, "note")?,
        contact: translate(&bundle, "contact")?,
        reason: translate(&bundle, "reason")?,
        report: translate(&bundle, "report")?,
        remove: translate(&bundle, "remove")?,
        dismiss: translate(&bundle, "dismiss")?,
        moderation: translate(&bundle, "moderation")?,
        empty: translate(&bundle, "empty")?,
        hint: translate(&bundle, "hint")?,
        hunt: translate(&bundle, "hunt")?,
        boss: translate(&bundle, "boss")?,
        quest: translate(&bundle, "quest")?,
        any: translate(&bundle, "any")?,
        knight: translate(&bundle, "knight")?,
        paladin: translate(&bundle, "paladin")?,
        sorcerer: translate(&bundle, "sorcerer")?,
        druid: translate(&bundle, "druid")?,
        monk: translate(&bundle, "monk")?,
    };
    for row in &mut rows {
        row.purpose = translate(&bundle, &row.purpose)?;
        row.profession = translate(&bundle, &row.profession)?;
    }
    Ok(oteryn_web::render(&Page {
        lang,
        token,
        t,
        rows,
        moderation,
        can_post,
    }))
}
pub(super) async fn headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"));
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}
