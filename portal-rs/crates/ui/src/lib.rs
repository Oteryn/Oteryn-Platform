//! Shared portal presentation. No module, database or session dependencies.
use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use std::sync::LazyLock;

// A shell renders many labels and can compose chrome for every resource request.
static EN: LazyLock<Option<FluentBundle<FluentResource>>> =
    LazyLock::new(|| catalogue(unic_langid::langid!("en"), include_str!("../i18n/en.ftl")));
static PL: LazyLock<Option<FluentBundle<FluentResource>>> =
    LazyLock::new(|| catalogue(unic_langid::langid!("pl"), include_str!("../i18n/pl.ftl")));

fn catalogue(
    locale: unic_langid::LanguageIdentifier,
    source: &str,
) -> Option<FluentBundle<FluentResource>> {
    let resource = FluentResource::try_new(source.to_owned()).ok()?;
    let mut bundle = FluentBundle::new_concurrent(vec![locale]);
    bundle.add_resource(resource).ok()?;
    Some(bundle)
}

/// Translate through the portal's existing Fluent catalogues.
pub fn text(lang: &str, key: &str) -> String {
    let catalogue = if lang == "en" { &EN } else { &PL };
    if let Some(bundle) = catalogue.as_ref()
        && let Some(pattern) = bundle.get_message(key).and_then(|message| message.value())
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
                let section_path = e.path.trim_start_matches('/');
                let current = suffix == section_path
                    || suffix
                        .strip_prefix(section_path)
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
            alternate_url: if let Some(rest) = clean
                .strip_prefix("/admin/pl/")
                .or_else(|| clean.strip_prefix("/admin/en/"))
            {
                format!("/admin/{other}/{rest}")
            } else if clean.starts_with("/pl/")
                || clean.starts_with("/en/")
                || matches!(clean, "/" | "/pl" | "/en")
            {
                format!("/{other}/{suffix}")
            } else {
                // Unprefixed endpoints do not all have a translated route alias.
                format!("/language/{other}")
            },
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
    fn concurrent_shell_catalogues_keep_locales_and_fallback_separate() {
        std::thread::scope(|scope| {
            let english = scope.spawn(|| text("en", "error-403-title"));
            let polish = scope.spawn(|| text("pl", "error-403-title"));
            assert_eq!(english.join().unwrap(), "Access denied");
            assert_eq!(polish.join().unwrap(), "Brak dostępu");
        });
        assert_eq!(text("unknown", "error-403-title"), "Brak dostępu");
        assert_eq!(text("en", "missing-translation-test"), "");
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
            ("/news?foo=bar", "/language/en"),
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
    fn language_switch_uses_mounted_staff_route_shapes() {
        for (path, expected) in [
            ("/atlas/spawns", "/language/en"),
            ("/account", "/language/en"),
            ("/oauth/authorize", "/language/en"),
            ("/admin", "/language/en"),
            ("/admin/news", "/language/en"),
            ("/console/dashboard", "/language/en"),
            ("/admin/pl/bazaar", "/admin/en/bazaar"),
            ("/pl/admin/support/tickets", "/en/admin/support/tickets"),
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
    fn unprefixed_aliases_and_localized_paths_share_the_active_section() {
        for lang in ["pl", "en"] {
            for path in ["/wiki", "/wiki/articles/guide", "/pl/wiki", "/en/wiki"] {
                let chrome = registry().chrome(lang, path, Account::Anonymous);
                assert_eq!(chrome.current_section, "wiki");
                assert_eq!(
                    chrome.entries.iter().filter(|entry| entry.current).count(),
                    1
                );
            }
            assert!(
                registry()
                    .chrome(lang, "/wiki-other", Account::Anonymous)
                    .current_section
                    .is_empty()
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
    fn shell_uses_compressed_web_fonts() {
        let static_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../static/thais");
        let css = std::fs::read_to_string(static_dir.join("shell.css")).expect("shell stylesheet");
        let fonts: Vec<_> = css
            .lines()
            .filter_map(|line| line.trim().strip_prefix("src: url(\""))
            .filter_map(|line| line.split_once('"').map(|(file, _)| file))
            .collect();
        assert_eq!(fonts.len(), 3);
        let total: usize = fonts
            .into_iter()
            .map(|file| {
                let bytes = std::fs::read(static_dir.join(file)).expect("shipped shell font");
                assert!(bytes.starts_with(b"wOF2"), "{file} uses WOFF2");
                bytes.len()
            })
            .sum();
        assert!(total < 566_128, "shell fonts use {total} bytes");
    }

    #[test]
    fn homepage_smooth_scroll_leaves_keyboard_focus_immediate() {
        let css = include_str!("../../../static/thais/home.css");
        assert_eq!(css.matches("scroll-behavior: smooth").count(), 1);
        assert!(css.contains("html:not(:has(:focus-visible)) { scroll-behavior: smooth; }"));
    }

    #[test]
    fn home_navigation_fallback_reserves_the_enhanced_bar_height() {
        let css = include_str!("../../../static/thais/home.css");
        assert!(css.contains(".om-mast { margin-bottom: calc(-1 * var(--om-bar-height)); }"));
        assert!(css.contains(concat!(
            ".om-mast:not(.is-enhanced):not(:has(.om-fallback-menu)) {\n",
            "  height: var(--om-bar-height);\n"
        )));
    }

    #[test]
    fn shell_backgrounds_use_smaller_gate_images() {
        let static_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../static/thais");
        let full_size = std::fs::metadata(static_dir.join("brama.webp"))
            .unwrap()
            .len();
        for stylesheet in ["interior.css", "shop.css"] {
            let css = std::fs::read_to_string(static_dir.join(stylesheet)).unwrap();
            for source in ["brama-8c66cbecc.webp", "brama-1692ad85.webp"] {
                assert!(css.contains(&format!("url(\"{source}\")")));
                assert!(std::fs::metadata(static_dir.join(source)).unwrap().len() < full_size);
            }
        }
    }

    #[test]
    fn home_prioritizes_panorama_over_below_fold_art() -> Result<(), askama::Error> {
        let chrome = Registry::default().chrome("en", "/en/", Account::Anonymous);
        let html = Home::new(&chrome, &HomeData::default()).render()?;
        let deferred = html.matches(r#"loading="lazy""#).count();
        assert!(deferred > 0);
        assert_eq!(
            html.matches(r#"loading="lazy" fetchpriority="low" decoding="async""#)
                .count(),
            deferred
        );
        assert_eq!(html.matches(r#"fetchpriority="high""#).count(), 1);
        Ok(())
    }

    #[test]
    fn home_panorama_offers_viewport_sized_sources() -> Result<(), askama::Error> {
        let chrome = Registry::default().chrome("en", "/en/", Account::Anonymous);
        let html = Home::new(&chrome, &HomeData::default()).render()?;
        for (name, width, original_size) in [
            ("brama-panorama-aee60a5b.webp", 1536, 718_188),
            ("brama-1692ad85.webp", 824, 628_654),
        ] {
            assert!(html.contains(&format!("/static/thais/{name} {width}w")));
            let asset = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../static/thais")
                .join(name);
            assert!(
                std::fs::metadata(asset)
                    .expect("shipped panorama source")
                    .len()
                    < original_size
            );
        }
        Ok(())
    }

    #[test]
    fn desktop_home_panorama_fits_the_request_budget() -> Result<(), askama::Error> {
        const PANORAMA: &str = "brama-panorama-f4db61b9.webp";
        let chrome = Registry::default().chrome("en", "/en/", Account::Anonymous);
        let html = Home::new(&chrome, &HomeData::default()).render()?;
        assert!(html.contains(&format!("/static/thais/{PANORAMA} 2868w")));
        let asset = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../static/thais")
            .join(PANORAMA);
        let size = std::fs::metadata(asset)
            .expect("shipped desktop panorama")
            .len();
        assert!(size < 1_000_000, "desktop panorama is {size} bytes");
        Ok(())
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
                assert!(html.contains(&format!(
                    "<h1 id=\"hero-title\">{}</h1>",
                    chrome.text("shell-news")
                )));
                assert!(html.contains("/static/thais/home.css"));
                assert!(html.contains("class=\"op-theme op-shell\""));
                assert_eq!(html.contains("/wiki\""), enabled);
                assert!(!html.contains("/puzzles\""));
                assert_eq!(html.contains("/login\""), enabled);
                assert_eq!(html.contains("/register\""), enabled);
                assert!(!html.contains("/logout\""));
                assert!(!html.contains("style="));
                assert_eq!(html.matches("<script").count(), 1);
                assert!(html.contains("src=\"/static/thais/home.js\""));
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
        assert!(html.contains("om-fallback-menu"));
        assert!(html.contains("<summary"));
        assert!(html.contains("id=\"portal-navigation\""));
        assert!(html.contains("aria-current=\"page\">Wiki</a>"));
        assert!(html.contains("href=\"/en/#profesje\""));
        assert!(!html.contains("href=\"#profesje\""));
        Ok(())
    }
    #[test]
    fn home_distinguishes_empty_and_unavailable_news_without_extra_hero_actions()
    -> Result<(), askama::Error> {
        for lang in ["pl", "en"] {
            let mut registry = registry();
            registry.section(true, "news", "shell-news", "news", Group::Community, false);
            let chrome = registry.chrome(lang, &format!("/{lang}/"), Account::Anonymous);
            for (news, status, other_status) in [
                (None, "stage-news-unavailable", "stage-news-empty"),
                (
                    Some(Vec::new()),
                    "stage-news-empty",
                    "stage-news-unavailable",
                ),
            ] {
                let data = HomeData {
                    news,
                    ..Default::default()
                };
                let html = Home::new(&chrome, &data).render()?;
                assert!(html.contains(&format!(
                    "<h1 id=\"hero-title\">{}</h1>",
                    chrome.text("shell-news")
                )));
                assert!(html.contains(&chrome.text(status)));
                assert!(!html.contains(&chrome.text(other_status)));
                assert!(!html.contains("class=\"hero-story front-short\""));
                assert!(html.contains(&format!("class=\"hero-news-all\" href=\"/{lang}/news\"")));
                assert!(!html.contains("class=\"cta\""));
                assert!(!html.contains("class=\"hero-today\""));
                assert!(html.contains(&chrome.text("thais-home-no-data")));
                assert!(html.contains("<strong aria-hidden=\"true\">—</strong>"));
            }
        }
        Ok(())
    }
    #[test]
    fn home_renders_three_news_links_and_observed_world_counts() -> Result<(), askama::Error> {
        let chrome = registry().chrome("pl", "/pl/", Account::Anonymous);
        let news = [
            "Nowa <kraina>",
            "Nowe zadanie",
            "Turniej",
            "Starsze wydarzenie",
        ]
        .into_iter()
        .enumerate()
        .map(|(i, title)| HomeNews {
            url: format!("/pl/news/test-{i}"),
            title: title.into(),
            date: "2026-11-07".into(),
            display_date: "07.11.2026".into(),
            excerpt: "Opis & zmiany".into(),
        })
        .collect();
        let data = HomeData {
            status: Some(HomeStatus {
                online: Some(58),
                worlds_fresh: 2,
                worlds_total: 3,
                pvp: Some("Open PvP".into()),
                as_of: Some("2026-11-08 00:46:04".into()),
            }),
            news: Some(news),
            ranking: Some(vec![HomeRank {
                rank: "1".into(),
                name: "Valendor".into(),
                level: "352".into(),
                url: Some("/pl/characters/Valendor".into()),
            }]),
            ranking_as_of: None,
        };
        let html = Home::new(&chrome, &data).render()?;
        let Some((hero, _)) = html.split_once("</section>") else {
            return Err(askama::Error::Custom("homepage hero missing".into()));
        };
        let stories: Vec<_> = hero
            .split("<details class=\"hero-story front-short\">")
            .skip(1)
            .collect();
        assert_eq!(stories.len(), 3);
        for (i, (story, title)) in stories
            .iter()
            .zip(["Nowa &#60;kraina&#62;", "Nowe zadanie", "Turniej"])
            .enumerate()
        {
            let Some((summary, content)) = story.split_once("</summary>") else {
                return Err(askama::Error::Custom("missing story summary".into()));
            };
            assert!(summary.contains(title));
            assert!(summary.contains("datetime=\"2026-11-07\""));
            assert!(summary.contains("class=\"hero-story-toggle\" aria-hidden=\"true\""));
            assert!(content.contains("Opis &#38; zmiany"));
            assert!(content.contains(&format!(
                "href=\"/pl/news/test-{i}\">{0}",
                chrome.text("stage-read-more")
            )));
        }
        assert!(!html.contains("/pl/news/test-3"));
        assert!(html.contains(">58<"));
        assert!(html.contains("2 / 3"));
        assert!(html.contains(&chrome.text("thais-home-count-scope")));
        assert!(!html.contains("/pl/characters/Valendor"));
        assert!(!html.contains("world-info-title"));
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
