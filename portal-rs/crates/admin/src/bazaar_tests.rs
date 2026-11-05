#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::rbac::{AccountState, AccountStateUnavailable};
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use tower::ServiceExt;
#[derive(Debug, Clone)]
struct Identity(Uuid);
impl AccountStateReader for Identity {
    async fn state(&self, _: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        Ok(AccountState::Active)
    }
}
impl StaffSession for Identity {
    async fn resolve(&self, _: &oteryn_web::session::BrowserSession) -> Option<web::StaffIdentity> {
        Some(web::StaffIdentity {
            account: self.0,
            mfa_satisfied: true,
        })
    }
}
struct Db {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    actor: Uuid,
}
impl Db {
    async fn new() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED T64 staff PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("t64_staff_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .after_connect(move |c, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(format!("SET search_path TO {path}")))
                        .execute(c)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        for script in [
            include_str!("../../../migrations/20261105144852_t02_web_sessions.sql"),
            include_str!("../../../migrations/20261105170000_t10_rbac_audit.sql"),
            include_str!("../../../migrations/20261105152830_t57_bazaar_core.sql"),
            include_str!("../../../migrations/20261105230107_t64_moderation.sql"),
        ] {
            sqlx::raw_sql(script).execute(&pool).await.unwrap();
        }
        let actor = Uuid::now_v7();
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).await.unwrap();
        Some(Self {
            pool,
            admin,
            schema,
            actor,
        })
    }
    fn service(&self) -> StaffBazaar<Identity> {
        StaffBazaar::new(self.pool.clone(), Identity(self.actor), true)
    }
    fn context(&self) -> CheckContext {
        CheckContext {
            account: self.actor,
            require_staff_mfa: true,
            mfa_verified: true,
        }
    }
    async fn listing(&self) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at) VALUES($1,$2,'item','auction','coins',100,false,'active',clock_timestamp()+interval '2 days')").bind(id).bind(Uuid::now_v7()).execute(&self.pool).await.unwrap();
        id
    }
    fn app(&self) -> Router {
        router(
            self.pool.clone(),
            PanelConfig {
                admin_panel: true,
                host: "staff.test".into(),
            },
            Identity(self.actor),
            Identity(self.actor),
            true,
        )
    }
    async fn end(self) {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await
        .unwrap();
        self.admin.close().await;
    }
}
fn suspend(listing: Uuid) -> StaffCommand {
    StaffCommand::Suspend {
        listing,
        reason: "Verified moderation reason".into(),
    }
}
#[test]
fn unknown_action_and_bad_numbers_deny() {
    assert!(
        Input {
            action: "unknown".into(),
            ..Input::default()
        }
        .command()
        .is_err()
    );
    assert!(
        Input {
            action: "fees".into(),
            id: Uuid::now_v7().to_string(),
            coin_bps: "no".into(),
            ..Input::default()
        }
        .command()
        .is_err()
    );
}
#[tokio::test]
async fn postgres_staff_authorization_mfa_and_audit() {
    let Some(db) = Db::new().await else { return };
    let listing = db.listing().await;
    let api = db.service();
    let mut context = db.context();
    context.mfa_verified = false;
    assert!(matches!(
        api.execute(context, suspend(listing)).await,
        Err(Error::Authorization(_))
    ));
    context = db.context();
    context.account = Uuid::now_v7();
    assert!(matches!(
        api.execute(context, suspend(listing)).await,
        Err(Error::Authorization(_))
    ));
    api.execute(db.context(), suspend(listing)).await.unwrap();
    let (actor, before, after): (Uuid, Value, Value) =
        sqlx::query_as("SELECT actor,before,after FROM audit_log WHERE action='bazaar.suspend'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(actor, db.actor);
    assert!(before["hold"].is_null());
    assert_eq!(after["hold"]["revision"], 1);
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(db.actor)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(api.execute(db.context(), suspend(listing)).await.is_err());
    db.end().await;
}
#[tokio::test]
async fn postgres_audit_failure_rolls_back_hold() {
    let Some(db) = Db::new().await else { return };
    let listing = db.listing().await;
    sqlx::raw_sql("CREATE FUNCTION reject_fixture_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture audit unavailable'; END; $$; CREATE TRIGGER fixture_audit_fail BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION reject_fixture_audit();").execute(&db.pool).await.unwrap();
    assert!(
        db.service()
            .execute(db.context(), suspend(listing))
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_moderation_holds")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    db.end().await;
}
#[tokio::test]
async fn postgres_staff_http_actor_to_result_csrf_and_localization() {
    let Some(db) = Db::new().await else { return };
    let listing = db.listing().await;
    let app = db.app();
    for lang in ["pl", "en"] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/admin/{lang}/bazaar"))
                    .header("host", "staff.test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response
            .headers()
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let html = String::from_utf8(
            to_bytes(response.into_body(), 100000)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(html.contains("width=device-width"));
        assert!(html.contains("for=\"suspend-listing\""));
        assert!(html.contains(&Locale::parse(lang).unwrap().text("staff")));
        std::fs::write(format!("/tmp/t64-staff-{lang}.html"), &html).unwrap();
        let token = html
            .split("name=\"_csrf\" value=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        let body = format!(
            "action=suspend&listing={listing}&reason=Verified+moderation+reason&_csrf={token}"
        );
        let bad = app
            .clone()
            .oneshot(
                Request::post(format!("/admin/{lang}/bazaar"))
                    .header("host", "staff.test")
                    .header("cookie", &cookie)
                    .header("origin", "https://staff.test")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!("action=suspend&listing={listing}")))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad.status(), StatusCode::FORBIDDEN);
        let result = app
            .clone()
            .oneshot(
                Request::post(format!("/admin/{lang}/bazaar"))
                    .header("host", "staff.test")
                    .header("cookie", &cookie)
                    .header("origin", "https://staff.test")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::SEE_OTHER);
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='bazaar.suspend'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(count, 2);
    let response = app
        .oneshot(
            Request::get("/admin/en/bazaar")
                .header("host", "wrong.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    db.end().await;
}
#[tokio::test]
async fn postgres_staff_four_eyes_requires_second_live_permission() {
    let Some(db) = Db::new().await else { return };
    let listing = db.listing().await;
    let api = db.service();
    api.execute(db.context(), suspend(listing)).await.unwrap();
    let id = Uuid::now_v7();
    api.execute(
        db.context(),
        StaffCommand::RequestInvalidation {
            id,
            listing,
            reason: "Confirmed invalid listing".into(),
        },
    )
    .await
    .unwrap();
    assert!(
        api.execute(db.context(), StaffCommand::ApproveInvalidation { id })
            .await
            .is_err()
    );
    let second = Uuid::now_v7();
    let context = CheckContext {
        account: second,
        ..db.context()
    };
    assert!(
        api.execute(context, StaffCommand::ApproveInvalidation { id })
            .await
            .is_err()
    );
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(second).execute(&db.pool).await.unwrap();
    api.execute(context, StaffCommand::ApproveInvalidation { id })
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_moderation_outbox")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    db.end().await;
}
