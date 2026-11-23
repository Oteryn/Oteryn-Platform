//! Published service notices and incident-derived availability for the portal and registered worlds.
mod public;
mod staff;
mod store;

use axum::{
    Router,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use fluent_bundle::{FluentResource, concurrent::FluentBundle};
use sqlx::PgPool;
use std::sync::LazyLock;
use store::Error;

type Catalogue = FluentBundle<FluentResource>;

fn catalogue(lang: unic_langid::LanguageIdentifier, source: &str) -> Option<Catalogue> {
    let resource = FluentResource::try_new(source.into()).ok()?;
    let mut bundle = FluentBundle::new_concurrent(vec![lang]);
    bundle.add_resource(resource).ok()?;
    Some(bundle)
}

#[derive(Clone, Copy)]
struct Labels(&'static str);

impl Labels {
    fn get(self, key: &str) -> String {
        text(self.0, key)
    }

    fn uptime(self, value: &f64) -> String {
        let percent = format!("{value:.2}%");
        if self.0 == "pl" {
            percent.replace('.', ",")
        } else {
            percent
        }
    }
}

pub(crate) fn text(lang: &str, key: &str) -> String {
    static EN: LazyLock<Option<Catalogue>> = LazyLock::new(|| {
        catalogue(
            unic_langid::langid!("en"),
            include_str!("../../../../templates/status/en.ftl"),
        )
    });
    static PL: LazyLock<Option<Catalogue>> = LazyLock::new(|| {
        catalogue(
            unic_langid::langid!("pl"),
            include_str!("../../../../templates/status/pl.ftl"),
        )
    });
    let bundle = if lang == "en" { &EN } else { &PL };
    bundle
        .as_ref()
        .and_then(|bundle| {
            let pattern = bundle.get_message(key)?.value()?;
            Some(
                bundle
                    .format_pattern(pattern, None, &mut Vec::new())
                    .into_owned(),
            )
        })
        .unwrap_or_default()
}

fn failure(error: Error, lang: &str) -> Response {
    let (status, label) = match error {
        Error::Invalid | Error::Authorization(crate::rbac::Error::InvalidInput) => {
            (StatusCode::UNPROCESSABLE_ENTITY, "invalid")
        }
        Error::Missing => (StatusCode::NOT_FOUND, "missing"),
        Error::Conflict => (StatusCode::CONFLICT, "conflict"),
        Error::Authorization(crate::rbac::Error::Denied) => (StatusCode::FORBIDDEN, "forbidden"),
        error => {
            tracing::warn!(%error, "status request unavailable");
            (StatusCode::SERVICE_UNAVAILABLE, "unavailable")
        }
    };
    (
        status,
        [(header::CONTENT_LANGUAGE, lang)],
        text(lang, label),
    )
        .into_response()
}

/// Mount both public and staff endpoints; staff routes use the shared panel boundary.
pub fn configured_router(db: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    Ok(public::router(db.clone()).merge(staff::router(db)?))
}

#[cfg(test)]
mod tests;
