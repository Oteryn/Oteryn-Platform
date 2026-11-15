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
    // Test-only seeding prints one JSON object on stdout, so it runs before logging starts.
    if std::env::args().nth(1).as_deref() == Some("seed") {
        return seed::main().await;
    }
    // Operator issuance prints only the intent on stdout, so it also runs before logging.
    if std::env::args().nth(1).as_deref() == Some("bootstrap-intent") {
        return bootstrap_intent::main().await;
    }
    if std::env::args().nth(1).as_deref() == Some("catalog") {
        return catalog_cli::main().await;
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .init();

    let arg = std::env::args().nth(1);
    if arg.as_deref() == Some("downloads-publish") {
        return oteryn_downloads::command::run().await;
    }
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
            let internal_configuration = game_routes::Configuration::from_env()?;
            let catalog_db = db.clone();
            let catalog_ready = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let refresh_ready = catalog_ready.clone();
            tokio::spawn(async move {
                match oteryn_world::catalog::refresh_in_background(catalog_db, refresh_ready).await
                {
                    Ok(Ok(Some(snapshot))) => {
                        tracing::info!(%snapshot, "native catalog refresh verified")
                    }
                    Ok(Ok(None)) => {}
                    _ => tracing::warn!(
                        "native catalog refresh refused; configured public source unavailable"
                    ),
                }
            });
            let league_listener =
                oteryn_minigames::league::transport::start_private_listener(db.clone()).await?;
            let bazaar = oteryn_bazaar::game::runtime::Runtime::configured(db.clone()).await?;
            let bazaar_listener = if let Some(runtime) = &bazaar {
                Some(runtime.start_private_listener().await?)
            } else {
                None
            };
            let closure = account_closure::configured(db.clone())?;
            let closure_listener = account_closure::start_listener(db.clone(), &closure).await?;
            let _admission_listener = game_login::start_admission_listener(db.clone()).await?;
            let static_dir =
                std::env::var("OTERYN_STATIC_DIR").unwrap_or_else(|_| "static".to_owned());
            let app = oteryn_web::router(AppState { db: db.clone() }, &static_dir);
            let app = oteryn_community::home::feed(db.clone()).layer(app);
            let app = app.merge(oteryn_community::ingest::configured_router(db.clone()));
            let app = app.merge(oteryn_web::favicon::router());
            let app = app.merge(oteryn_community::deaths::configured_router(db.clone()));
            let app = app.merge(oteryn_community::guilds::configured_router(db.clone()));
            let app = app.merge(oteryn_community::houses::configured_router(db.clone()));
            // G-4 skills (public) and owner-only character pages; ingest is internal only.
            let app = app.merge(oteryn_community::families::configured_router(db.clone()));
            let app = app.merge(oteryn_community::v2_watermark::configured_router(
                db.clone(),
            ));
            let app = app.merge(oteryn_community::worlds::configured_router(db.clone()));
            let app = app.merge(oteryn_community::online::configured_router(db.clone()));
            let app = app
                .merge(oteryn_community::referrals::campaign::web::configured_router(db.clone())?);
            let app = app.merge(oteryn_downloads::router(
                db.clone(),
                oteryn_downloads::FeatureFlags {
                    downloads: std::env::var("OTERYN_DOWNLOADS_ENABLED")
                        .is_ok_and(|value| value == "true"),
                },
            ));
            let app = app.merge(oteryn_community::profiles::configured_router(db.clone()));
            let app = app.merge(oteryn_community::rankings::configured_router(db.clone()));
            let app = app.merge(oteryn_wiki::calculators::configured_router(db.clone()));
            let app = app.merge(oteryn_community::party::configured_router(db.clone())?);
            let origin = std::env::var("OTERYN_PUBLIC_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:3000".to_owned());
            let production = std::env::var("OTERYN_ENV").is_ok_and(|value| value == "production");
            let mut seo = oteryn_web::seo::Seo::new(&origin, production)?;
            seo.register(oteryn_web::seo::Entry {
                pl: "/pl/".to_owned(),
                en: "/en/".to_owned(),
                lastmod: "2026-11-05".to_owned(),
            })?;
            // Legacy redirects are the site fallback (oteryn_web::router), so
            // module routes on the same paths never overlap with them.
            let app = app.merge(oteryn_web::seo::router(seo));
            let app = app.merge(oteryn_admin::console::configured_router(db.clone())?);
            let app = app.merge(oteryn_admin::web::configured_router(db.clone())?);
            let app = app.merge(oteryn_admin::analytics::configured_router(db.clone())?);
            let app = app.merge(oteryn_admin::console::access::configured_router(
                db.clone(),
            )?);
            let app = app.merge(oteryn_payments::account::configured_router(db.clone()));
            let app = app.merge(coin_purchases::configured_router(db.clone()));
            let app = app.merge(oteryn_atlas::web::configured_router(db.clone()));
            let app = app.merge(login_http::configured_router(db.clone()));
            let app = app.merge(oteryn_content::pages::configured_router(db.clone())?);
            let app = app.merge(oteryn_content::events::web::configured_router(db.clone())?);
            let app = app.merge(oteryn_wiki::web::configured_router(db.clone())?);
            let app = app.merge(oteryn_world::admin::configured_router(db.clone())?);
            let app = app.merge(oteryn_content::news::configured_router(db.clone())?);
            let app = app.merge(oteryn_content::announcements::configured_router(
                db.clone(),
            )?);
            let app = app.merge(oteryn_stats::character::http::configured_router(db.clone()));
            let app = app.merge(oteryn_stats::watch::http::configured_router(db.clone()));
            let app = app.merge(oteryn_stats::rankings::configured_router(db.clone()));
            let app = app.merge(oteryn_stats::world::http::configured_router(db.clone()));
            let app = app.merge(oteryn_stats::guilds::http::configured_router(db.clone()));
            let app = app.merge(oteryn_content::media::configured_router(db.clone())?);
            let app = app.merge(game_login::player_router(db.clone()));
            let app = app.layer(axum::Extension(game_login::device_authority(
                oteryn_gameauth::oauth::Config::from_env(),
                std::env::var("OTERYN_NATIVE_DEVICE_PROJECTION_AUTHORITY").unwrap_or_default(),
            )));
            let app = app.merge(oteryn_support::reports::configured_router(db.clone()));
            let app = app.merge(oteryn_admin::players::http::configured_router(db.clone())?);
            let wizard_pool = db.clone();
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
            let app = app.merge(oteryn_hirelings::web::configured_router(db.clone()));
            let app = app.merge(match &bazaar {
                Some(runtime) => runtime.browser_router(),
                None => oteryn_bazaar::characters::http::configured_router(),
            });
            let app = app.merge(reset_http::configured_router(db.clone()));
            let app = app.merge(passkeys_http::configured_router(db.clone())?);
            let app = oteryn_wiki::entities::mount(
                app,
                oteryn_wiki::entities::EntityState::configured(db.clone(), catalog_ready.clone()),
            );
            let app = app.merge(oteryn_support::tickets::configured_router(db.clone())?);
            let app = app.merge(credentials_http::configured_router(db.clone()));
            let app = app.merge(oteryn_identity::export::configured_router(db.clone()));
            let app = app.merge(oteryn_content::today::router(db.clone()));
            let app = app.layer(axum::middleware::from_fn_with_state(
                db.clone(),
                oteryn_content::announcements::banner,
            ));
            let app = app.merge(email_change_http::configured_router(db.clone()));
            let app = app.merge(oteryn_admin::commerce::configured_router(db.clone())?);
            let app = app.merge(oteryn_admin::controls::http::configured_router(db.clone())?);
            let app = sessions_http::configured(app, db.clone());
            let app = app.merge(privacy_web::configured_router(db.clone()));
            let app = app.merge(account_lifecycle_http::configured_router(
                db.clone(),
                closure.terminations.clone(),
            ));
            let app = app.merge(shop_web::router(db.clone()));
            let app = app.merge(oteryn_player_api::account_feed::configured_router());
            let app = app.merge(oteryn_support::tickets::configured_staff_router(
                db.clone(),
            )?);
            let app = app.merge(oteryn_atlas::spawns::configured_router(db.clone()));
            let app = app.layer(axum::middleware::from_fn(
                oteryn_atlas::spawns::viewer_policy,
            ));
            let app = app.merge(oteryn_minigames::puzzles::configured_router(db.clone()));
            let app = if bazaar.is_none() {
                app.merge(oteryn_bazaar::items::web::configured_router())
            } else {
                app
            };
            let app = app.merge(oteryn_playerdata::hunts::web::configured_router(db.clone()));
            let app = app.merge(oteryn_playerdata::items::configured_router(db.clone()));
            let app = app.merge(oteryn_playerdata::dashboard::configured_router());
            let app = app.merge(oteryn_playerdata::logins::web::configured_router(
                db.clone(),
            ));
            let app = mfa_http::configured(app, db.clone())?;
            let app = app.merge(oteryn_web::search::routes(
                oteryn_web::search::SearchState {
                    pool: db.clone(),
                    characters: None,
                    world: Some(std::sync::Arc::new({
                        let pool = db.clone();
                        let catalog = oteryn_world::catalog::publication_enabled().then(||
                            oteryn_world::catalog::CatalogStore::configured(pool.clone(), catalog_ready.clone()));
                        move |lang, query, page| {
                            let pool = pool.clone();
                            let catalog = catalog.clone();
                            Box::pin(async move {
                                if std::env::var("PORTAL_WIKI_ENTITIES_ENABLED").as_deref()
                                    != Ok("true")
                                {
                                    return Err(axum::http::StatusCode::SERVICE_UNAVAILABLE);
                                }
                                let world = oteryn_world::WorldStore::new(
                                    pool.clone(),
                                    oteryn_world::WorldOptions { enabled: true },
                                );
                                let selection: oteryn_web::search::SelectionReader = std::sync::Arc::new({
                                    let catalog = catalog.clone();
                                    let world = world.clone();
                                    move || {
                                        let catalog = catalog.clone();
                                        let world = world.clone();
                                        Box::pin(async move {
                                            let version = match catalog {
                                                Some(c) => c.active_version_id().await.map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?,
                                                None => world.active_version_id().await.map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?,
                                            };
                                            Ok(version.map(|v| v.to_string()))
                                        })
                                    }
                                });
                                oteryn_web::search::world_search_selected(
                                    &pool,
                                    &lang,
                                    &query,
                                    page,
                                    std::sync::Arc::new({
                                        let lang = lang.clone();
                                        move |kind, key| {
                                            let world = world.clone();
                                            let catalog = catalog.clone();
                                            let lang = lang.clone();
                                            Box::pin(async move {
                                                let entity = match catalog {
                                                    Some(c) => {
                                                        if !c.available(&kind).await.map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)? { return Ok(None); }
                                                        c.get_entity(&kind, &key).await.map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?
                                                    }
                                                    None => world.get_entity(&kind, &key).await.map_err(|_| axum::http::StatusCode::SERVICE_UNAVAILABLE)?,
                                                };
                                                Ok(entity.map(|e| {
                                                    oteryn_web::search::world_title(&e.data, &e.key, &lang)
                                                }))
                                            })
                                        }
                                    }),
                                    selection,
                                )
                                .await
                            })
                        }
                    })),
                },
            ));
            let app = app.merge(oteryn_bazaar::moderation::web::configured_router(
                db.clone(),
            ));
            let app = app.merge(oteryn_admin::bazaar::configured_router(db.clone())?);
            let app = app.merge(oteryn_playerdata::places::configured_router(db.clone()));
            let app = app.merge(oteryn_playerdata::reports::http::configured_router(
                db.clone(),
            ));
            let app = app.merge(oteryn_wiki::places::configured_router());
            let app = app.merge(oteryn_minigames::pass::web::configured_router(db.clone()));
            let app = app.merge(oteryn_minigames::league::web::configured_router(db.clone()));
            let app = app.merge(oteryn_minigames::collection_pvp::http::configured_router(
                db.clone(),
            ));
            let app = app.merge(oteryn_minigames::collection::configured_router(db.clone()));
            let app = app.merge(oteryn_admin::control_http::configured_router(db.clone())?);
            let app = app.merge(oteryn_atlas::bestiary::web::configured_router(db.clone()));
            let listener = tokio::net::TcpListener::bind(config.bind).await?;
            tracing::info!(bind = %config.bind, environment = ?config.environment, "serving");
            let app = app.merge(oteryn_atlas::delivery::configured_router(db.clone()));
            let app = oteryn_community::today::configured_mount(app, db.clone());
            let mut navigation = oteryn_ui::Registry::default();
            navigation.registration = true; // register::routes is always mounted.
            navigation.login = std::env::var("OTERYN_ACCOUNT_LOGIN").as_deref() == Ok("true");
            navigation.account = true; // /account is always mounted.
            oteryn_identity::navigation::register(&mut navigation);
            oteryn_characters::navigation::register(&mut navigation);
            oteryn_store::navigation::register(&mut navigation);
            oteryn_content::navigation::register(&mut navigation);
            oteryn_wiki::navigation::register(&mut navigation);
            oteryn_community::navigation::register(&mut navigation);
            oteryn_stats::navigation::register(&mut navigation);
            oteryn_atlas::navigation::register(&mut navigation);
            oteryn_minigames::navigation::register(&mut navigation);
            oteryn_hirelings::navigation::register(&mut navigation);
            oteryn_playerdata::navigation::register(&mut navigation);
            oteryn_payments::navigation::register(&mut navigation);
            oteryn_support::navigation::register(&mut navigation);
            oteryn_bazaar::navigation::register(&mut navigation);
            let app = app.layer(axum::middleware::from_fn_with_state(
                oteryn_web::chrome::Shell {
                    registry: navigation,
                    sessions: oteryn_web::session::SessionLayer::new(db.clone()),
                },
                oteryn_web::chrome::middleware,
            ));
            // Same router behind verified mTLS; the only adapter that admits /internal/v1 peers.
            let app = app.merge(oteryn_wallet::wire::configured_router(db.clone()));
            // Internal-only exports stay off the public and native listeners.
            // Apply browser configuration after all public modules are merged.
            let app = oteryn_web::register::configured(app);
            let public_app = app.clone();
            let app = app.merge(oteryn_support::sanctions::export::configured_router(
                db.clone(),
            ));
            let app = app.merge(oteryn_identity::security_revocations::configured_router(
                db.clone(),
            ));
            let app = app.merge(oteryn_store::loyalty::stream::http::configured_router(
                db.clone(),
            ));
            let app = match game_login::lcfa_router(
                db.clone(),
                std::env::var(game_login::LCFA_IDENTITY).ok(),
                oteryn_characters::overview::configured_projection(),
            )? {
                Some(lcfa) => app.merge(lcfa),
                None => app,
            };
            // Game-facing internal routes, each behind its own default-off flag
            // and client identity; composition and variables in game_routes.rs.
            let app =
                game_routes::mount(app, db.clone(), config.environment, internal_configuration)
                    .await;
            let internal_listener = internal_tls::start(app.clone()).await?;
            let app = public_app;
            let native_listener = oteryn_gameauth::devices::tls::start(app.clone()).await?;
            let app = app.into_make_service_with_connect_info::<std::net::SocketAddr>();
            let mut private_listeners = tokio::task::JoinSet::new();
            for (name, handle) in [
                ("internal TLS", internal_listener),
                ("native TLS", native_listener),
                ("league", league_listener),
                ("bazaar", bazaar_listener),
                ("account-closure", closure_listener),
            ] {
                if let Some(handle) = handle {
                    private_listeners.spawn(async move { (name, handle.await) });
                }
            }
            supervise_private_listeners(private_listeners, async move {
                axum::serve(listener, app)
                    .with_graceful_shutdown(async {
                        let _ = tokio::signal::ctrl_c().await;
                    })
                    .await
            })
            .await?;
        }
    }
    Ok(())
}

type ListenerCompletion = (&'static str, Result<(), tokio::task::JoinError>);

async fn supervise_private_listeners(
    mut listeners: tokio::task::JoinSet<ListenerCompletion>,
    server: impl std::future::Future<Output = std::io::Result<()>>,
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::select! {
        result = listeners.join_next(), if !listeners.is_empty() => {
            Err(listener_stopped(result).into())
        }
        result = server => { result?; Ok(()) }
    }
}

fn listener_stopped(result: Option<Result<ListenerCompletion, tokio::task::JoinError>>) -> String {
    match result {
        Some(Ok((name, result))) => {
            format!("{name} private listener stopped unexpectedly: {result:?}")
        }
        other => format!("private listener supervisor stopped unexpectedly: {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn all_private_listener_completions_are_supervised()
    -> Result<(), Box<dyn std::error::Error>> {
        for name in [
            "league",
            "bazaar",
            "account-closure",
            "internal TLS",
            "native TLS",
        ] {
            let handle = tokio::spawn(async {});
            let mut tasks = tokio::task::JoinSet::new();
            tasks.spawn(async move { (name, handle.await) });
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                supervise_private_listeners(tasks, std::future::pending()),
            )
            .await?;
            let error = result
                .err()
                .ok_or("a stopped listener must fail serve")?
                .to_string();
            assert!(error.contains(name));
            assert!(error.contains("stopped unexpectedly"));
        }
        Ok(())
    }

    #[test]
    fn roles_parse_with_serve_as_default() {
        assert_eq!(parse_role(None), Some(Role::Serve));
        assert_eq!(parse_role(Some("migrate")), Some(Role::Migrate));
        assert_eq!(parse_role(Some("worker")), Some(Role::Worker));
        assert_eq!(parse_role(Some("drop-database")), None);
    }
}

mod bootstrap_intent;
mod coin_purchases;
mod game_login;
mod game_routes;
mod gateway;
mod internal_tls;
mod seed;
mod wallet_topup;
mod worker;

#[path = "../../../crates/identity/src/login_http.rs"]
mod login_http;

#[path = "../../../crates/identity/src/credentials_http.rs"]
mod credentials_http;
#[path = "../../../crates/identity/src/email_change_http.rs"]
mod email_change_http;
#[path = "../../../crates/identity/src/passkeys_http.rs"]
mod passkeys_http;
#[path = "../../../crates/identity/src/privacy_web.rs"]
mod privacy_web;
#[path = "../../../crates/identity/src/reset_delivery.rs"]
mod reset_delivery;
#[path = "../../../crates/identity/src/reset_http.rs"]
mod reset_http;
#[path = "../../../crates/identity/src/sessions_http.rs"]
mod sessions_http;
#[path = "../../../crates/store/src/web_payments.rs"]
mod shop_web;

#[path = "../../../crates/identity/src/mfa_http.rs"]
mod mfa_http;

mod registration_delivery;

#[path = "../../../crates/identity/src/account_lifecycle_http.rs"]
mod account_lifecycle_http;

mod account_closure;

mod catalog_cli;

#[cfg(test)]
mod browser_configuration_tests {
    #[test]
    fn csrf_configuration_wraps_coin_and_moderation_routes() {
        let source = include_str!("main.rs");
        let composition = source.split("#[cfg(test)]").next().unwrap_or_default();
        let configuration = composition
            .find("let app = oteryn_web::register::configured(app);")
            .unwrap_or_default();
        for route in [
            "coin_purchases::configured_router",
            "oteryn_bazaar::moderation::web::configured_router",
        ] {
            assert!(composition.find(route).is_some_and(|at| at < configuration));
        }
        assert!(
            composition
                .find("let public_app = app.clone();")
                .is_some_and(|at| at > configuration)
        );
    }
}

#[cfg(test)]
mod large_pages_tests;
