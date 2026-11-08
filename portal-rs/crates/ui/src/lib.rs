//! Shared portal presentation. No module, database or session dependencies.
use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use fluent_bundle::{FluentBundle, FluentResource};

/// Translate through the portal's existing Fluent catalogues.
pub fn text(lang: &str, key: &str) -> String {
    let (locale, source) = if lang == "en" {
        (unic_langid::langid!("en"), include_str!("../i18n/en.ftl"))
    } else {
        (unic_langid::langid!("pl"), include_str!("../i18n/pl.ftl"))
    };
    let mut bundle = FluentBundle::new(vec![locale]);
    if let Ok(resource) = FluentResource::try_new(source.to_owned())
        && bundle.add_resource(resource).is_ok()
        && let Some(pattern) = bundle.get_message(key).and_then(|m| m.value())
    {
        return bundle
            .format_pattern(pattern, None, &mut Vec::new())
            .into_owned();
    }
    String::new()
}

/// Navigation group. Labels are Fluent keys, never language-dependent module code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Play,
    Community,
    World,
    Account,
}
impl Group {
    fn icon(self) -> &'static str {
        match self {
            Self::Play => "shop",
            Self::Community => "community",
            Self::World => "world",
            Self::Account => "support",
        }
    }
    fn key(self) -> &'static str {
        match self {
            Self::Play => "shell-play",
            Self::Community => "shell-community",
            Self::World => "shell-world",
            Self::Account => "shell-account",
        }
    }
}
/// A trusted, localized application route suffix, e.g. `wiki`.
#[derive(Clone, Debug)]
pub struct Entry {
    pub section: &'static str,
    pub label: &'static str,
    pub path: &'static str,
    pub group: Group,
    pub footer: bool,
}
/// Built once at startup from the same enable/mount decisions as module routers.
#[derive(Clone, Debug, Default)]
pub struct Registry {
    entries: Vec<Entry>,
    pub login: bool,
    pub registration: bool,
    pub account: bool,
}
impl Registry {
    /// Register only a mounted/enabled section. Disabled entries are never retained.
    pub fn register(&mut self, enabled: bool, entry: Entry) {
        if enabled && !self.entries.iter().any(|e| e.section == entry.section) {
            self.entries.push(entry);
        }
    }
    /// Convenience for module-owned navigation registration.
    pub fn section(
        &mut self,
        enabled: bool,
        section: &'static str,
        label: &'static str,
        path: &'static str,
        group: Group,
        footer: bool,
    ) {
        self.register(
            enabled,
            Entry {
                section,
                label,
                path,
                group,
                footer,
            },
        );
    }
    pub fn chrome(&self, lang: &str, path: &str, account: Account) -> Chrome {
        let lang = if lang == "en" { "en" } else { "pl" };
        let other = if lang == "en" { "pl" } else { "en" };
        let clean = path.split(['?', '#']).next().unwrap_or("/");
        let suffix = clean
            .strip_prefix("/pl/")
            .or_else(|| clean.strip_prefix("/en/"))
            .unwrap_or_else(|| {
                if matches!(clean, "/pl" | "/en") {
                    ""
                } else {
                    clean.trim_start_matches('/')
                }
            });
        let mut entries: Vec<NavLink> = self
            .entries
            .iter()
            .map(|e| {
                let url = if e.path.starts_with('/') {
                    e.path.to_owned()
                } else {
                    format!("/{lang}/{}", e.path)
                };
                let current = clean == url
                    || clean
                        .strip_prefix(&url)
                        .is_some_and(|tail| tail.starts_with('/'));
                NavLink {
                    section: e.section.to_owned(),
                    label: text(lang, e.label),
                    url,
                    current,
                    footer: e.footer,
                    group: e.group,
                }
            })
            .collect();
        let current_section = entries
            .iter()
            .filter(|e| e.current)
            .max_by_key(|e| e.url.len())
            .map(|e| e.section.clone())
            .unwrap_or_default();
        for entry in &mut entries {
            entry.current = entry.section == current_section;
        }
        let groups = [Group::Play, Group::Community, Group::World, Group::Account]
            .into_iter()
            .filter_map(|group| {
                let links: Vec<_> = entries
                    .iter()
                    .filter(|e| e.group == group)
                    .cloned()
                    .collect();
                (!links.is_empty()).then(|| NavGroup {
                    label: text(lang, group.key()),
                    icon: group.icon(),
                    links,
                })
            })
            .collect();
        Chrome {
            lang: lang.to_owned(),
            alternate_url: format!("/{other}/{suffix}"),
            alternate_lang: other.to_owned(),
            entries,
            groups,
            home_current: matches!(clean, "/" | "/pl" | "/en" | "/pl/" | "/en/"),
            current_section,
            account,
            login_mounted: self.login,
            registration_mounted: self.registration,
            account_mounted: self.account,
        }
    }
}
/// Only a valid session supplies the signed-in variant and its CSRF token.
#[derive(Clone, Debug, Default)]
pub enum Account {
    #[default]
    Anonymous,
    SignedIn {
        csrf: String,
    },
}
#[derive(Clone, Debug)]
pub struct NavLink {
    pub section: String,
    pub label: String,
    pub url: String,
    pub current: bool,
    pub footer: bool,
    group: Group,
}
#[derive(Clone, Debug)]
pub struct NavGroup {
    pub label: String,
    /// Template navigation icon name under `/static/img/home/nav-{icon}.webp`.
    pub icon: &'static str,
    pub links: Vec<NavLink>,
}
impl NavGroup {
    /// True when the current page belongs to one of this group's links.
    pub fn current(&self) -> bool {
        self.links.iter().any(|link| link.current)
    }
}
/// Request extension and template field. Default renders a usable anonymous shell.
#[derive(Clone, Debug)]
pub struct Chrome {
    pub lang: String,
    pub alternate_url: String,
    pub alternate_lang: String,
    pub entries: Vec<NavLink>,
    pub groups: Vec<NavGroup>,
    pub current_section: String,
    pub home_current: bool,
    pub account: Account,
    pub login_mounted: bool,
    pub registration_mounted: bool,
    pub account_mounted: bool,
}
impl Default for Chrome {
    fn default() -> Self {
        Registry::default().chrome("pl", "/pl/", Account::Anonymous)
    }
}
impl Chrome {
    pub fn text(&self, key: &str) -> String {
        text(&self.lang, key)
    }
    pub fn signed_in(&self) -> bool {
        matches!(self.account, Account::SignedIn { .. })
    }
    pub fn csrf(&self) -> &str {
        match &self.account {
            Account::Anonymous => "",
            Account::SignedIn { csrf } => csrf,
        }
    }
    pub fn home_url(&self) -> String {
        format!("/{}/", self.lang)
    }
    /// The mounted navigation link of one section, if that module is enabled.
    pub fn link(&self, section: &str) -> Option<&NavLink> {
        self.entries.iter().find(|link| link.section == section)
    }
    /// The first mounted link among comma-separated `sections`, in order.
    pub fn first_link(&self, sections: &str) -> Option<&NavLink> {
        sections.split(',').find_map(|section| self.link(section))
    }
}

/// Live server status for the home page; built only from fresh read models.
#[derive(Clone, Debug, Default)]
pub struct HomeStatus {
    /// Players on fresh worlds; `None` when no world has fresh evidence.
    pub online: Option<i64>,
    pub worlds_fresh: usize,
    pub worlds_total: usize,
    /// PvP type shared by every world, or `None` when worlds differ or none is set.
    pub pvp: Option<String>,
    /// UTC time of the newest accepted observation.
    pub as_of: Option<String>,
}
/// One published news entry in the current language.
#[derive(Clone, Debug)]
pub struct HomeNews {
    pub url: String,
    pub title: String,
    /// `YYYY-MM-DD` for `datetime`, and the localized display date.
    pub date: String,
    pub display_date: String,
    pub excerpt: String,
}
/// One visible character from the accepted level ranking.
#[derive(Clone, Debug)]
pub struct HomeRank {
    pub rank: String,
    pub name: String,
    pub level: String,
    pub url: Option<String>,
}
/// Read-model data for the home page. `None` means the source is unavailable
/// and the page says so; it never shows invented values.
#[derive(Clone, Debug, Default)]
pub struct HomeData {
    pub status: Option<HomeStatus>,
    pub news: Option<Vec<HomeNews>>,
    pub ranking: Option<Vec<HomeRank>>,
    pub ranking_as_of: Option<String>,
}

#[derive(Template)]
#[template(path = "pages/home.html")]
struct Home<'a> {
    chrome: &'a Chrome,
    data: &'a HomeData,
}
impl<'a> Home<'a> {
    fn new(chrome: &'a Chrome, data: &'a HomeData) -> Self {
        Self { chrome, data }
    }
    /// Cycles the three template illustrations over the news entries.
    fn news_art(&self, index: &usize) -> usize {
        index % 3 + 1
    }
}
/// Render the home page from enabled sections and the given read-model data.
pub fn home(chrome: &Chrome, data: &HomeData) -> Response {
    render(&Home::new(chrome, data), StatusCode::OK)
}
#[derive(Template)]
#[template(path = "components/shell-header.html")]
struct ShellHeader<'a> {
    chrome: &'a Chrome,
}
#[derive(Template)]
#[template(path = "components/shell-footer.html")]
struct ShellFooter<'a> {
    chrome: &'a Chrome,
}
/// Marker present in every page that already carries the shared shell.
pub const SHELL_MARKER: &str = "op-theme op-shell";
/// Shared header and footer markup for module pages that render their own document.
pub fn shell_parts(chrome: &Chrome) -> Option<(String, String)> {
    let header = ShellHeader { chrome }.render().ok()?;
    let footer = ShellFooter { chrome }.render().ok()?;
    Some((header, footer))
}
#[derive(Template)]
#[template(path = "pages/error.html")]
struct ErrorPage<'a> {
    chrome: &'a Chrome,
    title: String,
    description: String,
    status: u16,
}
/// Plain, translated refusal detail supplied by a module's existing decision.
#[derive(Clone, Debug)]
pub struct RefusalMessage(pub String);

/// Shared localized refusal renderer. The caller retains protocol/security headers.
pub fn error(chrome: &Chrome, status: StatusCode) -> Response {
    error_with_message(
        chrome,
        status,
        &chrome.text(&format!("error-{}-description", status.as_u16())),
    )
}
/// Render a domain-specific translated message with the shared status title and chrome.
/// The message remains HTML-escaped, like every other dynamic template value.
pub fn error_with_message(chrome: &Chrome, status: StatusCode, message: &str) -> Response {
    render(
        &ErrorPage {
            chrome,
            status: status.as_u16(),
            title: chrome.text(&format!("error-{}-title", status.as_u16())),
            description: message.to_owned(),
        },
        status,
    )
}

fn render(template: &impl Template, status: StatusCode) -> Response {
    match template.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn registry() -> Registry {
        let mut registry = Registry {
            login: true,
            registration: true,
            account: true,
            ..Registry::default()
        };
        registry.section(true, "wiki", "shell-wiki", "wiki", Group::World, true);
        registry.section(
            false,
            "puzzles",
            "shell-puzzles",
            "puzzles",
            Group::Play,
            false,
        );
        registry
    }
    #[test]
    fn enabled_navigation_and_path_switch() {
        let chrome = registry().chrome(
            "en",
            "/en/wiki/articles/example?q=private",
            Account::Anonymous,
        );
        assert_eq!(chrome.entries.len(), 1);
        assert_eq!(chrome.current_section, "wiki");
        assert_eq!(chrome.alternate_url, "/pl/wiki/articles/example");
        for (path, expected) in [
            ("/pl/", "/en/"),
            ("/pl", "/en/"),
            ("/pl/news?token=secret", "/en/news"),
            ("/news?foo=bar", "/en/news"),
        ] {
            assert_eq!(
                registry()
                    .chrome("pl", path, Account::Anonymous)
                    .alternate_url,
                expected
            );
        }
    }
    #[test]
    fn only_the_most_specific_navigation_section_is_current() {
        let mut registry = registry();
        registry.section(
            true,
            "calculators",
            "shell-calculators",
            "wiki/calculators",
            Group::World,
            false,
        );
        let chrome = registry.chrome("en", "/en/wiki/calculators", Account::Anonymous);
        let active: Vec<_> = chrome
            .entries
            .iter()
            .filter(|entry| entry.current)
            .map(|entry| entry.section.as_str())
            .collect();
        assert_eq!(active, ["calculators"]);
    }
    #[test]
    fn home_and_account_states_in_both_languages() -> Result<(), askama::Error> {
        for lang in ["pl", "en"] {
            for enabled in [false, true] {
                let registry = if enabled {
                    registry()
                } else {
                    Registry::default()
                };
                let chrome = registry.chrome(lang, &format!("/{lang}/"), Account::Anonymous);
                let html = Home::new(&chrome, &HomeData::default()).render()?;
                assert!(html.contains(&format!("<html lang=\"{lang}\" data-theme=\"dark\">")));
                assert!(html.contains(if lang == "en" {
                    "Welcome to the world of Oteryn!"
                } else {
                    "Witaj w świecie Oteryn!"
                }));
                assert!(html.contains("/static/css/home.css"));
                assert!(html.contains("class=\"op-theme op-shell\""));
                assert_eq!(html.contains("/wiki\""), enabled);
                assert!(!html.contains("/puzzles\""));
                assert_eq!(html.contains("/login\""), enabled);
                assert_eq!(html.contains("/register\""), enabled);
                assert!(!html.contains("/logout\""));
                assert!(!html.contains("style="));
                assert!(!html.contains("<script"));
            }
            let chrome = registry().chrome(
                lang,
                "/",
                Account::SignedIn {
                    csrf: "escaped<&token".into(),
                },
            );
            let html = Home::new(&chrome, &HomeData::default()).render()?;
            assert!(html.contains(&format!("action=\"/{lang}/logout\" method=\"post\"")));
            assert!(html.contains("name=\"_csrf\" value=\"escaped&#60;&#38;token\""));
            assert!(html.contains("href=\"/account\""));
            assert!(!html.contains("/login\""));
            assert!(!html.contains("/register\""));
        }
        Ok(())
    }
    #[test]
    fn navigation_has_native_keyboard_and_touch_disclosure() -> Result<(), askama::Error> {
        let chrome = registry().chrome("en", "/en/wiki", Account::Anonymous);
        let html = ShellHeader { chrome: &chrome }.render()?;
        assert!(html.contains("<details class=\"hx-nav__dropdown\"><summary"));
        assert!(html.contains("aria-current=\"page\">Wiki</a>"));
        let css = include_str!("../../../static/css/home.css");
        assert!(css.contains(".hx-nav__dropdown[open] .hx-nav__menu"));
        assert!(css.contains(".hx-nav__item--menu:focus-within .hx-nav__menu { display: block; }"));
        assert!(!css.contains(":focus-within .hx-nav__menu { display: none; }"));
        Ok(())
    }
    #[test]
    fn home_shows_live_data_or_says_unavailable() -> Result<(), askama::Error> {
        let chrome = registry().chrome("pl", "/pl/", Account::Anonymous);
        let html = Home::new(&chrome, &HomeData::default()).render()?;
        for key in [
            "stage-status-unavailable",
            "stage-news-unavailable",
            "stage-ranking-unavailable",
        ] {
            assert!(html.contains(&chrome.text(key)), "{key}");
        }
        let data = HomeData {
            status: Some(HomeStatus {
                online: Some(58),
                worlds_fresh: 2,
                worlds_total: 3,
                pvp: Some("Open PvP".into()),
                as_of: Some("2026-11-08 00:46:04".into()),
            }),
            news: Some(vec![HomeNews {
                url: "/pl/news/test-1".into(),
                title: "Nowa <kraina>".into(),
                date: "2026-11-07".into(),
                display_date: "07.11.2026".into(),
                excerpt: "Opis & zmiany".into(),
            }]),
            ranking: Some(vec![HomeRank {
                rank: "1".into(),
                name: "Valendor".into(),
                level: "352".into(),
                url: Some("/pl/characters/Valendor".into()),
            }]),
            ranking_as_of: Some("2026-11-08 00:40".into()),
        };
        let html = Home::new(&chrome, &data).render()?;
        assert!(html.contains("hx-status__dot--on"));
        assert!(html.contains(">58<"));
        assert!(html.contains("2 / 3"));
        assert!(html.contains("Nowa &#60;kraina&#62;"));
        assert!(html.contains("Opis &#38; zmiany"));
        assert!(html.contains("datetime=\"2026-11-07\""));
        assert!(html.contains("href=\"/pl/characters/Valendor\""));
        assert!(html.contains("medal-1.webp"));
        assert!(!html.contains(&chrome.text("stage-ranking-unavailable")));
        assert!(!html.contains("style="));
        assert!(!html.contains("<script"));
        let empty = HomeData {
            news: Some(vec![]),
            ranking: Some(vec![]),
            ..HomeData::default()
        };
        let html = Home::new(&chrome, &empty).render()?;
        assert!(html.contains(&chrome.text("stage-news-empty")));
        assert!(html.contains(&chrome.text("stage-ranking-empty")));
        Ok(())
    }
    #[test]
    fn shared_refusals_keep_status_and_layout() -> Result<(), askama::Error> {
        for lang in ["pl", "en"] {
            let chrome = registry().chrome(lang, "/missing", Account::Anonymous);
            for status in [
                StatusCode::NOT_FOUND,
                StatusCode::FORBIDDEN,
                StatusCode::UNPROCESSABLE_ENTITY,
                StatusCode::TOO_MANY_REQUESTS,
                StatusCode::INTERNAL_SERVER_ERROR,
                StatusCode::SERVICE_UNAVAILABLE,
            ] {
                assert_eq!(error(&chrome, status).status(), status);
                let title = chrome.text(&format!("error-{}-title", status.as_u16()));
                assert!(!title.is_empty());
                let html = ErrorPage {
                    chrome: &chrome,
                    title: title.clone(),
                    description: String::new(),
                    status: status.as_u16(),
                }
                .render()?;
                assert!(html.contains("<body class=\"op-theme op-shell\">"));
                assert!(html.contains(&title));
                assert!(html.contains("href=\"#main\""));
            }
        }
        Ok(())
    }
}
