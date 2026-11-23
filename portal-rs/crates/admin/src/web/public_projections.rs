use super::{language, text};
use askama::Template;
use axum::{
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};

#[derive(Template)]
#[template(path = "admin/public_projections.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    restricted: bool,
    projections: bool,
    deaths_ingest: bool,
    deaths_view: bool,
    world_kills_ingest: bool,
    world_stats_view: bool,
}

impl Page {
    fn read(lang: &'static str, lookup: impl Fn(&str) -> Option<String>) -> Self {
        let on = |key| lookup(key).as_deref() == Some("true");
        let restricted = oteryn_core::restricted_environment(&lookup);
        let projections = restricted && on("PORTAL_PUBLIC_PROJECTIONS_ENABLED");
        Self {
            lang,
            restricted,
            projections,
            deaths_ingest: projections && on("PORTAL_DEATHS_V2_INGEST_ENABLED"),
            deaths_view: projections && on("PORTAL_DEATHS_ENABLED"),
            world_kills_ingest: projections && on("PORTAL_STATS_WORLD_TOTALS_ENABLED"),
            world_stats_view: on("PORTAL_WORLD_STATS_ENABLED"),
        }
    }

    fn tr(&self, key: &str) -> String {
        text(self.lang, key)
    }

    fn switch(&self, enabled: &bool) -> String {
        self.tr(if *enabled {
            "public-projections-on"
        } else {
            "public-projections-off"
        })
    }
}

pub(super) async fn page(headers: HeaderMap) -> Response {
    (
        [(header::CACHE_CONTROL, "private, no-store")],
        oteryn_web::render(&Page::read(language(&headers), |key| {
            std::env::var(key).ok()
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_status_displays_the_two_qualification_states() -> Result<(), askama::Error> {
        let all_switches = |key: &str| {
            Some(
                if key.ends_with("_ENV") {
                    "testing"
                } else {
                    "true"
                }
                .into(),
            )
        };
        for lang in ["en", "pl"] {
            let release = Page::read(lang, all_switches);
            assert!(release.deaths_ingest && release.deaths_view && release.world_kills_ingest);
            let html = release.render()?;
            assert!(html.contains(&release.tr("public-projections-deaths-hold")));
            assert!(html.contains(&release.tr("public-projections-kills-hold")));
            assert!(!html.contains("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY"));
        }
        let disabled = Page::read("en", |_| None);
        assert!(!disabled.deaths_ingest && !disabled.deaths_view && !disabled.world_kills_ingest);
        Ok(())
    }
}
