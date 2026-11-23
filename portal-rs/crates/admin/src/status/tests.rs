use super::{
    store::{self, *},
    text,
};
use crate::{
    rbac::{Authorizer, CheckContext, Permission},
    web::PortalIdentity,
};
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use oteryn_web::ratelimit::{Keys, NamedPolicy, RateLimiter};
use serde_json::json;
use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions};
use std::{collections::BTreeSet, net::SocketAddr};
use time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

#[path = "../../../../test-support/rate_clock.rs"]
mod rate_clock;

#[test]
fn both_locales_define_the_same_labels() {
    let keys = |source: &str| {
        source
            .lines()
            .filter_map(|line| line.split_once(" = ").map(|(key, _)| key.to_owned()))
            .collect::<BTreeSet<_>>()
    };
    let pl = keys(include_str!("../../../../templates/status/pl.ftl"));
    let en = keys(include_str!("../../../../templates/status/en.ftl"));
    assert!(!pl.is_empty());
    assert_eq!(pl, en);
    for key in pl {
        assert!(!text("pl", &key).is_empty(), "{key}");
        assert!(!text("en", &key).is_empty(), "{key}");
    }
}

fn create(scope: &str, en: &str) -> Result<Command, serde_json::Error> {
    serde_json::from_value(json!({
        "scope": scope, "severity": "major", "title_pl": "Serwis", "title_en": "Service",
        "message_pl": "Zespół prowadzi analizę.", "message_en": en,
    }))
    .map(Command::Create)
}

fn transition(id: Uuid, transition: Transition) -> Result<Command, serde_json::Error> {
    serde_json::from_value(json!({
        "message_pl": "Aktualizacja zespołu", "message_en": "Team update",
    }))
    .map(|message| Command::Transition(id, transition, message))
}

fn window(scope: &str, starts: &str, ends: &str) -> Result<Command, serde_json::Error> {
    serde_json::from_value(json!({
        "scope": scope, "starts_at": starts, "ends_at": ends,
        "message_pl": "Aktualizacja serwera", "message_en": "Server update",
    }))
    .map(Command::Schedule)
}

async fn mutate(
    pool: &PgPool,
    context: CheckContext,
    auth: &Authorizer<PortalIdentity>,
    command: Command,
) -> Result<Uuid, Error> {
    store::mutate(pool, context, auth, Uuid::now_v7(), command).await
}

#[tokio::test]
async fn staff_actions_enforce_grants_lifecycle_limits_and_input()
-> Result<(), Box<dyn std::error::Error>> {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        return Ok(());
    };
    let root = PgPool::connect(&url).await?;
    let schema = format!("status_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let search_path = format!("{schema}, pg_catalog");
    let db = PgPoolOptions::new()
        .after_connect(move |db, _| {
            let path = search_path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(db)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&db).await?;
    let actor: Uuid = sqlx::query_scalar(
        "INSERT INTO accounts (id,email,password_hash,status,email_verified_at,locale)
         VALUES(uuidv7(),'status@example.test','test-only','active',now(),'en') RETURNING id",
    )
    .fetch_one(&db)
    .await?;
    let role: Uuid =
        sqlx::query_scalar("INSERT INTO roles (key) VALUES ('status_staff') RETURNING id")
            .fetch_one(&db)
            .await?;
    sqlx::query("INSERT INTO account_roles (account_id,role_id) VALUES ($1,$2)")
        .bind(actor)
        .bind(role)
        .execute(&db)
        .await?;
    sqlx::query("INSERT INTO role_permissions (role_id,permission) VALUES ($1,'admin.access'),($1,'status.incident.create')")
        .bind(role).execute(&db).await?;
    let context = CheckContext {
        account: actor,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: Some(crate::test_staff::proof(&db, actor).await?),
    };
    let auth = Authorizer::new(db.clone(), PortalIdentity::configured(db.clone()));
    let world: Uuid = sqlx::query_scalar(
        "INSERT INTO world_status (id,name) VALUES (uuidv7(),'Antica') RETURNING id",
    )
    .fetch_one(&db)
    .await?;
    for input in [
        create("", "     ")?,
        create("", "control\u{0}")?,
        create("", &"a".repeat(2001))?,
        create("wrong", "Valid")?,
        create(&Uuid::now_v7().to_string(), "Valid")?,
    ] {
        assert!(matches!(
            mutate(&db, context, &auth, input).await,
            Err(Error::Invalid)
        ));
    }
    let publication_key = Uuid::now_v7();
    let scope = world.to_string();
    let (first, second) = tokio::join!(
        store::mutate(
            &db,
            context,
            &auth,
            publication_key,
            create(&scope, "Investigating")?,
        ),
        store::mutate(
            &db,
            context,
            &auth,
            publication_key,
            create(&scope, "Investigating")?,
        ),
    );
    let id = first?;
    assert_eq!(second?, id);
    for action in [Transition::Update, Transition::Resolve] {
        assert!(matches!(
            mutate(&db, context, &auth, transition(id, action)?).await,
            Err(Error::Authorization(crate::rbac::Error::Denied))
        ));
    }
    for permission in [
        Permission::StatusIncidentUpdate,
        Permission::StatusIncidentResolve,
        Permission::StatusMaintenanceSchedule,
    ] {
        sqlx::query("INSERT INTO role_permissions (role_id,permission) VALUES ($1,$2)")
            .bind(role)
            .bind(permission.as_str())
            .execute(&db)
            .await?;
    }
    mutate(&db, context, &auth, transition(id, Transition::Update)?).await?;
    let mut connection = db.acquire().await?;
    let now = clock(&mut connection).await?;
    let rows = incidents(&mut connection, "pl", now, Incidents::Open).await?;
    assert_eq!(rows[0].title, "Serwis");
    assert_eq!(rows[0].message, "Aktualizacja zespołu");
    let current = components(&mut connection, now).await?;
    assert_eq!(current[0].state, "operational");
    assert_eq!(current[1].state, "major");

    sqlx::query(
        "INSERT INTO status_incident_updates (incident_id,sequence,kind,message_pl,message_en)
         SELECT $1,s,'updated','Treść','Message' FROM generate_series(3,99) s",
    )
    .bind(id)
    .execute(&mut *connection)
    .await?;
    assert!(matches!(
        mutate(&db, context, &auth, transition(id, Transition::Update)?).await,
        Err(Error::Conflict)
    ));
    mutate(&db, context, &auth, transition(id, Transition::Resolve)?).await?;
    assert!(matches!(
        mutate(&db, context, &auth, transition(id, Transition::Resolve)?).await,
        Err(Error::Conflict)
    ));
    let updates = timeline(&mut connection, id, "en").await?;
    assert_eq!(updates.len(), 100);
    assert_eq!(updates[0].kind, "resolved");
    assert_eq!(
        updates.last().map(|entry| entry.message.as_str()),
        Some("Investigating")
    );
    let date = |delta| iso(now + delta)[..16].to_owned();
    let start = date(Duration::hours(23));
    let end = date(Duration::hours(25));
    for input in [
        window("", &end, &start)?,
        window("", &date(Duration::days(-1)), &end)?,
        window("", &start, &date(Duration::days(35)))?,
        window("", "2026-02-30T11:00", &end)?,
    ] {
        assert!(matches!(
            mutate(&db, context, &auth, input).await,
            Err(Error::Invalid)
        ));
    }
    let maintenance_id = mutate(&db, context, &auth, window("", &start, &end)?).await?;
    let later = mutate(
        &db,
        context,
        &auth,
        window(&world.to_string(), &end, &date(Duration::hours(26)))?,
    )
    .await?;
    let notices = maintenance(&mut connection, "en", now, true).await?;
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].id, maintenance_id);
    assert!(notices[0].world.is_none());
    assert_eq!(
        maintenance(&mut connection, "pl", now, false).await?[1].id,
        later
    );
    let current = components(&mut connection, now + Duration::hours(24)).await?;
    assert_eq!(current[0].state, "operational");
    assert_eq!(current[1].state, "maintenance");

    let audit: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT request_id,action FROM audit_log WHERE actor=$1 ORDER BY created_at,id",
    )
    .bind(actor)
    .fetch_all(&db)
    .await?;
    assert_eq!(
        audit
            .iter()
            .map(|(_, action)| action.as_str())
            .collect::<Vec<_>>(),
        [
            "status.incident.create",
            "status.incident.update",
            "status.incident.resolve",
            "status.maintenance.schedule",
            "status.maintenance.schedule",
        ]
    );
    assert!(audit.iter().all(|(request, _)| !request.is_nil()));

    // Portal outages overlap across the lower edge of the reporting interval.
    sqlx::query(
        "INSERT INTO status_incidents (severity,title_pl,title_en,opened_at,resolved_at)
         VALUES ('major','A','A',$1 - interval '2161 hours',$1 - interval '2158 hours'),
                ('minor','B','B',$1 - interval '2159 hours',$1 - interval '2157 hours')",
    )
    .bind(now)
    .execute(&mut *connection)
    .await?;
    let current = components(&mut connection, now).await?;
    assert!((current[0].uptime - 100.0 * (1.0 - 3.0 / (90.0 * 24.0))).abs() < 0.00001);

    // The publication budget is shared across all status actions for this actor.
    sqlx::query(
        "INSERT INTO status_publications (actor_id, publication_key, command_digest, result_id)
         SELECT $1, uuidv7(), decode(repeat('00', 32), 'hex'), $2 FROM generate_series(1, 12)",
    )
    .bind(actor)
    .bind(id)
    .execute(&mut *connection)
    .await?;
    assert!(matches!(
        mutate(&db, context, &auth, create("", "Valid")?).await,
        Err(Error::Limited)
    ));
    let incident_count: i64 = sqlx::query_scalar("SELECT count(*) FROM status_incidents")
        .fetch_one(&mut *connection)
        .await?;
    assert_eq!(incident_count, 3);

    rate_clock::install(&db).await?;
    let limiter = RateLimiter::new(db.clone());
    let peer = SocketAddr::from(([192, 0, 2, 100], 49152));
    let policy = NamedPolicy::PublicGameData;
    let source = Keys::source(peer.ip());
    for _ in 0..policy.limit() {
        limiter.check(policy, std::slice::from_ref(&source)).await?;
    }
    let response = super::public::router(db.clone())
        .layer(axum::Extension(limiter))
        .oneshot(
            Request::get("/en/status/incidents.atom")
                .extension(ConnectInfo(peer))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers()[header::CONTENT_LANGUAGE], "en");
    assert!(response.headers().contains_key(header::RETRY_AFTER));

    drop(connection);
    db.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}
