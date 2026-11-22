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
    // Synthetic session authority for this fixture; production denies missing proofs.
    async fn authenticated_in(
        &self,
        connection: &mut sqlx::PgConnection,
        context: oteryn_admin::rbac::CheckContext,
    ) -> std::result::Result<bool, oteryn_admin::rbac::AccountStateUnavailable> {
        if let Some(proof) = context.session_proof {
            oteryn_admin::rbac::durable_session_in(connection, context.account, proof).await
        } else {
            Ok(!context.require_staff_mfa || context.mfa_verified)
        }
    }

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
        session_proof: None,
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
                coin_holds: true,
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
    assert!(!Flags::default().coin_holds);
    assert!(valid_id(Uuid::nil()).is_err());
    for value in ["", " \t\n", "bad\0input", "too long"] {
        assert!(valid_text(value, 4).is_err());
    }
    assert!(valid_text("valid", 5).is_ok());
}
#[tokio::test]
async fn postgres_read_note_and_logout_success_refusal_bad_input() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
async fn reset_approval_requires_support_permission() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else { return };
    let s = db.service();
    let request = s
        .request_reset(context(db.support), db.target, db.evidence, "Verified case")
        .await
        .unwrap();
    let security = Uuid::now_v7();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='security_admin'").bind(security).execute(&db.pool).await.unwrap();
    let result = s.approve_reset(context(security), db.target, request).await;
    let state: (bool, bool, bool) =
        sqlx::query_as("SELECT reset,revoked,verified FROM test_security WHERE account_id=$1")
            .bind(db.target)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    db.close().await;
    assert!(
        matches!(
            result,
            Err(Error::Authorization(crate::rbac::Error::Denied))
        ),
        "security-only approver cleared MFA"
    );
    assert_eq!(state, (false, false, true));
}
#[tokio::test]
async fn postgres_reset_requires_two_current_staff_and_is_single_use() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
#[derive(Clone)]
struct DeniedCharacters(Backend);
impl Accounts for DeniedCharacters {
    async fn search(&self, query: &str) -> Result<Vec<Account>, Error> {
        self.0.search(query).await
    }
    async fn account(&self, id: Uuid) -> Result<Option<Account>, Error> {
        let mut account = self.0.account(id).await?;
        if let Some(account) = &mut account {
            account.status = "closed".into();
            account.email_verified = false;
        }
        Ok(account)
    }
    async fn sessions(&self, id: Uuid) -> Result<Vec<Session>, Error> {
        self.0.sessions(id).await
    }
    async fn characters(&self, _: Uuid) -> Result<Vec<Character>, Error> {
        Err(Error::Denied)
    }
    async fn verified(
        &self,
        c: &mut PgConnection,
        id: Uuid,
        evidence: Uuid,
    ) -> Result<bool, Error> {
        self.0.verified(c, id, evidence).await
    }
    async fn revoke_all(&self, c: &mut PgConnection, id: Uuid) -> Result<(), Error> {
        self.0.revoke_all(c, id).await
    }
    async fn reset_mfa(&self, c: &mut PgConnection, id: Uuid, evidence: Uuid) -> Result<(), Error> {
        self.0.reset_mfa(c, id, evidence).await
    }
}
#[tokio::test]
async fn postgres_http_detail_keeps_account_and_notes_when_characters_denied() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = Db::open()
        .await
        .expect("PostgreSQL is required for this regression");
    db.service()
        .note(
            context(db.support),
            db.target,
            "Support note for closed account",
        )
        .await
        .unwrap();
    let app = http::router(
        db.pool.clone(),
        Flags {
            players: true,
            characters: true,
            coin_holds: false,
        },
        PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        Browser(db.support),
        Staff,
        DeniedCharacters(Backend { id: db.target }),
    );
    let response = app
        .oneshot(
            Request::get(format!("/admin/players/{}", db.target))
                .header("host", "staff.test")
                .header("accept-language", "en")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let target = db.target.to_string();
    let status = response.status();
    let body = String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    db.close().await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Support note for closed account"));
    assert!(body.contains(&format!("<h2>{target}</h2>")));
    assert!(body.contains("<dd>Closed</dd>"));
    assert!(body.contains("<h2>Characters</h2><p>This information is currently unavailable.</p>"));
}
#[tokio::test]
async fn postgres_http_actor_search_detail_csrf_note_and_refusals() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else {
        return;
    };
    sqlx::query(
        "INSERT INTO accounts(id,email,password_hash) VALUES($1,'note@example.test','test-only')",
    )
    .bind(db.target)
    .execute(&db.pool)
    .await
    .unwrap();
    let app = http::router(
        db.pool.clone(),
        Flags {
            players: true,
            characters: true,
            coin_holds: false,
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
                        "_csrf={token}&action=note&body=Example&reason=Verified+support+case"
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
            coin_holds: false,
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else {
        return;
    };
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'player@example.invalid','fixture-credential-never-exposed','active','en',now())").bind(db.target).execute(&db.pool).await.unwrap();
    let adapter = IdentityAccounts {
        accounts: oteryn_identity::accounts::Accounts {
            pool: db.pool.clone(),
        },
        projection: Some(projection::configured(db.pool.clone(), true, &|_| None)),
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
            session_proof: None,
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
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
                coin_holds: false,
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
/// A hold on `db.target` for a synthetic processing order (no wallet effect).
async fn coin_hold_fixture(db: &Db) -> Uuid {
    let quote = Uuid::now_v7();
    let order = Uuid::now_v7();
    sqlx::query("INSERT INTO coin_purchase_quotes(id,payer_account_id,package_id,coins,minor_units,currency,exponent,environment,expires_at,consumed_at) VALUES($1,$2,$3,500,499,'EUR',2,'testing',now(),now())")
        .bind(quote).bind(db.target).bind(Uuid::now_v7()).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO coin_purchase_orders(id,payer_account_id,request_id,quote_id,package_id,coins,minor_units,currency,exponent,environment,payment_intent_id,state,review_required,code) VALUES($1,$2,$3,$4,$5,500,499,'EUR',2,'testing',$6,'processing',true,'COIN_ORDER_REVERSAL_REVIEW_REQUIRED')")
        .bind(order).bind(db.target).bind(Uuid::now_v7()).bind(quote).bind(Uuid::now_v7()).bind(Uuid::now_v7()).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO coin_purchase_reversals(id,order_id,payer_account_id,kind,provider,provider_event_id,coins,debited,outcome) VALUES($1,$2,$3,'chargeback','testing','evt-hold',500,0,'held')")
        .bind(Uuid::now_v7()).bind(order).bind(db.target).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO coin_purchase_holds(account_id,order_id,reason) VALUES($1,$2,'reversal_insufficient_balance')")
        .bind(db.target).bind(order).execute(&db.pool).await.unwrap();
    order
}
async fn hold_identity(db: &Db) -> Uuid {
    sqlx::query_scalar("SELECT hold_id FROM coin_purchase_holds WHERE account_id=$1 UNION ALL SELECT hold_id FROM coin_purchase_hold_releases WHERE account_id=$1 LIMIT 1")
        .bind(db.target).fetch_one(&db.pool).await.unwrap()
}
/// (holds, release history rows, release audit rows)
async fn coin_hold_state(db: &Db) -> (i64, i64, i64) {
    let mut counts = [0; 3];
    for (count, sql) in counts.iter_mut().zip([
        "SELECT count(*) FROM coin_purchase_holds",
        "SELECT count(*) FROM coin_purchase_hold_releases",
        "SELECT count(*) FROM audit_log WHERE action='players.coin_hold.released'",
    ]) {
        *count = sqlx::query_scalar::<_, i64>(sql)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    }
    (counts[0], counts[1], counts[2])
}
#[tokio::test]
async fn postgres_coin_hold_release_permission_reason_audit_and_idempotent() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else {
        return;
    };
    let order = coin_hold_fixture(&db).await;
    let s = db.service();
    // The dedicated permission is held by the platform admin built-in only.
    assert!(matches!(
        s.release_coin_hold(
            context(db.support),
            db.target,
            hold_identity(&db).await,
            "Reviewed"
        )
        .await,
        Err(Error::Authorization(crate::rbac::Error::Denied))
    ));
    assert!(matches!(
        s.coin_hold(context(db.support), db.target).await,
        Err(Error::Authorization(crate::rbac::Error::Denied))
    ));
    for unverified in [
        CheckContext::new(db.approver),
        CheckContext {
            account: db.approver,
            require_staff_mfa: true,
            mfa_verified: false,
            session_proof: None,
        },
    ] {
        assert!(matches!(
            s.release_coin_hold(unverified, db.target, hold_identity(&db).await, "Reviewed")
                .await,
            Err(Error::Denied)
        ));
    }
    let long = "x".repeat(501);
    for (target, reason) in [
        (db.target, ""),
        (db.target, " "),
        (db.target, "bad\0reason"),
        (db.target, long.as_str()),
        (Uuid::nil(), "Reviewed"),
    ] {
        assert!(matches!(
            s.release_coin_hold(
                context(db.approver),
                target,
                hold_identity(&db).await,
                reason
            )
            .await,
            Err(Error::BadInput)
        ));
    }
    assert!(matches!(
        s.release_coin_hold(
            context(db.approver),
            Uuid::now_v7(),
            hold_identity(&db).await,
            "Reviewed"
        )
        .await,
        Err(Error::NotFound)
    ));
    // Behind the coin-native flag.
    let off = Players::new(
        db.pool.clone(),
        Flags {
            players: true,
            characters: false,
            coin_holds: false,
        },
        Staff,
        Backend { id: db.target },
    );
    assert!(matches!(
        off.release_coin_hold(
            context(db.approver),
            db.target,
            hold_identity(&db).await,
            "Reviewed"
        )
        .await,
        Err(Error::NotFound)
    ));
    assert_eq!(coin_hold_state(&db).await, (1, 0, 0));
    assert_eq!(
        s.coin_hold(context(db.approver), db.target)
            .await
            .unwrap()
            .map(|h| h.order_id),
        Some(order)
    );

    assert_eq!(
        s.release_coin_hold(
            context(db.approver),
            db.target,
            hold_identity(&db).await,
            " Debt reviewed "
        )
        .await
        .unwrap(),
        Some(order)
    );
    assert_eq!(coin_hold_state(&db).await, (0, 1, 1));
    let history: (Uuid, Uuid, String, Uuid, String, bool) = sqlx::query_as(
        "SELECT account_id,order_id,reason,released_by,release_reason,held_at<=released_at FROM coin_purchase_hold_releases",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(
        history,
        (
            db.target,
            order,
            "reversal_insufficient_balance".into(),
            db.approver,
            "Debt reviewed".into(),
            true
        )
    );
    let audit: (Option<Uuid>, String, serde_json::Value) = sqlx::query_as(
        "SELECT actor,target,after FROM audit_log WHERE action='players.coin_hold.released'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(
        audit,
        (
            Some(db.approver),
            db.target.to_string(),
            json!({"reason":"Debt reviewed","order_id":order,"hold_reason":"reversal_insufficient_balance","debt_coins":500})
        )
    );
    // The order keeps its review flag: releasing settles nothing.
    let review: bool =
        sqlx::query_scalar("SELECT review_required FROM coin_purchase_orders WHERE id=$1")
            .bind(order)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert!(review);
    // The debt is recorded in the history and never cleared or zeroed.
    let debt: (i64, i64, i64, String) = sqlx::query_as(
        "SELECT h.debt_coins,r.coins,r.debited,r.outcome FROM coin_purchase_hold_releases h JOIN coin_purchase_reversals r USING (order_id)",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(debt, (500, 500, 0, "held".into()));
    // Releasing again is a no-op: no second history or audit row.
    assert_eq!(
        s.release_coin_hold(
            context(db.approver),
            db.target,
            hold_identity(&db).await,
            "Debt reviewed"
        )
        .await
        .unwrap(),
        Some(order)
    );
    assert_eq!(coin_hold_state(&db).await, (0, 1, 1));
    assert_eq!(
        s.coin_hold(context(db.approver), db.target).await.unwrap(),
        None
    );
    // History does not block a later hold on the same account.
    sqlx::query("INSERT INTO coin_purchase_holds(account_id,order_id,reason) VALUES($1,$2,'reversal_insufficient_balance') ON CONFLICT (account_id) DO NOTHING")
        .bind(db.target).bind(order).execute(&db.pool).await.unwrap();
    assert_eq!(coin_hold_state(&db).await, (1, 1, 1));
    db.close().await;
}
#[tokio::test]
async fn postgres_coin_hold_release_audit_failure_keeps_the_hold() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else {
        return;
    };
    coin_hold_fixture(&db).await;
    sqlx::query("CREATE FUNCTION reject_test_audit() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RAISE EXCEPTION ''test audit unavailable''; END'").execute(&db.pool).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_test_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION reject_test_audit()").execute(&db.pool).await.unwrap();
    assert!(
        db.service()
            .release_coin_hold(
                context(db.approver),
                db.target,
                hold_identity(&db).await,
                "Reviewed"
            )
            .await
            .is_err()
    );
    assert_eq!(coin_hold_state(&db).await, (1, 0, 0));
    db.close().await;
}
async fn coin_hold_page(app: axum::Router, path: &str) -> String {
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
    String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}
async fn coin_hold_post(
    app: axum::Router,
    path: &str,
    cookie: &str,
    body: String,
) -> (StatusCode, String) {
    let response = app
        .oneshot(
            Request::post(path)
                .header("host", "staff.test")
                .header("origin", "https://staff.test")
                .header("cookie", cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let location = response
        .headers()
        .get("location")
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    (response.status(), location)
}
#[tokio::test]
async fn postgres_http_coin_hold_release_csrf_permission_mfa_and_result() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else {
        return;
    };
    let order = coin_hold_fixture(&db).await;
    let path = format!("/admin/players/{}", db.target);
    let app = |account: Uuid, mfa: bool| {
        http::router(
            db.pool.clone(),
            Flags {
                players: true,
                characters: false,
                coin_holds: true,
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
    let hold = hold_identity(&db).await;
    let release = |csrf: &str, reason: &str| {
        format!("_csrf={csrf}&action=release-coin-hold&hold={hold}&reason={reason}")
    };
    // Support staff see no hold section and may not release.
    assert!(
        !coin_hold_page(app(db.support, true), &path)
            .await
            .contains("release-coin-hold")
    );
    let held = coin_hold_page(app(db.approver, true), &path).await;
    assert!(held.contains("value=\"release-coin-hold\""));
    assert!(held.contains("for=\"coin-hold-reason\""));
    assert!(held.contains(&order.to_string()));
    assert!(held.contains("Zaległy dług w monetach (spłacany najpierw z każdego zasilenia): 500"));
    let (cookie, csrf) = kick_session(app(db.support, true), &path).await;
    assert_eq!(
        coin_hold_post(
            app(db.support, true),
            &path,
            &cookie,
            release(&csrf, "Reviewed")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (cookie, csrf) = kick_session(app(db.approver, true), &path).await;
    for (body, status) in [
        (release("wrong", "Reviewed"), StatusCode::FORBIDDEN),
        (
            "action=release-coin-hold&reason=Reviewed".to_owned(),
            StatusCode::FORBIDDEN,
        ),
        (release(&csrf, ""), StatusCode::BAD_REQUEST),
        (
            format!("_csrf={csrf}&action=release-coin-hold"),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        assert_eq!(
            coin_hold_post(app(db.approver, true), &path, &cookie, body)
                .await
                .0,
            status
        );
    }
    // Without confirmed staff MFA the guard refuses before the handler.
    assert_eq!(
        coin_hold_post(
            app(db.approver, false),
            &path,
            &cookie,
            release(&csrf, "Reviewed")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(coin_hold_state(&db).await, (1, 0, 0));
    assert_eq!(
        coin_hold_post(
            app(db.approver, true),
            &path,
            &cookie,
            release(&csrf, "Debt+reviewed")
        )
        .await,
        (StatusCode::SEE_OTHER, format!("{path}?coin_hold=released"))
    );
    assert_eq!(coin_hold_state(&db).await, (0, 1, 1));
    assert!(
        coin_hold_page(
            app(db.approver, true),
            &format!("{path}?coin_hold=released")
        )
        .await
        .contains("role=\"status\"")
    );
    assert_eq!(
        coin_hold_post(
            app(db.approver, true),
            &path,
            &cookie,
            release(&csrf, "Debt+reviewed")
        )
        .await,
        (StatusCode::SEE_OTHER, format!("{path}?coin_hold=released"))
    );
    assert_eq!(coin_hold_state(&db).await, (0, 1, 1));
    db.close().await;
}

#[tokio::test]
async fn postgres_reset_rechecks_live_expiry_after_advisory_lock() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let Some(db) = Db::open().await else {
        return;
    };
    let request = db
        .service()
        .request_reset(context(db.support), db.target, db.evidence, "Verified case")
        .await
        .unwrap();
    let mut lock = db.pool.begin().await.unwrap();
    let blocker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(71010)")
        .execute(&mut *lock)
        .await
        .unwrap();
    let service = db.service();
    let approver = db.approver;
    let target = db.target;
    let waiting = tokio::spawn(async move {
        service
            .approve_reset(context(approver), target, request)
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(blocker)
            .fetch_one(&db.pool)
            .await
            .unwrap();
            if blocked {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    // Expire after the approval transaction started, while it is still blocked.
    sqlx::query("UPDATE player_mfa_reset_requests SET expires_at=clock_timestamp() WHERE id=$1")
        .bind(request)
        .execute(&db.pool)
        .await
        .unwrap();
    lock.rollback().await.unwrap();
    assert!(matches!(waiting.await.unwrap(), Err(Error::Denied)));
    let state: (bool, bool, bool) =
        sqlx::query_as("SELECT reset,revoked,verified FROM test_security")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(state, (false, false, true));
    let approved: bool = sqlx::query_scalar(
        "SELECT approved_by IS NOT NULL FROM player_mfa_reset_requests WHERE id=$1",
    )
    .bind(request)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(!approved);
    db.close().await;
    eprintln!(
        "postgres_reset_rechecks_live_expiry_after_advisory_lock duration={:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn postgres_identity_adapter_sessions_logout_and_two_person_reset() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let db = Db::open().await.expect("PORTAL_TEST_DATABASE_URL required");
    for id in [db.target, db.support, db.approver] {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'synthetic-hash','active','en',now())")
            .bind(id).bind(format!("{id}@example.invalid")).execute(&db.pool).await.unwrap();
    }
    let adapter = IdentityAccounts {
        accounts: oteryn_identity::accounts::Accounts {
            pool: db.pool.clone(),
        },
        projection: None,
    };
    let service = Players::new(
        db.pool.clone(),
        Flags {
            players: true,
            ..Flags::default()
        },
        Staff,
        adapter.clone(),
    );
    let ticket: Uuid = sqlx::query_scalar("INSERT INTO tickets(account_id,request_key,category,subject) VALUES($1,$2,'account','Identity review') RETURNING id")
        .bind(db.target).bind(Uuid::now_v7()).fetch_one(&db.pool).await.unwrap();
    // Seed every credential class, with no real bearer material.
    for statement in [
        "INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,decode('01','hex'),true)",
        "INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token) VALUES(decode(repeat('01',32),'hex'),$1,now()+interval '1 hour',now()+interval '1 hour','private-device',repeat('a',64))",
        "INSERT INTO oauth_authorization_codes(code_hash,account_id,client_id,redirect_uri,challenge,credential_hash) VALUES(decode(repeat('02',32),'hex'),$1,'test','https://example.invalid','test',decode(repeat('03',32),'hex'))",
        "INSERT INTO oauth_token_families(id,account_id,client_id,credential_hash) VALUES(uuidv7(),$1,'test',decode(repeat('04',32),'hex'))",
        "INSERT INTO oauth_tokens(id,family_id,access_hash,refresh_hash) SELECT uuidv7(),id,decode(repeat('08',32),'hex'),decode(repeat('09',32),'hex') FROM oauth_token_families WHERE account_id=$1",
        "INSERT INTO account_password_resets(account_id,token_hash,expires_at) VALUES($1,decode(repeat('0a',32),'hex'),now()+interval '1 hour')",
        "INSERT INTO passkey_ceremonies(token_hash,session_hash,account_id,purpose,state) VALUES(decode(repeat('0b',32),'hex'),decode(repeat('0c',32),'hex'),$1,'authenticate','{}')",
        "INSERT INTO device_sessions(id,account_id,client_id,enrollment_token_id,game_generation,native_generation,absolute_expires_at,idle_expires_at) VALUES(uuidv7(),$1,uuidv7(),'test-enrollment',1,1,extract(epoch FROM now()+interval '1 hour')::bigint,extract(epoch FROM now()+interval '1 hour')::bigint)",
        "INSERT INTO device_session_credentials(secret_hash,family_id,sequence) SELECT decode(repeat('0d',32),'hex'),id,0 FROM device_sessions WHERE account_id=$1",
        "INSERT INTO game_login_tickets(ticket_hash,account_id,game_generation,native_generation,audience,expires_at) VALUES(decode(repeat('05',32),'hex'),$1,'1','1','oteryn-native-game-gateway',now()+interval '1 hour')",
        "INSERT INTO account_recovery_codes(account_id,code_hash) VALUES($1,decode(repeat('06',32),'hex'))",
        "INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) VALUES(decode(repeat('07',32),'hex'),$1,uuidv7(),decode('01','hex'),'trusted',now()+interval '1 hour')",
    ] {
        sqlx::query(statement)
            .bind(db.target)
            .execute(&db.pool)
            .await
            .unwrap();
    }
    let sessions = adapter.sessions(db.target).await.unwrap();
    // This assertion fails against the base's production IdentityAccounts stub.
    assert!(sessions.iter().any(|session| session.kind == "web"));
    assert!(sessions.iter().any(|session| session.kind == "oauth"));
    assert!(sessions.iter().any(|session| session.kind == "device"));
    assert!(
        sessions
            .iter()
            .any(|session| session.kind == "game-authorization")
    );
    // Use the production composition and real PortalIdentity/session guards.
    let host = "staff.test".to_owned();
    let lookup = |key: &str| match key {
        "OTERYN_ADMIN_PLAYERS" | "OTERYN_ADMIN_PANEL" => Some("true".to_owned()),
        "OTERYN_ADMIN_HOST" => Some(host.clone()),
        // Synthetic test key; grant checks never decrypt the fixture secret.
        "OTERYN_MFA_KEY" => Some("ab".repeat(32)),
        _ => None,
    };
    let app = http::configured_router_from(db.pool.clone(), &lookup).unwrap();
    let staff_cookie = staff_security_cookie(&db.pool, db.support, 'b').await;
    let approver_cookie = staff_security_cookie(&db.pool, db.approver, 'c').await;
    let path = format!("/admin/players/{}", db.target);
    let response = app
        .clone()
        .oneshot(
            Request::get(&path)
                .header("host", &host)
                .header("cookie", &staff_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = staff_cookie;
    let body = String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(!body.contains("private-device"));
    let csrf = body
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    sqlx::query("CREATE FUNCTION reject_staff_security_audit() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RAISE EXCEPTION ''test audit failure''; END'").execute(&db.pool).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_staff_security_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION reject_staff_security_audit()").execute(&db.pool).await.unwrap();
    assert_eq!(
        staff_security_post(
            &app,
            &path,
            &host,
            &cookie,
            csrf,
            "action=logout&reason=Support"
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM web_sessions WHERE account_id=$1")
            .bind(db.target)
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        1
    );
    sqlx::query("DROP TRIGGER reject_staff_security_audit ON audit_log")
        .execute(&db.pool)
        .await
        .unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::post(&path)
                .header("host", &host)
                .header("origin", format!("https://{host}"))
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "_csrf={csrf}&action=logout&reason=Support"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let active_credentials = [
        "SELECT count(*) FROM web_sessions WHERE account_id=$1",
        "SELECT count(*) FROM oauth_authorization_codes WHERE account_id=$1 AND revoked_at IS NULL",
        "SELECT count(*) FROM oauth_token_families WHERE account_id=$1 AND revoked_at IS NULL",
        "SELECT count(*) FROM device_sessions WHERE account_id=$1 AND revoked_at IS NULL",
        "SELECT count(*) FROM game_login_tickets WHERE account_id=$1 AND used_at IS NULL",
        "SELECT count(*) FROM oauth_tokens t JOIN oauth_token_families f ON f.id=t.family_id WHERE f.account_id=$1 AND f.revoked_at IS NULL",
        "SELECT count(*) FROM device_session_credentials c JOIN device_sessions d ON d.id=c.family_id WHERE d.account_id=$1 AND d.revoked_at IS NULL",
        "SELECT count(*) FROM account_password_resets WHERE account_id=$1 AND used_at IS NULL",
        "SELECT count(*) FROM passkey_ceremonies WHERE account_id=$1",
        "SELECT count(*) FROM account_mfa_tokens WHERE account_id=$1",
    ];
    for statement in active_credentials {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(statement)
                .bind(db.target)
                .fetch_one(&db.pool)
                .await
                .unwrap(),
            0
        );
    }
    // Issue new state after logout, so reset independently proves full revocation.
    for statement in [
        "INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token) VALUES(decode(repeat('10',32),'hex'),$1,now()+interval '1 hour',now()+interval '1 hour','reset-device',repeat('a',64))",
        "INSERT INTO oauth_authorization_codes(code_hash,account_id,client_id,redirect_uri,challenge,credential_hash) VALUES(decode(repeat('11',32),'hex'),$1,'test','https://example.invalid','test',decode('01','hex'))",
        "INSERT INTO oauth_token_families(id,account_id,client_id,credential_hash) VALUES(uuidv7(),$1,'reset-client',decode('01','hex'))",
        "INSERT INTO oauth_tokens(id,family_id,access_hash,refresh_hash) SELECT uuidv7(),id,decode(repeat('12',32),'hex'),decode(repeat('13',32),'hex') FROM oauth_token_families WHERE account_id=$1 AND client_id='reset-client'",
        "INSERT INTO device_sessions(id,account_id,client_id,enrollment_token_id,game_generation,native_generation,absolute_expires_at,idle_expires_at) VALUES(uuidv7(),$1,uuidv7(),'reset-enrollment',1,1,extract(epoch FROM now()+interval '1 hour')::bigint,extract(epoch FROM now()+interval '1 hour')::bigint)",
        "INSERT INTO device_session_credentials(secret_hash,family_id,sequence) SELECT decode(repeat('14',32),'hex'),id,0 FROM device_sessions WHERE account_id=$1 AND enrollment_token_id='reset-enrollment'",
        "INSERT INTO game_login_tickets(ticket_hash,account_id,game_generation,native_generation,audience,expires_at) VALUES(decode(repeat('15',32),'hex'),$1,'1','1','oteryn-native-game-gateway',now()+interval '1 hour')",
        "UPDATE account_password_resets SET used_at=NULL WHERE account_id=$1",
        "INSERT INTO passkey_ceremonies(token_hash,session_hash,account_id,purpose,state) VALUES(decode(repeat('16',32),'hex'),decode(repeat('17',32),'hex'),$1,'authenticate','{}')",
        "INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) VALUES(decode(repeat('18',32),'hex'),$1,uuidv7(),decode('01','hex'),'trusted',now()+interval '1 hour')",
    ] {
        sqlx::query(statement)
            .bind(db.target)
            .execute(&db.pool)
            .await
            .unwrap();
    }
    assert_eq!(
        staff_security_post(
            &app,
            &path,
            &host,
            &cookie,
            "wrong",
            &format!("action=verify-identity&ticket={ticket}")
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        staff_security_post(
            &app,
            &format!("/admin/players/{}", db.approver),
            &host,
            &cookie,
            csrf,
            &format!("action=verify-identity&ticket={ticket}")
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM support_mfa_reset_evidence")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        0
    );
    let response = app
        .clone()
        .oneshot(
            Request::post(&path)
                .header("host", &host)
                .header("origin", format!("https://{host}"))
                .header("cookie", &cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "_csrf={csrf}&action=verify-identity&ticket={ticket}"
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let evidence: Uuid = response.headers()["location"]
        .to_str()
        .unwrap()
        .split("?evidence=")
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let issued_metadata: (Uuid,Uuid,Uuid,bool) = sqlx::query_as("SELECT ticket_id,account_id,verified_by,expires_at>clock_timestamp() AND consumed_at IS NULL FROM support_mfa_reset_evidence WHERE id=$1").bind(evidence).fetch_one(&db.pool).await.unwrap();
    assert_eq!(issued_metadata, (ticket, db.target, db.support, true));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM audit_log WHERE action='players.mfa_reset.identity_verified'"
        )
        .fetch_one(&db.pool)
        .await
        .unwrap(),
        1
    );
    let response = app
        .clone()
        .oneshot(
            Request::get(format!("{path}?evidence={evidence}"))
                .header("host", &host)
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let issued = String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(issued.contains(&format!("name=\"evidence\" value=\"{evidence}\"")));
    assert!(
        service
            .request_reset(context(db.support), db.target, Uuid::now_v7(), "Unverified")
            .await
            .is_err()
    );
    sqlx::query("UPDATE support_mfa_reset_evidence SET expires_at=clock_timestamp()-interval '1 second' WHERE id=$1").bind(evidence).execute(&db.pool).await.unwrap();
    assert!(
        service
            .request_reset(context(db.support), db.target, evidence, "Expired")
            .await
            .is_err()
    );
    sqlx::query("UPDATE support_mfa_reset_evidence SET expires_at=clock_timestamp()+interval '1 hour' WHERE id=$1").bind(evidence).execute(&db.pool).await.unwrap();
    let response = staff_security_post(
        &app,
        &path,
        &host,
        &cookie,
        csrf,
        &format!("action=request-reset&evidence={evidence}&reason=Verified"),
    )
    .await;
    assert_eq!(response, StatusCode::SEE_OTHER);
    let request: Uuid =
        sqlx::query_scalar("SELECT id FROM player_mfa_reset_requests WHERE account_id=$1")
            .bind(db.target)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(
        staff_security_post(
            &app,
            &path,
            &host,
            &cookie,
            csrf,
            &format!("action=approve-reset&request={request}")
        )
        .await,
        StatusCode::FORBIDDEN
    );
    let response = app
        .clone()
        .oneshot(
            Request::get(&path)
                .header("host", &host)
                .header("cookie", &approver_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let approval_body = String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    let approval_csrf = approval_body
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    assert_eq!(
        staff_security_post(
            &app,
            &path,
            &host,
            &approver_cookie,
            approval_csrf,
            &format!("action=approve-reset&request={request}")
        )
        .await,
        StatusCode::SEE_OTHER
    );
    assert!(
        service
            .request_reset(context(db.support), db.target, evidence, "Replay")
            .await
            .is_err()
    );
    for statement in [
        "SELECT count(*) FROM account_mfa_totp WHERE account_id=$1",
        "SELECT count(*) FROM account_recovery_codes WHERE account_id=$1",
    ] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(statement)
                .bind(db.target)
                .fetch_one(&db.pool)
                .await
                .unwrap(),
            0
        );
    }
    for statement in active_credentials {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(statement)
                .bind(db.target)
                .fetch_one(&db.pool)
                .await
                .unwrap(),
            0
        );
    }
    db.close().await;
    eprintln!(
        "postgres_identity_adapter_sessions_logout_and_two_person_reset duration: {:?}",
        started.elapsed()
    );
}

async fn staff_security_cookie(pool: &PgPool, account: Uuid, marker: char) -> String {
    let bearer = marker.to_string().repeat(64);
    let grant = oteryn_identity::mfa::random_token().unwrap();
    let generation: Uuid = sqlx::query_scalar("INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,decode('01','hex'),true) RETURNING generation").bind(account).fetch_one(pool).await.unwrap();
    sqlx::query("INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) VALUES($1,$2,$3,$4,'grant',now()+interval '1 hour')").bind(oteryn_identity::mfa::digest(&grant)).bind(account).bind(generation).bind(oteryn_identity::mfa::digest("synthetic-hash")).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,data,authorization_generation) VALUES($1,$2,now()+interval '1 hour',now()+interval '1 hour','test',repeat('a',64),$3,(SELECT native_security_generation FROM accounts WHERE id=$2))").bind(oteryn_identity::mfa::digest(&bearer)).bind(account).bind(json!({"mfa_grant":grant})).execute(pool).await.unwrap();
    format!("__Host-oteryn_session={bearer}")
}
async fn staff_security_post(
    app: &axum::Router,
    path: &str,
    host: &str,
    cookie: &str,
    csrf: &str,
    action: &str,
) -> StatusCode {
    app.clone()
        .oneshot(
            Request::post(path)
                .header("host", host)
                .header("origin", format!("https://{host}"))
                .header("cookie", cookie)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!("_csrf={csrf}&{action}")))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn postgres_stale_hold_form_preserves_replacement_and_receipt() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    let db = Db::open().await.expect("PORTAL_TEST_DATABASE_URL required");
    let order = coin_hold_fixture(&db).await;
    let old = hold_identity(&db).await;
    let path = format!("/admin/players/{}", db.target);
    let app = http::router(
        db.pool.clone(),
        Flags {
            players: true,
            characters: false,
            coin_holds: true,
        },
        PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        KickBrowser {
            account: db.approver,
            mfa: true,
        },
        Staff,
        Backend { id: db.target },
    );
    let (cookie, csrf) = kick_session(app.clone(), &path).await;
    let reviewed_form =
        format!("_csrf={csrf}&action=release-coin-hold&hold={old}&reason=Reviewed+H1");
    assert_eq!(
        coin_hold_post(app.clone(), &path, &cookie, reviewed_form.clone())
            .await
            .0,
        StatusCode::SEE_OTHER
    );
    // Even another hold of the same order has a distinct instance identity.
    let replacement: Uuid = sqlx::query_scalar("INSERT INTO coin_purchase_holds(account_id,order_id,reason) VALUES($1,$2,'reversal_insufficient_balance') RETURNING hold_id")
        .bind(db.target).bind(order).fetch_one(&db.pool).await.unwrap();
    assert_ne!(old, replacement);
    assert_eq!(
        coin_hold_post(app.clone(), &path, &cookie, reviewed_form).await,
        (StatusCode::SEE_OTHER, format!("{path}?coin_hold=released"))
    );
    assert_eq!(hold_identity(&db).await, replacement);
    assert_eq!(coin_hold_state(&db).await, (1, 1, 1));
    let stale_unknown = format!(
        "_csrf={csrf}&action=release-coin-hold&hold={}&reason=Unreviewed",
        Uuid::now_v7()
    );
    assert_eq!(
        coin_hold_post(app, &path, &cookie, stale_unknown).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(hold_identity(&db).await, replacement);
    db.close().await;
    eprintln!(
        "postgres_stale_hold_form_preserves_replacement_and_receipt duration={:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn drc_kick_rechecks_grant_after_revocation_counter_wait() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else { return };
    let kicker = kick_fixture(&db).await;
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'kicker@example.invalid','synthetic-hash','active','en',now())").bind(kicker).execute(&db.pool).await.unwrap();
    staff_security_cookie(&db.pool, kicker, 'd').await;
    let (session_hash, grant_hash): (Vec<u8>, Vec<u8>) = sqlx::query_as("SELECT s.id_hash,t.token_hash FROM web_sessions s JOIN account_mfa_tokens t ON t.account_id=s.account_id WHERE s.account_id=$1").bind(kicker).fetch_one(&db.pool).await.unwrap();
    let mut context = context(kicker);
    context.session_proof = Some(crate::rbac::SessionProof {
        session_hash: session_hash.try_into().unwrap(),
        grant_hash: grant_hash.try_into().unwrap(),
    });
    let mut blocker = db.pool.begin().await.unwrap();
    sqlx::query("SELECT * FROM identity_security_revocation_counter FOR UPDATE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    sqlx::query("UPDATE account_mfa_tokens SET expires_at=clock_timestamp()+interval '1500 milliseconds' WHERE account_id=$1").bind(kicker).execute(&db.pool).await.unwrap();
    let service = db.service();
    let target = db.target;
    let task = tokio::spawn(async move { service.kick(context, target, "counter wait").await });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND $1=ANY(pg_blocking_pids(pid)))").bind(pid).fetch_one(&db.pool).await.unwrap();
            if waiting { break }
            tokio::task::yield_now().await;
        }
    }).await.expect("kick never waited on the revocation counter");
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let expired: bool = sqlx::query_scalar(
                "SELECT expires_at<=clock_timestamp() FROM account_mfa_tokens WHERE account_id=$1",
            )
            .bind(kicker)
            .fetch_one(&db.pool)
            .await
            .unwrap();
            if expired {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("MFA grant did not expire within the mutation budget");
    blocker.rollback().await.unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .expect("kick did not finish after counter release")
        .unwrap();
    assert!(
        matches!(result, Err(Error::Authorization(_))),
        "expired grant authorized kick: {result:?}"
    );
    assert_eq!(kick_state(&db).await.0, 1);
    assert_eq!(kick_state(&db).await.3, 0);
    let audits: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='players.kick'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(audits, 0);
    db.close().await;
}
#[tokio::test]
async fn postgres_http_kick_only_moderator_sees_only_the_kick_view() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Db::open().await else {
        return;
    };
    kick_fixture(&db).await;
    let moderator = Uuid::now_v7();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='moderator'")
        .bind(moderator)
        .execute(&db.pool)
        .await
        .unwrap();
    let path = format!("/admin/players/{}", db.target);
    let app = |account: Uuid| {
        http::router(
            db.pool.clone(),
            Flags {
                players: true,
                characters: false,
                coin_holds: false,
            },
            PanelConfig {
                admin_panel: true,
                host: "staff.test".into(),
            },
            KickBrowser { account, mfa: true },
            Staff,
            Backend { id: db.target },
        )
    };
    // Support staff still get the full support view.
    let response = app(db.support)
        .oneshot(
            Request::get(&path)
                .header("host", "staff.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = String::from_utf8(
        to_bytes(response.into_body(), 100000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("value=\"note\""));
    assert!(body.contains("value=\"logout\""));
    // A kick-only moderator reaches the kick view and nothing else.
    let response = app(moderator)
        .oneshot(
            Request::get(&path)
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
    for hidden in [
        "value=\"note\"",
        "value=\"logout\"",
        "value=\"verify-identity\"",
        "value=\"request-reset\"",
        "value=\"approve-reset\"",
        "action=\"/admin/players\"",
        "kick-target@example.invalid",
    ] {
        assert!(!body.contains(hidden), "kick view leaks {hidden}");
    }
    let csrf = body
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned();
    // Support data and actions stay refused for the moderator.
    let search = app(moderator)
        .oneshot(
            Request::get("/admin/players?q=kick-target")
                .header("host", "staff.test")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(search.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        kick_post(
            app(moderator),
            &path,
            &cookie,
            format!("_csrf={csrf}&action=note&body=Note")
        )
        .await,
        StatusCode::FORBIDDEN
    );
    let views: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action='players.kick.view' AND actor=$1",
    )
    .bind(moderator)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(views, 1);
    assert_eq!(
        kick_post(
            app(moderator),
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

#[tokio::test]
async fn r8_self_logout_revokes_operator_session() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = Db::open().await.unwrap();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,'self@example.invalid','fixture','active','en',now())").bind(db.support).execute(&db.pool).await.unwrap();
    let mut actor = context(db.support);
    actor.session_proof = Some(
        crate::test_staff::proof(&db.pool, db.support)
            .await
            .unwrap(),
    );
    let service = Players::new(
        db.pool.clone(),
        db.service().flags,
        Staff,
        IdentityAccounts {
            accounts: oteryn_identity::accounts::Accounts {
                pool: db.pool.clone(),
            },
            projection: None,
        },
    );
    let result = service.logout_all(actor, db.support, "Self logout").await;
    assert!(result.is_ok(), "self logout returned {result:?}");
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM web_sessions WHERE account_id=$1")
            .bind(db.support)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
    db.close().await;
}

#[tokio::test]
async fn self_kick_commits_with_live_session() {
    let db = Db::open().await.expect("PostgreSQL required");
    let kicker = kick_fixture(&db).await;
    sqlx::query("UPDATE account_roles SET account_id=$1 WHERE account_id=$2")
        .bind(db.target)
        .bind(kicker)
        .execute(&db.pool)
        .await
        .unwrap();
    let mut actor = context(db.target);
    actor.session_proof = Some(crate::test_staff::proof(&db.pool, db.target).await.unwrap());
    let before = kick_state(&db).await.0;
    let result = db
        .service()
        .kick(actor, db.target, "Own native sessions")
        .await;
    assert_eq!(result.expect("authorized self kick"), before + 1);
    assert_eq!(kick_state(&db).await.0, before + 1);
    db.close().await;
}

#[tokio::test]
async fn postgres_http_typed_account_edits_and_redacted_audit() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = Db::open().await.expect("PORTAL_TEST_DATABASE_URL required");
    for id in [db.target, db.support, db.approver] {
        sqlx::query(
            "INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at)
             VALUES($1,$2,'synthetic-hash','active','en',now())",
        )
        .bind(id)
        .bind(format!("{id}@example.test"))
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let app = http::configured_router_from(db.pool.clone(), &|key| match key {
        "OTERYN_ADMIN_PANEL"
        | "OTERYN_ADMIN_PLAYERS"
        | "OTERYN_ACCOUNT_EMAIL_CHANGE"
        | "OTERYN_ADMIN_ACCOUNT_SECURITY_EDITS_ENABLED" => Some("true".into()),
        "OTERYN_ADMIN_HOST" => Some("staff.test".into()),
        "OTERYN_PUBLIC_ORIGIN" => Some("https://staff.test".into()),
        "OTERYN_MFA_KEY" => Some("ab".repeat(32)),
        _ => None,
    })
    .unwrap();
    let admin = staff_security_cookie(&db.pool, db.approver, 'd').await;
    let support = staff_security_cookie(&db.pool, db.support, 'e').await;
    let path = format!("/admin/players/{}", db.target);
    let csrf = "a".repeat(64);
    for (action, good, bad, audit) in [
        ("review-flag", "yes", "true", "players.flags.changed"),
        (
            "account-email",
            "new%40example.test",
            "invalid",
            "players.email.change_requested",
        ),
        (
            "account-status",
            "suspended",
            "closed",
            "players.status.changed",
        ),
    ] {
        let valid = format!("action={action}&body={good}&reason=Verified+support+case");
        for (cookie, token, body, expected) in [
            (&admin, "wrong", valid.clone(), StatusCode::FORBIDDEN),
            (
                &support,
                csrf.as_str(),
                valid.clone(),
                StatusCode::FORBIDDEN,
            ),
            (
                &admin,
                csrf.as_str(),
                format!("action={action}&body={bad}&reason=Verified+support+case"),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (&admin, csrf.as_str(), valid, StatusCode::SEE_OTHER),
        ] {
            assert_eq!(
                staff_security_post(&app, &path, "staff.test", cookie, token, &body).await,
                expected,
                "{action}"
            );
        }
        let rows: Vec<(serde_json::Value, serde_json::Value, String)> = sqlx::query_as(
            "SELECT before,after,staff_reason FROM audit_log WHERE target=$1 AND action=$2",
        )
        .bind(db.target.to_string())
        .bind(audit)
        .fetch_all(&db.pool)
        .await
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].2, "Verified support case");
        assert!(!rows[0].1.to_string().contains("example.test"));
    }
    let saved: (String, bool, bool) = sqlx::query_as(
        "SELECT a.status, f.needs_review, c.confirmed_at IS NULL AND a.email=c.old_email
         FROM accounts a JOIN staff_account_flags f ON f.account_id=a.id
         JOIN account_email_changes c ON c.account_id=a.id WHERE a.id=$1",
    )
    .bind(db.target)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(saved, ("suspended".into(), true, true));
    let protected = format!("/admin/players/{}", db.approver);
    assert_eq!(
        staff_security_post(
            &app,
            &protected,
            "staff.test",
            &admin,
            &csrf,
            "action=account-status&body=suspended&reason=Verified+support+case"
        )
        .await,
        StatusCode::FORBIDDEN
    );
    sqlx::query(
        "INSERT INTO account_roles(account_id,role_id)
         SELECT $1,id FROM roles WHERE key='support'",
    )
    .bind(db.target)
    .execute(&db.pool)
    .await
    .unwrap();
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT cancelled_at IS NOT NULL FROM account_email_changes WHERE account_id=$1"
        )
        .bind(db.target)
        .fetch_one(&db.pool)
        .await
        .unwrap()
    );
    db.close().await;
}
