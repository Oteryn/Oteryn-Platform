//! Audit-before-response admin boundary, using the panel's trusted identity adapters.
#[path = "economy.rs"]
mod economy;
use super::{Config, wire};
use crate::{
    audit::{self, AuditContext, Entry},
    rbac::{AccountStateReader, Authorizer, CheckContext, Permission},
    web::{PanelConfig, PortalIdentity, StaffSession},
};
use askama::Template;
use axum::{
    Router,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use uuid::Uuid;
#[derive(Clone)]
struct Attempt {
    context: Arc<tokio::sync::Mutex<AuditContext>>,
    recorded: Arc<AtomicBool>,
}
struct Context<I, A> {
    pool: PgPool,
    config: Config,
    identity: I,
    auth: Authorizer<A>,
}
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    world: Option<Uuid>,
    family: Option<wire::Family>,
    start: Option<u64>,
    end: Option<u64>,
    from: Option<String>,
    to: Option<String>,
}
#[derive(serde::Serialize)]
struct Cell {
    key: String,
    state: String,
    values: Vec<String>,
    start: String,
    end: String,
    epoch: String,
    revision: String,
    observed: String,
    start_utc: String,
    end_utc: String,
    cut_utc: String,
    age: u64,
}
#[derive(Template)]
#[template(path = "admin/analytics.html", config = "src/web_askama.toml")]
struct Page {
    lang: &'static str,
    title: &'static str,
    unavailable: &'static str,
    cut: &'static str,
    range: &'static str,
    freshness: &'static str,
    epoch: &'static str,
    cells: Vec<Cell>,
    world: String,
    from: String,
    to: String,
    policy: String,
    family: String,
}
pub fn configured_router(
    pool: PgPool,
    config: Config,
) -> Result<Router, Box<dyn std::error::Error>> {
    Ok(router(
        pool.clone(),
        config,
        crate::web::config_from_env()?,
        PortalIdentity::configured(pool.clone()),
        PortalIdentity::configured(pool),
    ))
}
pub fn router<I: StaffSession, A: AccountStateReader + 'static>(
    pool: PgPool,
    config: Config,
    panel: PanelConfig,
    identity: I,
    accounts: A,
) -> Router {
    let mut host_config = panel.clone();
    host_config.admin_panel &= config.present;
    let mut session = SessionLayer::new(pool.clone());
    session.csrf.trusted_origins = vec![format!("https://{}", panel.host)];
    let state = Arc::new(Context {
        pool: pool.clone(),
        config,
        identity,
        auth: Authorizer::new(pool.clone(), accounts),
    });
    let mut routes = Router::new();
    for path in [
        "/admin/analytics",
        "/admin/analytics/creatures",
        "/admin/analytics/activity",
        "/admin/analytics/levels",
        "/admin/analytics/health",
        "/admin/analytics/economy",
    ] {
        routes = routes
            .route(path, get(read::<I, A>))
            .route(&format!("{path}/data"), get(read::<I, A>));
    }
    routes
        .with_state(state.clone())
        .route_layer(middleware::from_fn_with_state(state, authorize::<I, A>))
        .route_layer(middleware::from_fn_with_state(
            session,
            oteryn_web::session::middleware,
        ))
        .layer(middleware::from_fn_with_state(pool, audit_boundary))
        .layer(middleware::from_fn_with_state(
            (host_config, true),
            crate::web::host_guard,
        ))
}
async fn audit_boundary(State(pool): State<PgPool>, mut request: Request, next: Next) -> Response {
    let context = AuditContext {
        actor: None,
        request_id: Uuid::now_v7(),
    };
    let attempt = Attempt {
        context: Arc::new(tokio::sync::Mutex::new(context)),
        recorded: Arc::new(AtomicBool::new(false)),
    };
    let target = request.uri().path().to_owned();
    request.extensions_mut().insert(attempt.clone());
    let mut response = next.run(request).await;
    if !attempt.recorded.load(Ordering::SeqCst) {
        let context = *attempt.context.lock().await;
        if let Some(actor) = context.actor
            && denial(&pool, actor, context.request_id, &target)
                .await
                .is_err()
        {
            response = StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    }
    if let Ok(id) = axum::http::HeaderValue::from_str(&context.request_id.to_string()) {
        response.headers_mut().insert("x-request-id", id);
    }
    response
}
async fn denial(
    pool: &PgPool,
    actor: Uuid,
    id: Uuid,
    target: &str,
) -> Result<(), crate::rbac::Error> {
    let mut tx = pool.begin().await?;
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(actor),
                request_id: id,
            },
            action: "admin.analytics.denied",
            target,
            before: Value::Null,
            after: json!({"result":"denied"}),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
async fn authorize<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Context<I, A>>>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(attempt) = request.extensions().get::<Attempt>().cloned() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let mut actor = None;
    let mut allowed = false;
    if let Some(session) = request.extensions().get::<BrowserSession>()
        && let Some(identity) = s.identity.resolve(session).await
    {
        actor = Some(identity.account);
        let context = CheckContext {
            account: identity.account,
            require_staff_mfa: true,
            mfa_verified: identity.mfa_satisfied,
        };
        allowed = identity.mfa_satisfied
            && s.auth.check(context, Permission::Access).await.is_ok()
            && s.auth
                .check(context, Permission::AdminAnalyticsView)
                .await
                .is_ok();
    }
    let context = {
        let mut context = attempt.context.lock().await;
        context.actor = actor;
        *context
    };
    if !allowed {
        return StatusCode::FORBIDDEN.into_response();
    }
    request.extensions_mut().insert(context);
    next.run(request).await
}
async fn read<I: StaffSession, A: AccountStateReader>(
    State(s): State<Arc<Context<I, A>>>,
    request: Request,
) -> Response {
    let Some(context) = request.extensions().get::<AuditContext>().copied() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    if request.uri().path().starts_with("/admin/analytics/economy") {
        return economy::read(s, request, context).await;
    }
    let selection = match selection(request.uri().query(), request.uri().path()) {
        Ok(selection) => selection,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let mut tx = match s.pool.begin().await {
        Ok(t) => t,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    if sqlx::query("SELECT pg_advisory_xact_lock(71140001)")
        .execute(&mut *tx)
        .await
        .is_err()
    {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    // Freshness starts at the locked read cut, after any commit/connection queue.
    let now = super::now();
    let mut cells = Vec::new();
    let mut covered = false;
    let mut provenance = Vec::new();
    if s.config.present
        && let (Some(world), Some(family), Some(start), Some(end)) = (
            selection.world,
            selection.family,
            selection.start,
            selection.end,
        )
        && let Ok(family_value) = serde_json::to_value(family)
        && let Some(family_name) = family_value.as_str()
    {
        let coverage=sqlx::query("SELECT b.bucket_start::text AS start,b.epoch::text,b.revision::text FROM admin_analytics.subjects b JOIN admin_analytics.fence f ON f.authority=b.authority AND f.epoch=b.epoch AND f.active_generation=b.generation JOIN admin_analytics.generations g ON g.id=b.generation AND g.accepted_at IS NOT NULL AND g.epoch=f.epoch JOIN admin_analytics.watermarks w ON w.authority=b.authority AND w.family=b.family AND w.world=b.world AND w.epoch=f.epoch WHERE b.authority=$1 AND b.family=$2 AND b.world=$3 AND b.bucket_start >= $4::text::numeric AND b.bucket_end <= $5::text::numeric AND b.privacy=$6 AND f.privacy_floor=$6 AND w.privacy=$6 AND NOT w.conflict AND NOT b.conflict AND NOT b.erased AND w.observed >= $7::text::numeric-120 AND w.observed <= $7::text::numeric+2 AND w.cut <= w.observed AND w.cut >= b.observed AND w.cut >= g.baseline_cut AND b.bucket_end+86400 <= $7::text::numeric ORDER BY b.bucket_start LIMIT 745")
            .bind(&s.config.authority).bind(family_name).bind(world).bind(start.to_string()).bind(end.to_string()).bind(&s.config.privacy).bind(now.to_string()).fetch_all(&mut *tx).await;
        covered = match coverage {
            Ok(records) => {
                for row in &records {
                    let values = (
                        row.try_get::<String, _>("start"),
                        row.try_get::<String, _>("epoch"),
                        row.try_get::<String, _>("revision"),
                    );
                    match values {
                        (Ok(start), Ok(epoch), Ok(revision)) => provenance
                            .push(json!({"bucket_start":start,"epoch":epoch,"revision":revision})),
                        _ => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
                    }
                }
                records.len() as u64 == (end - start) / family.interval()
            }
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        };
        let result=sqlx::query("SELECT r.key,r.state,r.values,r.bucket_start::text AS start,r.bucket_end::text AS end,r.epoch::text,r.revision::text,r.observed::text,w.observed::text AS watermark FROM admin_analytics.rows r JOIN admin_analytics.fence f ON f.authority=r.authority AND f.active_generation=r.generation AND f.epoch=r.epoch JOIN admin_analytics.generations g ON g.id=r.generation AND g.accepted_at IS NOT NULL AND g.epoch=f.epoch JOIN admin_analytics.watermarks w ON w.authority=r.authority AND w.family=r.family AND w.world=r.world AND w.epoch=f.epoch JOIN admin_analytics.subjects b ON b.authority=r.authority AND b.family=r.family AND b.world=r.world AND b.bucket_start=r.bucket_start AND b.epoch=r.epoch AND b.revision=r.revision AND b.generation=r.generation WHERE r.authority=$1 AND r.family=$2 AND r.world=$3 AND r.bucket_start >= $4::text::numeric AND r.bucket_end <= $5::text::numeric AND r.privacy=$6 AND f.privacy_floor=$6 AND w.privacy=$6 AND NOT w.conflict AND NOT b.conflict AND NOT b.erased AND w.observed >= $7::text::numeric-120 AND w.observed <= $7::text::numeric+2 AND w.cut <= w.observed AND w.cut >= r.observed AND w.cut >= g.baseline_cut AND r.bucket_end::numeric+86400 <= $7::text::numeric ORDER BY r.bucket_start,r.key LIMIT 8193")
                        .bind(&s.config.authority).bind(family_name).bind(world).bind(start.to_string()).bind(end.to_string()).bind(&s.config.privacy).bind(now.to_string()).fetch_all(&mut *tx).await;
        let records = match result {
            Ok(r) => r,
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        };
        if records.len() <= 8192 && covered {
            for record in records {
                let cell = (|| -> Result<Cell, sqlx::Error> {
                    let end: String = record.try_get("end")?;
                    let expired = wire::uint(&end)
                        .ok()
                        .and_then(|end| end.checked_add(family.retention()))
                        .is_none_or(|expiry| expiry <= now);
                    let state: String = if expired {
                        "expired".into()
                    } else {
                        record.try_get("state")?
                    };
                    let values = if state == "visible" {
                        let v: Value = record.try_get("values")?;
                        serde_json::from_value(v).map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                    } else {
                        Vec::new()
                    };
                    Ok(Cell {
                        key: record.try_get("key")?,
                        state,
                        values,
                        start: record.try_get("start")?,
                        end: end.clone(),
                        epoch: record.try_get("epoch")?,
                        revision: record.try_get("revision")?,
                        start_utc: utc(&record.try_get::<String, _>("start")?),
                        end_utc: utc(&end),
                        cut_utc: utc(&record.try_get::<String, _>("observed")?),
                        age: now.saturating_sub(
                            wire::uint(&record.try_get::<String, _>("watermark")?)
                                .map_err(|_| sqlx::Error::Protocol("invalid watermark".into()))?,
                        ),
                        observed: record.try_get("observed")?,
                    })
                })();
                match cell {
                    Ok(cell) => cells.push(cell),
                    Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
                }
            }
        } else {
            covered = false;
        }
    }
    let expired = covered
        && selection
            .family
            .zip(selection.end)
            .is_some_and(|(family, end)| {
                end.checked_add(family.retention())
                    .is_none_or(|expiry| expiry <= now)
            });
    let state = if !covered {
        "unavailable"
    } else if expired {
        "expired"
    } else {
        "available"
    };
    let available = covered;
    let metadata = json!({"result":state,"family":selection.family,"world":selection.world,"start":selection.start,"end":selection.end,"epoch":cells.first().map(|c|&c.epoch),"revision":cells.first().map(|c|&c.revision),"privacy_policy_revision":s.config.privacy,"source_revisions":if covered {provenance} else {Vec::new()}});
    if audit::append(
        &mut tx,
        Entry {
            context,
            action: "admin.analytics.read",
            target: request.uri().path(),
            before: Value::Null,
            after: metadata,
        },
    )
    .await
    .is_err()
        || tx.commit().await.is_err()
    {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    if let Some(attempt) = request.extensions().get::<Attempt>() {
        attempt.recorded.store(true, Ordering::SeqCst);
    }
    if request.uri().path().ends_with("/data") {
        return axum::Json(json!({"state":state,"cells":cells})).into_response();
    }
    let english = request
        .headers()
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("en"));
    let title = match (request.uri().path().trim_end_matches("/data"), english) {
        ("/admin/analytics/creatures", true) => "Creature and boss deaths by world",
        ("/admin/analytics/creatures", false) => "Zgony stworzeń i bossów według świata",
        ("/admin/analytics/activity", true) => "Hourly activity — exact ONLINE concurrency",
        ("/admin/analytics/activity", false) => "Aktywność godzinowa — dokładna liczba ONLINE",
        ("/admin/analytics/levels", true) => "Daily level distribution",
        ("/admin/analytics/levels", false) => "Dzienny rozkład poziomów",
        ("/admin/analytics/health", true) => "Analytics feed health",
        ("/admin/analytics/health", false) => "Stan źródła analitycznego",
        (_, true) => "Admin analytics overview",
        (_, false) => "Przegląd analityki administratora",
    };
    let page = Page {
        lang: if english { "en" } else { "pl" },
        title,
        unavailable: if expired {
            if english { "Expired" } else { "Wygasłe" }
        } else if available {
            if english {
                "No eligible cells — no zero inferred"
            } else {
                "Brak kwalifikujących się komórek — nie oznacza zera"
            }
        } else if english {
            "Unavailable — accepted resync baseline and fresh evidence required"
        } else {
            "Niedostępne — wymagana zaakceptowana baza synchronizacji i aktualne dane"
        },
        cut: if english { "Data cut" } else { "Stan danych" },
        range: if english { "UTC range" } else { "Zakres UTC" },
        freshness: if english { "Freshness" } else { "Aktualność" },
        epoch: if english {
            "Epoch / revision"
        } else {
            "Epoka / rewizja"
        },
        cells,
        world: selection.world.map(|v| v.to_string()).unwrap_or_default(),
        from: selection.start.map(input_time).unwrap_or_default(),
        to: selection.end.map(input_time).unwrap_or_default(),
        policy: s.config.privacy.clone(),
        family: selection
            .family
            .and_then(|f| serde_json::to_value(f).ok())
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default(),
    };
    match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
fn selection(query: Option<&str>, path: &str) -> Result<Selection, ()> {
    let query = query.unwrap_or_default();
    if query.len() > 512 {
        return Err(());
    }
    let mut selection: Selection = serde_urlencoded::from_str(query).map_err(|_| ())?;
    if selection.family.is_none() {
        selection.family = Some(if path.contains("/economy") {
            wire::Family::EconomyCirculationDaily
        } else if path.contains("/activity") {
            wire::Family::WorldActivityHourly
        } else if path.contains("/levels") {
            wire::Family::LevelDistributionDaily
        } else if path.contains("/creatures") {
            wire::Family::CreatureKillsDaily
        } else {
            wire::Family::CharactersCreatedDaily
        });
    }
    if selection.family.is_some_and(wire::Family::economy) != path.contains("/economy") {
        return Err(());
    }
    match (&selection.from, &selection.to) {
        (Some(from), Some(to)) if selection.start.is_none() && selection.end.is_none() => {
            selection.start = Some(parse_time(from)?);
            selection.end = Some(parse_time(to)?);
        }
        (None, None) => (),
        _ => return Err(()),
    }
    if selection.start.is_none() && selection.end.is_none() {
        let interval = selection.family.ok_or(())?.interval();
        let end = super::now().checked_sub(86400).ok_or(())? / interval * interval;
        selection.start = Some(end.checked_sub(interval).ok_or(())?);
        selection.end = Some(end);
    }
    if selection
        .world
        .is_some_and(|id| id.is_nil() || id.get_version_num() != 7)
    {
        return Err(());
    }
    if let (Some(start), Some(end)) = (selection.start, selection.end) {
        let family = selection.family.ok_or(())?;
        if start >= end
            || end - start > 31 * 86400
            || start % family.interval() != 0
            || end % family.interval() != 0
        {
            return Err(());
        }
    } else if selection.start.is_some() || selection.end.is_some() {
        return Err(());
    }
    Ok(selection)
}

fn parse_time(s: &str) -> Result<u64, ()> {
    if s.len() != 16 {
        return Err(());
    }
    let format =
        time::format_description::parse_borrowed::<2>("[year]-[month]-[day]T[hour]:[minute]")
            .map_err(|_| ())?;
    let date = time::PrimitiveDateTime::parse(s, &format).map_err(|_| ())?;
    u64::try_from(date.assume_utc().unix_timestamp()).map_err(|_| ())
}
fn input_time(n: u64) -> String {
    let Ok(n) = i64::try_from(n) else {
        return String::new();
    };
    let Ok(date) = time::OffsetDateTime::from_unix_timestamp(n) else {
        return String::new();
    };
    let Ok(format) =
        time::format_description::parse_borrowed::<2>("[year]-[month]-[day]T[hour]:[minute]")
    else {
        return String::new();
    };
    date.format(&format).unwrap_or_default()
}
fn utc(s: &str) -> String {
    wire::uint(s)
        .map(input_time)
        .map(|s| format!("{s} UTC"))
        .unwrap_or_default()
}
