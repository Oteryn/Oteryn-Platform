//! Staff-only presentation and publication controls for immutable world versions.
use std::sync::Arc;

use crate::{WorldOptions, WorldStore};
use askama::Template;
use axum::{
    Extension, Form, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use oteryn_admin::{
    audit::{self, AuditContext, Entry},
    rbac::{AccountStateReader, Authorizer, CheckContext, Permission},
    web::{PanelConfig, PortalIdentity, StaffSession},
};
use oteryn_web::session::BrowserSession;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

struct Panel<A> {
    pool: PgPool,
    enabled: bool,
    auth: Arc<Authorizer<A>>,
}
/// Uses the same trusted session adapter and account-state reader as the staff panel.
/// The default adapter intentionally denies until identity integration is installed.
pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let enabled = std::env::var("OTERYN_WORLD_ADMIN").is_ok_and(|v| v == "true");
    if !enabled {
        return Ok(Router::new());
    }
    oteryn_admin::web::configured_guard(
        routes(
            pool.clone(),
            enabled,
            PortalIdentity::configured(pool.clone()),
        ),
        pool,
        Permission::GameCatalogAccess,
    )
}
/// Wrap routes in the shared staff host, session, CSRF, MFA and permission guard.
pub fn router<I: StaffSession, A: AccountStateReader + Clone + 'static>(
    pool: PgPool,
    config: PanelConfig,
    options: WorldOptions,
    identity: I,
    accounts: A,
) -> Router {
    if !options.enabled {
        return Router::new();
    }
    oteryn_admin::web::guarded(
        routes(pool.clone(), options.enabled, accounts.clone()),
        pool,
        config,
        identity,
        accounts,
        Permission::GameCatalogAccess,
    )
}
fn routes<A: AccountStateReader + 'static>(pool: PgPool, enabled: bool, accounts: A) -> Router {
    // Do not register game routes while their feature is disabled.
    if !enabled {
        return Router::new();
    }
    let state = Arc::new(Panel {
        auth: Arc::new(Authorizer::new(pool.clone(), accounts)),
        pool,
        enabled,
    });
    Router::new()
        .route("/admin/world", get(page::<A>).post(change::<A>))
        .with_state(state)
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    version: Option<Uuid>,
    #[serde(default, deserialize_with = "optional_uuid")]
    compare: Option<Uuid>,
    page: Option<u32>,
}
fn optional_uuid<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Uuid>, D::Error> {
    let value = String::deserialize(d)?;
    if value.is_empty() {
        Ok(None)
    } else {
        value.parse().map(Some).map_err(serde::de::Error::custom)
    }
}
#[derive(sqlx::FromRow)]
struct Version {
    id: Uuid,
    source_commit: String,
    status: String,
}
#[derive(sqlx::FromRow)]
struct Row {
    kind: String,
    key: String,
    allowed: bool,
    publishable: bool,
    pl: String,
    en: String,
}
#[derive(sqlx::FromRow)]
struct Difference {
    kind: String,
    key: String,
    before: String,
    after: String,
}
#[derive(Template)]
#[template(path = "admin/world/index.html", config = "src/admin_askama.toml")]
struct Page {
    lang: &'static str,
    labels: Labels,
    csrf: String,
    versions: Vec<Version>,
    selected: String,
    comparison: String,
    number: u32,
    active: String,
    rows: Vec<Row>,
    differences: Vec<Difference>,
    report: String,
    catalog: bool,
}
struct Labels {
    title: String,
    version: String,
    compare: String,
    inspect: String,
    active: String,
    publish: String,
    entities: String,
    allowed: String,
    names: String,
    save: String,
    validation: String,
    diff: String,
    before: String,
    after: String,
    limit: String,
    yes: String,
    no: String,
    polish: String,
    english: String,
    page: String,
}
fn text(lang: &str, key: &str) -> String {
    let Ok(locale) = lang.parse::<unic_langid::LanguageIdentifier>() else {
        return String::new();
    };
    let mut bundle = fluent_bundle::FluentBundle::new(vec![locale]);
    let source = if lang == "en" {
        include_str!("../../../templates/admin/world/en.ftl")
    } else {
        include_str!("../../../templates/admin/world/pl.ftl")
    };
    let Ok(resource) = fluent_bundle::FluentResource::try_new(source.to_owned()) else {
        return String::new();
    };
    if bundle.add_resource(resource).is_err() {
        return String::new();
    }
    bundle
        .get_message(key)
        .and_then(|m| m.value())
        .map(|p| bundle.format_pattern(p, None, &mut vec![]).into_owned())
        .unwrap_or_default()
}
async fn page<A: AccountStateReader + 'static>(
    State(s): State<Arc<Panel<A>>>,
    Extension(context): Extension<CheckContext>,
    Extension(session): Extension<BrowserSession>,
    headers: HeaderMap,
    Query(q): Query<Selection>,
) -> Response {
    if let Err(status) = authorize(&s, context, Permission::ViewGameCatalogSnapshots).await {
        return status.into_response();
    }
    match load_page(&s, session, headers, q).await {
        Ok(page) => oteryn_web::render(&page),
        Err(status) => status.into_response(),
    }
}
async fn load_page<A>(
    s: &Panel<A>,
    session: BrowserSession,
    headers: HeaderMap,
    q: Selection,
) -> Result<Page, StatusCode> {
    if q.compare.is_some() && (q.version.is_none() || q.version == q.compare) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let number = q.page.unwrap_or(1);
    if !(1..=10000).contains(&number) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let offset = i64::from(number - 1) * 200;
    let store = WorldStore::new(s.pool.clone(), WorldOptions { enabled: s.enabled });
    let legacy_active = store
        .active_version_id()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let catalog_active: Option<Uuid> = sqlx::query_scalar("SELECT v.version_id FROM world_catalog_publication p JOIN world_catalog_versions v USING(snapshot_id) WHERE p.singleton")
        .fetch_optional(&s.pool).await.map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
    let active = catalog_active.or(legacy_active);
    let selected = q.version.or(active);
    let catalog: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM world_catalog_versions WHERE version_id=$1)",
    )
    .bind(selected)
    .fetch_one(&s.pool)
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let versions = sqlx::query_as("SELECT id,source_commit,status FROM world_versions ORDER BY created_at DESC,id DESC LIMIT 100").fetch_all(&s.pool).await.map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
    let mut rows = Vec::new();
    let mut report = String::new();
    let mut differences = Vec::new();
    if let Some(id) = selected {
        let diagnostic = store
            .import_report(id)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .ok_or(StatusCode::NOT_FOUND)?;
        report = serde_json::to_string_pretty(&diagnostic)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        rows = sqlx::query_as("SELECT e.kind,e.key,COALESCE(v.allowed,false) AS allowed,e.publishable,COALESCE(n.pl,'') AS pl,COALESCE(n.en,'') AS en FROM world_entities e LEFT JOIN world_visibility v USING(kind,key) LEFT JOIN world_admin_names n USING(kind,key) WHERE e.version_id=$1 ORDER BY e.kind,e.key LIMIT 200 OFFSET $2").bind(id).bind(offset).fetch_all(&s.pool).await.map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
        if let Some(other) = q.compare {
            let comparison = store
                .import_report(other)
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .ok_or(StatusCode::NOT_FOUND)?;
            if comparison.status != crate::VersionStatus::Ready
                || diagnostic.status != crate::VersionStatus::Ready
            {
                return Err(StatusCode::CONFLICT);
            }
            differences = sqlx::query_as("SELECT COALESCE(a.kind,b.kind) AS kind,COALESCE(a.key,b.key) AS key,jsonb_build_object('data',a.data,'publishable',a.publishable)::text AS before,jsonb_build_object('data',b.data,'publishable',b.publishable)::text AS after FROM (SELECT * FROM world_entities WHERE version_id=$1) a FULL JOIN (SELECT * FROM world_entities WHERE version_id=$2) b USING(kind,key) WHERE a.data IS DISTINCT FROM b.data OR a.publishable IS DISTINCT FROM b.publishable ORDER BY kind,key LIMIT 200 OFFSET $3").bind(other).bind(id).bind(offset).fetch_all(&s.pool).await.map_err(|_|StatusCode::SERVICE_UNAVAILABLE)?;
        }
    }
    let lang = if headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("en"))
    {
        "en"
    } else {
        "pl"
    };
    macro_rules! labels { ($($key:ident),*) => { Labels { $($key: text(lang,stringify!($key))),* } }; }
    Ok(Page {
        catalog,
        lang,
        labels: labels!(
            title, version, compare, inspect, active, publish, entities, allowed, names, save,
            validation, diff, before, after, limit, yes, no, polish, english, page
        ),
        csrf: session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        versions,
        selected: selected.map(|v| v.to_string()).unwrap_or_default(),
        comparison: q.compare.map(|v| v.to_string()).unwrap_or_default(),
        number,
        active: active.map(|v| v.to_string()).unwrap_or_default(),
        rows,
        differences,
        report,
    })
}
#[derive(Deserialize)]
struct ChangeForm {
    action: String,
    version: Uuid,
    kind: Option<String>,
    key: Option<String>,
    allowed: Option<String>,
    pl: Option<String>,
    en: Option<String>,
}
async fn change<A: AccountStateReader + 'static>(
    State(s): State<Arc<Panel<A>>>,
    Extension(context): Extension<CheckContext>,
    Form(form): Form<ChangeForm>,
) -> Response {
    match mutate(&s, context, form).await {
        Ok(()) => Redirect::to("/admin/world").into_response(),
        Err(status) => status.into_response(),
    }
}
async fn mutate<A: AccountStateReader>(
    s: &Panel<A>,
    context: CheckContext,
    form: ChangeForm,
) -> Result<(), StatusCode> {
    let permission = match form.action.as_str() {
        "publish" => Permission::ActivateGameCatalogSnapshots,
        "visibility" => Permission::ManageGameCatalogProfiles,
        "names" => Permission::ManageGameCatalogTranslations,
        _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
    };
    authorize(s, context, Permission::ViewGameCatalogSnapshots).await?;
    authorize(s, context, permission).await?;
    if !s.enabled {
        return Err(StatusCode::NOT_FOUND);
    }
    let mut tx = s.pool.begin().await.map_err(unavailable)?;
    // Serialize with role writers, then recheck current grants before any change.
    sqlx::query("SELECT pg_advisory_xact_lock(71010)")
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    s.auth
        .check(context, permission)
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
    let (active,): (Option<Uuid>,) =
        sqlx::query_as("SELECT version_id FROM world_active_version WHERE singleton FOR UPDATE")
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM world_versions WHERE id=$1")
            .bind(form.version)
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?;
    let Some(status) = status else {
        return Err(StatusCode::NOT_FOUND);
    };
    if status != "ready" {
        return Err(StatusCode::CONFLICT);
    }
    let catalog_snapshot: Option<String> =
        sqlx::query_scalar("SELECT snapshot_id FROM world_catalog_versions WHERE version_id=$1")
            .bind(form.version)
            .fetch_optional(&mut *tx)
            .await
            .map_err(unavailable)?;
    let (catalog_active,): (Option<String>,) = sqlx::query_as(
        "SELECT snapshot_id FROM world_catalog_publication WHERE singleton FOR UPDATE",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(unavailable)?;
    // Native publication belongs exclusively to verified background refresh. Staff
    // may approve its records, but cannot route it through the legacy/map pointer
    // or author replacement Game names.
    if catalog_snapshot.is_some() && form.action != "visibility" {
        return Err(StatusCode::CONFLICT);
    }
    let (target, before, after) = if form.action == "publish" {
        if active == Some(form.version) {
            return Err(StatusCode::CONFLICT);
        }
        sqlx::query("UPDATE world_active_version SET version_id=$1,previous_version_id=$2,updated_at=now() WHERE singleton").bind(form.version).bind(active).execute(&mut *tx).await.map_err(unavailable)?;
        (
            form.version.to_string(),
            json!({"version":active}),
            json!({"version":form.version}),
        )
    } else {
        let current_catalog = catalog_snapshot.is_some() && catalog_snapshot == catalog_active;
        if active != Some(form.version) && !current_catalog {
            return Err(StatusCode::CONFLICT);
        }
        let kind = form
            .kind
            .as_deref()
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        let key = form
            .key
            .as_deref()
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
        crate::types::validate_label(kind, "kind").map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        crate::types::validate_label(key, "key").map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let publishable: Option<bool> = sqlx::query_scalar(
            "SELECT publishable FROM world_entities WHERE version_id=$1 AND kind=$2 AND key=$3",
        )
        .bind(form.version)
        .bind(kind)
        .bind(key)
        .fetch_optional(&mut *tx)
        .await
        .map_err(unavailable)?;
        let Some(publishable) = publishable else {
            return Err(StatusCode::NOT_FOUND);
        };
        let (before, after) = if form.action == "visibility" {
            let allowed = match form.allowed.as_deref() {
                Some("true") => true,
                Some("false") => false,
                _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
            };
            if allowed && !publishable {
                return Err(StatusCode::FORBIDDEN);
            }
            let before: Option<bool> =
                sqlx::query_scalar("SELECT allowed FROM world_visibility WHERE kind=$1 AND key=$2")
                    .bind(kind)
                    .bind(key)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(unavailable)?;
            sqlx::query("INSERT INTO world_visibility(kind,key,allowed) VALUES($1,$2,$3) ON CONFLICT(kind,key) DO UPDATE SET allowed=EXCLUDED.allowed,updated_at=now()").bind(kind).bind(key).bind(allowed).execute(&mut *tx).await.map_err(unavailable)?;
            (json!(before), json!(allowed))
        } else {
            let pl = form.pl.as_deref().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            let en = form.en.as_deref().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            for name in [pl, en] {
                crate::types::validate_label(name, "name")
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                if name.trim().is_empty() {
                    return Err(StatusCode::UNPROCESSABLE_ENTITY);
                }
            }
            let before: Option<(String, String)> =
                sqlx::query_as("SELECT pl,en FROM world_admin_names WHERE kind=$1 AND key=$2")
                    .bind(kind)
                    .bind(key)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(unavailable)?;
            sqlx::query("INSERT INTO world_admin_names(kind,key,pl,en) VALUES($1,$2,$3,$4) ON CONFLICT(kind,key) DO UPDATE SET pl=EXCLUDED.pl,en=EXCLUDED.en,updated_at=now()").bind(kind).bind(key).bind(pl).bind(en).execute(&mut *tx).await.map_err(unavailable)?;
            (json!(before), json!({"pl":pl,"en":en}))
        };
        (
            form.version.to_string(),
            json!({"kind":kind,"key":key,"value":before}),
            json!({"kind":kind,"key":key,"value":after}),
        )
    };
    audit::append(
        &mut tx,
        Entry {
            context: AuditContext {
                actor: Some(context.account),
                request_id: Uuid::now_v7(),
            },
            action: &format!("world.{}", form.action),
            target: &target,
            before,
            after,
        },
    )
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    tx.commit().await.map_err(unavailable)?;
    Ok(())
}
async fn authorize<A: AccountStateReader>(
    s: &Panel<A>,
    context: CheckContext,
    permission: Permission,
) -> Result<(), StatusCode> {
    match s.auth.check(context, permission).await {
        Ok(()) => Ok(()),
        Err(oteryn_admin::rbac::Error::Storage(_)) => Err(StatusCode::SERVICE_UNAVAILABLE),
        Err(_) => {
            let mut tx = s.pool.begin().await.map_err(unavailable)?;
            audit::append(
                &mut tx,
                Entry {
                    context: AuditContext {
                        actor: Some(context.account),
                        request_id: Uuid::now_v7(),
                    },
                    action: "world.access.denied",
                    target: "/admin/world",
                    before: json!(null),
                    after: json!({"allowed":false,"permission":permission.as_str()}),
                },
            )
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            tx.commit().await.map_err(unavailable)?;
            Err(StatusCode::FORBIDDEN)
        }
    }
}
fn unavailable(_: sqlx::Error) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}

#[cfg(test)]
#[path = "admin_tests.rs"]
mod tests;
