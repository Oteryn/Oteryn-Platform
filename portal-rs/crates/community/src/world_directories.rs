//! World catalogue selection and staff diagnostics for the G-4 readers.
use askama::Template;
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use fluent_bundle::{FluentBundle, FluentResource};
use oteryn_admin::{rbac::Permission, web};
use sqlx::PgPool;
use uuid::{Uuid, Variant};

// G-4 guild and house projections are candidate contracts pending Game qualification.
pub(crate) const CONTRACT_ACCEPTED: bool = false;

pub(crate) fn guilds_enabled(get: impl Fn(&str) -> Option<String>) -> bool {
    oteryn_core::restricted_environment(&get)
        && get("PORTAL_GUILDS_ENABLED").as_deref() == Some("true")
}

pub(crate) fn houses_enabled(get: impl Fn(&str) -> Option<String>) -> bool {
    oteryn_core::restricted_environment(&get)
        && get("PORTAL_PUBLIC_PROJECTIONS_ENABLED").as_deref() == Some("true")
        && get("PORTAL_HOUSES_SERVING_ENABLED").as_deref() == Some("true")
}

pub(crate) fn world_id(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|id| {
        id.get_version_num() == 7 && id.get_variant() == Variant::RFC4122 && id.to_string() == value
    })
}

#[derive(Default)]
pub(crate) struct WorldPicker {
    worlds: Vec<(String, String)>,
    selected: Option<String>,
}

impl WorldPicker {
    pub(crate) async fn load(db: &PgPool, selected: Option<&str>) -> Result<Self, StatusCode> {
        let mut worlds: Vec<(String, String)> = sqlx::query_as(
            "SELECT id::text, name FROM world_status ORDER BY name COLLATE \"C\", id LIMIT 257",
        )
        .fetch_all(db)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        if worlds.len() > 256 {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        worlds.retain(|(id, _)| world_id(id));
        let picker = Self {
            worlds,
            selected: selected.map(str::to_owned),
        };
        if selected.is_some_and(|id| picker.name(id).is_none()) {
            return Err(StatusCode::NOT_FOUND);
        }
        Ok(picker)
    }

    pub(crate) fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    pub(crate) fn name(&self, world: &str) -> Option<&str> {
        self.worlds
            .iter()
            .find(|(id, _)| id == world)
            .map(|(_, name)| name.as_str())
    }

    pub(crate) fn options(&self) -> &[(String, String)] {
        &self.worlds
    }

    pub(crate) fn is_selected(&self, world: &str) -> bool {
        self.selected() == Some(world)
    }

    pub(crate) fn selected_name(&self) -> &str {
        self.selected()
            .and_then(|world| self.name(world))
            .unwrap_or_default()
    }
}

#[derive(Clone)]
struct Status {
    db: PgPool,
    authority: String,
    guilds_enabled: bool,
    houses_enabled: bool,
}

fn status_routes(state: Status) -> Router {
    Router::new()
        .route("/admin/{lang}/world-directories", get(status_page))
        .with_state(state)
}

/// Reports the local projection watermarks and rollout gates to authorized world operators.
pub fn directory_status(db: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    directory_status_from(db, &|key| std::env::var(key).ok())
}

fn directory_status_from(
    db: PgPool,
    get: &dyn Fn(&str) -> Option<String>,
) -> Result<Router, Box<dyn std::error::Error>> {
    let routes = status_routes(Status {
        db: db.clone(),
        authority: get("PORTAL_PUBLIC_PROJECTIONS_AUTHORITY").unwrap_or_default(),
        guilds_enabled: guilds_enabled(get),
        houses_enabled: houses_enabled(get),
    });
    web::configured_guard_from(routes, db, Permission::GameWorldRead, get)
}

#[derive(Template)]
#[template(
    path = "world_directories/status.html",
    config = "src/guilds_askama.toml"
)]
struct StatusPage {
    lang: String,
    messages: FluentBundle<FluentResource>,
    contract_accepted: bool,
    guilds_enabled: bool,
    houses_enabled: bool,
    feeds: Vec<(&'static str, &'static str)>,
}

impl StatusPage {
    fn text(&self, key: &str) -> String {
        self.messages
            .get_message(key)
            .and_then(|message| message.value())
            .map(|pattern| {
                self.messages
                    .format_pattern(pattern, None, &mut vec![])
                    .into_owned()
            })
            .unwrap_or_default()
    }

    fn setting(&self, enabled: &bool) -> String {
        self.text(if *enabled { "enabled" } else { "disabled" })
    }
}

async fn status_page(State(state): State<Status>, Path(lang): Path<String>) -> Response {
    let (locale, source) = match lang.as_str() {
        "en" => (
            unic_langid::langid!("en"),
            include_str!("../../../templates/world_directories/en.ftl"),
        ),
        "pl" => (
            unic_langid::langid!("pl"),
            include_str!("../../../templates/world_directories/pl.ftl"),
        ),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let result = feed_status(&state.db, &state.authority).await;
    let mut messages = FluentBundle::new(vec![locale]);
    if let Ok(resource) = FluentResource::try_new(source.to_owned()) {
        let _ = messages.add_resource(resource);
    }
    let status = if result.is_ok() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let page = StatusPage {
        lang,
        messages,
        contract_accepted: CONTRACT_ACCEPTED,
        guilds_enabled: state.guilds_enabled,
        houses_enabled: state.houses_enabled,
        feeds: result.unwrap_or_default(),
    };
    match page.render() {
        Ok(body) => (status, Html(body)).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn feed_status(
    db: &PgPool,
    authority: &str,
) -> Result<Vec<(&'static str, &'static str)>, sqlx::Error> {
    let cuts: Vec<(bool, Option<i64>)> = sqlx::query_as(
        "SELECT public_projection_conflicted(feed.family), COALESCE(w.watermark, h.complete_through)
         FROM (VALUES (1, 'guild', 'v2'), (2, 'character_profile', ''), (3, 'house', '')) AS feed(ord, family, subject)
         LEFT JOIN public_projection_state s ON s.id=1
         LEFT JOIN public_snapshots w ON feed.family<>'house' AND w.family=feed.family
           AND w.subject=feed.subject AND w.is_watermark AND w.epoch=s.highest_epoch
           AND w.payload->>'source_authority'=$1
         LEFT JOIN public_houses_v2_watermark h ON feed.family='house' AND h.id=1
           AND h.epoch=s.highest_epoch AND h.source_authority=$1
         ORDER BY feed.ord",
    )
    .bind(authority)
    .fetch_all(db)
    .await?;
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    Ok(["guilds", "profiles", "houses"]
        .into_iter()
        .zip(cuts)
        .map(|(name, (conflict, through))| (name, feed_state(now, conflict, through)))
        .collect())
}

fn feed_state(now: i64, conflicted: bool, through: Option<i64>) -> &'static str {
    match through {
        _ if conflicted => "held",
        None => "missing",
        Some(time) if time > 0 && time <= now + 1 && time >= now - 29 => "current",
        Some(_) => "outside-window",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn operator_status_reports_candidate_and_current_epoch() -> crate::audit06_test_db::Result
    {
        use axum::{
            body::{Body, to_bytes},
            http::Request,
        };
        use tower::ServiceExt;

        let database = crate::audit06_test_db::Database::new().await?;
        let db = &database.db;
        let authority = "oteryn:game:primary";
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        sqlx::query("UPDATE public_projection_state SET highest_epoch=2 WHERE id=1")
            .execute(db)
            .await?;
        sqlx::query(
            "INSERT INTO public_houses_v2_watermark(id,source_authority,epoch,complete_through)
             VALUES (1,$1,2,$2)",
        )
        .bind(authority)
        .bind(now - 40)
        .execute(db)
        .await?;
        sqlx::query(
            "INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload)
             VALUES ('guild','v2',true,2,1,$2,jsonb_build_object('source_authority',$1::text)),
                    ('character_profile','',true,1,1,$2,jsonb_build_object('source_authority',$1::text))",
        )
        .bind(authority)
        .bind(now)
        .execute(db)
        .await?;
        let state = Status {
            db: db.clone(),
            authority: authority.into(),
            guilds_enabled: true,
            houses_enabled: false,
        };
        let routes = status_routes(state);
        let guard_config = |key: &str| match key {
            "OTERYN_ADMIN_PANEL" => Some("true".into()),
            "OTERYN_ADMIN_HOST" => Some("staff.example.test".into()),
            _ => None,
        };
        let guarded = directory_status_from(db.clone(), &guard_config)?;
        let path = "/admin/en/world-directories";
        let wrong_host = guarded
            .clone()
            .oneshot(
                Request::get(path)
                    .header("host", "public.example.test")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(wrong_host.status(), StatusCode::NOT_FOUND);
        let anonymous = guarded
            .oneshot(
                Request::get(path)
                    .header("host", "staff.example.test")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(anonymous.status(), StatusCode::SEE_OTHER);

        let response = routes
            .oneshot(Request::get(path).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
        assert!(html.contains("Awaiting Game qualification"));
        assert!(html.contains("<dt>Guilds</dt><dd>Current</dd>"));
        assert!(html.contains("<dt>Houses</dt><dd>Outside the freshness window</dd>"));
        assert!(html.contains("<dt>Character profiles</dt><dd>Missing from the configured authority or current epoch</dd>"));
        database.close().await
    }

    #[test]
    fn completeness_states_follow_the_serving_window() {
        assert_eq!(feed_state(100, false, Some(71)), "current");
        assert_eq!(feed_state(100, false, Some(70)), "outside-window");
        assert_eq!(feed_state(100, false, Some(101)), "current");
        assert_eq!(feed_state(100, false, Some(102)), "outside-window");
        assert_eq!(feed_state(100, false, None), "missing");
        assert_eq!(feed_state(100, true, Some(100)), "held");
    }
}
