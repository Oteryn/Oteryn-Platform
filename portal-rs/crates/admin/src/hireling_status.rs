//! Aggregate operational visibility for the Platform hireling ledger.
use sqlx::PgPool;

#[derive(sqlx::FromRow)]
pub(super) struct Counts {
    pub(super) unresolved: i64,
    pub(super) dispatch_attention: i64,
}

pub(super) struct Status {
    panel_visible: bool,
    pub(super) counts: Option<Counts>,
}

impl Status {
    pub(super) async fn load(pool: &PgPool) -> Self {
        // The current ledger has no terminal receipt store. Every retained
        // intent is unresolved regardless of its worker job's outcome.
        let counts = sqlx::query_as(
            "SELECT count(*) AS unresolved,
                    count(*) FILTER (WHERE job.id IS NULL OR job.state = 'dead')
                        AS dispatch_attention
             FROM hireling_command_intents AS intent
             LEFT JOIN outbox AS job ON job.kind = $1
                  AND job.idempotency_key = intent.operation_id::text",
        )
        .bind("hirelings.command.v1")
        .fetch_one(pool)
        .await
        .ok();
        Self {
            panel_visible: std::env::var("PORTAL_HIRELINGS_ENABLED").as_deref() == Ok("true"),
            counts,
        }
    }

    pub(super) fn panel_label(&self) -> &'static str {
        if self.panel_visible {
            "hireling-status-on"
        } else {
            "hireling-status-off"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ledger_summary_counts_all_intents_and_dispatch_attention()
    -> Result<(), Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping PostgreSQL hireling status test: PORTAL_TEST_DATABASE_URL unset");
            return Ok(());
        };
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        sqlx::query("SET search_path TO pg_temp")
            .execute(&pool)
            .await?;
        assert!(Status::load(&pool).await.counts.is_none());
        sqlx::raw_sql(
            "CREATE TEMP TABLE hireling_command_intents(operation_id uuid PRIMARY KEY);
             CREATE TEMP TABLE outbox(
                 id uuid PRIMARY KEY DEFAULT uuidv7(),
                 kind text NOT NULL,
                 idempotency_key text NOT NULL,
                 state text NOT NULL,
                 UNIQUE(kind, idempotency_key)
             );",
        )
        .execute(&pool)
        .await?;
        let empty = Status::load(&pool).await.counts.ok_or("missing counts")?;
        assert_eq!((empty.unresolved, empty.dispatch_attention), (0, 0));
        for state in ["pending", "done", "dead", "missing"] {
            let operation = uuid::Uuid::now_v7();
            sqlx::query("INSERT INTO hireling_command_intents VALUES ($1)")
                .bind(operation)
                .execute(&pool)
                .await?;
            if state != "missing" {
                sqlx::query("INSERT INTO outbox(kind, idempotency_key, state) VALUES ($1, $2, $3)")
                    .bind("hirelings.command.v1")
                    .bind(operation.to_string())
                    .bind(state)
                    .execute(&pool)
                    .await?;
            }
        }
        let counts = Status::load(&pool).await.counts.ok_or("missing counts")?;
        assert_eq!((counts.unresolved, counts.dispatch_attention), (4, 2));
        pool.close().await;
        Ok(())
    }
}
