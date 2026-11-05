//! Editorial rules, independent of persistence and HTTP.
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid wiki input")]
    Invalid,
    #[error("wiki state conflict")]
    Conflict,
    #[error("wiki article not found")]
    NotFound,
    #[error("wiki access denied")]
    Denied,
    #[error("wiki storage unavailable")]
    Storage(#[from] sqlx::Error),
    #[error("wiki authorization unavailable")]
    Authorization(#[from] oteryn_admin::rbac::Error),
}
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Translation {
    pub locale: String,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub markdown: String,
}
pub fn slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value.split('-').all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}
impl Translation {
    pub fn validate(&self, publish: bool) -> Result<(), Error> {
        let bounded = |s: &str, max| s.chars().count() <= max;
        if !matches!(self.locale.as_str(), "pl" | "en")
            || !slug(&self.slug)
            || self.title.trim().is_empty()
            || !bounded(&self.title, 200)
            || !bounded(&self.summary, 1000)
            || !bounded(&self.markdown, 100_000)
            || (publish && (self.summary.trim().is_empty() || self.markdown.trim().is_empty()))
        {
            return Err(Error::Invalid);
        }
        let restricted =
            regex::Regex::new(r"(?i)<!--|</?[a-z][^>]*>|(?:javascript|data|vbscript)\s*:")
                .map_err(|_| Error::Invalid)?;
        if restricted.is_match(&self.markdown) {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
pub fn translations(values: &[Translation], publish: bool) -> Result<(), Error> {
    if values.is_empty()
        || values.len() > 2
        || (values.len() == 2 && values[0].locale == values[1].locale)
        || (publish && values.len() != 2)
    {
        return Err(Error::Invalid);
    }
    for value in values {
        value.validate(publish)?;
    }
    Ok(())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Submit,
    ReturnDraft,
    Publish,
    Unpublish,
    Archive,
}
impl Action {
    pub fn target(self, current: &str) -> Result<&'static str, Error> {
        match (current, self) {
            ("draft", Self::Submit) => Ok("in_review"),
            ("in_review", Self::ReturnDraft) | ("published", Self::Unpublish) => Ok("draft"),
            ("in_review", Self::Publish) => Ok("published"),
            ("draft" | "in_review" | "published", Self::Archive) => Ok("archived"),
            _ => Err(Error::Conflict),
        }
    }
    pub fn publisher(self) -> bool {
        matches!(self, Self::Publish | Self::Unpublish | Self::Archive)
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Submit => "submit-review",
            Self::ReturnDraft => "return-draft",
            Self::Publish => "publish",
            Self::Unpublish => "unpublish",
            Self::Archive => "archive",
        }
    }
}
/// Sanitization runs on every render, including previously stored/imported source.
pub fn render(source: &str) -> String {
    use pulldown_cmark::{Event, Options, Parser, html};
    let parser = Parser::new_ext(
        source,
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH,
    )
    .filter(|e| !matches!(e, Event::Html(_) | Event::InlineHtml(_)));
    let mut html = String::new();
    html::push_html(&mut html, parser);
    ammonia::Builder::new()
        .tags(
            [
                "p",
                "br",
                "strong",
                "em",
                "del",
                "blockquote",
                "pre",
                "code",
                "ul",
                "ol",
                "li",
                "h2",
                "h3",
                "h4",
                "h5",
                "h6",
                "a",
                "table",
                "thead",
                "tbody",
                "tr",
                "th",
                "td",
                "hr",
            ]
            .into_iter()
            .collect(),
        )
        .url_schemes(["https"].into_iter().collect())
        .link_rel(Some("noopener noreferrer"))
        .attribute_filter(|tag, attribute, value| {
            if tag == "a" && attribute == "href" && !allowed_link(value) {
                None
            } else {
                Some(std::borrow::Cow::Borrowed(value))
            }
        })
        .clean(&html)
        .to_string()
}
pub fn search(query: &str, page: i64) -> Result<String, Error> {
    let query = query.split_whitespace().collect::<Vec<_>>().join(" ");
    if !(1..=1000).contains(&page)
        || (!query.is_empty() && !(2..=80).contains(&query.chars().count()))
    {
        return Err(Error::Invalid);
    }
    Ok(query)
}

// Match the existing portal's root-relative, fragment and HTTPS link boundary.
fn allowed_link(value: &str) -> bool {
    if value.is_empty() || value.starts_with("//") || value.chars().any(|c| c <= ' ' || c == '\\') {
        return false;
    }
    if value.starts_with('/') {
        return true;
    }
    if let Some(fragment) = value.strip_prefix('#') {
        return fragment.starts_with(|c: char| c.is_ascii_alphanumeric())
            && fragment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-'));
    }
    value.starts_with("https://")
        && url::Url::parse(value).is_ok_and(|url| {
            url.scheme() == "https"
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
        })
}
