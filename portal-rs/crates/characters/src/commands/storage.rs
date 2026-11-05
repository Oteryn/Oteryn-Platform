use super::{wire::*, *};
use oteryn_internal_api::canonical::Value;
use sqlx::Row;
use subtle::ConstantTimeEq;
fn status(row: sqlx::postgres::PgRow) -> Result<CommandStatus, Error> {
    Ok(CommandStatus {
        operation_id: row.try_get("id")?,
        command: row.try_get("command")?,
        intent_json: row.try_get("intent_json")?,
        receipt_json: row.try_get("receipt_json")?,
    })
}
impl Commands {
    pub(super) async fn load(&self, id: Uuid) -> Result<CommandStatus, Error> {
        status(
            sqlx::query(
                "SELECT id,command,intent_json,receipt_json FROM character_commands WHERE id=$1",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(Error::NotFound)?,
        )
    }
    pub(super) async fn issue(
        &self,
        decision: Uuid,
        key: &str,
        intent: Intent,
    ) -> Result<CommandStatus, Error> {
        let binding = binding(&intent);
        let binding_json = encode(&binding)?;
        let mut tx = self.pool.begin().await?;
        // Hold the issuer lock through commit so no visible key lands behind the cursor.
        sqlx::query("SELECT pg_advisory_xact_lock(22005160000)")
            .execute(&mut *tx)
            .await?;
        if let Some(row)=sqlx::query("SELECT id,command,intent_json,receipt_json,binding_json,issuer_decision_id FROM character_commands WHERE request_key=$1").bind(key).fetch_optional(&mut *tx).await? {
            if row.try_get::<String,_>("command")? != intent.command() || row.try_get::<String,_>("binding_json")? != binding_json || row.try_get::<Uuid,_>("issuer_decision_id")? != decision { return Err(Error::Conflict); }
            return status(row);
        }
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM character_commands WHERE state='PENDING'")
                .fetch_one(&mut *tx)
                .await?;
        if count >= i64::from(self.settings.max_pending) {
            return Err(Error::BacklogFull);
        }
        let issued: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *tx)
                .await?;
        let id = Uuid::now_v7();
        let (account, target) = intent.accounts();
        let revision: i64=sqlx::query_scalar("INSERT INTO character_commands (id,issuer_decision_id,request_key,command,account_id,target_account_id,binding_json,intent_json) VALUES ($1,$2,$3,$4,$5,$6,$7,'') RETURNING source_revision")
            .bind(id).bind(decision).bind(key).bind(intent.command()).bind(account).bind(target).bind(binding_json).fetch_one(&mut *tx).await?;
        let behind: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM character_commands WHERE source_revision > $1)",
        )
        .bind(revision)
        .fetch_one(&mut *tx)
        .await?;
        if revision <= 0 || behind {
            return Err(Error::Unavailable);
        }
        let raw = wire::intent(
            intent.command(),
            decision,
            revision,
            id,
            binding,
            issued,
            issued + i64::from(self.settings.ttl_seconds),
        )?;
        sqlx::query("UPDATE character_commands SET intent_json=$2 WHERE id=$1")
            .bind(id)
            .bind(&raw)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(CommandStatus {
            operation_id: id,
            command: intent.command().into(),
            intent_json: raw,
            receipt_json: None,
        })
    }
    pub(super) fn source(&self, v: &Value) -> Result<(), Error> {
        if !bool::from(
            get(v, "source_authority")?
                .as_bytes()
                .ct_eq(self.settings.source_authority.as_bytes()),
        ) {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }
    pub(super) async fn pending(&self, raw: &[u8]) -> Result<String, Error> {
        let v = decode(raw, 512)?;
        exact(
            &v,
            &[
                "contract_version",
                "operation",
                "source_authority",
                "after_source_revision",
                "after_operation_id",
                "max_entries",
            ],
        )?;
        head(&v, "ListPendingCharacterCommandsV1")?;
        self.source(&v)?;
        let revision = counter(get(&v, "after_source_revision")?, true)?;
        let after = get(&v, "after_operation_id")?;
        let id = if after == Uuid::nil().to_string() {
            Uuid::nil()
        } else {
            uuid(after)?
        };
        let limit = match v.get("max_entries") {
            Some(Value::Unsigned(n)) if (1..=32).contains(n) => *n as i64,
            _ => return Err(Error::BadInput),
        };
        let mut entries = Vec::new();
        if let Ok(revision) = i64::try_from(revision) {
            let rows=sqlx::query("SELECT id,command,source_revision FROM character_commands WHERE state='PENDING' AND (source_revision,id)>($1,$2) ORDER BY source_revision,id LIMIT $3").bind(revision).bind(id).bind(limit).fetch_all(&self.pool).await?;
            for row in rows {
                entries.push(object(vec![
                    ("command", text(row.try_get::<String, _>("command")?)),
                    ("operation_id", text(row.try_get::<Uuid, _>("id")?)),
                    (
                        "source_revision",
                        text(row.try_get::<i64, _>("source_revision")?),
                    ),
                ]));
            }
        }
        encode(&object(vec![
            ("contract_version", Value::Unsigned(1)),
            ("pending", Value::Array(entries)),
        ]))
    }
    pub(super) async fn read(&self, raw: &[u8]) -> Result<String, Error> {
        let v = decode(raw, 512)?;
        exact(
            &v,
            &[
                "contract_version",
                "operation",
                "source_authority",
                "issuer_authority",
                "command",
                "operation_id",
            ],
        )?;
        head(&v, "ReadCharacterCommandIntentV1")?;
        self.source(&v)?;
        let issuer = get(&v, "issuer_authority")?;
        if issuer.is_empty()
            || issuer.len() > 128
            || !issuer
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err(Error::BadInput);
        }
        let cmd = get(&v, "command")?;
        command(cmd)?;
        let id = uuid(get(&v, "operation_id")?)?;
        if issuer != ISSUER {
            return Err(Error::NotFound);
        }
        let stored = self.load(id).await?;
        if stored.command != cmd {
            return Err(Error::Conflict);
        }
        Ok(stored.intent_json)
    }
    pub(super) async fn publish(&self, raw: &[u8]) -> Result<(), Error> {
        let v = receipt(raw)?;
        self.source(&v)?;
        let id = uuid(get(&v, "operation_id")?)?;
        let mut tx = self.pool.begin().await?;
        let row=sqlx::query("SELECT command,binding_json,receipt_json FROM character_commands WHERE id=$1 FOR UPDATE").bind(id).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
        if row.try_get::<String, _>("command")? != get(&v, "command")? {
            return Err(Error::Conflict);
        }
        if let Some(stored) = row.try_get::<Option<String>, _>("receipt_json")? {
            return if stored.as_bytes() == raw {
                Ok(())
            } else {
                Err(Error::Conflict)
            };
        }
        if get(&v, "outcome")? == "COMMITTED" {
            let binding = decode(row.try_get::<String, _>("binding_json")?.as_bytes(), 4096)?;
            let pairs: &[(&str, &str)] = if get(&v, "command")? == CREATE {
                &[
                    ("account_id", "account_id"),
                    ("world_id", "target_world_id"),
                    ("name", "requested_name"),
                ]
            } else {
                &[
                    ("character_id", "character_id"),
                    ("from_account_id", "expected_from_account_id"),
                    ("to_account_id", "to_account_id"),
                ]
            };
            for (receipt, binding_key) in pairs {
                if get(&v, receipt)? != get(&binding, binding_key)? {
                    return Err(Error::Conflict);
                }
            }
        }
        let raw = std::str::from_utf8(raw).map_err(|_| Error::BadInput)?;
        sqlx::query("UPDATE character_commands SET receipt_json=$2,state='TERMINAL',updated_at=clock_timestamp() WHERE id=$1").bind(id).bind(raw).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}
