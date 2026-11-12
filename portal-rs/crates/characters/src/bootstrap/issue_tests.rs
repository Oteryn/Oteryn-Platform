#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::bootstrap::{Reader, Settings};
use crate::lcfa::{AccountState, AccountStateLookup};
use sqlx::postgres::PgPoolOptions;
use std::{future::Future, pin::Pin, sync::Arc};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Change = Box<dyn Fn(&mut IssueInput)>;
const ACCOUNT: &str = "019a0000-0000-7000-8000-00000000a001";
const CLOSED: &str = "019a0000-0000-7000-8000-00000000a002";
const OP: &str = "019a0000-0000-7000-8000-0000000b0001";
const WORLD: &str = "019a0000-0000-7000-8000-00000000e001";

struct Database {
    admin: PgPool,
    pool: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED bootstrap issue PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return Ok(None);
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let schema = format!("bootstrap_issue_{}", Uuid::now_v7().simple());
        // Schema identifiers contain only a fixed prefix and generated UUID hex.
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |connection, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path', $1, false)")
                        .bind(path)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        // The Identity columns issuance reads; the full schema is covered by the
        // oteryn-portal seed tests, which run every migration.
        sqlx::raw_sql("CREATE TABLE accounts (id uuid PRIMARY KEY, status text NOT NULL)")
            .execute(&pool)
            .await?;
        sqlx::raw_sql(include_str!(
            "../../../../migrations/20261105220103_t22b_bootstrap_intents.sql"
        ))
        .execute(&pool)
        .await?;
        sqlx::query(
            "INSERT INTO accounts (id, status) VALUES ($1::uuid, 'active'), ($2::uuid, 'closed')",
        )
        .bind(ACCOUNT)
        .bind(CLOSED)
        .execute(&pool)
        .await?;
        Ok(Some(Self {
            admin,
            pool,
            schema,
        }))
    }
    async fn count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT count(*) FROM character_bootstrap_intents")
            .fetch_one(&self.pool)
            .await
    }
    async fn close(self) -> TestResult {
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

#[derive(Debug)]
struct Active;
impl AccountStateLookup for Active {
    fn state(&self, _: AccountId) -> Pin<Box<dyn Future<Output = AccountState> + Send + '_>> {
        Box::pin(async { AccountState::Active })
    }
}

fn input() -> IssueInput {
    IssueInput {
        identity_id: ACCOUNT.into(),
        operation_id: OP.into(),
        target_world_id: WORLD.into(),
        requested_name: "Aldric Vale".into(),
        requested_sex: "Male".into(),
        context: Context {
            profile_revision: "profile:1".into(),
            ruleset_revision: "ruleset:1".into(),
            content_revision: "content:1".into(),
            starter_template_revision: "starter:1".into(),
        },
        ttl_seconds: 60,
    }
}

#[test]
fn every_field_is_validated_before_any_transaction() {
    assert!(validate(&input()).is_ok());
    let cases: Vec<(&str, Change)> = vec![
        (
            "identity not uuid",
            Box::new(|i| i.identity_id = "x".into()),
        ),
        (
            "identity upper case",
            Box::new(|i| i.identity_id = ACCOUNT.to_uppercase()),
        ),
        (
            "identity not v7",
            Box::new(|i| i.identity_id = "019a0000-0000-4000-8000-00000000a001".into()),
        ),
        (
            "operation",
            Box::new(|i| i.operation_id = "not-a-uuid".into()),
        ),
        (
            "operation upper case",
            Box::new(|i| i.operation_id = OP.to_uppercase()),
        ),
        ("world", Box::new(|i| i.target_world_id = String::new())),
        ("name short", Box::new(|i| i.requested_name = "A".into())),
        ("name digit", Box::new(|i| i.requested_name = "Al1".into())),
        (
            "name double space",
            Box::new(|i| i.requested_name = "Two  Spaces".into()),
        ),
        ("name long", Box::new(|i| i.requested_name = "A".repeat(30))),
        (
            "name non-ascii",
            Box::new(|i| i.requested_name = "Żaba".into()),
        ),
        ("sex case", Box::new(|i| i.requested_sex = "male".into())),
        ("sex numeric", Box::new(|i| i.requested_sex = "1".into())),
        (
            "profile",
            Box::new(|i| i.context.profile_revision = ":leading".into()),
        ),
        (
            "ruleset",
            Box::new(|i| i.context.ruleset_revision = String::new()),
        ),
        (
            "content",
            Box::new(|i| i.context.content_revision = "a".repeat(129)),
        ),
        (
            "starter",
            Box::new(|i| i.context.starter_template_revision = "a b".into()),
        ),
        ("ttl zero", Box::new(|i| i.ttl_seconds = 0)),
        ("ttl negative", Box::new(|i| i.ttl_seconds = -1)),
        ("ttl large", Box::new(|i| i.ttl_seconds = 301)),
    ];
    for (case, change) in cases {
        let mut bad_input = input();
        change(&mut bad_input);
        assert!(
            matches!(validate(&bad_input), Err(IssueError::BadInput(_))),
            "{case}"
        );
    }
    let mut longest = input();
    longest.context.content_revision = "a".repeat(128);
    longest.ttl_seconds = 300;
    assert!(validate(&longest).is_ok());
}

#[tokio::test]
async fn issues_once_replays_exactly_and_refuses_conflicts() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let first = issue(&db.pool, input()).await?;
    assert_eq!(first.source_revision, "1");
    assert_eq!(first.account_id, ACCOUNT);
    let issued: i64 = first.issued_at_source.parse()?;
    let expires: i64 = first.expires_at_source.parse()?;
    assert_eq!(expires - issued, 60);
    assert_eq!(
        format!("{:x}", Sha256::digest(first.intent_json.as_bytes())),
        first.intent_sha256
    );

    // The internal read route accepts the stored intent byte for byte.
    let reader = Reader::new(
        db.pool.clone(),
        Settings { enabled: true },
        Arc::new(Active),
    );
    assert_eq!(
        reader.current(OP, issued).await?,
        first.intent_json.as_bytes()
    );

    // An exact retry returns the stored decision unchanged.
    assert_eq!(issue(&db.pool, input()).await?, first);
    assert_eq!(db.count().await?, 1);

    // Any changed binding under the same operation id is a conflict.
    let changes: Vec<Change> = vec![
        Box::new(|i| i.target_world_id = "019a0000-0000-7000-8000-00000000e002".into()),
        Box::new(|i| i.requested_name = "Other Name".into()),
        Box::new(|i| i.requested_sex = "Female".into()),
        Box::new(|i| i.context.profile_revision = "profile:2".into()),
        Box::new(|i| i.context.ruleset_revision = "ruleset:2".into()),
        Box::new(|i| i.context.content_revision = "content:2".into()),
        Box::new(|i| i.context.starter_template_revision = "starter:2".into()),
    ];
    for change in changes {
        let mut changed = input();
        change(&mut changed);
        assert_eq!(issue(&db.pool, changed).await, Err(IssueError::Conflict));
    }
    // A different TTL is not part of the binding: the stored decision is returned.
    let mut ttl = input();
    ttl.ttl_seconds = 5;
    assert_eq!(issue(&db.pool, ttl).await?, first);

    // A new operation advances the issuer revision.
    let mut next = input();
    next.operation_id = "019a0000-0000-7000-8000-0000000b0002".into();
    let second = issue(&db.pool, next).await?;
    assert_eq!(second.source_revision, "2");
    assert_ne!(second.issuer_decision_id, first.issuer_decision_id);
    assert_eq!(db.count().await?, 2);
    db.close().await
}

#[tokio::test]
async fn refuses_inactive_or_unknown_identity_and_bad_input() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    for identity in [CLOSED, "019a0000-0000-7000-8000-00000000dead"] {
        let mut refused = input();
        refused.identity_id = identity.into();
        assert!(matches!(
            issue(&db.pool, refused).await,
            Err(IssueError::BadInput(_))
        ));
    }
    let mut bad_input = input();
    bad_input.requested_sex = "female".into();
    assert!(matches!(
        issue(&db.pool, bad_input).await,
        Err(IssueError::BadInput(_))
    ));
    assert_eq!(db.count().await?, 0);
    // Refusals allocate no source revision.
    assert_eq!(issue(&db.pool, input()).await?.source_revision, "1");
    db.close().await
}

#[tokio::test]
async fn authority_rollback_and_retired_rows_fail_closed() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    issue(&db.pool, input()).await?;

    // A retained version-2 row (no requested sex) is not current: retry is unavailable.
    sqlx::query("UPDATE character_bootstrap_intents SET requested_sex=NULL")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        issue(&db.pool, input()).await,
        Err(IssueError::Unavailable("stored intent"))
    );

    // Authority row removed or lowered beneath a retained row: nothing is issued.
    let mut fresh = input();
    fresh.operation_id = "019a0000-0000-7000-8000-0000000b0009".into();
    sqlx::query("DELETE FROM character_bootstrap_intent_authority")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        issue(&db.pool, fresh.clone()).await,
        Err(IssueError::Unavailable("authority"))
    );
    // Equal revision bound to another decision identity.
    sqlx::query("INSERT INTO character_bootstrap_intent_authority VALUES (1, '1', '019a0000-0000-7000-8000-00000000ffff')")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        issue(&db.pool, fresh).await,
        Err(IssueError::Unavailable("authority"))
    );
    assert_eq!(db.count().await?, 1);
    db.close().await
}
