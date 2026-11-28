use super::{Flash, World, http::WorldFilters};
use crate::commands::{Sex, Vocation};
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
    path = "account/character_create.html",
    config = "src/overview/askama.toml"
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
    sex_label: String,
    sexes: Vec<Choice>,
    vocation_label: String,
    vocations: Vec<Choice>,
    submit: String,
    refresh: String,
    roster: String,
    roster_state: String,
}
/// One required radio option: `value` is the exact wire name, `label` is localized.
struct Choice {
    value: &'static str,
    label: String,
    checked: bool,
}
fn choices(
    lang: &str,
    values: impl IntoIterator<Item = &'static str>,
    prefix: &str,
) -> Vec<Choice> {
    values
        .into_iter()
        .map(|value| Choice {
            value,
            label: text(lang, &format!("{prefix}-{}", value.to_ascii_lowercase())),
            checked: false,
        })
        .collect()
}
fn check(choices: &mut [Choice], selected: &str) {
    for choice in choices {
        choice.checked = choice.value == selected;
    }
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
        sex_label: text(lang, "sex"),
        sexes: choices(lang, Sex::ALL.map(Sex::wire), "sex"),
        vocation_label: text(lang, "vocation"),
        vocations: choices(lang, Vocation::ALL.map(Vocation::wire), "vocation"),
        submit: text(lang, "submit"),
        refresh: text(lang, "refresh"),
        roster: String::new(),
        roster_state: String::new(),
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
        check(&mut page.sexes, &flash.sex);
        check(&mut page.vocations, &flash.vocation);
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

pub(super) fn reconciling(lang: &str) -> Response {
    let mut page = page(lang, "");
    page.message = if lang == "en" {
        "This request uses a retired format and is awaiting reconciliation."
    } else {
        "To zgłoszenie korzysta z wycofanego formatu i oczekuje na uzgodnienie wyniku."
    }
    .into();
    render(page, StatusCode::OK)
}

pub(super) fn created(
    lang: &str,
    character: Uuid,
    roster: Option<&[crate::lcfa::CharacterSummary]>,
    overview_enabled: bool,
) -> Response {
    let mut page = page(lang, "");
    let visible = overview_enabled
        && roster.is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| entry.character_id.as_uuid() == character)
        });
    page.message = text(
        lang,
        if visible {
            "created-visible"
        } else {
            "created"
        },
    );
    if overview_enabled {
        page.roster = text(lang, "open-roster");
        if roster.is_none() {
            page.roster_state = text(lang, "roster-unavailable");
        }
    }
    render(page, StatusCode::OK)
}
