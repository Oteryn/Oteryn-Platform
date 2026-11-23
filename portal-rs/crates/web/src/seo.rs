//! Explicit-origin SEO documents. Request Host headers never influence absolute URLs.
use axum::{Extension, Router, extract::State, http::header, routing::get};

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
/// A public document can have one or both currently published locales.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct SitemapPage {
    pub pl: Option<String>,
    pub en: Option<String>,
    pub lastmod: String,
}

/// Publication data provided by the editorial module at request time.
#[derive(Clone, Debug)]
pub enum EditorialSitemap {
    Pages {
        rows: Vec<SitemapPage>,
        include_static: bool,
    },
    Index(usize),
}

fn valid_date(value: &str) -> bool {
    let date = value.as_bytes();
    date.len() == 10
        && date[4] == b'-'
        && date[7] == b'-'
        && date
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
        && ("01"..="12").contains(&&value[5..7])
        && ("01"..="31").contains(&&value[8..10])
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
        if !path(&entry.pl) || !path(&entry.en) || !valid_date(&entry.lastmod) {
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
    pub fn absolute(&self, page: &str) -> Result<String, &'static str> {
        if !path(page) {
            return Err("invalid canonical path");
        }
        Ok(format!("{}{page}", self.origin))
    }
    pub fn canonical(&self, page: &str) -> Result<String, &'static str> {
        Ok(format!(
            "<link rel=\"canonical\" href=\"{}\">",
            escape(&self.absolute(page)?)
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
        self.sitemap_with(&[], true)
    }
    fn sitemap_with(&self, editorial: &[SitemapPage], include_static: bool) -> String {
        let mut xml = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">",
        );
        if include_static {
            for entry in &self.entries {
                self.write_pages(&mut xml, Some(&entry.pl), Some(&entry.en), &entry.lastmod);
            }
        }
        for entry in editorial {
            self.write_pages(
                &mut xml,
                entry.pl.as_deref(),
                entry.en.as_deref(),
                &entry.lastmod,
            );
        }
        xml.push_str("</urlset>");
        xml
    }
    fn sitemap_index(&self, pages: usize) -> String {
        let origin = escape(&self.origin);
        let mut xml = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">",
        );
        for page in 1..=pages {
            xml.push_str(&format!(
                "<sitemap><loc>{origin}/sitemap.xml?page={page}</loc></sitemap>"
            ));
        }
        xml.push_str("</sitemapindex>");
        xml
    }
    fn write_pages(&self, xml: &mut String, pl: Option<&str>, en: Option<&str>, lastmod: &str) {
        if !valid_date(lastmod) {
            return;
        }
        let pl = pl.and_then(|page| self.absolute(page).ok());
        let en = en.and_then(|page| self.absolute(page).ok());
        for page in [pl.as_ref(), en.as_ref()].into_iter().flatten() {
            xml.push_str(&format!(
                "<url><loc>{}</loc><lastmod>{lastmod}</lastmod>",
                escape(page)
            ));
            if let (Some(pl), Some(en)) = (&pl, &en) {
                for (lang, alternate) in [("pl", pl), ("en", en)] {
                    xml.push_str(&format!(
                        "<xhtml:link rel=\"alternate\" hreflang=\"{lang}\" href=\"{}\"/>",
                        escape(alternate)
                    ));
                }
            }
            xml.push_str("</url>");
        }
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
            get(
                async |State(s): State<Seo>, dynamic: Option<Extension<EditorialSitemap>>| {
                    (
                        [
                            (header::CONTENT_TYPE, "application/xml; charset=utf-8"),
                            (header::CACHE_CONTROL, "no-cache"),
                        ],
                        match dynamic {
                            Some(Extension(EditorialSitemap::Pages {
                                rows,
                                include_static,
                            })) => s.sitemap_with(&rows, include_static),
                            Some(Extension(EditorialSitemap::Index(pages))) => {
                                s.sitemap_index(pages)
                            }
                            None => s.sitemap(),
                        },
                    )
                },
            ),
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
