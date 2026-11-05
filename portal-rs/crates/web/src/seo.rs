//! Explicit-origin SEO documents. Request Host headers never influence absolute URLs.
use axum::{Router, extract::State, http::header, routing::get};

#[derive(Clone, Debug)]
pub struct Seo {
    origin: String,
    pub production: bool,
    pub allow_indexing: bool,
    entries: Vec<Entry>,
}
#[derive(Clone, Debug)]
pub struct Entry {
    pub pl: String,
    pub en: String,
    pub lastmod: String,
}
fn path(value: &str) -> bool {
    value.starts_with('/')
        && !value.starts_with("//")
        && !value.contains(['\\', '?', '#', '"', '<', '>', '&', '%'])
        && !value.chars().any(char::is_whitespace)
        && !value.split('/').any(|s| s == "." || s == "..")
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
impl Seo {
    pub fn new(origin: &str, production: bool) -> Result<Self, &'static str> {
        let uri = origin
            .parse::<axum::http::Uri>()
            .map_err(|_| "invalid origin")?;
        if !matches!(uri.scheme_str(), Some("https" | "http"))
            || uri.authority().is_none()
            || uri.authority().is_some_and(|a| a.as_str().contains('@'))
            || uri.path() != "/"
            || uri.query().is_some()
        {
            return Err("expected an HTTP origin without credentials, path or query");
        }
        Ok(Self {
            origin: origin.trim_end_matches('/').into(),
            production,
            allow_indexing: true,
            entries: Vec::new(),
        })
    }
    /// Modules register only published, enabled public pages, using real modification dates.
    pub fn register(&mut self, entry: Entry) -> Result<(), &'static str> {
        let date = entry.lastmod.as_bytes();
        if !path(&entry.pl)
            || !path(&entry.en)
            || date.len() != 10
            || date[4] != b'-'
            || date[7] != b'-'
            || !date
                .iter()
                .enumerate()
                .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
            || !("01"..="12").contains(&&entry.lastmod[5..7])
            || !("01"..="31").contains(&&entry.lastmod[8..10])
        {
            return Err("invalid sitemap entry");
        }
        if self
            .entries
            .iter()
            .any(|e| e.pl == entry.pl || e.en == entry.en)
        {
            return Err("duplicate sitemap entry");
        }
        self.entries.push(entry);
        Ok(())
    }
    pub fn canonical(&self, page: &str) -> Result<String, &'static str> {
        if !path(page) {
            return Err("invalid canonical path");
        }
        Ok(format!(
            "<link rel=\"canonical\" href=\"{}{}\">",
            escape(&self.origin),
            escape(page)
        ))
    }
    pub fn hreflang(&self, pl: &str, en: &str) -> Result<String, &'static str> {
        if !path(pl) || !path(en) {
            return Err("invalid alternate path");
        }
        Ok([("pl", pl), ("en", en)]
            .into_iter()
            .map(|(lang, p)| {
                format!(
                    "<link rel=\"alternate\" hreflang=\"{lang}\" href=\"{}{}\">",
                    escape(&self.origin),
                    escape(p)
                )
            })
            .collect())
    }
    pub fn robots(&self) -> String {
        if !self.production || !self.allow_indexing {
            return "User-agent: *\nDisallow: /\n".into();
        }
        format!(
            "User-agent: *\nAllow: /\nSitemap: {}/sitemap.xml\n",
            self.origin
        )
    }
    pub fn sitemap(&self) -> String {
        let mut xml = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">",
        );
        for entry in &self.entries {
            for page in [&entry.pl, &entry.en] {
                xml.push_str(&format!(
                    "<url><loc>{}{}</loc><lastmod>{}</lastmod>",
                    escape(&self.origin),
                    escape(page),
                    entry.lastmod
                ));
                for (lang, p) in [("pl", &entry.pl), ("en", &entry.en)] {
                    xml.push_str(&format!(
                        "<xhtml:link rel=\"alternate\" hreflang=\"{lang}\" href=\"{}{}\"/>",
                        escape(&self.origin),
                        escape(p)
                    ));
                }
                xml.push_str("</url>");
            }
        }
        xml.push_str("</urlset>");
        xml
    }
}
/// Merge into the application before applying its common response policy.
pub fn router(config: Seo) -> Router {
    Router::new()
        .route(
            "/robots.txt",
            get(async |State(s): State<Seo>| {
                (
                    [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                    s.robots(),
                )
            }),
        )
        .route(
            "/sitemap.xml",
            get(async |State(s): State<Seo>| {
                (
                    [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
                    s.sitemap(),
                )
            }),
        )
        .with_state(config)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn documents_and_bad_input() -> Result<(), &'static str> {
        let mut seo = Seo::new("https://portal.example", false)?;
        assert!(seo.robots().contains("Disallow: /"));
        seo.production = true;
        assert!(seo.robots().contains("Allow: /"));
        seo.allow_indexing = false;
        assert!(seo.robots().contains("Disallow: /"));
        seo.register(Entry {
            pl: "/pl/news".into(),
            en: "/en/news".into(),
            lastmod: "2026-11-05".into(),
        })?;
        let xml = seo.sitemap();
        assert!(xml.contains("hreflang=\"pl\""));
        assert!(xml.contains("hreflang=\"en\""));
        assert!(xml.ends_with("</urlset>"));
        assert_eq!(xml.matches("<url>").count(), 2);
        assert_eq!(xml.matches("<xhtml:link ").count(), 4);
        assert!(
            seo.register(Entry {
                pl: "//evil.example".into(),
                en: "/en/news".into(),
                lastmod: "bad".into()
            })
            .is_err()
        );
        assert!(
            seo.canonical("/pl/news")?
                .contains("https://portal.example/pl/news")
        );
        assert!(
            seo.hreflang("/pl/news", "/en/news")?
                .contains("hreflang=\"en\"")
        );

        assert!(seo.canonical("//evil.example").is_err());
        assert!(seo.hreflang("/pl", "\"bad").is_err());
        assert!(Seo::new("https://portal.example/path", true).is_err());
        Ok(())
    }
}
