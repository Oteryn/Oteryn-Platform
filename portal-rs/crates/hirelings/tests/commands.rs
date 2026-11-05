use oteryn_db::outbox::{OffsetDateTime, PgPool, Uuid};
use oteryn_hirelings::commands::*;
use std::error::Error as StdError;
type TestResult<T = ()> = std::result::Result<T, Box<dyn StdError + Send + Sync>>;
const SOURCE: &str = "oteryn:game:primary";
#[derive(Debug, Clone, Copy)]
enum Mode {
    Verified,
    Unknown,
    Ambiguous,
    Foreign,
    Stale,
    InSession,
    OldState,
    SourceMoved,
}
#[derive(Debug)]
struct Authority(Mode);
impl CommandAuthority for Authority {
    fn inspect(&self, account: Uuid, r: &CommandRequest) -> std::result::Result<Preflight, Reject> {
        let mut p = Preflight {
            source_authority: SOURCE.into(),
            account_id: account,
            character_id: r.character_id,
            world_id: r.world_id,
            hireling_id: r.hireling_id,
            character_revision: r.expected_character_revision,
            hireling_revision: r.expected_hireling_revision,
            complete_through: OffsetDateTime::now_utc().unix_timestamp(),
            baseline_and_recovery: Evidence::Confirmed,
            active_account_and_security: Evidence::Confirmed,
            current_ownership_and_privacy: Evidence::Confirmed,
            placed_in_owned_house: Evidence::Confirmed,
            character_offline: Evidence::Confirmed,
            approved_non_currency_quote: Evidence::Confirmed,
            chest_capacity_available: Evidence::Confirmed,
            weekly_budget_available: Evidence::Confirmed,
            unfinished_account_expeditions: Some(0),
            state: match &r.action {
                Action::StartHirelingExpedition(b) => ObservedState::Ready(b.clone()),
                Action::CollectHirelingExpedition(b) => ObservedState::ReadyToCollect(b.clone()),
            },
        };
        match self.0 {
            Mode::Verified => {}
            Mode::SourceMoved => p.source_authority = "oteryn:game:secondary".into(),
            Mode::Unknown => p.current_ownership_and_privacy = Evidence::Unknown,
            Mode::Ambiguous => p.state = ObservedState::Ambiguous,
            Mode::Foreign => p.current_ownership_and_privacy = Evidence::Denied,
            Mode::Stale => p.complete_through -= 31,
            Mode::InSession => p.character_offline = Evidence::Denied,
            Mode::OldState => {
                p.hireling_revision = Positive::new(99)?;
                p.state = ObservedState::Collected;
            }
        }
        Ok(p)
    }
}
fn request(collect: bool) -> TestResult<CommandRequest> {
    Ok(CommandRequest {
        idempotency_key: Uuid::now_v7(),
        character_id: Uuid::now_v7(),
        world_id: Uuid::now_v7(),
        hireling_id: Uuid::now_v7(),
        expected_character_revision: Positive::new(12)?,
        expected_hireling_revision: Positive::new(3)?,
        action: if collect {
            Action::CollectHirelingExpedition(CollectBinding {
                expedition_id: Uuid::now_v7(),
                expected_expedition_revision: Positive::new(2)?,
            })
        } else {
            Action::StartHirelingExpedition(StartBinding {
                reward_table_key: "home-v1".into(),
                reward_table_revision: Positive::new(7)?,
                reward_row_key: "miner-1-short".into(),
            })
        },
    })
}
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> TestResult<Option<Self>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("skipping PostgreSQL command test: PORTAL_TEST_DATABASE_URL unset");
            return Ok(None);
        };
        let admin = PgPool::connect(&url).await?;
        let schema = format!("t92b_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let search_path = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |connection, _| {
                let sql = format!("SET search_path TO {search_path}");
                Box::pin(async move {
                    sqlx::query(sqlx::AssertSqlSafe(sql))
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        sqlx::raw_sql(include_str!(
            "../../../migrations/20261105120000_t37_outbox.sql"
        ))
        .execute(&pool)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../../migrations/20261105231304_t92b_command_boundary.sql"
        ))
        .execute(&pool)
        .await?;
        Ok(Some(Self {
            pool,
            admin,
            schema,
        }))
    }
    fn service(&self, mode: Mode) -> Commands<Authority> {
        Commands {
            pool: self.pool.clone(),
            flags: CommandFlags {
                projection_enabled: true,
                commands_enabled: true,
                start_enabled: true,
                collect_enabled: true,
            },
            source_authority: SOURCE.into(),
            authority: Authority(mode),
        }
    }
    async fn counts(&self) -> TestResult<(i64, i64)> {
        Ok(sqlx::query_as(
            "SELECT (SELECT count(*) FROM hireling_command_intents),(SELECT count(*) FROM outbox)",
        )
        .fetch_one(&self.pool)
        .await?)
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
fn rejected<T: std::fmt::Debug>(result: Result<T>, reason: Reject) {
    assert!(
        matches!(result, Err(Error::Rejected(actual)) if actual == reason),
        "expected {reason:?}"
    );
}
#[tokio::test]
async fn postgres_start_concurrent_replay_conflict_and_immutable_bytes() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let service = db.service(Mode::Verified);
    let account = Uuid::now_v7();
    let r = request(false)?;
    let (a, b) = tokio::join!(service.enqueue(account, &r), service.enqueue(account, &r));
    let first = a?;
    assert_eq!(first, b?);
    assert_eq!(db.counts().await?, (1, 1));
    assert_eq!(
        first,
        db.service(Mode::OldState).enqueue(account, &r).await?
    );

    rejected(
        db.service(Mode::Ambiguous).enqueue(account, &r).await,
        Reject::Unavailable,
    );
    let mut moved = db.service(Mode::SourceMoved);
    moved.source_authority = "oteryn:game:secondary".into();
    rejected(moved.enqueue(account, &r).await, Reject::Unavailable);
    let mut changed = r.clone();
    changed.action = request(true)?.action;
    rejected(
        service.enqueue(account, &changed).await,
        Reject::IdempotencyConflict,
    );
    changed = r.clone();
    changed.idempotency_key = Uuid::now_v7();
    rejected(service.enqueue(account, &changed).await, Reject::Busy);
    let body: serde_json::Value = serde_json::from_str(&first.intent_bytes)?;
    assert_eq!(body["command"], "StartHirelingExpedition");
    assert_eq!(body["operation_id"], first.operation_id.to_string());
    assert_eq!(body["expected_character_revision"], "12");
    assert!(body.get("expedition_id").is_none());
    assert!(body.get("rewards").is_none());
    assert!(body.get("idempotency_key").is_none());
    let issued: i64 = body["issued_at_source"]
        .as_str()
        .ok_or("missing issue time")?
        .parse()?;
    let expires: i64 = body["expires_at_source"]
        .as_str()
        .ok_or("missing expiry")?
        .parse()?;
    assert_eq!(expires - issued, 300);
    let payload: String =
        sqlx::query_scalar("SELECT payload::text FROM outbox WHERE kind='hirelings.command.v1'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&payload)?,
        serde_json::json!({"operation_id": first.operation_id})
    );
    assert!(
        sqlx::query("UPDATE hireling_command_intents SET intent_bytes='{}'")
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM hireling_command_intents")
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert_eq!(first, service.enqueue(account, &r).await?);
    // Same browser key in another authenticated account has an independent scope.
    let mut separate = r.clone();
    separate.hireling_id = Uuid::now_v7();
    let second = service.enqueue(Uuid::now_v7(), &separate).await?;
    assert_ne!(first.operation_id, second.operation_id);
    assert!(second.source_revision > first.source_revision);
    assert_eq!(db.counts().await?, (2, 2));
    db.close().await
}
#[tokio::test]
async fn postgres_collect_refusals_and_no_inferred_outcome() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let account = Uuid::now_v7();
    let r = request(true)?;
    for (mode, reason) in [
        (Mode::Unknown, Reject::Unavailable),
        (Mode::Ambiguous, Reject::Unavailable),
        (Mode::Foreign, Reject::NotEligible),
        (Mode::Stale, Reject::Unavailable),
        (Mode::InSession, Reject::CharacterInSession),
    ] {
        rejected(db.service(mode).enqueue(account, &r).await, reason);
        assert_eq!(db.counts().await?, (0, 0));
    }
    let mut bad = r.clone();
    bad.character_id = Uuid::nil();
    rejected(
        db.service(Mode::Verified).enqueue(account, &bad).await,
        Reject::InvalidInput,
    );
    let first = db.service(Mode::Verified).enqueue(account, &r).await?;
    let bytes: serde_json::Value = serde_json::from_str(&first.intent_bytes)?;
    assert_eq!(bytes["command"], "CollectHirelingExpedition");
    assert_eq!(bytes["expected_expedition_revision"], "2");
    assert!(bytes.get("reward_table_key").is_none());
    assert!(bytes.get("chest_id").is_none());
    rejected(
        db.service(Mode::Foreign).enqueue(account, &r).await,
        Reject::NotEligible,
    );
    let mut off = db.service(Mode::Verified);
    off.flags.collect_enabled = false;
    assert_eq!(first, off.enqueue(account, &r).await?);
    let new = request(true)?;
    rejected(off.enqueue(account, &new).await, Reject::Disabled);
    off.flags.collect_enabled = true;
    off.flags.commands_enabled = false;
    rejected(off.enqueue(account, &new).await, Reject::Disabled);
    assert_eq!(db.counts().await?, (1, 1));
    assert_eq!(
        first,
        db.service(Mode::Verified).enqueue(account, &r).await?
    );
    db.close().await
}
#[tokio::test]
async fn postgres_outbox_failure_rolls_back_intent_and_sequence() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    sqlx::raw_sql("CREATE FUNCTION refuse_outbox() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test refusal'; END; $$; CREATE TRIGGER refuse_outbox BEFORE INSERT ON outbox FOR EACH ROW EXECUTE FUNCTION refuse_outbox();").execute(&db.pool).await?;
    let service = db.service(Mode::Verified);
    let account = Uuid::now_v7();
    let r = request(false)?;
    assert!(matches!(
        service.enqueue(account, &r).await,
        Err(Error::Storage(_))
    ));
    assert_eq!(db.counts().await?, (0, 0));
    let revision: String =
        sqlx::query_scalar("SELECT revision::text FROM hireling_command_sequence")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(revision, "0");
    sqlx::raw_sql("DROP TRIGGER refuse_outbox ON outbox")
        .execute(&db.pool)
        .await?;
    assert_eq!(service.enqueue(account, &r).await?.source_revision.get(), 1);
    db.close().await
}
#[tokio::test]
async fn postgres_account_rate_and_authority_backlog_are_serialized() -> TestResult {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let service = db.service(Mode::Verified);
    let account = Uuid::now_v7();
    for index in 0..40 {
        let owner = if index < 10 { account } else { Uuid::now_v7() };
        let queued = service.enqueue(owner, &request(false)?).await?;
        assert_eq!(queued.source_revision.get(), index + 1);
        if index == 9 {
            rejected(
                service.enqueue(account, &request(false)?).await,
                Reject::RateLimited,
            );
        }
    }
    rejected(
        service.enqueue(Uuid::now_v7(), &request(true)?).await,
        Reject::BacklogFull,
    );
    assert_eq!(db.counts().await?, (40, 40));
    db.close().await
}
