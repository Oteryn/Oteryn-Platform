use super::*;
use oteryn_admin::rbac::CheckContext;
use oteryn_gameauth::admission::{self, AccountStateLookup, AdmissionAuthority};
use sqlx::{PgPool, postgres::PgPoolOptions};
type TestResult = Result<(), Box<dyn std::error::Error>>;
const ENABLED: Features = Features {
    sanctions: true,
    game_access: true,
};

struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
    actor: Uuid,
    reviewer: Uuid,
    target: Uuid,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIP sanctions PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        let schema = format!("sanctions_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .after_connect(move |connection, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        for migration in [
            include_str!("../../../../migrations/20261105155256_t03_accounts.sql"),
            include_str!("../../../../migrations/20261105170000_t10_rbac_audit.sql"),
            include_str!("../../../../migrations/20261105190100_t19_native_admissions.sql"),
            include_str!("../../../../migrations/20261105220235_t34_sanctions.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await?;
        }
        let actor = Uuid::now_v7();
        let reviewer = Uuid::now_v7();
        let target = Uuid::now_v7();
        for id in [actor, reviewer, target] {
            sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES ($1,$2,'synthetic-not-a-password','active','en')")
                .bind(id).bind(format!("{id}@example.invalid")).execute(&pool).await?;
        }
        for id in [actor, reviewer] {
            sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='moderator'")
                .bind(id).execute(&pool).await?;
        }
        sqlx::raw_sql("CREATE TABLE fixture_issued_tickets (id uuid PRIMARY KEY)")
            .execute(&pool)
            .await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
            actor,
            reviewer,
            target,
        }))
    }
    fn service(&self) -> Service {
        Service::new(self.pool.clone(), ENABLED)
    }
    async fn now(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&self.pool)
            .await
    }
    async fn decision(&self, category: Category, permanent: bool) -> Result<Decision, sqlx::Error> {
        let now = self.now().await?;
        Ok(Decision {
            account: self.target,
            category,
            public_reason: "  Rules violation  ".into(),
            moderator_notes: Some("Private evidence".into()),
            effective_at_ms: now - 1000,
            expires_at_ms: if permanent { None } else { Some(now + 60000) },
        })
    }
    async fn ticket(&self) -> Result<Uuid, Error> {
        ticket_transaction(&self.pool, self.target, ENABLED, |connection| {
            Box::pin(async move {
                let id = Uuid::now_v7();
                sqlx::query("INSERT INTO fixture_issued_tickets VALUES ($1)")
                    .bind(id)
                    .execute(connection)
                    .await?;
                Ok(id)
            })
        })
        .await
    }
    async fn cleanup(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}
fn staff(account: Uuid) -> CheckContext {
    CheckContext {
        account,
        require_staff_mfa: true,
        mfa_verified: true,
    }
}

#[test]
fn input_and_defaults_fail_closed() {
    assert!(!Features::default().sanctions && !Features::default().game_access);
    for category in ["unknown", "Suspension", "", " suspension"] {
        assert!(Category::parse(category).is_err());
    }
    assert!(matches!(Category::parse("warning"), Ok(Category::Warning)));
    let mut decision = Decision {
        account: Uuid::now_v7(),
        category: Category::Warning,
        public_reason: "reason".into(),
        moderator_notes: None,
        effective_at_ms: 100,
        expires_at_ms: Some(101),
    };
    assert!(decision.validate().is_ok());
    decision.expires_at_ms = Some(100);
    assert!(decision.validate().is_err());
    decision.expires_at_ms = None;
    decision.public_reason = " \n ".into();
    assert!(decision.validate().is_err());
    decision.public_reason = "x".repeat(4001);
    assert!(decision.validate().is_err());
    decision.public_reason = "reason".into();
    decision.account = Uuid::nil();
    assert!(decision.validate().is_err());
}

#[tokio::test]
async fn db_warning_restriction_and_suspension_ticket_lifecycle() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let service = db.service();
    for category in [Category::Warning, Category::Restriction] {
        let receipt = service
            .create(
                staff(db.actor),
                Uuid::now_v7(),
                db.decision(category, false).await?,
            )
            .await?;
        assert!(!receipt.pending);
        db.ticket().await?;
    }
    let receipt = service
        .create(
            staff(db.actor),
            Uuid::now_v7(),
            db.decision(Category::Suspension, false).await?,
        )
        .await?;
    assert!(matches!(db.ticket().await, Err(Error::Denied)));
    assert!(matches!(
        service
            .revoke(staff(db.actor), Uuid::now_v7(), receipt.id, 9)
            .await,
        Err(Error::Conflict)
    ));
    service
        .revoke(staff(db.actor), Uuid::now_v7(), receipt.id, 1)
        .await?;
    db.ticket().await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_log")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 4);
    let leaked: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM audit_log WHERE after::text LIKE '%Private evidence%')",
    )
    .fetch_one(&db.pool)
    .await?;
    assert!(!leaked);
    db.cleanup().await
}

#[tokio::test]
async fn db_permanent_decision_requires_distinct_current_authorized_reviewer() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let service = db.service();
    let receipt = service
        .create(
            staff(db.actor),
            Uuid::now_v7(),
            db.decision(Category::Suspension, true).await?,
        )
        .await?;
    assert!(receipt.pending);
    db.ticket().await?;
    assert!(matches!(
        service
            .approve(staff(db.actor), Uuid::now_v7(), receipt.id, 1)
            .await,
        Err(Error::Denied)
    ));
    assert!(
        service
            .approve(staff(db.target), Uuid::now_v7(), receipt.id, 1)
            .await
            .is_err()
    );
    let approved = service
        .approve(staff(db.reviewer), Uuid::now_v7(), receipt.id, 1)
        .await?;
    assert_eq!(approved.version, 2);
    assert!(!approved.pending);
    assert!(matches!(db.ticket().await, Err(Error::Denied)));
    assert!(matches!(
        service
            .approve(staff(db.reviewer), Uuid::now_v7(), receipt.id, 1)
            .await,
        Err(Error::Conflict)
    ));
    db.cleanup().await
}

#[tokio::test]
async fn db_bad_input_mfa_permission_and_disabled_feature_have_no_effect() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let decision = db.decision(Category::Warning, false).await?;
    let mut bad = decision.clone();
    bad.expires_at_ms = Some(bad.effective_at_ms);
    assert!(matches!(
        db.service()
            .create(staff(db.actor), Uuid::now_v7(), bad)
            .await,
        Err(Error::InvalidInput)
    ));
    let mut context = staff(db.actor);
    context.mfa_verified = false;
    assert!(
        db.service()
            .create(context, Uuid::now_v7(), decision.clone())
            .await
            .is_err()
    );
    // Staff MFA cannot be disabled through a trusted context's optional flag.
    context.require_staff_mfa = false;
    assert!(
        db.service()
            .create(context, Uuid::now_v7(), decision.clone())
            .await
            .is_err()
    );
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(db.actor)
        .execute(&db.pool)
        .await?;
    assert!(
        db.service()
            .create(staff(db.actor), Uuid::now_v7(), decision.clone())
            .await
            .is_err()
    );
    assert!(matches!(
        Service::new(db.pool.clone(), Features::default())
            .create(staff(db.reviewer), Uuid::now_v7(), decision)
            .await,
        Err(Error::Disabled)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM sanctions")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    db.cleanup().await
}

#[tokio::test]
async fn db_suspension_boundaries_overlap_and_account_state_deny() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let service = db.service();
    let mut expired = db.decision(Category::Suspension, false).await?;
    expired.effective_at_ms -= 60000;
    expired.expires_at_ms = Some(db.now().await? - 1);
    service
        .create(staff(db.actor), Uuid::now_v7(), expired)
        .await?;
    db.ticket().await?;
    let mut future = db.decision(Category::Suspension, false).await?;
    future.effective_at_ms += 30000;
    service
        .create(staff(db.actor), Uuid::now_v7(), future)
        .await?;
    db.ticket().await?;
    let a = service
        .create(
            staff(db.actor),
            Uuid::now_v7(),
            db.decision(Category::Suspension, false).await?,
        )
        .await?;
    service
        .create(
            staff(db.actor),
            Uuid::now_v7(),
            db.decision(Category::Suspension, false).await?,
        )
        .await?;
    service
        .revoke(staff(db.actor), Uuid::now_v7(), a.id, 1)
        .await?;
    assert!(matches!(db.ticket().await, Err(Error::Denied)));
    for status in ["suspended", "closed", "pending_verification"] {
        sqlx::query("UPDATE accounts SET status=$2 WHERE id=$1")
            .bind(db.target)
            .bind(status)
            .execute(&db.pool)
            .await?;
        assert!(matches!(db.ticket().await, Err(Error::Denied)));
    }
    let mut tx = db.pool.begin().await?;
    assert!(matches!(
        require_game_access(&mut tx, Uuid::now_v7(), ENABLED).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        require_game_access(&mut tx, db.target, Features::default()).await,
        Err(Error::Disabled)
    ));
    tx.rollback().await?;
    db.cleanup().await
}

#[derive(Debug)]
struct Authority(Uuid);
impl AdmissionAuthority for Authority {
    fn redeem<'a>(
        &'a self,
        _: &'a mut sqlx::PgConnection,
        _: &'a admission::AdmissionRequest,
    ) -> admission::Future<'a, admission::AuthorizedAccount> {
        Box::pin(async move {
            Ok(admission::AuthorizedAccount {
                id: self.0,
                security_generation: 1,
            })
        })
    }
    fn validate_replay<'a>(
        &'a self,
        _: &'a mut sqlx::PgConnection,
        _: &'a admission::AdmissionRequest,
        _: &'a [u8],
    ) -> admission::Future<'a, ()> {
        Box::pin(async { Ok(()) })
    }
    fn issue<'a>(
        &'a self,
        connection: &'a mut sqlx::PgConnection,
        _: &'a admission::AdmissionRequest,
        _: admission::AuthorizedAccount,
    ) -> admission::Future<'a, admission::Material> {
        Box::pin(async move {
            let now: i64 = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
            )
            .fetch_one(connection)
            .await
            .map_err(|_| admission::Error::Unavailable)?;
            Ok(admission::Material {
                world_id: Uuid::now_v7(),
                channel_id: Uuid::now_v7(),
                endpoint: admission::Endpoint {
                    host: "game.example.invalid".into(),
                    port: 7172,
                    tls_server_name: "game.example.invalid".into(),
                    alpn: "oteryn-game/1".into(),
                    protocol_major: 1,
                    transport_profile: 1,
                },
                token: "synthetic.header.signature".into(),
                expires_at_ms: now + 20000,
            })
        })
    }
}
fn request(attempt: Uuid) -> Result<admission::AdmissionRequest, admission::Error> {
    admission::AdmissionRequest::decode(format!(r#"{{"protocol_version":2,"game_login_ticket":"synthetic-ticket","attempt_ref":"{attempt}","character_id":"{}","channel_id":null,"offer":{{"client_build":"0.1.0+test","client_platform":"windows","transports":[{{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}}]}}}}"#,Uuid::now_v7()).as_bytes())
}
#[tokio::test]
async fn db_admission_fresh_and_replay_recheck_suspension() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let authority = GuardedAdmission::new(Authority(db.target), ENABLED);
    let attempt = Uuid::now_v7();
    let request = request(attempt)?;
    sqlx::query("INSERT INTO native_admissions(attempt_ref,ticket_hash,request_digest,account_id,response_body,expires_at_ms) VALUES ($1,$2,$2,$3,$4,1)")
        .bind(attempt).bind(vec![0u8;32]).bind(db.target).bind(vec![0u8]).execute(&db.pool).await?;
    let mut tx = db.pool.begin().await?;
    authority.redeem(&mut tx, &request).await?;
    authority
        .validate_replay(&mut tx, &request, b"synthetic-response")
        .await?;
    assert_eq!(
        AdmissionAccounts(ENABLED).state(&mut tx, db.target).await?,
        admission::AccountState::Active
    );
    tx.rollback().await?;
    db.service()
        .create(
            staff(db.actor),
            Uuid::now_v7(),
            db.decision(Category::Suspension, false).await?,
        )
        .await?;
    let mut tx = db.pool.begin().await?;
    assert!(matches!(
        authority.redeem(&mut tx, &request).await,
        Err(admission::Error::SecurityDenied)
    ));
    assert!(matches!(
        authority
            .validate_replay(&mut tx, &request, b"synthetic-response")
            .await,
        Err(admission::Error::SecurityDenied)
    ));
    assert_eq!(
        AdmissionAccounts(ENABLED).state(&mut tx, db.target).await?,
        admission::AccountState::Suspended
    );
    tx.rollback().await?;
    db.cleanup().await
}

#[tokio::test]
async fn db_access_transaction_serializes_suspension_and_rolls_back_ticket_failure() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let result: Result<(), Error> =
        ticket_transaction(&db.pool, db.target, ENABLED, |connection| {
            Box::pin(async move {
                sqlx::query("INSERT INTO fixture_issued_tickets VALUES ($1)")
                    .bind(Uuid::now_v7())
                    .execute(connection)
                    .await?;
                Err(Error::Denied)
            })
        })
        .await;
    assert!(result.is_err());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM fixture_issued_tickets")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    let mut ticket_tx = db.pool.begin().await?;
    require_game_access(&mut ticket_tx, db.target, ENABLED).await?;
    let mut writer = db.pool.begin().await?;
    sqlx::query("SET LOCAL lock_timeout='50ms'")
        .execute(&mut *writer)
        .await?;
    // Every sanctions writer must take this exclusive account lock first.
    assert!(
        sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(db.target)
            .execute(&mut *writer)
            .await
            .is_err()
    );
    writer.rollback().await?;
    ticket_tx.rollback().await?;
    db.service()
        .create(
            staff(db.actor),
            Uuid::now_v7(),
            db.decision(Category::Suspension, false).await?,
        )
        .await?;
    assert!(matches!(db.ticket().await, Err(Error::Denied)));
    db.cleanup().await
}

#[tokio::test]
async fn db_http_admission_success_then_suspension_denies_fresh_and_replay() -> TestResult {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use sha2::{Digest, Sha256};
    use tower::ServiceExt;
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let path = std::env::temp_dir().join(format!("sanctions-service-{}", Uuid::now_v7()));
    std::fs::write(&path, format!("{:x}", Sha256::digest(b"synthetic-service")))?;
    let tokens = admission::ServiceTokens::from_files(&path, None)?;
    std::fs::remove_file(path)?;
    let app = admission_router(
        db.pool.clone(),
        admission::AdmissionConfig {
            enabled: true,
            tokens: Some(tokens),
            ..Default::default()
        },
        Authority(db.target),
        ENABLED,
    );
    let attempt = Uuid::now_v7();
    let body = |attempt: Uuid| {
        format!(
            r#"{{"protocol_version":2,"game_login_ticket":"synthetic-ticket-{attempt}","attempt_ref":"{attempt}","character_id":"{}","channel_id":null,"offer":{{"client_build":"0.1.0+test","client_platform":"windows","transports":[{{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}}]}}}}"#,
            db.target
        )
    };
    let send = |raw: String| {
        Request::post("/internal/v1/game-auth/native-admissions")
            .header("authorization", "Bearer synthetic-service")
            .header("content-type", "application/json")
            .body(Body::from(raw))
    };
    assert_eq!(
        app.clone().oneshot(send(body(attempt))?).await?.status(),
        StatusCode::OK
    );
    db.service()
        .create(
            staff(db.actor),
            Uuid::now_v7(),
            db.decision(Category::Suspension, false).await?,
        )
        .await?;
    for id in [attempt, Uuid::now_v7()] {
        let response = app.clone().oneshot(send(body(id))?).await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            to_bytes(response.into_body(), 6144).await?.as_ref(),
            admission::Error::SecurityDenied.body(Some(&id.to_string()))
        );
    }
    assert_eq!(
        app.clone().oneshot(send("{}".into())?).await?.status(),
        StatusCode::BAD_REQUEST
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM native_admissions")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 1);
    drop(app);
    db.cleanup().await
}

#[tokio::test]
async fn db_unknown_sanction_state_and_unavailable_storage_never_authorize() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let receipt = db
        .service()
        .create(
            staff(db.actor),
            Uuid::now_v7(),
            db.decision(Category::Warning, false).await?,
        )
        .await?;
    // Simulate a newer/unsupported database vocabulary; unknown is not absence.
    sqlx::raw_sql("ALTER TABLE sanctions DROP CONSTRAINT sanctions_status_check")
        .execute(&db.pool)
        .await?;
    sqlx::query("UPDATE sanctions SET status='unknown' WHERE id=$1")
        .bind(receipt.id)
        .execute(&db.pool)
        .await?;
    assert!(matches!(db.ticket().await, Err(Error::Denied)));
    sqlx::raw_sql("DROP TABLE sanctions")
        .execute(&db.pool)
        .await?;
    assert!(matches!(db.ticket().await, Err(Error::Storage(_))));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM fixture_issued_tickets")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(count, 0);
    db.cleanup().await
}
