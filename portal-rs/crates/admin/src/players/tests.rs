#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::{
    rbac::{AccountState, AccountStateUnavailable},
    web::{PanelConfig, StaffIdentity, StaffSession},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_web::session::BrowserSession;
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use tower::ServiceExt;

#[derive(Clone)]
struct Staff;
impl AccountStateReader for Staff {
    async fn state(&self, _: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        Ok(AccountState::Active)
    }
}
#[derive(Clone)]
struct Backend {
    id: Uuid,
}
impl Accounts for Backend {
    async fn search(&self, _: &str) -> Result<Vec<Account>, Error> {
        Ok(vec![self.account(self.id).await?.unwrap()])
    }
    async fn account(&self, id: Uuid) -> Result<Option<Account>, Error> {
        Ok((id == self.id).then_some(Account {
            id,
            status: "active".into(),
            locale: "pl".into(),
            email_verified: true,
        }))
    }
    async fn sessions(&self, _: Uuid) -> Result<Vec<Session>, Error> {
        Ok(vec![Session {
            kind: "web".into(),
            expires_at: "2030-01-01".into(),
        }])
    }
    async fn characters(&self, _: Uuid) -> Result<Vec<Character>, Error> {
        Ok(vec![Character {
            id: Uuid::now_v7(),
            name: "Example".into(),
            world: Uuid::now_v7(),
        }])
    }
    async fn verified(
        &self,
        c: &mut PgConnection,
        id: Uuid,
        evidence: Uuid,
    ) -> Result<bool, Error> {
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM test_security WHERE account_id=$1 AND evidence=$2 AND verified AND NOT reset)").bind(id).bind(evidence).fetch_one(c).await?)
    }
    async fn revoke_all(&self, c: &mut PgConnection, id: Uuid) -> Result<(), Error> {
        sqlx::query("UPDATE test_security SET revoked=true WHERE account_id=$1")
            .bind(id)
            .execute(c)
            .await?;
        Ok(())
    }
    async fn reset_mfa(&self, c: &mut PgConnection, id: Uuid, evidence: Uuid) -> Result<(), Error> {
        if !self.verified(c, id, evidence).await? {
            return Err(Error::Denied);
        }
        sqlx::query(
            "UPDATE test_security SET reset=true,revoked=true,verified=false WHERE account_id=$1",
        )
        .bind(id)
        .execute(c)
        .await?;
        Ok(())
    }
}
fn context(id: Uuid) -> CheckContext {
    CheckContext {
        account: id,
        require_staff_mfa: true,
        mfa_verified: true,
    }
}
struct Db {
    pool: PgPool,
    root: PgPool,
    schema: String,
    target: Uuid,
    support: Uuid,
    approver: Uuid,
    evidence: Uuid,
}
impl Db {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let root = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("t49_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await
            .unwrap();
        let path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(6)
            .after_connect(move |c, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(path)).execute(c).await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        sqlx::query("CREATE TABLE test_security(account_id uuid PRIMARY KEY,evidence uuid,verified boolean,reset boolean DEFAULT false,revoked boolean DEFAULT false)").execute(&pool).await.unwrap();
        let db = Self {
            pool,
            root,
            schema,
            target: Uuid::now_v7(),
            support: Uuid::now_v7(),
            approver: Uuid::now_v7(),
            evidence: Uuid::now_v7(),
        };
        sqlx::query("INSERT INTO test_security(account_id,evidence,verified) VALUES($1,$2,true)")
            .bind(db.target)
            .bind(db.evidence)
            .execute(&db.pool)
            .await
            .unwrap();
        for (id, role) in [(db.support, "support"), (db.approver, "platform_admin")] {
            sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2").bind(id).bind(role).execute(&db.pool).await.unwrap();
        }
        Some(db)
    }
    fn service(&self) -> Players<Staff, Backend> {
        Players::new(
            self.pool.clone(),
            Flags {
                players: true,
                characters: true,
            },
            Staff,
            Backend { id: self.target },
        )
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await
        .unwrap();
        self.root.close().await;
    }
}
#[test]
fn rejects_bad_input_and_defaults_off() {
    assert!(!Flags::default().players);
    assert!(!Flags::default().characters);
    assert!(valid_id(Uuid::nil()).is_err());
    for value in ["", " \t\n", "bad\0input", "too long"] {
        assert!(valid_text(value, 4).is_err());
    }
    assert!(valid_text("valid", 5).is_ok());
}
#[tokio::test]
async fn postgres_read_note_and_logout_success_refusal_bad_input() {
    let Some(db) = Db::open().await else {
        return;
    };
    let s = db.service();
    let c = context(db.support);
    assert_eq!(s.search(c, "example").await.unwrap().len(), 1);
    assert_eq!(s.account(c, db.target).await.unwrap().id, db.target);
    assert_eq!(s.sessions(c, db.target).await.unwrap().len(), 1);
    assert_eq!(s.characters(c, db.target).await.unwrap().len(), 1);
    s.note(c, db.target, "<script>example</script>")
        .await
        .unwrap();
    assert_eq!(
        s.notes(c, db.target).await.unwrap(),
        vec!["<script>example</script>"]
    );
    assert!(matches!(
        s.note(c, db.target, " ").await,
        Err(Error::BadInput)
    ));
    assert!(s.account(context(Uuid::now_v7()), db.target).await.is_err());
    assert!(matches!(
        s.account(c, Uuid::now_v7()).await,
        Err(Error::NotFound)
    ));
    s.logout_all(c, db.target, "Support request").await.unwrap();
    let revoked: bool = sqlx::query_scalar("SELECT revoked FROM test_security")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert!(revoked);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action LIKE 'players.%'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(count, 7);
    db.close().await;
}
#[tokio::test]
async fn postgres_reset_requires_two_current_staff_and_is_single_use() {
    let Some(db) = Db::open().await else {
        return;
    };
    let s = db.service();
    let request = s
        .request_reset(
            context(db.support),
            db.target,
            db.evidence,
            "Verified support case",
        )
        .await
        .unwrap();
    assert!(
        s.approve_reset(context(db.support), db.target, request)
            .await
            .is_err()
    );
    assert!(
        s.approve_reset(context(db.approver), db.target, Uuid::now_v7())
            .await
            .is_err()
    );
    s.approve_reset(context(db.approver), db.target, request)
        .await
        .unwrap();
    assert!(
        s.approve_reset(context(db.approver), db.target, request)
            .await
            .is_err()
    );
    let state: (bool, bool, bool) =
        sqlx::query_as("SELECT reset,revoked,verified FROM test_security")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(state, (true, true, false));
    db.close().await;
}
#[tokio::test]
async fn postgres_reset_refuses_missing_evidence_expiry_and_revoked_requester() {
    let Some(db) = Db::open().await else {
        return;
    };
    let s = db.service();
    let c = context(db.support);
    assert!(
        s.request_reset(c, db.target, Uuid::now_v7(), "Reason")
            .await
            .is_err()
    );
    assert!(
        s.request_reset(c, db.target, db.evidence, "")
            .await
            .is_err()
    );
    let request = s
        .request_reset(c, db.target, db.evidence, "Reason")
        .await
        .unwrap();
    sqlx::query("UPDATE player_mfa_reset_requests SET expires_at=now()-interval '1 second'")
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(
        s.approve_reset(context(db.approver), db.target, request)
            .await
            .is_err()
    );
    let request = s
        .request_reset(c, db.target, db.evidence, "Reason")
        .await
        .unwrap();
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(db.support)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(
        s.approve_reset(context(db.approver), db.target, request)
            .await
            .is_err()
    );
    let reset: bool = sqlx::query_scalar("SELECT reset FROM test_security")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert!(!reset);
    db.close().await;
}
#[tokio::test]
async fn postgres_audit_failure_rolls_back_notes_logout_and_reset() {
    let Some(db) = Db::open().await else {
        return;
    };
    let s = db.service();
    let request = s
        .request_reset(context(db.support), db.target, db.evidence, "Reason")
        .await
        .unwrap();
    sqlx::query("CREATE FUNCTION reject_test_audit() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RAISE EXCEPTION ''test audit unavailable''; END'").execute(&db.pool).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_test_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION reject_test_audit()").execute(&db.pool).await.unwrap();
    assert!(
        s.note(context(db.support), db.target, "example")
            .await
            .is_err()
    );
    assert!(
        s.logout_all(context(db.support), db.target, "Reason")
            .await
            .is_err()
    );
    assert!(
        s.approve_reset(context(db.approver), db.target, request)
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM player_notes")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let state: (bool, bool) = sqlx::query_as("SELECT reset,revoked FROM test_security")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(state, (false, false));
    db.close().await;
}
#[tokio::test]
async fn postgres_concurrent_approvals_execute_once() {
    let Some(db) = Db::open().await else {
        return;
    };
    let s = db.service();
    let request = s
        .request_reset(context(db.support), db.target, db.evidence, "Reason")
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        s.approve_reset(context(db.approver), db.target, request),
        s.approve_reset(context(db.approver), db.target, request)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action='players.mfa_reset.completed'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
    db.close().await;
}
#[derive(Clone)]
struct Browser(Uuid);
impl StaffSession for Browser {
    async fn resolve(&self, _: &BrowserSession) -> Option<StaffIdentity> {
        Some(StaffIdentity {
            account: self.0,
            mfa_satisfied: true,
        })
    }
}
#[tokio::test]
async fn postgres_http_actor_search_detail_csrf_note_and_refusals() {
    let Some(db) = Db::open().await else {
        return;
    };
    let app = http::router(
        db.pool.clone(),
        Flags {
            players: true,
            characters: true,
        },
        PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Browser(db.support),
        Staff,
        Backend { id: db.target },
    );
    let path = format!("/admin/players/{}", db.target);
    let response = app
        .clone()
        .oneshot(
            Request::get(&path)
                .header("host", "staff.test")
                .header("accept-language", "en")
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
    let body = String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    let csrf = body
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    for (token, status) in [
        ("wrong", StatusCode::FORBIDDEN),
        (csrf, StatusCode::SEE_OTHER),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post(&path)
                    .header("host", "staff.test")
                    .header("origin", "https://staff.test")
                    .header("cookie", &cookie)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!(
                        "_csrf={token}&action=note&body=Example"
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    for (uri, host, status) in [
        ("/admin/players?q=example", "staff.test", StatusCode::OK),
        (
            "/admin/players?q=%00",
            "staff.test",
            StatusCode::BAD_REQUEST,
        ),
        ("/admin/players", "wrong.test", StatusCode::NOT_FOUND),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::get(uri)
                    .header("host", host)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    db.close().await;
}
#[tokio::test]
async fn postgres_flags_and_pending_security_fail_closed() {
    let Some(db) = Db::open().await else {
        return;
    };
    let s = Players::new(
        db.pool.clone(),
        Flags::default(),
        Staff,
        Backend { id: db.target },
    );
    assert!(matches!(
        s.account(context(db.support), db.target).await,
        Err(Error::NotFound)
    ));
    let s = Players::new(
        db.pool.clone(),
        Flags {
            players: true,
            characters: false,
        },
        Staff,
        Backend { id: db.target },
    );
    assert!(matches!(
        s.characters(context(db.support), db.target).await,
        Err(Error::NotFound)
    ));
    assert!(
        s.account(CheckContext::new(db.support), db.target)
            .await
            .is_err()
    );
    let adapter = IdentityAccounts {
        accounts: oteryn_identity::accounts::Accounts {
            pool: db.pool.clone(),
        },
        projection: None,
    };
    let mut tx = db.pool.begin().await.unwrap();
    assert!(adapter.sessions(db.target).await.is_err());
    assert!(adapter.characters(db.target).await.is_err());
    assert!(adapter.revoke_all(&mut tx, db.target).await.is_err());
    assert!(
        adapter
            .reset_mfa(&mut tx, db.target, db.evidence)
            .await
            .is_err()
    );
    assert!(
        adapter
            .verified(&mut tx, db.target, db.evidence)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    db.close().await;
}

#[tokio::test]
async fn postgres_real_identity_lookup_safe_projection_and_missing_character_watermark() {
    let Some(db) = Db::open().await else {
        return;
    };
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'player@example.invalid','fixture-credential-never-exposed','active','en',now())").bind(db.target).execute(&db.pool).await.unwrap();
    let adapter = IdentityAccounts {
        accounts: oteryn_identity::accounts::Accounts {
            pool: db.pool.clone(),
        },
        projection: Some(projection::configured(db.pool.clone(), true)),
    };
    let account = adapter
        .search("PLAYER@example.invalid")
        .await
        .unwrap()
        .remove(0);
    assert_eq!(account.id, db.target);
    let debug = format!("{account:?}");
    assert!(!debug.contains("example.invalid"));
    assert!(!debug.contains("fixture-credential"));
    assert!(adapter.search("partial").await.is_err());
    assert!(
        adapter
            .search("missing@example.invalid")
            .await
            .unwrap()
            .is_empty()
    );
    assert!(adapter.characters(db.target).await.is_err());
    sqlx::query("UPDATE accounts SET status='closed' WHERE id=$1")
        .bind(db.target)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(adapter.characters(db.target).await.is_err());
    db.close().await;
}
async fn kick_fixture(db: &Db) -> Uuid {
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'kick-target@example.invalid','synthetic-hash','active','en',now())").bind(db.target).execute(&db.pool).await.unwrap();
    let kicker = Uuid::now_v7();
    for role in ["support", "moderator"] {
        sqlx::query(
            "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2",
        )
        .bind(kicker)
        .bind(role)
        .execute(&db.pool)
        .await
        .unwrap();
    }
    kicker
}
async fn kick_state(db: &Db) -> (i64, String, String, i64) {
    let (generation, status, hash): (i64, String, String) = sqlx::query_as(
        "SELECT native_security_generation,status,password_hash FROM accounts WHERE id=$1",
    )
    .bind(db.target)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    let records: i64 =
        sqlx::query_scalar("SELECT count(*) FROM identity_security_revocations WHERE cause=4")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    (generation, status, hash, records)
}
#[tokio::test]
async fn postgres_kick_permission_mfa_bad_input_audit_and_stream() {
    let Some(db) = Db::open().await else {
        return;
    };
    let kicker = kick_fixture(&db).await;
    let s = db.service();
    // Support staff lack the dedicated permission.
    assert!(matches!(
        s.kick(context(db.support), db.target, "Reason").await,
        Err(Error::Authorization(crate::rbac::Error::Denied))
    ));
    // Confirmed staff MFA is mandatory even outside HTTP.
    for unverified in [
        CheckContext::new(kicker),
        CheckContext {
            account: kicker,
            require_staff_mfa: true,
            mfa_verified: false,
        },
    ] {
        assert!(matches!(
            s.kick(unverified, db.target, "Reason").await,
            Err(Error::Denied)
        ));
    }
    for (target, reason) in [
        (db.target, " "),
        (db.target, "bad\0reason"),
        (Uuid::nil(), "Reason"),
    ] {
        assert!(matches!(
            s.kick(context(kicker), target, reason).await,
            Err(Error::BadInput)
        ));
    }
    assert!(matches!(
        s.kick(context(kicker), Uuid::now_v7(), "Reason").await,
        Err(Error::NotFound)
    ));
    assert_eq!(
        kick_state(&db).await,
        (1, "active".into(), "synthetic-hash".into(), 0)
    );
    assert_eq!(
        s.kick(context(kicker), db.target, " Abusive session ")
            .await
            .unwrap(),
        2
    );
    // Only the generation advanced; one cause-4 record carries the new value.
    assert_eq!(
        kick_state(&db).await,
        (2, "active".into(), "synthetic-hash".into(), 1)
    );
    let record: (Uuid, i64, i16) = sqlx::query_as(
        "SELECT account_id,native_security_generation,cause FROM identity_security_revocations",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(record, (db.target, 2, 4));
    let audit: (Option<Uuid>, String, serde_json::Value, bool) = sqlx::query_as(
        "SELECT actor,target,after,created_at<=now() FROM audit_log WHERE action='players.kick'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(
        audit,
        (
            Some(kicker),
            db.target.to_string(),
            json!({"reason":"Abusive session","native_security_generation":2}),
            true
        )
    );
    let sanctions: i64 = sqlx::query_scalar("SELECT count(*) FROM sanctions")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(sanctions, 0);
    db.close().await;
}
#[tokio::test]
async fn postgres_kick_audit_failure_rolls_back_generation_and_stream() {
    let Some(db) = Db::open().await else {
        return;
    };
    let kicker = kick_fixture(&db).await;
    sqlx::query("CREATE FUNCTION reject_test_audit() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RAISE EXCEPTION ''test audit unavailable''; END'").execute(&db.pool).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_test_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION reject_test_audit()").execute(&db.pool).await.unwrap();
    assert!(
        db.service()
            .kick(context(kicker), db.target, "Reason")
            .await
            .is_err()
    );
    assert_eq!(
        kick_state(&db).await,
        (1, "active".into(), "synthetic-hash".into(), 0)
    );
    db.close().await;
}
#[derive(Clone)]
struct KickBrowser {
    account: Uuid,
    mfa: bool,
}
impl StaffSession for KickBrowser {
    async fn resolve(&self, _: &BrowserSession) -> Option<StaffIdentity> {
        Some(StaffIdentity {
            account: self.account,
            mfa_satisfied: self.mfa,
        })
    }
}
async fn kick_session(app: axum::Router, path: &str) -> (String, String) {
    let response = app
        .oneshot(
            Request::get(path)
                .header("host", "staff.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let body = String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("value=\"kick\""));
    assert!(body.contains("for=\"kick-reason\""));
    let csrf = body
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned();
    (cookie, csrf)
}
async fn kick_post(app: axum::Router, path: &str, cookie: &str, body: String) -> StatusCode {
    app.oneshot(
        Request::post(path)
            .header("host", "staff.test")
            .header("origin", "https://staff.test")
            .header("cookie", cookie)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap(),
    )
    .await
    .unwrap()
    .status()
}
#[tokio::test]
async fn postgres_http_kick_csrf_permission_and_mfa() {
    let Some(db) = Db::open().await else {
        return;
    };
    let kicker = kick_fixture(&db).await;
    let path = format!("/admin/players/{}", db.target);
    let app = |account: Uuid, mfa: bool| {
        http::router(
            db.pool.clone(),
            Flags {
                players: true,
                characters: false,
            },
            PanelConfig {
                admin_panel: true,
                host: "staff.test".into(),
            },
            KickBrowser { account, mfa },
            Staff,
            Backend { id: db.target },
        )
    };
    // Support staff reach the page but lack the kick permission.
    let (cookie, csrf) = kick_session(app(db.support, true), &path).await;
    assert_eq!(
        kick_post(
            app(db.support, true),
            &path,
            &cookie,
            format!("_csrf={csrf}&action=kick&reason=Reason")
        )
        .await,
        StatusCode::FORBIDDEN
    );
    let (cookie, csrf) = kick_session(app(kicker, true), &path).await;
    for (body, status) in [
        (
            "_csrf=wrong&action=kick&reason=Reason".to_owned(),
            StatusCode::FORBIDDEN,
        ),
        (
            format!("_csrf={csrf}&action=kick&reason="),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        assert_eq!(
            kick_post(app(kicker, true), &path, &cookie, body).await,
            status
        );
    }
    // Without confirmed staff MFA the guard refuses before the handler.
    assert_eq!(
        kick_post(
            app(kicker, false),
            &path,
            &cookie,
            format!("_csrf={csrf}&action=kick&reason=Reason")
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        kick_state(&db).await,
        (1, "active".into(), "synthetic-hash".into(), 0)
    );
    assert_eq!(
        kick_post(
            app(kicker, true),
            &path,
            &cookie,
            format!("_csrf={csrf}&action=kick&reason=Abusive+session")
        )
        .await,
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        kick_state(&db).await,
        (2, "active".into(), "synthetic-hash".into(), 1)
    );
    db.close().await;
}
