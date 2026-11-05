use super::*;

fn cut() -> Cut {
    Cut {
        character_revision: 1,
        projection_revision: 2,
        projection_epoch: 3,
        recovery_generation: 4,
    }
}
fn session() -> Session {
    Session {
        session_id: Uuid::now_v7(),
        cut: cut(),
        started_at: 1000,
        ended_at: 1060,
        active_seconds: 60,
        experience_gained: 100,
        valuation: Some(Valuation {
            basis: "npc:r1".into(),
            loot: 1000,
            supplies: 1200,
        }),
        drops: vec![Drop {
            event_id: Uuid::now_v7(),
            occurred_at: 1010,
            catalog_revision: "r1".into(),
            item_key: "item:ruby".into(),
            quantity: 1,
            monster_key: None,
            place_key: None,
            rarity_rule_key: "rarity:rare".into(),
        }],
    }
}
#[test]
fn rates_records_and_chronicle() -> Result<(), Error> {
    let first = session();
    let mut second = session();
    second.started_at = 1060;
    second.ended_at = 1120;
    second.drops[0].occurred_at = 1070;
    second.experience_gained = 200;
    second.valuation = None;
    let result = Presentation::from_sessions(vec![second.clone(), first])?;
    assert_eq!(result.drops.len(), 2);
    assert_eq!(result.chronicle[0], second);
    assert_eq!(
        result
            .best_experience_per_hour
            .as_ref()
            .map(|r| r.session_id),
        Some(second.session_id)
    );
    assert_eq!(result.largest_loot_by_basis["npc:r1"].value, 1000);
    assert_eq!(
        result.best_profit_per_hour_by_basis["npc:r1"]
            .value
            .numerator,
        -720000
    );
    assert!(result.first_boss_kill.is_none());
    Ok(())
}
#[test]
fn zero_duration_and_missing_valuation_are_unavailable() -> Result<(), Error> {
    let mut value = session();
    value.active_seconds = 0;
    value.valuation = None;
    let result = Presentation::from_sessions(vec![value])?;
    assert!(result.best_experience_per_hour.is_none());
    assert!(result.best_profit_per_hour_by_basis.is_empty());
    assert!(result.largest_loot_by_basis.is_empty());
    assert_eq!(
        Presentation::from_sessions(vec![])?,
        Presentation::default()
    );
    Ok(())
}
#[test]
fn bad_input_and_duplicate_occurrences() {
    let original = session();
    let mut value = original.clone();
    value.drops[0].quantity = 0;
    assert!(matches!(value.validate(), Err(Error::BadInput)));
    value = original.clone();
    value.drops[0].occurred_at = value.ended_at;
    assert!(matches!(value.validate(), Err(Error::BadInput)));
    value = original.clone();
    value.active_seconds = 61;
    assert!(matches!(value.validate(), Err(Error::BadInput)));
    value = original.clone();
    value.drops[0].item_key = "invalid".into();
    assert!(matches!(value.validate(), Err(Error::BadInput)));
    assert!(matches!(
        Presentation::from_sessions(vec![original.clone(), original]),
        Err(Error::Conflict)
    ));
}
#[derive(Debug)]
struct Fixture {
    owner: Uuid,
    character: Uuid,
    cut: Option<Cut>,
    session: Option<Session>,
}
impl Journal for Fixture {
    async fn current_cut(
        &self,
        _: &mut sqlx::PgConnection,
        account: Uuid,
        character: Uuid,
    ) -> Result<Option<Cut>, Error> {
        Ok(if account == self.owner && character == self.character {
            self.cut
        } else {
            None
        })
    }
    async fn session(
        &self,
        _: &mut sqlx::PgConnection,
        account: Uuid,
        character: Uuid,
        id: Uuid,
    ) -> Result<Option<Session>, Error> {
        Ok(self
            .session
            .as_ref()
            .filter(|s| account == self.owner && character == self.character && id == s.session_id)
            .cloned())
    }
}
#[tokio::test]
async fn disabled_and_denied_before_database_access() -> Result<(), Box<dyn std::error::Error>> {
    let pool = oteryn_db::lazy_pool("postgres://localhost:1/unreachable")?;
    let id = Uuid::now_v7();
    let source = Fixture {
        owner: id,
        character: id,
        cut: Some(cut()),
        session: Some(session()),
    };
    let repo = Repository::new(pool.clone(), Flags::default());
    assert!(matches!(
        repo.ingest(&source, id, id, id).await,
        Err(Error::Disabled)
    ));
    assert!(matches!(
        repo.read(&source, id, id, id).await,
        Err(Error::Disabled)
    ));
    let repo = Repository::new(
        pool,
        Flags {
            ingest: true,
            read: true,
        },
    );
    assert!(matches!(
        repo.read(&source, Uuid::now_v7(), id, id).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        repo.ingest(&source, Uuid::nil(), id, id).await,
        Err(Error::BadInput)
    ));
    Ok(())
}
async fn database() -> Result<Option<PgPool>, Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("PostgreSQL test skipped: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(None);
    };
    let schema = format!("t75_{}", Uuid::now_v7().simple());
    let admin = PgPool::connect(&url).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    admin.close().await;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(move |connection, _| {
            let command = format!("SET search_path TO {schema}");
            Box::pin(async move {
                sqlx::query(sqlx::AssertSqlSafe(command))
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105230111_t75_drops.sql"
    ))
    .execute(&pool)
    .await?;
    Ok(Some(pool))
}
#[tokio::test]
async fn postgres_replay_conflict_owner_and_cut_fences() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let owner = Uuid::now_v7();
    let character = Uuid::now_v7();
    let value = session();
    let id = value.session_id;
    let mut source = Fixture {
        owner,
        character,
        cut: Some(cut()),
        session: Some(value),
    };
    let repo = Repository::new(
        pool.clone(),
        Flags {
            ingest: true,
            read: true,
        },
    );
    repo.ingest(&source, owner, character, id).await?;
    repo.ingest(&source, owner, character, id).await?;
    let result = repo.read(&source, owner, owner, character).await?;
    assert_eq!(result.chronicle.len(), 1);
    assert_eq!(result.drops.len(), 1);
    assert!(matches!(
        repo.read(&source, Uuid::now_v7(), owner, character).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        repo.read(&source, owner, owner, Uuid::now_v7()).await,
        Err(Error::Unavailable)
    ));
    source
        .session
        .as_mut()
        .ok_or(Error::Unavailable)?
        .experience_gained += 1;
    assert!(matches!(
        repo.ingest(&source, owner, character, id).await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        repo.read(&source, owner, owner, character).await,
        Err(Error::Unavailable)
    ));
    source
        .cut
        .as_mut()
        .ok_or(Error::Unavailable)?
        .recovery_generation += 1;
    assert!(
        repo.read(&source, owner, owner, character)
            .await?
            .chronicle
            .is_empty()
    );
    source.cut = None;
    assert!(matches!(
        repo.read(&source, owner, owner, character).await,
        Err(Error::Unavailable)
    ));
    sqlx::query("DELETE FROM pd_drop_sessions WHERE account_id=$1")
        .bind(owner)
        .execute(&pool)
        .await?;
    cleanup(&pool).await?;
    eprintln!("PostgreSQL T75 replay/conflict/owner/cut test ran");
    Ok(())
}
#[tokio::test]
async fn postgres_incomplete_and_bad_input_have_no_effect() -> Result<(), Box<dyn std::error::Error>>
{
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let owner = Uuid::now_v7();
    let character = Uuid::now_v7();
    let value = session();
    let id = value.session_id;
    let mut source = Fixture {
        owner,
        character,
        cut: Some(cut()),
        session: None,
    };
    let repo = Repository::new(
        pool.clone(),
        Flags {
            ingest: true,
            read: true,
        },
    );
    assert!(matches!(
        repo.ingest(&source, owner, character, id).await,
        Err(Error::Unavailable)
    ));
    source.session = Some(value);
    source
        .session
        .as_mut()
        .ok_or(Error::Unavailable)?
        .active_seconds = -1;
    assert!(matches!(
        repo.ingest(&source, owner, character, id).await,
        Err(Error::BadInput)
    ));
    let (count,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM pd_drop_sessions WHERE account_id=$1")
            .bind(owner)
            .fetch_one(&pool)
            .await?;
    assert_eq!(count, 0);
    cleanup(&pool).await?;
    eprintln!("PostgreSQL T75 incomplete/bad-input test ran");
    Ok(())
}

async fn cleanup(pool: &PgPool) -> Result<(), sqlx::Error> {
    let (schema,): (String,) = sqlx::query_as("SELECT current_schema()")
        .fetch_one(pool)
        .await?;
    assert!(
        schema.starts_with("t75_")
            && schema
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    );
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(pool)
        .await?;
    pool.close().await;
    Ok(())
}
