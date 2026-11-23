use super::*;
use sqlx::postgres::PgPoolOptions;

fn goal(date: &str) -> NewGoal {
    NewGoal {
        request_id: Uuid::now_v7(),
        kind: "quest".into(),
        title: " Gnomish mission ".into(),
        remind_on: date.into(),
    }
}

#[test]
fn validates_goal_kind_title_and_calendar_date() -> Result<(), Box<dyn std::error::Error>> {
    let today = Date::from_calendar_date(2026, time::Month::November, 25)?;
    assert_eq!(
        goal("2026-11-25").validate(today)?,
        ("Gnomish mission", Some(today))
    );
    for date in ["2026-11-24", "2026-11-31", "2026-1-01", "2027-11-26"] {
        assert!(matches!(goal(date).validate(today), Err(Error::Invalid)));
    }
    let mut input = goal("");
    input.title = "a".repeat(121);
    assert!(matches!(input.validate(today), Err(Error::Invalid)));
    input.title = "a\nb".into();
    assert!(matches!(input.validate(today), Err(Error::Invalid)));
    input.title = "Gnomish mission".into();
    input.kind = "unknown".into();
    assert!(matches!(input.validate(today), Err(Error::Invalid)));
    Ok(())
}

#[tokio::test]
async fn owner_gets_one_due_reminder_and_can_complete_it() -> Result<(), Box<dyn std::error::Error>>
{
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        return Ok(());
    };
    let admin = PgPool::connect(&url).await?;
    let schema = format!("gap47_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let options = (*admin.connect_options())
        .clone()
        .options([("search_path", &schema)]);
    let pool = PgPoolOptions::new().connect_with(options).await?;
    let result = async {
        sqlx::query("CREATE TABLE accounts (id uuid PRIMARY KEY, status text, email_verified_at timestamptz)").execute(&pool).await?;
        sqlx::raw_sql(include_str!("../../../../../migrations/20261125100026_gap47_journal_goals.sql"))
            .execute(&pool).await?;
        let owner = Uuid::now_v7();
        let other = Uuid::now_v7();
        sqlx::query("INSERT INTO accounts VALUES ($1,'active',now()),($2,'active',now())")
            .bind(owner).bind(other).execute(&pool).await?;
        let today: Date = sqlx::query_scalar("SELECT (now() AT TIME ZONE 'UTC')::date").fetch_one(&pool).await?;
        let first = goal(&today.to_string());
        let id = first.request_id;
        let mut retry = goal(&today.to_string());
        retry.request_id = id;
        create(&pool, owner, first).await?;
        create(&pool, owner, retry).await?;
        let saved = list(&pool, owner).await?;
        assert_eq!(saved.len(), 1);
        assert!(saved[0].due);
        assert!(list(&pool, other).await?.is_empty());
        assert!(matches!(change(&pool, other, id, Action::Complete).await, Err(Error::Missing)));
        change(&pool, owner, id, Action::Complete).await?;
        let saved = list(&pool, owner).await?;
        assert!(saved[0].completed && !saved[0].due);
        for _ in 1..LIMIT {
            create(&pool, owner, goal("")).await?;
        }
        assert!(matches!(create(&pool, owner, goal("")).await, Err(Error::Limit)));
        change(&pool, owner, id, Action::Delete).await?;
        create(&pool, owner, goal("")).await?;
        sqlx::query("UPDATE accounts SET status='closed' WHERE id=$1").bind(owner).execute(&pool).await?;
        assert!(matches!(create(&pool, owner, goal("")).await, Err(Error::Denied)));
        let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM pd_journal_goals WHERE account_id=$1")
            .bind(owner).fetch_one(&pool).await?;
        assert_eq!(remaining, 0);
        Ok::<_, Box<dyn std::error::Error>>(())
    }.await;
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    result
}
