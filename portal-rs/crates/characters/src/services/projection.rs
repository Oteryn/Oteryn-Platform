use super::*;
use oteryn_internal_api::canonical::Value;
#[derive(Debug, Clone)]
pub(super) struct State {
    pub character_revision: u64,
    pub name: String,
    pub lifecycle: String,
    pub deletion: Option<(Uuid, u64, u64)>,
}
impl State {
    pub fn permits(&self, action: &Action, now: u64) -> Result<(), Error> {
        match action {
            Action::Schedule | Action::Rename { .. } if self.lifecycle == "ACTIVE" => Ok(()),
            Action::Cancel {
                deletion_operation_id,
            } if self.lifecycle == "DELETION_PENDING"
                && self.deletion.is_some_and(|(id, _, effective)| {
                    id == *deletion_operation_id && now < effective
                }) =>
            {
                Ok(())
            }
            _ => Err(Error::Denied),
        }
    }
}
fn nullable(v: &Value, k: &str) -> Result<Option<u64>, Error> {
    match v.get(k) {
        Some(Value::Null) => Ok(None),
        _ => Ok(Some(wire::number(wire::string(v, k)?, false)?)),
    }
}
fn state(v: &Value) -> Result<State, Error> {
    let lifecycle = wire::string(v, "lifecycle")?;
    if !matches!(
        lifecycle,
        "ACTIVE" | "DELETION_PENDING" | "DELETED" | "REMOVED_FROM_ACCOUNT"
    ) {
        return Err(Error::BadInput);
    }
    let scheduled = nullable(v, "deletion_scheduled_at")?;
    let effective = nullable(v, "deletion_effective_at")?;
    let deletion = match (v.get("deletion_operation_id"), scheduled, effective) {
        (Some(Value::String(id)), Some(s), Some(e))
            if matches!(lifecycle, "DELETION_PENDING" | "DELETED") && e > s =>
        {
            Some((wire::identifier(id)?, s, e))
        }
        (Some(Value::Null), None, None)
            if matches!(lifecycle, "ACTIVE" | "REMOVED_FROM_ACCOUNT") =>
        {
            None
        }
        _ => return Err(Error::BadInput),
    };
    match v.get("sex") {
        Some(Value::Null) => {}
        Some(Value::Unsigned(1 | 2)) if lifecycle != "REMOVED_FROM_ACCOUNT" => {}
        _ => return Err(Error::BadInput),
    }
    let name = wire::string(v, "name")?;
    wire::display_name(name)?;
    Ok(State {
        character_revision: wire::number(wire::string(v, "character_revision")?, false)?,
        name: name.into(),
        lifecycle: lifecycle.into(),
        deletion,
    })
}
impl Services {
    pub async fn ingest(&self, raw: &[u8], watermark: bool) -> Result<&'static str, Error> {
        if !self.settings.projection_ready() {
            return Err(Error::Unavailable);
        }
        let operation = if watermark {
            "PublishCharacterServiceWatermarkV2"
        } else {
            "PublishCharacterServiceStateV2"
        };
        let v = wire::parse(raw, operation, if watermark { 512 } else { 2048 })?;
        source(&v, &self.settings.projection_authority)?;
        let mut keys = vec!["contract_version", "operation", "source_authority"];
        if watermark {
            keys.extend(["projection_epoch", "complete_through", "observed_at"]);
        } else {
            keys.extend([
                "account_id",
                "character_id",
                "projection_epoch",
                "projection_revision",
                "source_observed_at",
                "character_revision",
                "world_id",
                "name",
                "sex",
                "lifecycle",
                "deletion_operation_id",
                "deletion_scheduled_at",
                "deletion_effective_at",
            ]);
        }
        wire::exact_fields(&v, &keys)?;
        let epoch = wire::number(wire::string(&v, "projection_epoch")?, false)?;
        let observed = wire::number(
            wire::string(
                &v,
                if watermark {
                    "observed_at"
                } else {
                    "source_observed_at"
                },
            )?,
            false,
        )?;
        let complete = if watermark {
            Some(wire::number(wire::string(&v, "complete_through")?, false)?)
        } else {
            None
        };
        if complete.is_some_and(|c| c > observed) {
            return Err(Error::BadInput);
        }
        let mut tx = self.pool.begin().await?;
        let current_time = now(&mut tx).await?;
        let uncertainty = u64::from(
            self.settings
                .clock_uncertainty_seconds
                .ok_or(Error::Unavailable)?,
        );
        if observed
            > current_time
                .checked_add(uncertainty)
                .ok_or(Error::Unavailable)?
        {
            return Err(Error::BadInput);
        }
        // LCFA and services share this non-rollback fence, never their revisions.
        let highest: Option<String> = sqlx::query_scalar(
            "SELECT highest_epoch FROM account_character_feed WHERE id=1 FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?;
        let highest = highest.map(|s| wire::number(&s, false)).transpose()?;
        if highest.is_some_and(|h| epoch < h) {
            return Ok("superseded");
        }
        if highest.is_none_or(|h| epoch > h) {
            sqlx::query("UPDATE account_character_feed SET highest_epoch=$1,watermark_epoch=NULL,source_authority=NULL,complete_through=NULL,observed_at=NULL,updated_at=now() WHERE id=1").bind(epoch.to_string()).execute(&mut *tx).await?;
        }
        if watermark {
            let row=sqlx::query("SELECT epoch::text,complete_through,observed_at FROM character_service_feed WHERE id=1 FOR UPDATE").fetch_one(&mut *tx).await?;
            let old_epoch = row
                .try_get::<Option<String>, _>("epoch")?
                .map(|s| wire::number(&s, false))
                .transpose()?;
            let old_complete: Option<i64> = row.try_get("complete_through")?;
            let complete =
                i64::try_from(complete.ok_or(Error::BadInput)?).map_err(|_| Error::BadInput)?;
            let observed = i64::try_from(observed).map_err(|_| Error::BadInput)?;
            if old_epoch == Some(epoch) && old_complete.is_some_and(|c| c >= complete) {
                return Ok("superseded");
            }
            sqlx::query("UPDATE character_service_feed SET epoch=$1::text::numeric,complete_through=$2,observed_at=$3,updated_at=now() WHERE id=1").bind(epoch.to_string()).bind(complete).bind(observed).execute(&mut *tx).await?;
        } else {
            let parsed = state(&v)?;
            let account = wire::identifier(wire::string(&v, "account_id")?)?;
            let character = wire::identifier(wire::string(&v, "character_id")?)?;
            wire::identifier(wire::string(&v, "world_id")?)?;
            let revision = wire::number(wire::string(&v, "projection_revision")?, false)?;
            if let Some(row)=sqlx::query("SELECT epoch::text,revision::text,snapshot_json,invalid FROM character_service_states WHERE account_id=$1 AND character_id=$2 FOR UPDATE").bind(account).bind(character).fetch_optional(&mut *tx).await? {
                let pair=(wire::number(&row.try_get::<String,_>("epoch")?,false)?,wire::number(&row.try_get::<String,_>("revision")?,false)?);
                if (epoch,revision)<pair { return Ok("superseded"); }
                if (epoch,revision)==pair {
                    if row.try_get::<String,_>("snapshot_json")?.as_bytes()!=raw || row.try_get::<bool,_>("invalid")? {
                        sqlx::query("UPDATE character_service_states SET invalid=true,updated_at=now() WHERE account_id=$1 AND character_id=$2").bind(account).bind(character).execute(&mut *tx).await?;
                        tx.commit().await?; return Err(Error::Conflict);
                    }
                    return Ok("accepted");
                }
            }
            let raw = std::str::from_utf8(raw).map_err(|_| Error::BadInput)?;
            sqlx::query("INSERT INTO character_service_states(account_id,character_id,epoch,revision,character_revision,snapshot_json) VALUES ($1,$2,$3::text::numeric,$4::text::numeric,$5,$6) ON CONFLICT (account_id,character_id) DO UPDATE SET epoch=EXCLUDED.epoch,revision=EXCLUDED.revision,character_revision=EXCLUDED.character_revision,snapshot_json=EXCLUDED.snapshot_json,invalid=false,updated_at=now()")
                .bind(account).bind(character).bind(epoch.to_string()).bind(revision.to_string()).bind(parsed.character_revision.to_string()).bind(raw).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok("accepted")
    }
    pub(super) async fn owner_state(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: Uuid,
        character: Uuid,
    ) -> Result<State, Error> {
        if !self.settings.projection_ready() {
            return Err(Error::Unavailable);
        }
        let row=sqlx::query("SELECT f.highest_epoch,s.epoch::text,s.snapshot_json,s.invalid AS state_invalid,w.epoch::text AS watermark_epoch,w.complete_through,w.observed_at,w.invalid FROM account_character_feed f CROSS JOIN character_service_feed w JOIN character_service_states s ON s.account_id=$1 AND s.character_id=$2 WHERE f.id=1 AND w.id=1 FOR SHARE OF f,w,s")
            .bind(account).bind(character).fetch_optional(&mut **tx).await?.ok_or(Error::Denied)?;
        let epoch: String = row.try_get("epoch")?;
        if row.try_get::<bool, _>("invalid")?
            || row.try_get::<bool, _>("state_invalid")?
            || row
                .try_get::<Option<String>, _>("highest_epoch")?
                .as_deref()
                != Some(&epoch)
            || row
                .try_get::<Option<String>, _>("watermark_epoch")?
                .as_deref()
                != Some(&epoch)
        {
            return Err(Error::Unavailable);
        }
        let complete = row
            .try_get::<Option<i64>, _>("complete_through")?
            .and_then(|v| u64::try_from(v).ok())
            .ok_or(Error::Unavailable)?;
        let observed = row
            .try_get::<Option<i64>, _>("observed_at")?
            .and_then(|v| u64::try_from(v).ok())
            .ok_or(Error::Unavailable)?;
        let current = now(tx).await?;
        let uncertainty = u64::from(
            self.settings
                .clock_uncertainty_seconds
                .ok_or(Error::Unavailable)?,
        );
        let freshness = u64::from(self.settings.freshness_seconds.ok_or(Error::Unavailable)?);
        if complete > observed
            || observed > current.saturating_add(uncertainty)
            || current.saturating_sub(complete).saturating_add(uncertainty) > freshness
        {
            return Err(Error::Unavailable);
        }
        let recovering: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM character_restore_notices n CROSS JOIN account_character_feed f CROSS JOIN character_service_feed w WHERE n.acknowledged_at IS NULL OR f.updated_at<n.received_at OR w.updated_at<n.received_at)").fetch_one(&mut **tx).await?;
        if recovering {
            return Err(Error::Unavailable);
        }
        let raw: String = row.try_get("snapshot_json")?;
        let v = wire::decode_value(raw.as_bytes(), 2048)?;
        source(&v, &self.settings.projection_authority)?;
        if wire::number(wire::string(&v, "source_observed_at")?, false)?
            > current.saturating_add(uncertainty)
        {
            return Err(Error::Unavailable);
        }
        let parsed = state(&v)?;
        if !matches!(parsed.lifecycle.as_str(), "ACTIVE" | "DELETION_PENDING") {
            return Err(Error::Denied);
        }
        // Independent LCFA freshness and ownership association also have to agree.
        let lcfa:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM account_characters c JOIN account_character_snapshots s USING(account_id) CROSS JOIN account_character_feed f WHERE c.account_id=$1 AND c.character_id=$2 AND NOT s.invalid AND s.projection_epoch=f.highest_epoch AND f.watermark_epoch=f.highest_epoch AND f.complete_through IS NOT NULL AND f.observed_at<=$3 AND f.complete_through>=$4)")
            .bind(account).bind(character).bind(i64::try_from(current.saturating_add(uncertainty)).map_err(|_|Error::Unavailable)?).bind(i64::try_from(current.saturating_sub(freshness.saturating_sub(uncertainty))).map_err(|_|Error::Unavailable)?).fetch_one(&mut **tx).await?;
        if !lcfa {
            return Err(Error::Unavailable);
        }
        Ok(parsed)
    }
}
