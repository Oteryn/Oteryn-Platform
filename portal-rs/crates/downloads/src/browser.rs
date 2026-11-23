use askama::Template;
use axum::{
    Extension,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use oteryn_ui::{Account, Chrome, Registry};
use serde::Deserialize;

use crate::{Channel, Downloads, Error, domain::identifier, storage::Release};

const NO_STORE: &str = "no-store";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Selection {
    channel: Option<String>,
}

#[derive(Template)]
#[template(path = "downloads/index.html")]
struct DownloadPage {
    chrome: Chrome,
    channel: Channel,
    release: Option<Release>,
}

#[derive(Template)]
#[template(path = "downloads/start.html")]
struct StartPage {
    chrome: Chrome,
}

pub(crate) async fn download(
    State(downloads): State<Downloads>,
    Path(lang): Path<String>,
    chrome: Option<Extension<Chrome>>,
    query: Result<Query<Selection>, QueryRejection>,
) -> Response {
    let Some(chrome) = page_chrome(chrome, &lang, "download") else {
        return status(StatusCode::NOT_FOUND);
    };
    let Ok(channel) = query
        .map_err(|_| Error::Invalid("channel"))
        .and_then(|Query(selection)| {
            Channel::parse(selection.channel.as_deref().unwrap_or("stable"))
        })
    else {
        return refusal(&chrome, StatusCode::BAD_REQUEST);
    };
    let release = match downloads.active_release(channel).await {
        Ok(release) => Some(release),
        Err(Error::NotFound | Error::Disabled) => None,
        Err(_) => return refusal(&chrome, StatusCode::SERVICE_UNAVAILABLE),
    };
    let page = DownloadPage {
        chrome,
        channel,
        release,
    };
    html(&page, &page.chrome)
}

pub(crate) async fn start(Path(lang): Path<String>, chrome: Option<Extension<Chrome>>) -> Response {
    let Some(chrome) = page_chrome(chrome, &lang, "start") else {
        return status(StatusCode::NOT_FOUND);
    };
    let page = StartPage { chrome };
    html(&page, &page.chrome)
}

#[derive(Deserialize)]
pub(crate) struct PackagePath {
    lang: String,
    channel: String,
    build: String,
    platform: String,
    architecture: String,
}

impl PackagePath {
    fn channel(&self) -> Result<Channel, Error> {
        let channel = Channel::parse(&self.channel)?;
        if !identifier(&self.build)
            || !matches!(self.platform.as_str(), "windows" | "macos" | "linux")
            || !matches!(self.architecture.as_str(), "x86_64" | "arm64" | "x86")
        {
            return Err(Error::Invalid("build or platform"));
        }
        Ok(channel)
    }
}

pub(crate) async fn package(
    State(downloads): State<Downloads>,
    Path(path): Path<PackagePath>,
) -> Response {
    if !matches!(path.lang.as_str(), "pl" | "en") {
        return status(StatusCode::NOT_FOUND);
    }
    let Ok(channel) = path.channel() else {
        return status(StatusCode::BAD_REQUEST);
    };
    let release = match downloads.active_release(channel).await {
        Ok(release) => release,
        Err(Error::NotFound | Error::Disabled) => return status(StatusCode::NOT_FOUND),
        Err(_) => return status(StatusCode::SERVICE_UNAVAILABLE),
    };
    let Some(package) = release.packages.iter().find(|p| {
        release.id == path.build
            && p.platform == path.platform
            && p.architecture == path.architecture
    }) else {
        return status(StatusCode::NOT_FOUND);
    };
    match HeaderValue::from_str(&package.url) {
        Ok(location) => (
            StatusCode::TEMPORARY_REDIRECT,
            [
                (header::LOCATION, location),
                (header::CACHE_CONTROL, HeaderValue::from_static(NO_STORE)),
            ],
        )
            .into_response(),
        Err(_) => status(StatusCode::SERVICE_UNAVAILABLE),
    }
}

fn status(code: StatusCode) -> Response {
    (code, [(header::CACHE_CONTROL, NO_STORE)]).into_response()
}

fn page_chrome(chrome: Option<Extension<Chrome>>, lang: &str, route: &str) -> Option<Chrome> {
    if !matches!(lang, "pl" | "en") {
        return None;
    }
    Some(chrome.map_or_else(
        || Registry::default().chrome(lang, &format!("/{lang}/{route}"), Account::Anonymous),
        |Extension(chrome)| chrome,
    ))
}

fn html(page: &impl Template, chrome: &Chrome) -> Response {
    match page.render() {
        Ok(html) => (
            [
                (header::CACHE_CONTROL, NO_STORE),
                (header::CONTENT_LANGUAGE, chrome.lang.as_str()),
            ],
            Html(html),
        )
            .into_response(),
        Err(_) => refusal(chrome, StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn refusal(chrome: &Chrome, code: StatusCode) -> Response {
    (
        [
            (header::CACHE_CONTROL, NO_STORE),
            (header::CONTENT_LANGUAGE, chrome.lang.as_str()),
        ],
        oteryn_ui::error(chrome, code),
    )
        .into_response()
}
