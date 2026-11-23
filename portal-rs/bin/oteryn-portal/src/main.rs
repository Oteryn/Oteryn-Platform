//! Oteryn portal binary with server roles and one-off operator commands.

use std::{collections::HashMap, ffi::OsString, fs::File, io::Read, process::ExitCode};

use oteryn_content::pages::{LegalEdition, legal, publish_legal_edition};
use oteryn_core::Config;
use oteryn_identity::export;
use oteryn_web::AppState;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Serve,
    Migrate,
    InstallLegalPages,
    PublishLegalEdition,
    CheckSignupLegal,
    CheckSignupLegalUpgrade,
    Worker,
}

fn parse_role(arg: Option<&str>) -> Option<Role> {
    match arg {
        Some("serve") | None => Some(Role::Serve),
        Some("migrate") => Some(Role::Migrate),
        Some("install-legal-pages") => Some(Role::InstallLegalPages),
        Some("publish-legal-edition") => Some(Role::PublishLegalEdition),
        Some("check-signup-legal") => Some(Role::CheckSignupLegal),
        Some("check-signup-legal-upgrade") => Some(Role::CheckSignupLegalUpgrade),
        Some("worker") => Some(Role::Worker),
        Some(_) => None,
    }
}

#[cfg(test)]
#[test]
fn legal_install_requires_an_explicit_disposable_profile() {
    for profile in [None, Some("production"), Some("preprod"), Some("unknown")] {
        assert!(!legal::permits_install(profile));
    }
    for profile in ["local", "testing"] {
        assert!(legal::permits_install(Some(profile)));
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let arg = std::env::args().nth(1);
    if arg.as_deref() == Some("check-image") {
        let directory = std::env::var("OTERYN_STATIC_DIR").unwrap_or_else(|_| "/app/static".into());
        return if std::fs::metadata(directory).is_ok_and(|metadata| metadata.is_dir()) {
            println!(
                "oteryn-portal {}: serve worker migrate gateway",
                env!("CARGO_PKG_VERSION")
            );
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    // Test-only seeding prints one JSON object on stdout, so it runs before logging starts.
    if std::env::args().nth(1).as_deref() == Some("seed") {
        return seed::main().await;
    }
    // Operator issuance prints only the intent on stdout, so it also runs before logging.
    if std::env::args().nth(1).as_deref() == Some("bootstrap-intent") {
        return bootstrap_intent::main().await;
    }
    // Operator route publication prints one JSON object, so it also runs before logging.
    if std::env::args().nth(1).as_deref() == Some("game-route") {
        return game_route_cli::main().await;
    }
    if arg.as_deref() == Some("admin-cli") {
        return admin_cli::main().await;
    }
    if std::env::args().nth(1).as_deref() == Some("catalog") {
        return catalog_cli::main().await;
    }
    if std::env::args().nth(1).as_deref() == Some("commerce-refunds-reconcile") {
        return refund_reconcile_cli::main().await;
    }
    if std::env::args().nth(1).as_deref() == Some("migrate") && std::env::args_os().nth(2).is_some()
    {
        eprintln!("usage: oteryn-portal migrate");
        return ExitCode::from(2);
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .init();

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
        tracing::error!(role = ?arg, "unknown role; expected serve, migrate, install-legal-pages, publish-legal-edition, check-signup-legal, check-signup-legal-upgrade or worker");
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

const PUBLISH_LEGAL_USAGE: &str = "usage: oteryn-portal publish-legal-edition --actor <account uuid> --slug <terms-of-service|privacy-policy|cookie-policy> --locale <en|pl> --title <text> --body-file <path> --version <v> --effective <YYYY-MM-DD> --final-approved";

fn parse_legal_edition_args(
    mut args: impl Iterator<Item = OsString>,
) -> Result<(uuid::Uuid, LegalEdition), Box<dyn std::error::Error>> {
    let mut values = HashMap::new();
    let mut final_approved = false;
    while let Some(flag) = args.next() {
        match flag.to_str() {
            Some("--final-approved") if !final_approved => final_approved = true,
            Some(
                "--actor" | "--slug" | "--locale" | "--title" | "--body-file" | "--version"
                | "--effective",
            ) => {
                let value = args.next().ok_or(PUBLISH_LEGAL_USAGE)?;
                if values.insert(flag, value).is_some() {
                    return Err(PUBLISH_LEGAL_USAGE.into());
                }
            }
            _ => return Err(PUBLISH_LEGAL_USAGE.into()),
        }
    }
    if !final_approved {
        return Err(PUBLISH_LEGAL_USAGE.into());
    }
    let body_file = values
        .remove(std::ffi::OsStr::new("--body-file"))
        .ok_or(PUBLISH_LEGAL_USAGE)?;
    let mut value = |key: &str| {
        values
            .remove(std::ffi::OsStr::new(key))
            .and_then(|value| value.into_string().ok())
            .ok_or(PUBLISH_LEGAL_USAGE)
    };
    let actor = uuid::Uuid::parse_str(&value("--actor")?)?;
    let mut edition = LegalEdition {
        slug: value("--slug")?,
        locale: value("--locale")?,
        title: value("--title")?,
        body: String::new(),
        version: value("--version")?,
        effective: value("--effective")?,
    };
    // 100,000 Unicode scalar values can occupy at most 400,000 UTF-8 bytes.
    File::open(body_file)?
        .take(400_001)
        .read_to_string(&mut edition.body)?;
    if edition.body.len() > 400_000 {
        return Err("--body-file exceeds the CMS body limit".into());
    }
    Ok((actor, edition))
}

async fn run(role: Role) -> Result<(), Box<dyn std::error::Error>> {
    let legal_profile = std::env::var("OTERYN_ENV").ok();
    if role == Role::InstallLegalPages && !legal::permits_install(legal_profile.as_deref()) {
        return Err(
            "install-legal-pages requires explicit OTERYN_ENV=local or OTERYN_ENV=testing".into(),
        );
    }
    let config = Config::from_env()?;
    let db = oteryn_web::session::configured_pool(config.database_url())?;
    if matches!(role, Role::Worker) {
        return worker::run(db).await;
    }
    match role {
        Role::Migrate => {
            // An installed predecessor helper can still call the same migrate command.
            if std::env::var_os("OTERYN_ORACLE_DEPLOY_IMAGE").is_some() {
                oteryn_web::register::check_live_signup(&db).await?;
            }
            oteryn_db::migrate(&db).await?;
            tracing::info!("migrations applied");
        }
        Role::InstallLegalPages => {
            let installed = legal::install(&db, legal_profile.as_deref()).await?;
            tracing::info!(installed, "default legal document pairs installed");
        }
        Role::PublishLegalEdition => {
            let (actor, edition) = parse_legal_edition_args(std::env::args_os().skip(2))?;
            publish_legal_edition(&db, actor, edition).await?;
            tracing::info!(%actor, "legal edition published");
        }
        Role::CheckSignupLegal | Role::CheckSignupLegalUpgrade => {
            if role == Role::CheckSignupLegalUpgrade
                && sqlx::query_scalar::<_, bool>(
                    "SELECT current_schema() IS NOT NULL
                     AND to_regclass('_sqlx_migrations') IS NULL
                     AND to_regclass('pages') IS NULL
                     AND NOT EXISTS (
                       SELECT 1 FROM pg_catalog.pg_class c
                       JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
                       WHERE n.nspname = current_schema()
                     )",
                )
                .fetch_one(&db)
                .await?
            {
                tracing::info!("empty portal schema; first installation may proceed");
                return Ok(());
            }
            oteryn_web::register::check_live_signup(&db).await?;
            tracing::info!("signup legal editions ready in both locales");
        }
        Role::Worker => {
            tracing::info!("worker started");
            tokio::signal::ctrl_c().await?;
        }
        Role::Serve => {
            let proxies = oteryn_web::ratelimit::TrustedProxies::configured()?;
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
            let bazaar = oteryn_bazaar::game::runtime::Runtime::configured(db.clone()).await?;
            let closure = account_closure::configured(db.clone())?;
            let game_ingress = game_ingress::GameIngress::configured(
                db.clone(),
                account_closure::private_ingress(db.clone(), &closure, config.environment)?,
                bazaar.as_ref(),
            )?;
            let admission_listener = game_login::start_admission_listener(db.clone()).await?;
            let static_dir =
                std::env::var("OTERYN_STATIC_DIR").unwrap_or_else(|_| "static".to_owned());
            let app = oteryn_web::router_with_portal_entry_points(
                AppState { db: db.clone() },
                &static_dir,
            );
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
            let app = app.merge(oteryn_community::worlds::configured_router(db.clone())?);
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
            let app = app.merge(oteryn_admin::status::configured_router(db.clone())?);
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
            let app = app.merge(oteryn_support::appeals::router(db.clone(), &origin));
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
            let app = app.merge(export_http::router(
                db.clone(),
                export::ExportConfig::configured(),
            ));
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
            let account_oauth = oteryn_gameauth::oauth::Config::from_env();
            let account_reads = oteryn_admin::web::AccountReadStatus {
                configured: account_oauth.has_registered_client(),
            };
            let app = app
                .merge(oteryn_player_api::account_feed::configured_router(
                    db.clone(),
                    account_oauth,
                ))
                .merge(oteryn_playerdata::dashboard::configured_router(db.clone()))
                .layer(axum::Extension(account_reads));
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
            oteryn_bazaar::navigation::register(&mut navigation, bazaar.is_some());
            oteryn_web::entry_points::register(&mut navigation);
            let app = app.merge(oteryn_forum::router(db.clone(), &mut navigation)?);
            let app = app.layer(axum::middleware::from_fn_with_state(
                oteryn_web::chrome::Shell {
                    registry: navigation,
                    sessions: oteryn_web::session::SessionLayer::new(db.clone()),
                },
                oteryn_content::discovery::shell,
            ));
            // Internal-only exports stay off the public and native listeners.
            // Apply browser configuration after all public modules are merged.
            let app = oteryn_web::register::configured(app);
            let app = registration_delivery::admission(app, db.clone());
            let app = app.merge(code_routes::configured(db.clone())?);
            let app = with_proxy_policy(app, db.clone(), proxies);
            let (public_app, app) =
                coin_listener_apps(app, oteryn_wallet::wire::configured_router(db.clone()));
            let public_app = oteryn_minigames::with_browser_deadlines(public_app);
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
            #[cfg(test)]
            record_route_registrations("internal", &app)?;
            #[cfg(test)]
            record_route_registrations("public", &public_app)?;
            let (public_app, app) = oteryn_web::observability::instrument(public_app, app, db);
            let internal_listener = internal_tls::start(app, game_ingress).await?;
            let app = public_app;
            let native_listener = oteryn_gameauth::devices::tls::start(app.clone()).await?;
            #[cfg(test)]
            if native_listener.is_some() {
                record_route_registrations("native", &app)?;
            }
            let app = app.into_make_service_with_connect_info::<std::net::SocketAddr>();
            let private_listeners = register_private_listeners(
                [
                    ("internal-mtls", internal_listener),
                    ("native-tls", native_listener),
                ],
                admission_listener,
            );
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

#[cfg(test)]
fn record_route_registrations(name: &str, router: &axum::Router) -> std::io::Result<()> {
    if let Some(dir) = std::env::var_os("FLAG_BOUNDARY_ROUTE_DIR") {
        std::fs::write(std::path::Path::new(&dir).join(name), format!("{router:?}"))?;
    }
    Ok(())
}

type ListenerCompletion = (
    &'static str,
    Result<std::io::Result<()>, tokio::task::JoinError>,
);

fn register_private_listeners(
    handles: [(&'static str, Option<tokio::task::JoinHandle<()>>); 2],
    admission: tokio::task::JoinHandle<std::io::Result<()>>,
) -> tokio::task::JoinSet<ListenerCompletion> {
    let mut listeners = tokio::task::JoinSet::new();
    for (name, handle) in handles {
        if let Some(handle) = handle {
            listeners.spawn(async move { (name, handle.await.map(|()| Ok(()))) });
        }
    }
    listeners.spawn(async move { ("admission", admission.await) });
    listeners
}

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
            "internal-mtls",
            "native-tls",
        ] {
            let handle = tokio::spawn(async {});
            let mut tasks = tokio::task::JoinSet::new();
            tasks.spawn(async move { (name, handle.await.map(|()| Ok(()))) });
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

    #[tokio::test]
    async fn production_admission_registration_preserves_termination_cause()
    -> Result<(), Box<dyn std::error::Error>> {
        for panic in [false, true] {
            let admission = tokio::spawn(async move {
                assert!(!panic, "admission task panic");
                Err(std::io::Error::other("admission serve failed"))
            });
            let listeners = register_private_listeners(
                [("internal-mtls", None), ("native-tls", None)],
                admission,
            );
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                supervise_private_listeners(listeners, std::future::pending()),
            )
            .await?;
            let error = result
                .err()
                .ok_or("serve accepted admission termination")?
                .to_string();
            assert!(error.contains("admission"));
            assert!(error.contains(if panic {
                "admission task panic"
            } else {
                "admission serve failed"
            }));
        }
        Ok(())
    }

    #[test]
    fn roles_parse_with_serve_as_default() {
        assert_eq!(parse_role(None), Some(Role::Serve));
        assert_eq!(parse_role(Some("migrate")), Some(Role::Migrate));
        assert_eq!(
            parse_role(Some("install-legal-pages")),
            Some(Role::InstallLegalPages)
        );
        assert_eq!(
            parse_role(Some("publish-legal-edition")),
            Some(Role::PublishLegalEdition)
        );
        assert_eq!(parse_role(Some("worker")), Some(Role::Worker));
        assert_eq!(parse_role(Some("drop-database")), None);
    }

    #[test]
    fn publish_legal_edition_requires_explicit_approval_and_reads_markdown()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;

        let mut body = tempfile::NamedTempFile::new()?;
        let markdown = "# Regulamin Oteryn\n\nWarunki świadczenia usług.";
        body.write_all(markdown.as_bytes())?;
        let actor = uuid::Uuid::now_v7();
        let mut args: Vec<OsString> = [
            "--slug",
            "terms-of-service",
            "--locale",
            "pl",
            "--title",
            "Regulamin Oteryn",
            "--version",
            "test-v1",
            "--effective",
            "2026-11-23",
            "--body-file",
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        args.push(body.path().into());
        args.extend(["--actor".into(), actor.to_string().into()]);
        assert!(parse_legal_edition_args(args.iter().cloned()).is_err());
        args.push("--final-approved".into());
        let (parsed_actor, edition) = parse_legal_edition_args(args.iter().cloned())?;
        assert_eq!(parsed_actor, actor);
        assert_eq!(edition.body, markdown);
        assert_eq!(edition.slug, "terms-of-service");
        assert_eq!(edition.locale, "pl");
        assert_eq!(edition.title, "Regulamin Oteryn");
        assert_eq!(edition.version, "test-v1");
        assert_eq!(edition.effective, "2026-11-23");
        args.push("--final-approved".into());
        assert!(parse_legal_edition_args(args.into_iter()).is_err());
        Ok(())
    }
}

mod admin_cli;
mod bootstrap_intent;
mod coin_purchases;
mod game_ingress;
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
#[path = "../../../crates/identity/src/export_http.rs"]
mod export_http;
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
mod code_routes;
mod game_route_cli;
mod refund_reconcile_cli;

#[cfg(test)]
mod browser_configuration_tests {
    #[test]
    fn csrf_configuration_wraps_coin_and_moderation_routes() {
        let source = include_str!("main.rs");
        let composition = source
            .split_once("async fn run(role: Role)")
            .and_then(|(_, rest)| rest.split_once("\ntype ListenerCompletion"))
            .map_or("", |(body, _)| body);
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
                .find("coin_listener_apps(app,")
                .is_some_and(|at| at > configuration)
        );
    }
}

#[cfg(test)]
mod large_pages_tests;

/// Last browser composition layer shares one validated policy across module adapters.
fn with_proxy_policy(
    app: axum::Router,
    pool: sqlx::PgPool,
    proxies: oteryn_web::ratelimit::TrustedProxies,
) -> axum::Router {
    let mut limiter = oteryn_web::ratelimit::RateLimiter::new(pool);
    limiter.trusted_proxies = proxies;
    app.layer(axum::middleware::from_fn_with_state(
        limiter,
        oteryn_web::ratelimit::supply,
    ))
}
#[cfg(test)]
mod audit_l9_tests;

#[cfg(test)]
#[path = "../../../crates/admin/src/test_staff.rs"]
pub mod test_staff;

/// Capture the shared public/native router before adding internal coin exports.
fn coin_listener_apps(app: axum::Router, coins: axum::Router) -> (axum::Router, axum::Router) {
    let public = app.clone();
    (public, app.merge(coins))
}

#[cfg(test)]
#[path = "coin_listener_tests.rs"]
mod coin_listener_tests;
#[cfg(test)]
#[path = "../../../test-support/rate_clock.rs"]
mod rate_clock;
