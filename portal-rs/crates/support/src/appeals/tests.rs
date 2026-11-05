use super::*;
use sqlx::{Connection, PgConnection};

async fn database() -> Result<Option<PgConnection>, Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("PostgreSQL appeal test skipped: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(None);
    };
    let mut db = PgConnection::connect(&url).await?;
    let migration = include_str!("../../../../migrations/20261105220236_t34b_appeals.sql")
        .replace("CREATE TABLE", "CREATE TEMP TABLE");
    // The only substitution is a fixed temporary-table keyword in a checked-in migration.
    sqlx::raw_sql(sqlx::AssertSqlSafe(migration))
        .execute(&mut db)
        .await?;
    Ok(Some(db))
}

#[tokio::test]
async fn postgres_success_acknowledge_reject_resubmit_accept()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(mut db) = database().await? else {
        return Ok(());
    };
    let api = Appeals { enabled: true };
    let owner = Uuid::now_v7();
    let sanction = Uuid::now_v7();
    api.register(&mut db, sanction, owner).await?;
    let first = api
        .change(&mut db, owner, sanction, 1, Action::Acknowledge)
        .await?;
    assert_eq!(first.version, 2);
    assert_eq!(
        api.change(&mut db, owner, sanction, 2, Action::Acknowledge)
            .await?,
        first
    );
    api.change(
        &mut db,
        owner,
        sanction,
        2,
        Action::Submit("  explanation  "),
    )
    .await?;
    let reviewing = api
        .change(
            &mut db,
            owner,
            sanction,
            3,
            Action::Decide {
                status: Status::Reviewing,
                outcome: "",
            },
        )
        .await?;
    assert_eq!(reviewing.message.as_deref(), Some("explanation"));
    api.change(
        &mut db,
        owner,
        sanction,
        4,
        Action::Decide {
            status: Status::Rejected,
            outcome: "Public reason",
        },
    )
    .await?;
    let submitted = api
        .change(&mut db, owner, sanction, 5, Action::Submit("new evidence"))
        .await?;
    assert_eq!(submitted.outcome, None);
    let accepted = api
        .change(
            &mut db,
            owner,
            sanction,
            6,
            Action::Decide {
                status: Status::Accepted,
                outcome: "Decision corrected",
            },
        )
        .await?;
    assert_eq!(accepted.status, Status::Accepted);
    assert_eq!(api.history(&mut db, owner, 20, 0).await?, vec![accepted]);
    Ok(())
}

#[tokio::test]
async fn postgres_refuses_foreign_owner_stale_version_and_duplicate_appeal()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(mut db) = database().await? else {
        return Ok(());
    };
    let api = Appeals { enabled: true };
    let owner = Uuid::now_v7();
    let sanction = Uuid::now_v7();
    api.register(&mut db, sanction, owner).await?;
    assert!(matches!(
        Appeals::default().history(&mut db, owner, 20, 0).await,
        Err(Error::Disabled)
    ));
    let foreign = Uuid::now_v7();
    assert!(api.history(&mut db, foreign, 20, 0).await?.is_empty());
    assert!(matches!(
        api.change(&mut db, foreign, sanction, 1, Action::Acknowledge)
            .await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        api.change(
            &mut db,
            owner,
            sanction,
            1,
            Action::Decide {
                status: Status::Accepted,
                outcome: "reason"
            }
        )
        .await,
        Err(Error::Conflict)
    ));
    api.change(&mut db, owner, sanction, 1, Action::Submit("reason"))
        .await?;
    assert!(matches!(
        api.change(&mut db, owner, sanction, 1, Action::Acknowledge)
            .await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        api.change(&mut db, owner, sanction, 2, Action::Submit("duplicate"))
            .await,
        Err(Error::Conflict)
    ));
    Ok(())
}

#[tokio::test]
async fn postgres_bad_input_preserves_state() -> Result<(), Box<dyn std::error::Error>> {
    let Some(mut db) = database().await? else {
        return Ok(());
    };
    let api = Appeals { enabled: true };
    let owner = Uuid::now_v7();
    let sanction = Uuid::now_v7();
    api.register(&mut db, sanction, owner).await?;
    for message in ["   ".to_owned(), "x".repeat(4001), "a\0b".to_owned()] {
        assert!(matches!(
            api.change(&mut db, owner, sanction, 1, Action::Submit(&message))
                .await,
            Err(Error::Invalid)
        ));
    }
    assert!(matches!(
        api.history(&mut db, owner, 101, 0).await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        api.change(&mut db, owner, sanction, 0, Action::Acknowledge)
            .await,
        Err(Error::Invalid)
    ));
    api.change(&mut db, owner, sanction, 1, Action::Submit("reason"))
        .await?;
    assert!(matches!(
        api.change(
            &mut db,
            owner,
            sanction,
            2,
            Action::Decide {
                status: Status::Accepted,
                outcome: " "
            }
        )
        .await,
        Err(Error::Invalid)
    ));
    assert_eq!(api.history(&mut db, owner, 1, 0).await?[0].version, 2);
    Ok(())
}

#[test]
fn unknown_status_denies_and_http_errors_match_reference() {
    assert!(matches!(Status::parse("unknown"), Err(Error::Conflict)));
    assert_eq!(Error::NotFound.status_code(), 404);
    assert_eq!(Error::Invalid.status_code(), 422);
    assert_eq!(Error::Conflict.status_code(), 409);
    assert!(!Appeals::default().enabled);
}

#[derive(Debug)]
struct Authority {
    fail_effects: bool,
}
impl Sanctions for Authority {
    async fn account_access(&self, _: &mut PgConnection, _: Uuid) -> Result<bool, Error> {
        Ok(true)
    }
    async fn authorize(
        &self,
        _: &mut PgConnection,
        _: Uuid,
        _: Uuid,
        _: Uuid,
        _: bool,
    ) -> Result<bool, Error> {
        Ok(true)
    }
    async fn decision_effects(
        &self,
        db: &mut PgConnection,
        _: Uuid,
        _: Uuid,
        _: &History,
    ) -> Result<(), Error> {
        sqlx::query("INSERT INTO appeal_test_effects DEFAULT VALUES")
            .execute(db)
            .await?;
        if self.fail_effects {
            Err(Error::Conflict)
        } else {
            Ok(())
        }
    }
}

async fn service_pool() -> Result<Option<sqlx::PgPool>, Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("PostgreSQL appeal test skipped: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(None);
    };
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    let mut db = pool.acquire().await?;
    // Fixed keyword substitution in repository SQL, no external input.
    let migration = include_str!("../../../../migrations/20261105220236_t34b_appeals.sql")
        .replace("CREATE TABLE", "CREATE TEMP TABLE");
    sqlx::raw_sql(sqlx::AssertSqlSafe(migration))
        .execute(&mut *db)
        .await?;
    sqlx::query("CREATE TEMP TABLE appeal_test_effects (id bigint GENERATED ALWAYS AS IDENTITY)")
        .execute(&mut *db)
        .await?;
    drop(db);
    Ok(Some(pool))
}

#[tokio::test]
async fn postgres_service_rolls_back_decision_and_effects_together()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = service_pool().await? else {
        return Ok(());
    };
    let owner = Uuid::now_v7();
    let sanction = Uuid::now_v7();
    let api = Appeals { enabled: true };
    api.register(&mut *pool.acquire().await?, sanction, owner)
        .await?;
    let service = Workflow {
        pool: pool.clone(),
        appeals: api,
        sanctions: Authority { fail_effects: true },
    };
    service
        .change(owner, owner, sanction, 1, Action::Submit("reason"))
        .await?;
    assert!(matches!(
        service
            .change(
                Uuid::now_v7(),
                owner,
                sanction,
                2,
                Action::Decide {
                    status: Status::Accepted,
                    outcome: "corrected"
                }
            )
            .await,
        Err(Error::Conflict)
    ));
    let records = service.history(owner, 0).await?;
    assert_eq!(records[0].status, Status::Requested);
    assert_eq!(records[0].version, 2);
    assert_eq!(service.detail(owner, sanction).await?, records[0]);
    assert!(matches!(
        service.detail(Uuid::now_v7(), sanction).await,
        Err(Error::NotFound)
    ));
    let effects: i64 = sqlx::query_scalar("SELECT count(*) FROM appeal_test_effects")
        .fetch_one(&pool)
        .await?;
    assert_eq!(effects, 0);
    let service = Workflow {
        pool: pool.clone(),
        appeals: api,
        sanctions: Authority {
            fail_effects: false,
        },
    };
    service
        .change(
            Uuid::now_v7(),
            owner,
            sanction,
            2,
            Action::Decide {
                status: Status::Accepted,
                outcome: "corrected",
            },
        )
        .await?;
    let effects: i64 = sqlx::query_scalar("SELECT count(*) FROM appeal_test_effects")
        .fetch_one(&pool)
        .await?;
    assert_eq!(effects, 1);
    assert!(matches!(
        service
            .change(owner, owner, sanction, 3, Action::Submit("again"))
            .await,
        Err(Error::Conflict)
    ));
    pool.close().await;
    Ok(())
}

#[tokio::test]
async fn postgres_service_unknown_authority_denies() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = service_pool().await? else {
        return Ok(());
    };
    let owner = Uuid::now_v7();
    let sanction = Uuid::now_v7();
    let api = Appeals { enabled: true };
    api.register(&mut *pool.acquire().await?, sanction, owner)
        .await?;
    let service = Workflow {
        pool: pool.clone(),
        appeals: api,
        sanctions: UnavailableSanctions,
    };
    assert!(matches!(
        service.history(owner, 0).await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        service
            .change(owner, owner, sanction, 1, Action::Acknowledge)
            .await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        service
            .change(Uuid::now_v7(), owner, sanction, 1, Action::Submit("reason"))
            .await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        service.history(owner, i64::MAX).await,
        Err(Error::Invalid)
    ));
    assert_eq!(
        api.history(&mut *pool.acquire().await?, owner, 20, 0)
            .await?[0]
            .version,
        1
    );
    pool.close().await;
    Ok(())
}
