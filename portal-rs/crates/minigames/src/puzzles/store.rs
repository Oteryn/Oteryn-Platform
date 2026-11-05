use super::*;
use oteryn_identity::accounts::Accounts;
use oteryn_world::WorldStore;
use sqlx::{PgPool, Row};

#[derive(Debug, Clone)]
pub struct Puzzles {
    pool: PgPool,
    world: WorldStore,
    flags: Flags,
}
impl Puzzles {
    pub fn new(pool: PgPool, world: WorldStore, flags: Flags) -> Self {
        Self { pool, world, flags }
    }
    fn enabled(&self) -> Result<(), Error> {
        if self.flags.daily_puzzles {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }
    async fn eligible(&self, account: Uuid) -> Result<(), Error> {
        self.enabled()?;
        let account = Accounts {
            pool: self.pool.clone(),
        }
        .find_by_id(account)
        .await?;
        if account.is_some_and(|account| {
            account.status == "active" && account.email_verified_at.is_some()
        }) {
            Ok(())
        } else {
            Err(Error::Forbidden)
        }
    }
    async fn visible(&self, definition: &Definition) -> Result<(), Error> {
        definition.validate()?;
        for choice in &definition.choices {
            let entity = self
                .world
                .get_entity(&choice.source.kind, &choice.source.key)
                .await
                .map_err(|_| Error::Unavailable)?
                .ok_or(Error::Unavailable)?;
            if entity.version_id != definition.world_version {
                return Err(Error::Unavailable);
            }
        }
        Ok(())
    }
    /// Caller is a trusted editorial service with publication authority. Definitions
    /// cannot be replaced, including after a wrong answer or before their day.
    pub async fn publish(
        &self,
        day: Date,
        definition: Definition,
        media: Option<Media>,
    ) -> Result<Uuid, Error> {
        self.enabled()?;
        self.visible(&definition).await?;
        match (&media, definition.kind) {
            (None, Kind::Quiz) => {}
            (Some(media), Kind::Monster | Kind::Atlas) if media.kind == definition.kind => {
                let report = self
                    .world
                    .import_report(definition.world_version)
                    .await
                    .map_err(|_| Error::Unavailable)?
                    .ok_or(Error::Unavailable)?;
                let expected = if definition.kind == Kind::Monster {
                    report.source_commit
                } else {
                    definition.world_version.to_string()
                };
                if media.version != expected
                    || media.bytes.is_empty()
                    || media.bytes.len() > 1_048_576
                {
                    return Err(Error::Unavailable);
                }
            }
            _ => return Err(Error::Invalid),
        }
        let id = Uuid::now_v7();
        let result = sqlx::query("INSERT INTO puzzles_daily (id,day,kind,world_version,definition,image) SELECT $1,$2,$3,$4,$5,$6 WHERE $2 >= (clock_timestamp() AT TIME ZONE 'UTC')::date ON CONFLICT (day,kind) DO NOTHING")
            .bind(id).bind(day).bind(definition.kind.key()).bind(definition.world_version)
            .bind(sqlx::types::Json(&definition)).bind(media.map(|media| media.bytes))
            .execute(&self.pool).await?;
        if result.rows_affected() == 0 {
            return Err(Error::Conflict);
        }
        Ok(id)
    }
    pub async fn today(&self, account: Uuid) -> Result<Vec<Daily>, Error> {
        self.eligible(account).await?;
        let rows = sqlx::query("SELECT p.id,p.day,p.definition,p.image IS NOT NULL AS has_image,a.choice,a.correct FROM puzzles_daily p LEFT JOIN puzzles_answers a ON a.puzzle_id=p.id AND a.account_id=$1 WHERE p.day=(clock_timestamp() AT TIME ZONE 'UTC')::date ORDER BY p.kind")
            .bind(account).fetch_all(&self.pool).await?;
        let mut daily = Vec::new();
        for row in rows {
            let definition: sqlx::types::Json<Definition> = row.try_get("definition")?;
            self.visible(&definition).await?;
            let choice: Option<i16> = row.try_get("choice")?;
            daily.push(Daily {
                id: row.try_get("id")?,
                day: row.try_get("day")?,
                kind: definition.kind,
                question: definition.question.clone(),
                clue: definition.clue.clone(),
                choices: definition
                    .choices
                    .iter()
                    .map(|choice| choice.label.clone())
                    .collect(),
                has_image: row.try_get("has_image")?,
                answer: choice
                    .map(|choice| -> Result<_, Error> {
                        Ok(Answer {
                            choice: u8::try_from(choice).map_err(|_| Error::Invalid)?,
                            correct: row.try_get("correct")?,
                            replayed: true,
                        })
                    })
                    .transpose()?,
            });
        }
        Ok(daily)
    }
    pub async fn image(&self, account: Uuid, id: Uuid) -> Result<Vec<u8>, Error> {
        self.eligible(account).await?;
        let row = sqlx::query("SELECT definition,image FROM puzzles_daily WHERE id=$1 AND day=(clock_timestamp() AT TIME ZONE 'UTC')::date")
            .bind(id).fetch_optional(&self.pool).await?.ok_or(Error::NotFound)?;
        let definition: sqlx::types::Json<Definition> = row.try_get("definition")?;
        self.visible(&definition).await?;
        row.try_get::<Option<Vec<u8>>, _>("image")?
            .ok_or(Error::NotFound)
    }
    pub async fn answer(&self, account: Uuid, id: Uuid, choice: u8) -> Result<Answer, Error> {
        self.eligible(account).await?;
        let definition: sqlx::types::Json<Definition> = sqlx::query_scalar(
            "SELECT definition FROM puzzles_daily WHERE id=$1 AND day=(clock_timestamp() AT TIME ZONE 'UTC')::date",
        ).bind(id).fetch_optional(&self.pool).await?.ok_or(Error::NotFound)?;
        self.visible(&definition).await?;
        if usize::from(choice) >= definition.choices.len() {
            return Err(Error::Invalid);
        }
        // Public source/account APIs acquire their own pool connections. Finish
        // authorization before opening a transaction to avoid pool starvation.
        self.eligible(account).await?;
        let mut tx = self.pool.begin().await?;
        // Serialize all three kinds for an account, including first-use races.
        sqlx::query("INSERT INTO puzzles_streaks (account_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(account)
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT account_id FROM puzzles_streaks WHERE account_id=$1 FOR UPDATE")
            .bind(account)
            .fetch_one(&mut *tx)
            .await?;
        // Read wall-clock UTC after acquiring the lock: a queued request crossing
        // midnight cannot answer yesterday's puzzle using transaction start time.
        let today: Date = sqlx::query_scalar("SELECT (clock_timestamp() AT TIME ZONE 'UTC')::date")
            .fetch_one(&mut *tx)
            .await?;
        let available: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM puzzles_daily WHERE id=$1 AND day=$2)")
                .bind(id)
                .bind(today)
                .fetch_one(&mut *tx)
                .await?;
        if !available {
            return Err(Error::NotFound);
        }
        let existing: Option<(i16, bool)> = sqlx::query_as(
            "SELECT choice,correct FROM puzzles_answers WHERE account_id=$1 AND puzzle_id=$2",
        )
        .bind(account)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((previous, correct)) = existing {
            if previous != i16::from(choice) {
                return Err(Error::Conflict);
            }
            tx.commit().await?;
            return Ok(Answer {
                choice,
                correct,
                replayed: true,
            });
        }
        let correct = choice == definition.correct_choice;
        sqlx::query("INSERT INTO puzzles_answers (account_id,puzzle_id,choice,correct) VALUES ($1,$2,$3,$4)")
            .bind(account).bind(id).bind(i16::from(choice)).bind(correct).execute(&mut *tx).await?;
        let completed: i64 = sqlx::query_scalar("SELECT count(*) FROM puzzles_answers a JOIN puzzles_daily p ON p.id=a.puzzle_id WHERE a.account_id=$1 AND p.day=$2 AND a.correct")
            .bind(account).bind(today).fetch_one(&mut *tx).await?;
        if completed == 3 {
            sqlx::query("UPDATE puzzles_streaks SET current_streak=CASE WHEN last_completed=$2-1 THEN current_streak+1 ELSE 1 END, best_streak=GREATEST(best_streak,CASE WHEN last_completed=$2-1 THEN current_streak+1 ELSE 1 END), last_completed=$2,updated_at=now() WHERE account_id=$1 AND (last_completed IS NULL OR last_completed<$2)")
                .bind(account).bind(today).execute(&mut *tx).await?;
        }
        let still_today: bool =
            sqlx::query_scalar("SELECT (clock_timestamp() AT TIME ZONE 'UTC')::date=$1")
                .bind(today)
                .fetch_one(&mut *tx)
                .await?;
        if !still_today {
            return Err(Error::NotFound);
        }
        tx.commit().await?;
        Ok(Answer {
            choice,
            correct,
            replayed: false,
        })
    }
    pub async fn progress(&self, account: Uuid) -> Result<Progress, Error> {
        self.eligible(account).await?;
        let row: Option<(i32, i32)> = sqlx::query_as("SELECT CASE WHEN last_completed >= (clock_timestamp() AT TIME ZONE 'UTC')::date-1 THEN current_streak ELSE 0 END,best_streak FROM puzzles_streaks WHERE account_id=$1")
            .bind(account).fetch_optional(&self.pool).await?;
        Ok(
            row.map_or_else(Progress::default, |(current, best)| Progress {
                current,
                best,
            }),
        )
    }
}
