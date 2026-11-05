use super::{Flash, World, http::WorldFilters};
use askama::Template;
use axum::{
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use fluent_bundle::{FluentBundle, FluentResource};
use uuid::Uuid;

fn text(lang: &str, key: &str) -> String {
    let locale = if lang == "en" {
        unic_langid::langid!("en")
    } else {
        unic_langid::langid!("pl")
    };
    let source = if lang == "en" {
        include_str!("en.ftl")
    } else {
        include_str!("pl.ftl")
    };
    let mut bundle = FluentBundle::new(vec![locale]);
    if let Ok(resource) = FluentResource::try_new(source.into())
        && bundle.add_resource(resource).is_ok()
        && let Some(value) = bundle.get_message(key).and_then(|m| m.value())
    {
        return bundle
            .format_pattern(value, None, &mut Vec::new())
            .into_owned();
    }
    String::new()
}
#[derive(Template)]
#[template(
    source = r##"<!doctype html>
<html lang="{{ lang }}" class="op-theme"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{{ title }}</title><link rel="stylesheet" href="/static/css/tokens.css"><link rel="stylesheet" href="/static/css/components.css"><link rel="stylesheet" href="/static/css/surface.css"></head>
<body class="op-theme"><header><a href="#main">{{ skip }}</a><nav aria-label="{{ navigation }}"><a href="/{{ lang }}/account">{{ account }}</a><a href="/{{ lang }}/account/characters/new">{{ title }}</a></nav></header>
<main id="main" tabindex="-1" class="op-stack"><h1>{{ title }}</h1>
{% if !message.is_empty() %}<p class="op-alert" role="status">{{ message }}</p>{% endif %}
{% if form %}<p id="rules">{{ rules }}</p><details class="op-disclosure"><summary>{{ reserved_label }}</summary><p>{{ reserved_words }}</p></details><p>{{ supported }}</p>
{% if show_filters %}<form method="get" action="/{{ lang }}/account/characters/new" class="op-stack">
<div class="op-field"><label for="pvp">{{ pvp_label }}</label><select class="op-input" id="pvp" name="pvp"><option value="">{{ any }}</option>{% for value in pvp_values %}<option{% if filters.pvp == value.as_str() %} selected{% endif %}>{{ value }}</option>{% endfor %}</select></div>
<div class="op-field"><label for="location">{{ location_label }}</label><select class="op-input" id="location" name="location"><option value="">{{ any }}</option>{% for value in locations %}<option{% if filters.location == value.as_str() %} selected{% endif %}>{{ value }}</option>{% endfor %}</select></div>
<button class="op-button op-button--secondary" type="submit">{{ apply }}</button></form>{% endif %}
<form method="post" action="/{{ lang }}/account/characters/new" class="op-stack">
<input type="hidden" name="_csrf" value="{{ csrf }}"><input type="hidden" name="submission" value="{{ submission }}">
<div class="op-field"><label for="name">{{ name_label }}</label><input class="op-input" id="name" name="name" value="{{ name }}" required maxlength="255" autocomplete="off" aria-describedby="rules"></div>
<div><a class="op-button op-button--secondary" href="/{{ lang }}/account/characters/suggest">{{ suggest }}</a><p class="op-hint">{{ suggestion_note }}</p></div>
<div class="op-field"><label for="world">{{ world_label }}</label><select class="op-input" id="world" name="world" required><option value="">{{ choose }}</option>{% for world in worlds %}<option value="{{ world.id }}"{% if selected_world == world.id.to_string() %} selected{% endif %}>{{ world.name }}{% if !world.pvp.is_empty() %} · {{ world.pvp }}{% endif %}{% if !world.location.is_empty() %} · {{ world.location }}{% endif %}</option>{% endfor %}</select></div>
<button class="op-button" type="submit"{% if worlds.is_empty() %} disabled{% endif %}>{{ submit }}</button></form>
{% else %}<p><a class="op-button op-button--secondary" href="">{{ refresh }}</a></p>{% endif %}
</main></body></html>"##,
    ext = "html"
)]
struct Page<'a> {
    lang: &'a str,
    title: String,
    skip: String,
    navigation: String,
    account: String,
    message: String,
    form: bool,
    show_filters: bool,
    pvp_label: String,
    location_label: String,
    any: String,
    apply: String,
    pvp_values: Vec<String>,
    locations: Vec<String>,
    filters: WorldFilters,
    rules: String,
    supported: String,
    reserved_label: String,
    reserved_words: String,
    csrf: &'a str,
    submission: String,
    name_label: String,
    name: String,
    suggest: String,
    suggestion_note: String,
    world_label: String,
    choose: String,
    worlds: Vec<World>,
    selected_world: String,
    submit: String,
    refresh: String,
}
fn page<'a>(lang: &'a str, csrf: &'a str) -> Page<'a> {
    Page {
        lang,
        csrf,
        title: text(lang, "title"),
        skip: text(lang, "skip"),
        navigation: text(lang, "navigation"),
        account: text(lang, "account"),
        message: String::new(),
        form: false,
        show_filters: false,
        pvp_label: text(lang, "pvp"),
        location_label: text(lang, "location"),
        any: text(lang, "any"),
        apply: text(lang, "apply"),
        pvp_values: Vec::new(),
        locations: Vec::new(),
        filters: WorldFilters::default(),
        rules: text(lang, "rules"),
        supported: text(lang, "supported"),
        reserved_label: text(lang, "reserved-label"),
        reserved_words: text(lang, "reserved-words"),
        submission: Uuid::now_v7().to_string(),
        name_label: text(lang, "name"),
        name: String::new(),
        suggest: text(lang, "suggest"),
        suggestion_note: text(lang, "suggestion-note"),
        world_label: text(lang, "world"),
        choose: text(lang, "choose"),
        worlds: Vec::new(),
        selected_world: String::new(),
        submit: text(lang, "submit"),
        refresh: text(lang, "refresh"),
    }
}
fn render(page: Page<'_>, status: StatusCode) -> Response {
    match page.render() {
        Ok(html) => (status, [(header::CACHE_CONTROL, "no-store")], Html(html)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
pub(super) fn form(
    lang: &str,
    csrf: &str,
    flash: Option<Flash>,
    worlds: Vec<World>,
    filters: WorldFilters,
) -> Response {
    let mut page = page(lang, csrf);
    page.form = true;
    page.show_filters = worlds.len() > 1;
    page.pvp_values = worlds
        .iter()
        .map(|world| world.pvp.clone())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    page.locations = worlds
        .iter()
        .map(|world| world.location.clone())
        .filter(|value| !value.is_empty())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    page.worlds = worlds
        .into_iter()
        .filter(|world| {
            (filters.pvp.is_empty() || world.pvp == filters.pvp)
                && (filters.location.is_empty() || world.location == filters.location)
        })
        .collect();
    if page.worlds.is_empty() {
        page.message = text(lang, "no-worlds");
    }
    page.filters = filters;
    if let Some(flash) = flash {
        page.name = flash.name;
        page.selected_world = flash.world;
        if super::http::valid_submission(&flash.submission) {
            page.submission = flash.submission;
        }
        page.message = text(lang, &flash.error);
    }
    render(page, StatusCode::OK)
}
pub(super) fn notice(lang: &str, status: StatusCode, key: &str) -> Response {
    let mut page = page(lang, "");
    page.message = text(lang, key);
    render(page, status)
}
