use super::*;
use crate::audit06_test_db::{Database, Result};
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use oteryn_admin::rbac::{AccountState, AccountStateReader, AccountStateUnavailable, Authorizer};
use std::sync::Arc;
use tower::ServiceExt;

#[derive(Clone)]
struct ActiveAccount;
impl AccountStateReader for ActiveAccount {
    async fn state(&self, _: Uuid) -> std::result::Result<AccountState, AccountStateUnavailable> {
        Ok(AccountState::Active)
    }

    async fn authenticated_in(
        &self,
        _: &mut sqlx::PgConnection,
        context: CheckContext,
    ) -> std::result::Result<bool, AccountStateUnavailable> {
        Ok(context.mfa_verified)
    }
}

async fn register_world(db: &PgPool) -> Result<Uuid> {
    Ok(sqlx::query_scalar(
        "INSERT INTO world_status(id,name) VALUES(uuidv7(),'Test World') RETURNING id",
    )
    .fetch_one(db)
    .await?)
}

#[tokio::test]
async fn publishes_and_resolves_bilingual_notice_with_audit() -> Result {
    let db = Database::new().await?;
    let world = register_world(&db.db).await?;
    let context = CheckContext {
        account: Uuid::now_v7(),
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: None,
    };
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'status@example.invalid','test','active','en',now())")
        .bind(context.account).execute(&db.db).await?;
    let auth = MutationAuthorization::new(Arc::new(Authorizer::new(db.db.clone(), ActiveAccount)));
    let en = "Temporary interruption <retry later>";
    let pl = "Przejściowa przerwa";
    let open = || Change::Open { en, pl };
    assert_eq!(
        change(&db.db, &auth, context, world, open()).await,
        Err(StatusCode::FORBIDDEN)
    );
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'")
        .bind(context.account).execute(&db.db).await?;

    assert_eq!(
        change(&db.db, &auth, context, Uuid::now_v7(), open()).await,
        Err(StatusCode::NOT_FOUND)
    );
    let (first, second) = tokio::join!(
        change(&db.db, &auth, context, world, open()),
        change(&db.db, &auth, context, world, open()),
    );
    assert!(matches!(
        (first, second),
        (Ok(()), Err(StatusCode::CONFLICT)) | (Err(StatusCode::CONFLICT), Ok(()))
    ));

    let app = crate::worlds::router(
        db.db.clone(),
        crate::worlds::Config {
            enabled: true,
            ..Default::default()
        },
    );
    for (lang, message) in [
        ("en", "Temporary interruption &#60;retry later&#62;"),
        ("pl", pl),
    ] {
        let response = app
            .clone()
            .oneshot(Request::get(format!("/{lang}/worlds/{world}/uptime")).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(to_bytes(response.into_body(), 100_000).await?.to_vec())?;
        assert!(html.contains(message));
    }

    let id: Uuid = sqlx::query_scalar("SELECT id FROM world_incidents WHERE world_id=$1")
        .bind(world)
        .fetch_one(&db.db)
        .await?;
    let other_world = register_world(&db.db).await?;
    assert_eq!(
        change(&db.db, &auth, context, other_world, Change::Resolve(id)).await,
        Err(StatusCode::NOT_FOUND)
    );
    assert_eq!(
        change(&db.db, &auth, context, world, Change::Resolve(id)).await,
        Ok(())
    );
    assert_eq!(
        change(&db.db, &auth, context, world, Change::Resolve(id)).await,
        Err(StatusCode::CONFLICT)
    );
    let page = history_page(&db.db, Language::En, world, History { before: None })
        .await
        .map_err(|status| format!("history returned {status}"))?;
    assert!(page.incidents[0].resolved.is_some());
    assert!(page.can_publish);
    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_log WHERE actor=$1 AND target=$2 ORDER BY created_at,id",
    )
    .bind(context.account)
    .bind(id.to_string())
    .fetch_all(&db.db)
    .await?;
    assert_eq!(
        actions,
        [
            "portal.world_incident.published",
            "portal.world_incident.resolved"
        ]
    );
    db.close().await
}

#[tokio::test]
async fn validates_registry_and_history_paths_and_paginates_notices() -> Result {
    let db = Database::new().await?;
    let world = register_world(&db.db).await?;
    let enabled = crate::worlds::Config {
        enabled: true,
        ..Default::default()
    };
    let app = crate::worlds::router(db.db.clone(), enabled);
    let disabled = crate::worlds::router(db.db.clone(), crate::worlds::Config::default());
    let path = |world| format!("/en/worlds/{world}/uptime");
    for (app, uri, status) in [
        (disabled, path(world), StatusCode::NOT_FOUND),
        (app.clone(), path(Uuid::now_v7()), StatusCode::NOT_FOUND),
        (
            app.clone(),
            format!("/de/worlds/{world}/uptime"),
            StatusCode::NOT_FOUND,
        ),
        (
            app.clone(),
            "/en/worlds/bad/uptime".into(),
            StatusCode::BAD_REQUEST,
        ),
        (
            app.clone(),
            format!("{}?before=bad", path(world)),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let response = app.oneshot(Request::get(uri).body(Body::empty())?).await?;
        assert_eq!(response.status(), status);
    }

    sqlx::query(
        "INSERT INTO world_incidents(world_id,summary_en,summary_pl,opened_at,resolved_at)
        SELECT $1,'Notice '||i,'Komunikat '||i,now(),now() FROM generate_series(1,$2) i",
    )
    .bind(world)
    .bind((PAGE_SIZE * 2 + 1) as i32)
    .execute(&db.db)
    .await?;
    let mut before = None;
    let mut ids = Vec::new();
    for size in [PAGE_SIZE, PAGE_SIZE, 1] {
        let page = history_page(
            &db.db,
            Language::En,
            world,
            History {
                before: before.clone(),
            },
        )
        .await
        .map_err(|status| format!("history returned {status}"))?;
        assert_eq!(page.incidents.len(), size);
        if let Some(cursor) = &before {
            let response = app
                .clone()
                .oneshot(
                    Request::get(format!("{}?before={cursor}", path(world))).body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
            let html = String::from_utf8(to_bytes(response.into_body(), 100_000).await?.to_vec())?;
            assert!(html.contains(&format!("/pl/worlds/{world}/uptime?before={cursor}")));
        }
        ids.extend(page.incidents.into_iter().map(|notice| notice.id));
        before = page.next.map(|cursor| cursor.to_string());
    }
    assert!(before.is_none());
    assert!(ids.windows(2).all(|pair| pair[0] > pair[1]));
    db.close().await
}

#[tokio::test]
async fn incident_change_budget_is_per_staff_account() -> Result {
    let db = Database::new().await?;
    let actor = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO web_rate_limits(limiter_name,key_hash,window_start,count)
         SELECT 'world-incident-change', sha256(uuid_send($1)),
                date_trunc('minute',statement_timestamp()) + minutes * interval '1 minute', $2
         FROM generate_series(0,1) AS windows(minutes)",
    )
    .bind(actor)
    .bind(CHANGES_PER_MINUTE)
    .execute(&db.db)
    .await?;
    assert!(matches!(
        charge_change(&db.db, actor).await,
        Err(Denial::Limited {
            retry_after: 1..=60
        })
    ));
    assert!(charge_change(&db.db, Uuid::now_v7()).await.is_ok());
    db.close().await
}

#[test]
fn validates_both_incident_translations() {
    let notice = |summary_en: String, summary_pl: &str| Notice {
        _csrf: String::new(),
        summary_en,
        summary_pl: summary_pl.into(),
    };
    for input in [
        notice(" ".into(), "Przerwa"),
        notice("Notice".into(), ""),
        notice("a".repeat(241), "Przerwa"),
        notice("Control\0character".into(), "Przerwa"),
    ] {
        assert_eq!(input.summaries(), Err(StatusCode::BAD_REQUEST));
    }
    let input = notice(" Notice ".into(), " Przerwa ");
    assert_eq!(input.summaries(), Ok(("Notice", "Przerwa")));
}
