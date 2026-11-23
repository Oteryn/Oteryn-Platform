use super::{
    schema::{Event, EventSnapshot, Snapshot},
    *,
};
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};

const RETENTION_SECONDS: u64 = 90 * 86400;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Form {
    #[serde(rename = "_csrf")]
    _csrf: String,
    pub world: String,
    pub event: String,
    pub epoch: String,
    pub generation: String,
    pub revision: String,
    pub choice: String,
}

pub(super) struct Overview {
    pub decisions: Vec<Snapshot>,
    pub active: Vec<EventSnapshot>,
    pub choices: HashMap<(String, Uuid, Uuid), String>,
    pub eligible: bool,
    pub now: u64,
    pub generation: u64,
}

pub(super) fn open(event: &Event, now: u64) -> bool {
    // Leave the same one-second uncertainty margin as the feed fence.
    event.state == "SCHEDULED"
        && schema::number(&event.lock_at, false).is_ok_and(|lock| now.saturating_add(1) < lock)
}

pub(super) fn final_score(event: &Event, choice: &str) -> Option<u8> {
    if event.state != "CLOSED" {
        return None;
    }
    event
        .outcome
        .first()
        .map(|outcome| u8::from(outcome == choice))
}

async fn eligible(tx: &mut Transaction<'_, Postgres>, account: Uuid, lock: bool) -> Result<bool> {
    let query = if lock {
        "SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR UPDATE"
    } else {
        "SELECT status='active' AND email_verified_at IS NOT NULL FROM accounts WHERE id=$1"
    };
    Ok(sqlx::query_scalar(query)
        .bind(account)
        .fetch_optional(&mut **tx)
        .await?
        .unwrap_or(false))
}

impl<A: Authority> League<A> {
    pub(super) async fn overview(&self, account: Option<Uuid>) -> Result<Overview> {
        tokio::time::timeout(Duration::from_secs(3), async {
            let mut tx = store::begin(&self.pool).await?;
            let (fence, now) = self.serving_fence(&mut tx).await?;
            let decisions = self.read_decisions(&mut tx, fence, now).await?;
            let active = self
                .authority
                .prediction_events(&mut tx, fence, now, None)
                .await?;
            let mut choices = HashMap::new();
            let mut can_predict = false;
            if let Some(account) = account {
                can_predict = eligible(&mut tx, account, false).await?;
                if can_predict {
                    let events = decisions.iter().filter_map(|s| match s {
                        Snapshot::Event(e) => Some(e),
                        Snapshot::Season(_) => None,
                    });
                    let ids = active
                        .iter()
                        .chain(events)
                        .map(|event| schema::id(&event.event_id))
                        .collect::<Result<Vec<_>>>()?;
                    let rows: Vec<(String, Uuid, Uuid, String)> = sqlx::query_as(
                        "SELECT authority,world_id,event_id,choice FROM league_predictions
                         WHERE account_id=$1 AND event_id=ANY($2) AND expires_at>$3",
                    )
                    .bind(account)
                    .bind(ids)
                    .bind(i64::try_from(now).map_err(|_| Error::Unavailable)?)
                    .fetch_all(&mut *tx)
                    .await?;
                    choices.extend(rows.into_iter().map(|(authority, world, event, choice)| {
                        ((authority, world, event), choice)
                    }));
                }
            }
            tx.commit().await?;
            Ok(Overview {
                decisions,
                active,
                choices,
                eligible: can_predict,
                now,
                generation: fence.recovery_generation,
            })
        })
        .await
        .map_err(|_| Error::Unavailable)?
    }

    pub(super) async fn predict(&self, account: Uuid, form: Form) -> Result<()> {
        let world = schema::id(&form.world)?;
        let event = schema::id(&form.event)?;
        schema::number(&form.epoch, false)?;
        schema::number(&form.generation, false)?;
        schema::number(&form.revision, false)?;
        schema::ensure(schema::key(&form.choice))?;
        tokio::time::timeout(Duration::from_secs(3), async {
            let mut tx = store::begin(&self.pool).await?;
            let (fence, now) = self.serving_fence(&mut tx).await?;
            if !eligible(&mut tx, account, true).await? {
                return Err(Error::Denied);
            }
            let events = self.authority.prediction_events(&mut tx, fence, now, Some((world, event))).await?;
            let snapshot = events.first().ok_or(Error::Conflict)?;
            let record = snapshot.event.first().ok_or(Error::Conflict)?;
            if form.epoch != snapshot.projection_epoch
                || form.generation != fence.recovery_generation.to_string()
                || form.revision != snapshot.projection_revision
                || !open(record, now)
            {
                return Err(Error::Conflict);
            }
            schema::ensure(record.options.contains(&form.choice))?;
            let cutoff = schema::number(&record.lock_at, false)?.saturating_sub(1);
            let expires = schema::number(&record.scheduled_end, false)?.saturating_add(RETENTION_SECONDS);
            let cutoff = i64::try_from(cutoff).map_err(|_| Error::Unavailable)?;
            let expires = i64::try_from(expires).map_err(|_| Error::Unavailable)?;
            let inserted = sqlx::query(
                "INSERT INTO league_predictions(account_id,authority,world_id,event_id,choice,expires_at)
                 SELECT $1,$2,$3,$4,$5,$6 WHERE extract(epoch FROM clock_timestamp()) < $7::bigint
                 ON CONFLICT (account_id,authority,world_id,event_id) DO NOTHING",
            )
            .bind(account)
            .bind(&snapshot.source_authority)
            .bind(world)
            .bind(event)
            .bind(form.choice)
            .bind(expires)
            .bind(cutoff)
            .execute(&mut *tx)
            .await?;
            if inserted.rows_affected() != 1 {
                return Err(Error::Conflict);
            }
            tx.commit().await?;
            Ok(())
        })
        .await
        .map_err(|_| Error::Unavailable)?
    }
}
