use crate::{
    markdown,
    model::{Board, Error, Post, Thread, Topic},
};
use askama::Template;
use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use oteryn_ui::Chrome;
use std::sync::LazyLock;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

type Bundle = FluentBundle<FluentResource>;

fn bundle(lang: &str, data: &str) -> Option<Bundle> {
    let mut bundle = Bundle::new_concurrent(vec![lang.parse().ok()?]);
    bundle
        .add_resource(FluentResource::try_new(data.into()).ok()?)
        .ok()?;
    Some(bundle)
}

pub fn text(lang: &str, key: &str) -> String {
    static PL: LazyLock<Option<Bundle>> =
        LazyLock::new(|| bundle("pl", include_str!("../../../templates/forum/pl.ftl")));
    static EN: LazyLock<Option<Bundle>> =
        LazyLock::new(|| bundle("en", include_str!("../../../templates/forum/en.ftl")));
    (if lang == "pl" { &PL } else { &EN })
        .as_ref()
        .and_then(|b| {
            let pattern = b.get_message(key)?.value()?;
            Some(b.format_pattern(pattern, None, &mut vec![]).into_owned())
        })
        .unwrap_or_default()
}

#[derive(Debug, PartialEq, Eq)]
pub enum Screen {
    Index,
    Board,
    Thread,
    Preferences,
    Profile,
    Failure,
}

#[derive(Template)]
#[template(path = "forum/page.html", config = "src/askama.toml")]
pub struct Page {
    pub chrome: Chrome,
    pub lang: String,
    pub staff: bool,
    pub base: String,
    pub screen: Screen,
    pub title: String,
    pub csrf: String,
    pub signed_in: bool,
    pub can_post: bool,
    pub post_notice: &'static str,
    pub handle: Option<String>,
    pub boards: Vec<Board>,
    pub board: Option<Board>,
    pub topics: Vec<Topic>,
    pub thread: Option<Thread>,
    pub posts: Vec<PostView>,
    pub profile_posts: Vec<(Uuid, String)>,
    pub page: u32,
    pub next: bool,
    pub quote: Option<(Uuid, String)>,
    pub seen: Option<Uuid>,
    pub subscribed: bool,
    pub email: bool,
    pub mail_available: bool,
    pub reported: bool,
    pub categories: &'static [&'static str],
    pub privileges: Vec<oteryn_admin::rbac::Permission>,
}

impl Page {
    pub fn new(chrome: Chrome, lang: String, staff: bool, screen: Screen) -> Self {
        let base = if staff {
            "/admin/forum".into()
        } else {
            format!("/{lang}/forum")
        };
        Self {
            chrome,
            lang,
            staff,
            base,
            screen,
            title: String::new(),
            csrf: String::new(),
            signed_in: false,
            can_post: false,
            post_notice: "",
            handle: None,
            boards: vec![],
            board: None,
            topics: vec![],
            thread: None,
            posts: vec![],
            profile_posts: vec![],
            page: 1,
            next: false,
            quote: None,
            seen: None,
            subscribed: false,
            email: false,
            mail_available: false,
            reported: false,
            categories: oteryn_support::reports::categories("content"),
            privileges: vec![],
        }
    }

    pub fn tr(&self, key: &str) -> String {
        text(&self.lang, key)
    }

    pub fn thread_command(&self, key: &str, thread: &Thread) -> String {
        self.tr(match key {
            "lock" if thread.locked => "unlock",
            "pin" if thread.pinned => "unpin",
            other => other,
        })
    }

    pub fn when(&self, date: &OffsetDateTime) -> String {
        format!(
            "{} {:02}:{:02} UTC",
            date.date(),
            date.hour(),
            date.minute()
        )
    }

    pub fn iso(&self, date: &OffsetDateTime) -> String {
        date.format(&Rfc3339).unwrap_or_default()
    }

    pub fn query(&self) -> String {
        if self.staff {
            format!("?lang={}", self.lang)
        } else {
            String::new()
        }
    }

    pub fn page_url(&self, page: u32) -> String {
        format!("?lang={}&page={page}", self.lang)
    }

    pub fn permitted(&self, scope: &str) -> bool {
        self.privileges
            .iter()
            .any(|permission| permission.as_str() == scope)
    }

    pub fn render(&self) -> Response {
        oteryn_web::render(self)
    }
}

pub struct PostView {
    pub post: Post,
    pub rendered: String,
}

impl From<Post> for PostView {
    fn from(post: Post) -> Self {
        Self {
            rendered: markdown::render(&post.body),
            post,
        }
    }
}

pub fn failure(chrome: &Chrome, lang: &str, staff: bool, error: Error) -> Response {
    let (status, key) = match error {
        Error::SignIn => {
            return axum::response::Redirect::to(&format!("/{lang}/login")).into_response();
        }
        Error::NotFound => (StatusCode::NOT_FOUND, "missing"),
        Error::Invalid => (StatusCode::UNPROCESSABLE_ENTITY, "invalid"),
        Error::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
        Error::Ineligible => (StatusCode::FORBIDDEN, "ineligible"),
        Error::HandleRequired => (StatusCode::FORBIDDEN, "handle-required"),
        Error::HandleTaken => (StatusCode::CONFLICT, "handle-taken"),
        Error::Locked => (StatusCode::CONFLICT, "locked"),
        Error::Conflict => (StatusCode::CONFLICT, "conflict"),
        Error::Limit => (StatusCode::TOO_MANY_REQUESTS, "limit"),
        Error::Storage(_) => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
    };
    let mut page = Page::new(chrome.clone(), lang.into(), staff, Screen::Failure);
    page.title = page.tr(key);
    let response = (status, page.render()).into_response();
    if matches!(error, Error::Limit) {
        ([(header::RETRY_AFTER, "60")], response).into_response()
    } else {
        response
    }
}

#[derive(Template)]
#[template(path = "forum/email.html", config = "src/askama.toml")]
struct Email<'a> {
    lang: &'a str,
    subject: &'a str,
    message: &'a str,
    open: &'a str,
    manage: &'a str,
    thread: &'a str,
    preferences: &'a str,
}

pub fn email(
    lang: &str,
    thread: &str,
    preferences: &str,
) -> Result<oteryn_notify::Rendered, sqlx::Error> {
    let subject = text(lang, "mail-subject");
    let message = text(lang, "mail-message");
    let open = text(lang, "mail-open");
    let manage = text(lang, "mail-manage");
    let html = Email {
        lang,
        subject: &subject,
        message: &message,
        open: &open,
        manage: &manage,
        thread,
        preferences,
    }
    .render()
    .map_err(|_| sqlx::Error::Protocol("forum email render failed".into()))?;
    Ok(oteryn_notify::Rendered {
        subject,
        text: format!("{message}\n\n{open}\n{thread}\n\n{manage}\n{preferences}\n"),
        html,
    })
}
