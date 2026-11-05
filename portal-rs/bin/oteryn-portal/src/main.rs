//! Oteryn portal binary. One process role per invocation: serve, migrate or worker.

use std::process::ExitCode;

use oteryn_core::Config;
use oteryn_web::AppState;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Serve,
    Migrate,
    Worker,
}

fn parse_role(arg: Option<&str>) -> Option<Role> {
    match arg {
        Some("serve") | None => Some(Role::Serve),
        Some("migrate") => Some(Role::Migrate),
        Some("worker") => Some(Role::Worker),
        Some(_) => None,
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .init();

    let arg = std::env::args().nth(1);
    if arg.as_deref() == Some("gateway") {
        return match gateway::run().await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                tracing::error!(%error, "gateway stopped");
                ExitCode::FAILURE
            }
        };
    }
    let Some(role) = parse_role(arg.as_deref()) else {
        tracing::error!(role = ?arg, "unknown role; expected serve, migrate or worker");
        return ExitCode::from(2);
    };
    match run(role).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, ?role, "portal stopped");
            ExitCode::FAILURE
        }
    }
}

async fn run(role: Role) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    let db = oteryn_db::lazy_pool(config.database_url())?;
    if matches!(role, Role::Worker) {
        return worker::run(db).await;
    }
    match role {
        Role::Migrate => {
            oteryn_db::migrate(&db).await?;
            tracing::info!("migrations applied");
        }
        Role::Worker => {
            tracing::info!("worker started");
            tokio::signal::ctrl_c().await?;
        }
        Role::Serve => {
            let _admission_listener =
                oteryn_gameauth::admission::start_private_listener(db.clone()).await?;
            let static_dir =
                std::env::var("OTERYN_STATIC_DIR").unwrap_or_else(|_| "static".to_owned());
            let app = oteryn_web::router(AppState { db }, &static_dir);
            let app = app.merge(oteryn_community::ingest::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            ));
            let app = app.merge(oteryn_community::worlds::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            ));
            let app = app.merge(oteryn_community::online::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            ));
            let app = app.merge(oteryn_downloads::router(
                oteryn_db::lazy_pool(config.database_url())?,
                oteryn_downloads::FeatureFlags {
                    downloads: std::env::var("OTERYN_DOWNLOADS_ENABLED")
                        .is_ok_and(|value| value == "true"),
                },
            ));
            let app = app.merge(oteryn_community::rankings::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            ));
            let app = app.merge(oteryn_wiki::calculators::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            ));
            let origin = std::env::var("OTERYN_PUBLIC_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3000".to_owned());
            let production = std::env::var("OTERYN_ENV").is_ok_and(|value| value == "production");
            let mut seo = oteryn_web::seo::Seo::new(&origin, production)?;
            seo.register(oteryn_web::seo::Entry {
                pl: "/pl/".to_owned(),
                en: "/en/".to_owned(),
                lastmod: "2026-11-05".to_owned(),
            })?;
            let app = app
                .merge(oteryn_web::seo::router(seo))
                .merge(oteryn_web::redirects::router());
            let app = oteryn_web::register::configured(app);
            let app = app.merge(oteryn_admin::web::configured_router(oteryn_db::lazy_pool(
                config.database_url(),
            )?)?);
            let app = app.merge(oteryn_payments::account::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            ));
            let app = app.merge(oteryn_atlas::web::configured_router(oteryn_db::lazy_pool(
                config.database_url(),
            )?));
            let app = app.merge(login_http::configured_router(oteryn_db::lazy_pool(
                config.database_url(),
            )?));
            let app = app.merge(oteryn_content::pages::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            )?);
            let app = app.merge(oteryn_content::events::web::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            )?);
            let app = app.merge(oteryn_wiki::web::configured_router(oteryn_db::lazy_pool(
                config.database_url(),
            )?)?);
            let app = app.merge(oteryn_world::admin::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            )?);
            let app = app.merge(oteryn_content::news::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            )?);
            let app = app.merge(oteryn_stats::rankings::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            ));
            let app = app.merge(oteryn_stats::world::http::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            ));
            let app = app.merge(oteryn_content::media::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            )?);
            let app = app.merge(oteryn_player_api::configured_router(oteryn_db::lazy_pool(
                config.database_url(),
            )?));
            let app = app.merge(oteryn_support::reports::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            ));
            let app = app.merge(oteryn_admin::players::http::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            )?);
            let wizard_pool = oteryn_db::lazy_pool(config.database_url())?;
            let wizard = oteryn_identity::security_wizard::configured(wizard_pool.clone())
                .layer(axum::middleware::from_fn(
                    |mut request: axum::extract::Request, next: axum::middleware::Next| async move {
                        if let Some(session) = request
                            .extensions()
                            .get::<oteryn_web::session::BrowserSession>()
                            && let Some(id) = session.account_id().await
                        {
                            request
                                .extensions_mut()
                                .insert(oteryn_identity::security_wizard::Viewer(id));
                        }
                        next.run(request).await
                    },
                ))
                .layer(axum::middleware::from_fn_with_state(
                    oteryn_web::session::SessionLayer::new(wizard_pool),
                    oteryn_web::session::middleware,
                ));
            let app = app.merge(wizard);
            let app = app.merge(oteryn_hirelings::web::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            ));
            let app = app.merge(oteryn_bazaar::characters::http::configured_router());
            let app = oteryn_wiki::entities::mount(
                app,
                oteryn_wiki::entities::EntityState::configured(oteryn_db::lazy_pool(
                    config.database_url(),
                )?),
            );
            let app = app.merge(oteryn_bazaar::items::web::configured_router());
            let app = app.merge(oteryn_playerdata::hunts::web::configured_router(
                oteryn_db::lazy_pool(config.database_url())?,
            ));
            let listener = tokio::net::TcpListener::bind(config.bind).await?;
            tracing::info!(bind = %config.bind, environment = ?config.environment, "serving");
            let app = app.into_make_service_with_connect_info::<std::net::SocketAddr>();
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_parse_with_serve_as_default() {
        assert_eq!(parse_role(None), Some(Role::Serve));
        assert_eq!(parse_role(Some("migrate")), Some(Role::Migrate));
        assert_eq!(parse_role(Some("worker")), Some(Role::Worker));
        assert_eq!(parse_role(Some("drop-database")), None);
    }
}

mod gateway;
mod worker;

#[path = "../../../crates/identity/src/login_http.rs"]
mod login_http;
