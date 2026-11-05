#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    routing::get,
};
use oteryn_web::session::{self, BrowserSession, PrivilegeChange, SessionLayer};
use tower::ServiceExt;

#[test]
fn canonical_payload_success_refusal_and_bad_input() {
    let good = format!(
        r#"{{"login_event_id":"{}","entered_at":"1790000000000","left_at":null,"end_reason":null}}"#,
        Uuid::now_v7()
    );
    assert!(Login::parse(good.as_bytes()).is_ok());
    let invalid_id = good.replace(
        &serde_json::from_str::<serde_json::Value>(&good).unwrap()["login_event_id"]
            .as_str()
            .unwrap()
            .to_owned(),
        "01934f10-7c04-7001-005b-3b1122334413",
    );
    assert!(Login::parse(invalid_id.as_bytes()).is_err());
    for bad in [
        good.replace(
            "null,\"end_reason\":null",
            "\"1789999999999\",\"end_reason\":\"LOGOUT\"",
        ),
        good.replace("1790000000000", "01790000000000"),
        good.replace("1790000000000", "253402300800000"),
        good.replace("1790000000000", "-1"),
        good.replace("1790000000000", "1e3"),
        good.replace(",\"left_at\":null", ""),
        good.replace("\"end_reason\":null", "\"end_reason\":\"OTHER\""),
        good.replace("}", ",\"device\":\"x\"}"),
        good.replace("}", ",\"entered_at\":\"1\"}"),
        format!("{good}\n"),
    ] {
        assert!(Login::parse(bad.as_bytes()).is_err(), "accepted {bad}");
    }
}
#[test]
fn cut_denies_unknown_time_scope_and_recovery() {
    let cut = Cut {
        epoch: 1,
        recovery_generation: 1,
        complete_through: 0,
        observed_at_ms: 100,
        time_uncertainty_ms: 0,
        baseline_complete: true,
        visibility_floor_ms: 0,
        projection_revision: None,
        ownership_character_revision: None,
        world_id: None,
    };
    assert!(check_cut(&cut, None, 100).is_ok());
    assert!(check_cut(&cut, Some(Uuid::now_v7()), 100).is_err());
    for bad in [
        Cut {
            baseline_complete: false,
            ..cut.clone()
        },
        Cut {
            time_uncertainty_ms: 5001,
            ..cut.clone()
        },
        Cut {
            epoch: 0,
            ..cut.clone()
        },
        Cut {
            recovery_generation: 0,
            ..cut.clone()
        },
        Cut {
            observed_at_ms: -1,
            ..cut.clone()
        },
        Cut {
            observed_at_ms: 101,
            ..cut.clone()
        },
    ] {
        assert!(check_cut(&bad, None, 100).is_err());
    }
    assert!(check_cut(&cut, None, 86_400_101).is_err());
}

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED T80 PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = oteryn_db::lazy_pool(&url).unwrap();
        let schema = format!("logins_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |conn, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        Some(Self {
            pool,
            admin,
            schema,
        })
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await
        .unwrap();
        self.admin.close().await;
    }
}
#[derive(Debug)]
struct TestAuthority {
    account: Uuid,
    cut: Cut,
}
impl Authority for TestAuthority {
    fn current<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        account: Uuid,
        character: Option<Uuid>,
    ) -> AuthorityFuture<'a> {
        Box::pin(async move {
            let active: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM accounts WHERE id=$1 AND status='active' FOR UPDATE",
            )
            .bind(account)
            .fetch_optional(&mut **tx)
            .await?;
            if active != Some(self.account) || character.is_some() {
                return Err(Error::Denied);
            }
            Ok(self.cut.clone())
        })
    }
}
async fn account(pool: &PgPool) -> Uuid {
    sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,email_verified_at,locale) VALUES($1,'unused','active',now(),'en') RETURNING id").bind(format!("{}@example.invalid", Uuid::now_v7())).fetch_one(pool).await.unwrap()
}
async fn fixture(pool: &PgPool) -> (Logins, PromotedLogin) {
    let account = account(pool).await;
    let mut tx = pool.begin().await.unwrap();
    let now = now(&mut tx).await.unwrap();
    tx.rollback().await.unwrap();
    let cut = Cut {
        epoch: 1,
        recovery_generation: 1,
        complete_through: 10,
        observed_at_ms: now,
        time_uncertainty_ms: 0,
        baseline_complete: true,
        visibility_floor_ms: 0,
        projection_revision: None,
        ownership_character_revision: None,
        world_id: None,
    };
    let record = PromotedLogin {
        account,
        character: None,
        character_revision: None,
        cut: cut.clone(),
        source_revision: 1,
        login: Login {
            login_event_id: Uuid::now_v7(),
            entered_at: (now - 1000).to_string(),
            left_at: None,
            end_reason: None,
        },
    };
    let service = Logins {
        pool: pool.clone(),
        flags: Flags {
            progress_ingest_enabled: true,
            progress_read_enabled: true,
            game_login_history_enabled: true,
        },
        authority: Arc::new(TestAuthority { account, cut }),
    };
    (service, record)
}
#[tokio::test]
async fn postgres_owner_close_and_refusals() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (service, mut record) = fixture(&db.pool).await;
    let mut tx = db.pool.begin().await.unwrap();
    service.apply(&mut tx, &record).await.unwrap();
    tx.commit().await.unwrap();
    let history = service.history(record.account).await.unwrap();
    assert_eq!(history.len(), 1);
    assert!(history[0].left_at.is_none());
    let other = account(&db.pool).await;
    assert!(matches!(service.history(other).await, Err(Error::Denied)));
    let mut bad = record.clone();
    bad.login.left_at = Some("0".into());
    bad.login.end_reason = Some(EndReason::Logout);
    let mut tx = db.pool.begin().await.unwrap();
    assert!(matches!(
        service.apply(&mut tx, &bad).await,
        Err(Error::Invalid)
    ));
    tx.rollback().await.unwrap();
    bad = record.clone();
    bad.cut.epoch = 2;
    let mut tx = db.pool.begin().await.unwrap();
    assert!(matches!(
        service.apply(&mut tx, &bad).await,
        Err(Error::Conflict)
    ));
    tx.rollback().await.unwrap();
    record.source_revision = 2;
    record.login.left_at = Some(record.cut.observed_at_ms.to_string());
    record.login.end_reason = Some(EndReason::Logout);
    let mut tx = db.pool.begin().await.unwrap();
    service.apply(&mut tx, &record).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(service.history(record.account).await.unwrap().len(), 1);
    assert!(
        service.history(record.account).await.unwrap()[0]
            .left_at
            .is_some()
    );
    let mut tx = db.pool.begin().await.unwrap();
    assert!(matches!(
        service.apply(&mut tx, &record).await,
        Err(Error::Conflict)
    ));
    tx.rollback().await.unwrap();
    let disabled = Logins {
        flags: Flags::default(),
        ..service.clone()
    };
    assert!(matches!(
        disabled.history(record.account).await,
        Err(Error::Unavailable)
    ));
    let mut tx = db.pool.begin().await.unwrap();
    assert!(matches!(
        disabled.apply(&mut tx, &record).await,
        Err(Error::Unavailable)
    ));
    tx.rollback().await.unwrap();
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(record.account)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(matches!(
        service.history(record.account).await,
        Err(Error::Denied)
    ));
    db.close().await;
}
#[tokio::test]
async fn postgres_retention_removal_and_unknown_authority() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (service, mut record) = fixture(&db.pool).await;
    record.login.entered_at = (record.cut.observed_at_ms - 31 * 86_400_000).to_string();
    let mut tx = db.pool.begin().await.unwrap();
    service.apply(&mut tx, &record).await.unwrap();
    tx.commit().await.unwrap();
    assert!(service.history(record.account).await.unwrap().is_empty());
    record.login.entered_at = (record.cut.observed_at_ms - 1000).to_string();
    let mut tx = db.pool.begin().await.unwrap();
    service.apply(&mut tx, &record).await.unwrap();
    tx.commit().await.unwrap();
    let unavailable = Logins {
        authority: Arc::new(UnavailableAuthority),
        ..service.clone()
    };
    assert!(matches!(
        unavailable.history(record.account).await,
        Err(Error::Unavailable)
    ));
    sqlx::query("UPDATE pd_game_logins SET projection_epoch=2 WHERE account_id=$1")
        .bind(record.account)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(matches!(
        service.history(record.account).await,
        Err(Error::Unavailable)
    ));
    sqlx::query("UPDATE pd_game_logins SET entered_at=entered_at-2592000001 WHERE account_id=$1")
        .bind(record.account)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(service.history(record.account).await.unwrap().is_empty());
    assert_eq!(service.purge_expired().await.unwrap(), 1);
    let mut tx = db.pool.begin().await.unwrap();
    service.apply(&mut tx, &record).await.unwrap();
    Logins::remove(&mut tx, record.account, record.login.login_event_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(service.history(record.account).await.unwrap().is_empty());
    db.close().await;
}
#[tokio::test]
async fn postgres_browser_presentation_and_owner_denial() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (service, record) = fixture(&db.pool).await;
    let mut tx = db.pool.begin().await.unwrap();
    service.apply(&mut tx, &record).await.unwrap();
    tx.commit().await.unwrap();
    let owner = record.account;
    let layer = SessionLayer::new(db.pool.clone());
    let app = web::router(service, layer.clone()).merge(
        Router::new()
            .route(
                "/test-login",
                get(
                    move |Extension(session): Extension<BrowserSession>| async move {
                        session::rotate(&session, PrivilegeChange::Login(owner))
                            .await
                            .unwrap();
                        StatusCode::NO_CONTENT
                    },
                ),
            )
            .route_layer(axum::middleware::from_fn_with_state(
                layer,
                session::middleware,
            )),
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/account/game-logins")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/test-login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = login.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/en/account/game-logins")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let body = String::from_utf8(
        to_bytes(response.into_body(), 65536)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("<summary>This was not me</summary>"));
    assert!(body.contains("No sessions have been revoked."));
    assert!(body.contains("Unknown; this does not confirm an active session"));
    assert!(body.contains("width=device-width"));
    let polish = app
        .oneshot(
            Request::builder()
                .uri("/pl/account/game-logins")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(polish.status(), StatusCode::OK);
    assert_eq!(polish.headers()["content-language"], "pl");
    assert!(polish.headers().contains_key("content-security-policy"));
    let body =
        String::from_utf8(to_bytes(polish.into_body(), 65536).await.unwrap().to_vec()).unwrap();
    assert!(body.contains("To nie ja"));

    db.close().await;
}

#[derive(Debug)]
struct CharacterAuthority {
    owner: Uuid,
    character: Uuid,
    cut: Cut,
}
impl Authority for CharacterAuthority {
    fn current<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        account: Uuid,
        character: Option<Uuid>,
    ) -> AuthorityFuture<'a> {
        Box::pin(async move {
            if account != self.owner {
                return Err(Error::Denied);
            }
            match character {
                None => Ok(Cut {
                    projection_revision: None,
                    ownership_character_revision: None,
                    world_id: None,
                    ..self.cut.clone()
                }),
                Some(id) if id == self.character => Ok(self.cut.clone()),
                _ => Err(Error::Denied),
            }
        })
    }
}
#[tokio::test]
async fn postgres_character_ownership_cut_denies_transferred_history() {
    let Some(db) = Database::open().await else {
        return;
    };
    let (mut service, mut record) = fixture(&db.pool).await;
    let character = Uuid::now_v7();
    record.character = Some(character);
    record.character_revision = Some(8);
    record.cut.projection_revision = Some(4);
    record.cut.ownership_character_revision = Some(8);
    record.cut.world_id = Some(Uuid::now_v7());
    service.authority = Arc::new(CharacterAuthority {
        owner: record.account,
        character,
        cut: record.cut.clone(),
    });
    let mut tx = db.pool.begin().await.unwrap();
    service.apply(&mut tx, &record).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        service.history(record.account).await.unwrap()[0].character,
        Some(character)
    );
    let mut new_cut = record.cut.clone();
    new_cut.projection_revision = Some(5);
    new_cut.ownership_character_revision = Some(9);
    service.authority = Arc::new(CharacterAuthority {
        owner: record.account,
        character,
        cut: new_cut,
    });
    assert!(matches!(
        service.history(record.account).await,
        Err(Error::Unavailable)
    ));
    let buyer = account(&db.pool).await;
    assert!(matches!(service.history(buyer).await, Err(Error::Denied)));
    db.close().await;
}
