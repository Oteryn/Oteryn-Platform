use super::*;
use sqlx::{PgPool, Row};

#[derive(Debug)]
pub struct Passes<A, P, R> {
    pool: PgPool,
    flags: Flags,
    authority: A,
    payment: P,
    rewards: R,
}
impl<A: Authority, P: Payment, R: Rewards> Passes<A, P, R> {
    pub fn new(pool: PgPool, flags: Flags, authority: A, payment: P, rewards: R) -> Self {
        Self {
            pool,
            flags,
            authority,
            payment,
            rewards,
        }
    }
    fn enabled(&self) -> Result<()> {
        if self.flags.enabled {
            Ok(())
        } else {
            Err(Error::Disabled)
        }
    }
    /// Staff authority is checked inside the same transaction. Published
    /// definitions cannot be updated through this service.
    pub async fn publish(&self, actor: Uuid, season: &Season) -> Result<()> {
        self.enabled()?;
        season.validate()?;
        if !valid_id(actor) {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        self.authority.publish(&mut tx, actor).await?;
        let now: OffsetDateTime = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        if season.starts_at <= now {
            return Err(Error::Invalid);
        }
        sqlx::query("INSERT INTO pass_seasons (id,starts_at,ends_at,price) VALUES ($1,$2,$3,$4)")
            .bind(season.id)
            .bind(season.starts_at)
            .bind(season.ends_at)
            .bind(season.price)
            .execute(&mut *tx)
            .await?;
        for c in &season.challenges {
            sqlx::query("INSERT INTO pass_challenges (id,season_id,source,metric,target,xp) VALUES ($1,$2,$3,$4,$5,$6)")
                .bind(c.id).bind(season.id).bind(c.source.key()).bind(&c.metric).bind(c.target).bind(c.xp).execute(&mut *tx).await?;
        }
        for r in &season.rewards {
            sqlx::query("INSERT INTO pass_rewards (id,season_id,track,xp,grant_key,grant_revision) VALUES ($1,$2,$3,$4,$5,$6)")
                .bind(r.id).bind(season.id).bind(if r.paid {"paid"} else {"free"}).bind(r.xp).bind(&r.grant_key).bind(&r.grant_revision).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    async fn lock(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        season: Uuid,
    ) -> Result<(Progress, i64)> {
        if !valid_id(account) || !valid_id(season) {
            return Err(Error::Invalid);
        }
        self.authority.account(tx, account).await?;
        let price: Option<i64> = sqlx::query_scalar("SELECT price FROM pass_seasons WHERE id=$1 AND starts_at<=clock_timestamp() AND ends_at>clock_timestamp() FOR SHARE")
            .bind(season).fetch_optional(&mut **tx).await?;
        let price = price.ok_or(Error::Denied)?;
        sqlx::query("INSERT INTO pass_progress (account_id,season_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
            .bind(account).bind(season).execute(&mut **tx).await?;
        let row = sqlx::query(
            "SELECT xp,paid FROM pass_progress WHERE account_id=$1 AND season_id=$2 FOR UPDATE",
        )
        .bind(account)
        .bind(season)
        .fetch_one(&mut **tx)
        .await?;
        Ok((
            Progress {
                xp: row.try_get("xp")?,
                paid: row.try_get("paid")?,
            },
            price,
        ))
    }
    pub async fn progress(&self, account: Uuid, season: Uuid) -> Result<Progress> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        let (progress, _) = self.lock(&mut tx, account, season).await?;
        tx.commit().await?;
        Ok(progress)
    }
    /// The account comes from the browser session, never a form field.
    pub async fn overview(&self, account: Uuid) -> Result<Overview> {
        self.enabled()?;
        let mut tx = self.pool.begin().await?;
        self.authority.account(&mut tx, account).await?;
        let row = sqlx::query("SELECT id,ends_at FROM pass_seasons WHERE starts_at<=clock_timestamp() AND ends_at>clock_timestamp()")
            .fetch_optional(&mut *tx).await?.ok_or(Error::Unavailable)?;
        let season: Uuid = row.try_get("id")?;
        let (progress, price) = self.lock(&mut tx, account, season).await?;
        let mut challenges = Vec::new();
        for row in sqlx::query("SELECT c.id,c.source,c.metric,c.target,c.xp,COALESCE(p.amount,0) AS amount FROM pass_challenges c LEFT JOIN pass_challenge_progress p ON p.challenge_id=c.id AND p.account_id=$2 AND p.season_id=c.season_id WHERE c.season_id=$1 ORDER BY c.id")
            .bind(season).bind(account).fetch_all(&mut *tx).await? {
            let source = match row.try_get::<String,_>("source")?.as_str() { "journal"=>Source::Journal, "portal"=>Source::Portal, _=>return Err(Error::Denied) };
            challenges.push(ChallengeView { definition: Challenge { id:row.try_get("id")?, source, metric:row.try_get("metric")?, target:row.try_get("target")?, xp:row.try_get("xp")? }, amount:row.try_get("amount")? });
        }
        let mut rewards = Vec::new();
        for row in sqlx::query("SELECT r.id,r.track,r.xp,r.grant_key,r.grant_revision,c.id IS NOT NULL AS claimed FROM pass_rewards r LEFT JOIN pass_claims c ON c.reward_id=r.id AND c.season_id=r.season_id AND c.account_id=$2 WHERE r.season_id=$1 ORDER BY r.xp,r.track,r.id")
            .bind(season).bind(account).fetch_all(&mut *tx).await? {
            let paid=match row.try_get::<String,_>("track")?.as_str() { "paid"=>true,"free"=>false,_=>return Err(Error::Denied) };
            let definition=Reward { id:row.try_get("id")?, paid, xp:row.try_get("xp")?, grant_key:row.try_get("grant_key")?, grant_revision:row.try_get("grant_revision")? };
            let claimed: bool=row.try_get("claimed")?;
            let eligible=self.flags.rewards && !claimed && progress.xp>=definition.xp && (!paid || (self.flags.paid && progress.paid));
            rewards.push(RewardView {
                operation: Uuid::now_v7(),definition,claimed,eligible});
        }
        let result = Overview {
            can_buy: self.flags.paid && !progress.paid,
            season,
            ends_at: row.try_get("ends_at")?,
            price,
            progress,
            challenges,
            rewards,
        };
        tx.commit().await?;
        Ok(result)
    }
    /// Only a trusted producer invokes this method; it is never mounted as a
    /// browser endpoint. Duplicate immutable facts contribute at most once.
    pub async fn record(&self, event: &Event) -> Result<Progress> {
        self.enabled()?;
        if !valid_id(event.id)
            || !valid_id(event.challenge)
            || event.amount <= 0
            || !key(&event.metric)
        {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        let (mut progress, _) = self.lock(&mut tx, event.account, event.season).await?;
        self.authority.event(&mut tx, event).await?;
        let c = sqlx::query("SELECT c.source,c.metric,c.target,c.xp FROM pass_challenges c JOIN pass_seasons s ON s.id=c.season_id WHERE c.id=$1 AND c.season_id=$2 AND $3>=s.starts_at AND $3<s.ends_at AND $3<=clock_timestamp()")
            .bind(event.challenge).bind(event.season).bind(event.occurred_at).fetch_optional(&mut *tx).await?.ok_or(Error::Denied)?;
        if c.try_get::<String, _>("source")? != event.source.key()
            || c.try_get::<String, _>("metric")? != event.metric
        {
            return Err(Error::Denied);
        }
        if let Some(old) = sqlx::query("SELECT season_id,amount,occurred_at FROM pass_events WHERE account_id=$1 AND challenge_id=$2 AND event_id=$3")
            .bind(event.account).bind(event.challenge).bind(event.id).fetch_optional(&mut *tx).await? {
            if old.try_get::<Uuid,_>("season_id")? != event.season || old.try_get::<i64,_>("amount")? != event.amount || old.try_get::<OffsetDateTime,_>("occurred_at")?.unix_timestamp_nanos() / 1000 != event.occurred_at.unix_timestamp_nanos() / 1000 { return Err(Error::Conflict); }
            tx.commit().await?;
            return Ok(progress);
        }
        let target: i64 = c.try_get("target")?;
        let prior: i64 = sqlx::query_scalar("SELECT amount FROM pass_challenge_progress WHERE account_id=$1 AND season_id=$2 AND challenge_id=$3")
            .bind(event.account).bind(event.season).bind(event.challenge).fetch_optional(&mut *tx).await?.unwrap_or(0);
        let next = prior.saturating_add(event.amount).min(target);
        sqlx::query("INSERT INTO pass_events (account_id,season_id,challenge_id,event_id,amount,occurred_at) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(event.account).bind(event.season).bind(event.challenge).bind(event.id).bind(event.amount).bind(event.occurred_at).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO pass_challenge_progress (account_id,season_id,challenge_id,amount) VALUES ($1,$2,$3,$4) ON CONFLICT (account_id,season_id,challenge_id) DO UPDATE SET amount=EXCLUDED.amount")
            .bind(event.account).bind(event.season).bind(event.challenge).bind(next).execute(&mut *tx).await?;
        if prior < target && next == target {
            progress.xp = progress
                .xp
                .checked_add(c.try_get("xp")?)
                .ok_or(Error::Invalid)?;
            sqlx::query("UPDATE pass_progress SET xp=$3,updated_at=clock_timestamp() WHERE account_id=$1 AND season_id=$2")
                .bind(event.account).bind(event.season).bind(progress.xp).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(progress)
    }
    pub async fn buy(&self, account: Uuid, season: Uuid, operation: Uuid) -> Result<Progress> {
        self.enabled()?;
        if !self.flags.paid {
            return Err(Error::Disabled);
        }
        if !valid_id(operation) {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        let (mut progress, price) = self.lock(&mut tx, account, season).await?;
        if progress.paid {
            let key: Option<Uuid> = sqlx::query_scalar(
                "SELECT purchase_key FROM pass_progress WHERE account_id=$1 AND season_id=$2",
            )
            .bind(account)
            .bind(season)
            .fetch_one(&mut *tx)
            .await?;
            if key != Some(operation) {
                return Err(Error::Conflict);
            }
        } else {
            self.payment
                .debit(&mut tx, account, operation, price)
                .await?;
            sqlx::query("UPDATE pass_progress SET paid=true,purchase_key=$3,updated_at=clock_timestamp() WHERE account_id=$1 AND season_id=$2")
                .bind(account).bind(season).bind(operation).execute(&mut *tx).await?;
            progress.paid = true;
        }
        tx.commit().await?;
        Ok(progress)
    }
    pub async fn claim(
        &self,
        account: Uuid,
        season: Uuid,
        reward: Uuid,
        operation: Uuid,
    ) -> Result<Claim> {
        self.enabled()?;
        if !self.flags.rewards {
            return Err(Error::Disabled);
        }
        if !valid_id(reward) || !valid_id(operation) {
            return Err(Error::Invalid);
        }
        let mut tx = self.pool.begin().await?;
        let (progress, _) = self.lock(&mut tx, account, season).await?;
        let row = sqlx::query("SELECT track,xp,grant_key,grant_revision FROM pass_rewards WHERE id=$1 AND season_id=$2")
            .bind(reward).bind(season).fetch_optional(&mut *tx).await?.ok_or(Error::Denied)?;
        let track: String = row.try_get("track")?;
        if !matches!(track.as_str(), "free" | "paid") {
            return Err(Error::Denied);
        }
        let definition = Reward {
            id: reward,
            paid: track == "paid",
            xp: row.try_get("xp")?,
            grant_key: row.try_get("grant_key")?,
            grant_revision: row.try_get("grant_revision")?,
        };
        if progress.xp < definition.xp || (definition.paid && (!self.flags.paid || !progress.paid))
        {
            return Err(Error::Denied);
        }
        if let Some(old) = sqlx::query("SELECT id,delivery_id FROM pass_claims WHERE account_id=$1 AND season_id=$2 AND reward_id=$3")
            .bind(account).bind(season).bind(reward).fetch_optional(&mut *tx).await? {
            if old.try_get::<Uuid,_>("id")? != operation { return Err(Error::Conflict); }
            let result = Claim { operation, delivery: old.try_get("delivery_id")? };
            tx.commit().await?;
            return Ok(result);
        }
        let delivery = self
            .rewards
            .enqueue(&mut tx, account, operation, &definition)
            .await?;
        if !valid_id(delivery) {
            return Err(Error::Unavailable);
        }
        sqlx::query("INSERT INTO pass_claims (id,account_id,season_id,reward_id,delivery_id) VALUES ($1,$2,$3,$4,$5)")
            .bind(operation).bind(account).bind(season).bind(reward).bind(delivery).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Claim {
            operation,
            delivery,
        })
    }
}
