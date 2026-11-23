//! Read-only portal system status and process configuration.
mod probes;
mod registry;

use std::{sync::Arc, time::Instant};

use askama::Template;
use axum::{
    Extension, Json, Router, extract::State, http::HeaderMap, response::Response, routing::get,
};
use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use oteryn_core::Environment;
use oteryn_web::Language;
use serde::Serialize;
use sqlx::PgPool;

use probes::{Backlog, MigrationStatus, Probe};

pub(crate) struct Status {
    db: PgPool,
    started_at: Instant,
    environment: Option<Environment>,
}

pub(crate) fn routes<S: Clone + Send + Sync + 'static>(db: PgPool) -> Router<S> {
    Router::new()
        .route("/admin/system", get(page))
        .route("/admin/system/data", get(data))
        .with_state(Arc::new(Status {
            db,
            started_at: Instant::now(),
            environment: std::env::var("OTERYN_ENV").map_or_else(
                |error| match error {
                    std::env::VarError::NotPresent => Some(Environment::Local),
                    std::env::VarError::NotUnicode(_) => None,
                },
                |raw| Environment::parse(&raw).ok(),
            ),
        }))
}

pub async fn record_worker_heartbeat(db: &PgPool) -> Result<(), impl std::error::Error> {
    probes::record_worker_heartbeat(db).await
}

#[derive(Serialize)]
struct Snapshot {
    environment: Option<Environment>,
    build_version: &'static str,
    build_commit: Option<&'static str>,
    uptime_seconds: u64,
    postgres: Probe<String>,
    migrations: Probe<MigrationStatus>,
    worker: Probe<i64>,
    outbox: Probe<Backlog>,
    mail_outbox: Probe<i64>,
    gateway: Probe<String>,
    game_link: Probe<String>,
    flags: Vec<registry::Flag>,
    configuration: Vec<registry::Setting>,
}

impl Status {
    async fn snapshot(&self, locale: &Locale) -> Snapshot {
        let (postgres, migrations, worker, outbox, mail_outbox, gateway, game_link) = tokio::join!(
            probes::postgres(&self.db),
            probes::migrations(&self.db),
            probes::worker(&self.db),
            probes::outbox(&self.db),
            probes::mail_outbox(&self.db),
            probes::gateway(&self.db),
            probes::game_link(&self.db),
        );
        Snapshot {
            environment: self.environment,
            build_version: env!("CARGO_PKG_VERSION"),
            build_commit: Some(env!("OTERYN_SYSTEM_COMMIT")).filter(|commit| !commit.is_empty()),
            uptime_seconds: self.started_at.elapsed().as_secs(),
            postgres,
            migrations,
            worker,
            outbox,
            mail_outbox,
            gateway,
            game_link,
            flags: registry::flags(locale, |name| std::env::var_os(name)),
            configuration: registry::configuration(|name| std::env::var_os(name).is_some()),
        }
    }
}

struct Locale(FluentBundle<FluentResource>);

impl Locale {
    fn new(lang: &str) -> Self {
        let mut bundle = FluentBundle::new_concurrent(lang.parse().into_iter().collect());
        if let Ok(resource) = FluentResource::try_new(super::web::locale_source(lang).into()) {
            let _ = bundle.add_resource(resource);
        }
        Self(bundle)
    }

    fn text(&self, key: &str) -> String {
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
#[template(path = "admin/system.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    locale: Locale,
    snapshot: Snapshot,
}

impl Page {
    fn text(&self, key: &str) -> String {
        self.locale.text(key)
    }

    fn environment(&self) -> &str {
        match self.snapshot.environment {
            Some(Environment::Local) => "local",
            Some(Environment::Testing) => "testing",
            Some(Environment::Preprod) => "preprod",
            Some(Environment::Production) => "production",
            None => "",
        }
    }
}

fn language(headers: &HeaderMap, negotiated: Option<Extension<Language>>) -> &'static str {
    negotiated.map_or_else(
        || super::web::language(headers),
        |Extension(lang)| lang.code(),
    )
}

async fn page(
    State(status): State<Arc<Status>>,
    negotiated: Option<Extension<Language>>,
    headers: HeaderMap,
) -> Response {
    let lang = language(&headers, negotiated);
    let locale = Locale::new(lang);
    let snapshot = status.snapshot(&locale).await;
    oteryn_web::render(&Page {
        lang,
        locale,
        snapshot,
    })
}

async fn data(
    State(status): State<Arc<Status>>,
    negotiated: Option<Extension<Language>>,
    headers: HeaderMap,
) -> Json<Snapshot> {
    let locale = Locale::new(language(&headers, negotiated));
    Json(status.snapshot(&locale).await)
}

#[cfg(test)]
mod tests;
