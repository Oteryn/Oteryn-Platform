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
    reason: String,
    report: String,
    remove: String,
    dismiss: String,
    moderation: String,
    more_reports: String,
    empty: String,
    hint: String,
    all: String,
    reset: String,
    owned: String,
    no_owned: String,
    discoveries: String,
    queue_empty: String,
    queue: String,
    status: String,
    feed_status: String,
    feed_hint: String,
    ownership_feed: String,
    profile_feed: String,
    fresh: String,
    data_unavailable: String,
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
#[derive(Default)]
pub(super) struct PublicPage {
    pub filter: Filter,
    pub worlds: Vec<(Uuid, String)>,
    pub actor: Option<Uuid>,
    pub can_report: bool,
    pub characters: Vec<Character>,
    pub owned: Vec<party_store::Listing>,
    pub staff_url: Option<String>,
}
impl PublicPage {
    fn can_report(&self, owner: &Uuid) -> bool {
        self.can_report && self.actor.as_ref() != Some(owner)
    }

    fn selected_world_missing(&self) -> bool {
        self.filter
            .world
            .is_some_and(|selected| !self.worlds.iter().any(|(world, _)| *world == selected))
    }
}

pub(super) enum View {
    Public(PublicPage),
    Staff((bool, bool)),
}

#[derive(Template)]
#[template(path = "community/party.html", config = "src/party.askama.toml")]
struct Page<'a> {
    lang: &'a str,
    token: &'a str,
    t: Labels,
    rows: Vec<party_store::Listing>,
    view: View,
    purposes: [(&'static str, String); 3],
    professions: [(&'static str, String); 6],
}
pub(super) fn page(
    lang: &str,
    token: &str,
    mut rows: Vec<party_store::Listing>,
    mut view: View,
) -> Result<Response, Error> {
    let resource = FluentResource::try_new(
        if lang == "pl" {
            include_str!("../../../../templates/community/party_pl.ftl")
        } else {
            include_str!("../../../../templates/community/party_en.ftl")
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
        reason: translate(&bundle, "reason")?,
        report: translate(&bundle, "report")?,
        remove: translate(&bundle, "remove")?,
        dismiss: translate(&bundle, "dismiss")?,
        moderation: translate(&bundle, "moderation")?,
        more_reports: translate(&bundle, "more-reports")?,
        empty: translate(&bundle, "empty")?,
        hint: translate(&bundle, "hint")?,
        all: translate(&bundle, "all")?,
        reset: translate(&bundle, "reset")?,
        owned: translate(&bundle, "owned")?,
        no_owned: translate(&bundle, "no-owned")?,
        discoveries: translate(&bundle, "discoveries")?,
        queue_empty: translate(&bundle, "queue-empty")?,
        queue: translate(&bundle, "queue")?,
        status: translate(&bundle, "status")?,
        feed_status: translate(&bundle, "feed-status")?,
        feed_hint: translate(&bundle, "feed-hint")?,
        ownership_feed: translate(&bundle, "ownership-feed")?,
        profile_feed: translate(&bundle, "profile-feed")?,
        fresh: translate(&bundle, "fresh")?,
        data_unavailable: translate(&bundle, "data-unavailable")?,
    };
    let option = |key| translate(&bundle, key).map(|label| (key, label));
    let purposes = [option("hunt")?, option("boss")?, option("quest")?];
    let professions = [
        option("any")?,
        option("knight")?,
        option("paladin")?,
        option("sorcerer")?,
        option("druid")?,
        option("monk")?,
    ];
    let owned = match &mut view {
        View::Public(public) => public.owned.as_mut_slice(),
        View::Staff(_) => &mut [],
    };
    for row in rows.iter_mut().chain(owned) {
        row.purpose = translate(&bundle, &row.purpose)?;
        row.profession = translate(&bundle, &row.profession)?;
        row.status = translate(&bundle, &row.status)?;
    }
    Ok(oteryn_web::render(&Page {
        lang,
        token,
        t,
        rows,
        view,
        purposes,
        professions,
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
