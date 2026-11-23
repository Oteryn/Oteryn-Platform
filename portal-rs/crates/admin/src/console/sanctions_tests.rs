use super::*;

#[test]
fn validates_typed_review_inputs() {
    let now = OffsetDateTime::UNIX_EPOCH + time::Duration::hours(1);
    assert!(expiry(Kind::Mute, "1970-01-01T02:00", now).is_ok());
    assert!(expiry(Kind::Jail, "1970-01-01T01:00", now).is_err());
    assert!(expiry(Kind::Mute, "", now).is_err());
    assert!(expiry(Kind::Ban, "", now).is_ok());
    assert!(expiry(Kind::Ban, "1970-03-01T01:00", now).is_err());
    assert!(expiry(Kind::Ban, "1970-01-01T02:00+01:00", now).is_err());
    let target = Uuid::now_v7();
    let report = Uuid::now_v7();
    let form = format!(
        "_csrf=test&lang=en&target={target}&report={report}&kind=mute&expires=1970-01-01T02%3A00"
    );
    assert!(serde_urlencoded::from_str::<Apply>(&format!("{form}&reason=conduct")).is_ok());
    assert!(serde_urlencoded::from_str::<Apply>(&format!("{form}&reason=arbitrary")).is_err());
}

#[test]
fn review_lifecycle_requires_current_revision_and_independent_approval() {
    let requester = Uuid::now_v7();
    let reviewer = Uuid::now_v7();
    let advance = |stage, revision, action, actor, eligible| {
        transition(stage, revision, action, requester, actor, eligible)
    };
    assert_eq!(advance(1, 1, Action::Approve, reviewer, true), Ok(2));
    assert_eq!(advance(2, 2, Action::Lift, requester, true), Ok(3));
    assert_eq!(advance(3, 3, Action::Approve, reviewer, true), Ok(4));
    assert_eq!(
        advance(1, 1, Action::Approve, requester, true),
        Err(Error::Denied)
    );
    assert_eq!(
        advance(3, 3, Action::Approve, requester, true),
        Err(Error::Denied)
    );
    assert_eq!(
        advance(3, 2, Action::Approve, reviewer, true),
        Err(Error::Conflict)
    );
    assert_eq!(
        advance(4, 4, Action::Lift, requester, true),
        Err(Error::Conflict)
    );
    assert_eq!(
        advance(1, 1, Action::Approve, reviewer, false),
        Err(Error::Conflict)
    );
}

#[tokio::test]
async fn persists_scoped_reviews_and_audits() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("sanction_review_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let search_path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .after_connect(move |conn, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(search_path)
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    let result = exercise_review(&pool).await;
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    result
}

async fn account(pool: &PgPool) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO accounts(email,password_hash,status,email_verified_at,locale)
         VALUES($1,'test-only','active',clock_timestamp(),'en') RETURNING id",
    )
    .bind(format!("{}@example.invalid", Uuid::now_v7()))
    .fetch_one(pool)
    .await
}

async fn staff(pool: &PgPool, seed: u8) -> Result<CheckContext, sqlx::Error> {
    let actor = account(pool).await?;
    let session_hash = [seed; 32];
    let grant = format!("grant-{seed}");
    use sha2::{Digest, Sha256};
    let grant_hash: [u8; 32] = Sha256::digest(grant.as_bytes()).into();
    sqlx::query(
        "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='game_master_senior'",
    ).bind(actor).execute(pool).await?;
    sqlx::query(
        "WITH m AS (INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,$2,true) RETURNING generation)
         INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at)
         SELECT $3,$1,generation,sha256(convert_to('test-only','UTF8')),'grant',clock_timestamp()+interval '1 hour' FROM m",
    ).bind(actor).bind([0u8; 32].as_slice()).bind(grant_hash.as_slice()).execute(pool).await?;
    sqlx::query(
        "INSERT INTO web_sessions(id_hash,account_id,user_agent_summary,csrf_token,idle_expires_at,absolute_expires_at,sudo_until,data,authorization_generation)
         SELECT $2,id,'review-test',repeat('a',64),clock_timestamp()+interval '1 hour',clock_timestamp()+interval '1 hour',
                clock_timestamp()+interval '14 minutes',jsonb_build_object('mfa_grant',$3::text),native_security_generation
         FROM accounts WHERE id=$1",
    ).bind(actor).bind(session_hash.as_slice()).bind(grant).execute(pool).await?;
    Ok(CheckContext {
        account: actor,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: Some(rbac::SessionProof {
            session_hash,
            grant_hash,
        }),
    })
}

async fn exercise_review(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    oteryn_db::migrate(pool).await?;
    let service = Service {
        pool: pool.clone(),
        auth: Authorizer::new(
            pool.clone(),
            crate::web::PortalIdentity::configured(pool.clone()),
        )
        .with_game_flags(rbac::GamePermissionFlags {
            enabled: true,
            deploy: false,
        }),
    };
    let proposer = staff(pool, 1).await?;
    let reviewer = staff(pool, 2).await?;
    let subject = account(pool).await?;
    let target: Uuid = sqlx::query_scalar(
        "WITH w AS (INSERT INTO world_status(id,name) VALUES(uuidv7(),'Review world') RETURNING id)
         INSERT INTO admin_sanction_review_targets(world_id,account_id,character_id)
         SELECT id,$1,uuidv7() FROM w RETURNING admin_sanction_review_targets.id",
    )
    .bind(subject)
    .fetch_one(pool)
    .await?;
    let report = sqlx::query_scalar(
        "INSERT INTO player_reports(reporter_account_id,request_key,report_type,category,target_reference)
         VALUES($1,uuidv7(),'player','conduct',($1::uuid)::text) RETURNING id",
    ).bind(subject).fetch_one(pool).await?;
    let grant = |staff: CheckContext| async move {
        sqlx::query("INSERT INTO admin_sanction_review_grants(staff_id,target_id) VALUES($1,$2)")
            .bind(staff.account)
            .bind(target)
            .execute(pool)
            .await
    };
    grant(proposer).await?;
    let apply = || Apply {
        _csrf: String::new(),
        lang: "en".into(),
        target,
        report,
        kind: Kind::Ban,
        reason: Reason::Conduct,
        expires: String::new(),
    };
    let advance = |revision, action, reason| Advance {
        _csrf: String::new(),
        lang: "en".into(),
        revision,
        action,
        reason,
    };
    assert_eq!(service.propose(reviewer, apply()).await, Err(Error::Denied));
    let id = service.propose(proposer, apply()).await?;
    assert_eq!(
        service.propose(proposer, apply()).await,
        Err(Error::Conflict)
    );
    assert_eq!(
        service
            .advance(proposer, id, advance(1, Action::Approve, None))
            .await,
        Err(Error::Denied)
    );
    assert_eq!(
        service
            .advance(reviewer, id, advance(1, Action::Approve, None))
            .await,
        Err(Error::Denied)
    );
    grant(reviewer).await?;
    service
        .advance(reviewer, id, advance(1, Action::Approve, None))
        .await?;
    service
        .advance(
            proposer,
            id,
            advance(2, Action::Lift, Some(LiftReason::Review)),
        )
        .await?;
    service
        .advance(reviewer, id, advance(3, Action::Approve, None))
        .await?;
    let row: Review = sqlx::query_as("SELECT * FROM admin_sanction_reviews WHERE id=$1")
        .bind(id)
        .fetch_one(pool)
        .await?;
    assert_eq!(row.stage(), 4);
    let revisions: Vec<String> = sqlx::query_scalar(
        "SELECT after->>'revision' FROM audit_log WHERE target=$1 ORDER BY after->>'revision'",
    )
    .bind(id.to_string())
    .fetch_all(pool)
    .await?;
    assert_eq!(revisions, ["1", "2", "3", "4"]);
    let game_commands: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM support_enforcement_commands) + (SELECT count(*) FROM admin_control_requests)",
    ).fetch_one(pool).await?;
    assert_eq!(game_commands, 0);
    Ok(())
}
