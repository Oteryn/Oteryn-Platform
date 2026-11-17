use oteryn_admin::rbac::{Authorizer, CheckContext};
use sqlx::PgPool;
use uuid::Uuid;
type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

struct TestDb {
    pool: PgPool,
    root: PgPool,
    schema: String,
    context: CheckContext,
}
impl TestDb {
    async fn open() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
        let root = PgPool::connect(&url).await?;
        let schema = format!("last_wait_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await?;
        let path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |c, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(path)
                        .execute(c)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        let actor = Uuid::now_v7();
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'fixture-password','active','en',now())").bind(actor).bind(format!("{actor}@example.invalid")).execute(&pool).await?;
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&pool).await?;
        let context = CheckContext {
            account: actor,
            require_staff_mfa: true,
            mfa_verified: true,
            session_proof: Some(proof(&pool, actor).await?),
        };
        Ok(Self {
            pool,
            root,
            schema,
            context,
        })
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await?;
        self.root.close().await;
        Ok(())
    }
}

// Session/grant rows are retained FOR UPDATE by authorization, so arrange a
// database-clock grant deadline before starting, then observe it elapse while
// the real mutation is blocked on this backend's audit table lock.
async fn audit_wait<F, T>(
    pool: &PgPool,
    context: CheckContext,
    operation: F,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let mut blocker = pool.begin().await?;
    sqlx::query("LOCK TABLE audit_log IN SHARE MODE")
        .execute(&mut *blocker)
        .await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;
    sqlx::query("UPDATE account_mfa_tokens SET expires_at=clock_timestamp()+interval '1 second' WHERE account_id=$1").bind(context.account).execute(pool).await?;
    let task = tokio::spawn(operation);
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            sqlx::query("SELECT pg_stat_clear_snapshot()").execute(&mut *blocker).await?;
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)) AND wait_event_type='Lock')").bind(pid).fetch_one(&mut *blocker).await?;
            if waiting { break; }
            tokio::task::yield_now().await;
        }
        loop {
            sqlx::query("SELECT pg_stat_clear_snapshot()").execute(&mut *blocker).await?;
            let expired_while_waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid))) AND EXISTS(SELECT 1 FROM account_mfa_tokens WHERE account_id=$2 AND expires_at<=clock_timestamp())").bind(pid).bind(context.account).fetch_one(&mut *blocker).await?;
            if expired_while_waiting { break; }
            tokio::task::yield_now().await;
        }
        Ok::<(), sqlx::Error>(())
    }).await.map_err(|_| "mutation must reach audit wait and grant expiry within two seconds")??;
    blocker.rollback().await?;
    Ok(tokio::time::timeout(std::time::Duration::from_secs(2), task).await??)
}

#[tokio::test]
async fn postgres_last_wait_grant_expiry_rolls_back_party() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = TestDb::open().await?;
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO pd_party_posts(id,account_id,character_id,character_name,world_id,purpose,min_level,max_level,profession,note,starts_at,expires_at) VALUES($1,$2,$3,'Fixture',$4,'hunt',1,100,'any','Fixture',now(),now()+interval '1 hour')").bind(id).bind(db.context.account).bind(Uuid::now_v7()).bind(Uuid::now_v7()).execute(&db.pool).await?;
    let report_id: Uuid = sqlx::query_scalar(
        "INSERT INTO pd_party_reports(post_id,account_id,reason) VALUES($1,$2,'spam') RETURNING id",
    )
    .bind(id)
    .bind(db.context.account)
    .fetch_one(&db.pool)
    .await?;
    let pool = db.pool.clone();
    let context = db.context;
    let result = audit_wait(&db.pool, context, async move {
        let identity = oteryn_admin::web::PortalIdentity::configured(pool.clone());
        let authority = oteryn_admin::rbac::MutationAuthorization::new(std::sync::Arc::new(
            Authorizer::new(pool.clone(), identity),
        ));
        super::Board::new(
            pool.clone(),
            std::sync::Arc::new(super::ProductionAuthority { db: pool }),
        )
        .moderate(context, &authority, id, "remove", "spam", &[report_id])
        .await
    })
    .await?;
    assert!(matches!(result, Err(super::Error::Denied)));
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM pd_party_posts WHERE id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?,
        "active"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM pd_party_reports WHERE post_id=$1")
            .bind(id)
            .fetch_one(&db.pool)
            .await?,
        "open"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM audit_log")
            .fetch_one(&db.pool)
            .await?,
        0
    );
    db.close().await
}

use oteryn_admin::rbac::SessionProof;
use sha2::{Digest, Sha256};
async fn proof(pool: &PgPool, actor: Uuid) -> Result<SessionProof, sqlx::Error> {
    let password: String = sqlx::query_scalar("SELECT password_hash FROM accounts WHERE id=$1")
        .bind(actor)
        .fetch_one(pool)
        .await?;
    let grant = format!("{:x}", Sha256::digest(Uuid::now_v7().as_bytes()));
    let session = format!("{:x}", Sha256::digest(Uuid::now_v7().as_bytes()));
    let generation: Uuid = sqlx::query_scalar("INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,$2,true) ON CONFLICT(account_id) DO UPDATE SET confirmed=true RETURNING generation")
        .bind(actor).bind(Sha256::digest(Uuid::now_v7().as_bytes()).to_vec()).fetch_one(pool).await?;
    let grant_hash: [u8; 32] = Sha256::digest(grant.as_bytes()).into();
    let session_hash: [u8; 32] = Sha256::digest(session.as_bytes()).into();
    sqlx::query("INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at) VALUES($1,$2,$3,$4,'grant',clock_timestamp()+interval '1 hour')")
        .bind(grant_hash.as_slice()).bind(actor).bind(generation).bind(Sha256::digest(password.as_bytes()).to_vec()).execute(pool).await?;
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,data,authorization_generation) SELECT $1,id,now()+interval '1 hour',now()+interval '1 day','staff-fixture',$2,jsonb_build_object('mfa_grant',$3::text),native_security_generation FROM accounts WHERE id=$4")
        .bind(session_hash.as_slice()).bind(format!("{:x}", Sha256::digest(Uuid::now_v7().as_bytes()))).bind(grant).bind(actor).execute(pool).await?;
    Ok(SessionProof {
        session_hash,
        grant_hash,
    })
}
