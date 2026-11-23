use super::{
    Boost, DailyView,
    selection::{self, Options, WorldChoice},
};
use askama::Template;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub(super) struct Labels(fluent_bundle::FluentBundle<fluent_bundle::FluentResource>);

impl Labels {
    pub(super) fn new(lang: &str) -> Self {
        let (locale, source) = if lang == "en" {
            (
                unic_langid::langid!("en"),
                include_str!("../../../../templates/today/en.ftl"),
            )
        } else {
            (
                unic_langid::langid!("pl"),
                include_str!("../../../../templates/today/pl.ftl"),
            )
        };
        let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
        if let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) {
            let _ = bundle.add_resource(resource);
        }
        Self(bundle)
    }

    pub(super) fn text(&self, key: &str) -> String {
        self.0
            .get_message(key)
            .and_then(|message| message.value())
            .map(|pattern| {
                self.0
                    .format_pattern(pattern, None, &mut vec![])
                    .into_owned()
            })
            .unwrap_or_default()
    }
}

#[derive(Template)]
#[template(path = "today/widget.html", config = "src/today.askama.toml")]
struct Widget<'a> {
    labels: Labels,
    boost: Option<&'a Boost>,
}

impl Widget<'_> {
    fn text(&self, key: &str) -> String {
        self.labels.text(key)
    }
}

pub(super) fn render_widget(view: Option<&DailyView>, lang: &str) -> String {
    Widget {
        labels: Labels::new(lang),
        boost: view.map(|v| &v.current),
    }
    .render()
    .unwrap_or_default()
}

#[derive(Template)]
#[template(path = "today/section.html", config = "src/today.askama.toml")]
struct Section<'a> {
    lang: &'a str,
    labels: Labels,
    available: bool,
    invalid: bool,
    worlds: Vec<WorldChoice>,
    world: &'a str,
    date: String,
    today: String,
    filtered: bool,
    current_url: String,
    current: String,
    coverage: String,
    history: Vec<String>,
    raids_available: bool,
    raids: Vec<RaidRow<'a>>,
}

impl Section<'_> {
    fn text(&self, key: &str) -> String {
        self.labels.text(key)
    }
}

struct RaidRow<'a> {
    name: &'a str,
    state: String,
    started: String,
    slot: Option<String>,
}

pub(super) fn timestamp(seconds: i64) -> String {
    OffsetDateTime::from_unix_timestamp(seconds)
        .ok()
        .and_then(|date| date.format(&Rfc3339).ok())
        .unwrap_or_default()
}

fn boost(value: &Boost, labels: &Labels) -> String {
    format!(
        "{} · {}: {} · {}: {} ({} – {})",
        labels.text("boost"),
        labels.text("creature"),
        value.creature,
        labels.text("boss"),
        value.boss,
        timestamp(value.from),
        timestamp(value.until)
    )
}

pub(super) fn render_section(
    view: Option<&DailyView>,
    lang: &str,
    options: Result<&Options, super::Error>,
    worlds: Vec<WorldChoice>,
    now: i64,
) -> String {
    let labels = Labels::new(lang);
    let fallback = Options::default();
    let invalid = options.is_err();
    let options = options.unwrap_or(&fallback);
    let today = selection::day(now)
        .map(|day| day.to_string())
        .unwrap_or_default();
    let current_url = worlds.iter().find(|world| world.selected).map_or_else(
        || format!("/{lang}/today"),
        |world| format!("/{lang}/today?world={}", world.id),
    );
    Section {
        lang,
        available: view.is_some(),
        invalid,
        worlds,
        world: view.map(|v| v.world.as_str()).unwrap_or_default(),
        date: options
            .date
            .map_or_else(|| today.clone(), |date| date.to_string()),
        today,
        filtered: options.date.is_some(),
        current_url,
        current: view.map(|v| boost(&v.current, &labels)).unwrap_or_default(),
        coverage: view
            .map(|v| {
                format!(
                    "{} {}",
                    labels.text("coverage"),
                    timestamp(v.history_complete_from)
                )
            })
            .unwrap_or_default(),
        history: view
            .map(|v| v.history.iter().map(|h| boost(h, &labels)).collect())
            .unwrap_or_default(),
        raids_available: view.is_some_and(|v| v.raid_history_available()),
        raids: view
            .map(|v| {
                v.raids
                    .iter()
                    .map(|raid| RaidRow {
                        name: &raid.name,
                        state: labels.text(&raid.state),
                        started: timestamp(raid.started_at),
                        slot: raid.scheduled_at.map(timestamp),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        labels,
    }
    .render()
    .unwrap_or_default()
}
