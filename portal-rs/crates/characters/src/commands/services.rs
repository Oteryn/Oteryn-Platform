//! Version two commands share the issuer lock, identity namespace and restore history.
use super::{Error, ISSUER, storage, wire::*};
use oteryn_internal_api::canonical::Value;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

pub const SCHEDULE: &str = "ScheduleCharacterDeletion";
pub const CANCEL: &str = "CancelCharacterDeletion";
pub const RENAME: &str = "RenameCharacter";
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Schedule,
    Cancel { deletion_operation_id: Uuid },
    Rename { requested_name: String },
}
impl Action {
    pub fn command(&self) -> &'static str {
        match self {
            Self::Schedule => SCHEDULE,
            Self::Cancel { .. } => CANCEL,
            Self::Rename { .. } => RENAME,
        }
    }
    fn variant(&self) -> &'static str {
        match self {
            Self::Schedule => "PLATFORM_USER_DELETE",
            Self::Cancel { .. } => "PLATFORM_USER_CANCEL_DELETE",
            Self::Rename { .. } => "PLATFORM_USER_RENAME",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intent {
    pub character_id: Uuid,
    pub account_id: Uuid,
    pub expected_character_revision: u64,
    pub action: Action,
}
impl Intent {
    fn binding(&self) -> Result<Value, Error> {
        uuid(&self.character_id.to_string())?;
        uuid(&self.account_id.to_string())?;
        counter(&self.expected_character_revision.to_string(), false)?;
        let mut fields = vec![
            ("character_id", text(self.character_id)),
            ("account_id", text(self.account_id)),
            (
                "expected_character_revision",
                text(self.expected_character_revision),
            ),
        ];
        match &self.action {
            Action::Schedule => {}
            Action::Cancel {
                deletion_operation_id,
            } => {
                uuid(&deletion_operation_id.to_string())?;
                fields.push(("deletion_operation_id", text(deletion_operation_id)));
            }
            Action::Rename { requested_name } => {
                name(requested_name)?;
                fields.push(("requested_name", text(requested_name)));
            }
        }
        Ok(object(fields))
    }
}
/// The caller must authorize ownership and reserve the price in this same transaction.
/// On any error roll back. Returns the existing operation for an exact retry.
pub async fn enqueue_in(
    tx: &mut Transaction<'_, Postgres>,
    decision: Uuid,
    key: &str,
    intent: &Intent,
) -> Result<super::CommandStatus, Error> {
    uuid(&decision.to_string())?;
    if key.is_empty() || key.len() > 191 || !key.bytes().all(|b| (0x21..=0x7e).contains(&b)) {
        return Err(Error::BadInput);
    }
    let binding = intent.binding()?;
    let binding_json = encode(&binding)?;
    sqlx::query("SELECT pg_advisory_xact_lock(22005160000)")
        .execute(&mut **tx)
        .await?;
    if let Some(row)=sqlx::query("SELECT id,command,intent_json,receipt_json,binding_json,issuer_decision_id FROM character_commands WHERE request_key=$1").bind(key).fetch_optional(&mut **tx).await? {
        if row.try_get::<String,_>("command")? != intent.action.command() || row.try_get::<String,_>("binding_json")? != binding_json || row.try_get::<Uuid,_>("issuer_decision_id")? != decision { return Err(Error::Conflict); }
        return storage::service_status(row);
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM character_commands WHERE state='PENDING'")
            .fetch_one(&mut **tx)
            .await?;
    if count >= 40 {
        return Err(Error::BacklogFull);
    }
    let issued: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let expires = issued.checked_add(300).ok_or(Error::Unavailable)?;
    if issued <= 0 {
        return Err(Error::Unavailable);
    }
    let id = Uuid::now_v7();
    let revision:i64=sqlx::query_scalar("INSERT INTO character_commands(id,issuer_decision_id,request_key,command,account_id,binding_json,intent_json) VALUES ($1,$2,$3,$4,$5,$6,'') RETURNING source_revision")
        .bind(id).bind(decision).bind(key).bind(intent.action.command()).bind(intent.account_id).bind(binding_json).fetch_one(&mut **tx).await?;
    let behind: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM character_commands WHERE source_revision>$1)",
    )
    .bind(revision)
    .fetch_one(&mut **tx)
    .await?;
    if revision <= 0 || behind {
        return Err(Error::Unavailable);
    }
    let mut fields = vec![
        ("contract_version".into(), Value::Unsigned(2)),
        ("command".into(), text(intent.action.command())),
        ("variant".into(), text(intent.action.variant())),
        ("issuer_authority".into(), text(ISSUER)),
        ("issuer_decision_id".into(), text(decision)),
        ("source_revision".into(), text(revision)),
        ("operation_id".into(), text(id)),
    ];
    if let Value::Object(binding) = binding {
        fields.extend(binding);
    }
    fields.extend([
        ("issued_at_source".into(), text(issued)),
        ("expires_at_source".into(), text(expires)),
        ("audience".into(), text("OTERYN_GAME_CHARACTER_AUTHORITY")),
    ]);
    let raw = encode(&Value::Object(fields))?;
    sqlx::query("UPDATE character_commands SET intent_json=$2 WHERE id=$1")
        .bind(id)
        .bind(&raw)
        .execute(&mut **tx)
        .await?;
    Ok(super::CommandStatus {
        operation_id: id,
        command: intent.action.command().into(),
        intent_json: raw,
        receipt_json: None,
    })
}
pub fn parse(raw: &[u8], operation: &str, max: usize) -> Result<Value, Error> {
    let v = decode(raw, max)?;
    if v.get("contract_version") != Some(&Value::Unsigned(2)) || get(&v, "operation")? != operation
    {
        return Err(Error::BadInput);
    }
    authority(get(&v, "source_authority")?)?;
    Ok(v)
}
pub fn receipt(raw: &[u8]) -> Result<Value, Error> {
    let v = parse(raw, "PublishCharacterCommandReceiptV2", 1024)?;
    let cmd = get(&v, "command")?;
    if !matches!(cmd, SCHEDULE | CANCEL | RENAME) {
        return Err(Error::BadInput);
    }
    uuid(get(&v, "operation_id")?)?;
    let mut keys = vec![
        "contract_version",
        "operation",
        "source_authority",
        "command",
        "operation_id",
        "outcome",
        "result_code",
    ];
    if get(&v, "outcome")? == "COMMITTED" && get(&v, "result_code")? == "CHAR_CMD_OK" {
        keys.extend([
            "character_id",
            "account_id",
            "character_revision",
            "projection_epoch",
            "projection_revision",
        ]);
        for k in ["character_id", "account_id"] {
            uuid(get(&v, k)?)?;
        }
        for k in [
            "character_revision",
            "projection_epoch",
            "projection_revision",
        ] {
            counter(get(&v, k)?, false)?;
        }
        match cmd {
            SCHEDULE => {
                keys.extend([
                    "lifecycle",
                    "deletion_operation_id",
                    "deletion_scheduled_at",
                    "deletion_effective_at",
                ]);
                if get(&v, "lifecycle")? != "DELETION_PENDING"
                    || get(&v, "deletion_operation_id")? != get(&v, "operation_id")?
                {
                    return Err(Error::BadInput);
                }
                if counter(get(&v, "deletion_effective_at")?, false)?
                    <= counter(get(&v, "deletion_scheduled_at")?, false)?
                {
                    return Err(Error::BadInput);
                }
            }
            CANCEL => {
                keys.extend(["lifecycle", "deletion_operation_id"]);
                uuid(get(&v, "deletion_operation_id")?)?;
                if get(&v, "lifecycle")? != "ACTIVE" {
                    return Err(Error::BadInput);
                }
            }
            RENAME => {
                keys.extend(["previous_name", "name"]);
                for k in ["previous_name", "name"] {
                    display_name(get(&v, k)?)?;
                }
            }
            _ => return Err(Error::BadInput),
        }
    } else if get(&v, "outcome")? != "REJECTED" || !refusal(cmd, get(&v, "result_code")?) {
        return Err(Error::BadInput);
    }
    keys.extend(["recovery_generation", "decided_at"]);
    exact(&v, &keys)?;
    counter(get(&v, "recovery_generation")?, false)?;
    counter(get(&v, "decided_at")?, false)?;
    Ok(v)
}
fn refusal(cmd: &str, code: &str) -> bool {
    match code {
        "CHAR_CMD_INTENT_INVALID"
        | "CHAR_CMD_INTENT_EXPIRED"
        | "CHAR_CMD_UNSUPPORTED"
        | "CHAR_CMD_OPERATION_CONFLICT"
        | "CHAR_CMD_ACCOUNT_NOT_ELIGIBLE"
        | "CHAR_CMD_CHARACTER_NOT_FOUND"
        | "CHAR_CMD_OWNER_MISMATCH"
        | "CHAR_CMD_CHARACTER_IN_SESSION"
        | "CHAR_CMD_STALE_PRECONDITION"
        | "CHAR_CMD_LIFECYCLE_CONFLICT" => true,
        "CHAR_CMD_DELETION_WINDOW_CLOSED" => cmd == CANCEL,
        "CHAR_CMD_NAME_INVALID" | "CHAR_CMD_NAME_UNAVAILABLE" => cmd == RENAME,
        _ => false,
    }
}
pub fn display_name(s: &str) -> Result<(), Error> {
    use unicode_normalization::UnicodeNormalization;
    use unicode_properties::UnicodeGeneralCategory;
    if s.is_empty()
        || s.len() > 64
        || s.nfc().collect::<String>() != s
        || s.chars().any(|c| {
            matches!(
                c.general_category(),
                unicode_properties::GeneralCategory::Control
                    | unicode_properties::GeneralCategory::Format
            ) || matches!(c, '"' | '\\')
        })
    {
        return Err(Error::BadInput);
    }
    Ok(())
}
/// Validate echoes against the immutable intent before any financial effect.
pub fn validate_binding(v: &Value, raw: &str) -> Result<(), Error> {
    let intent = decode(raw.as_bytes(), 4096)?;
    if intent.get("contract_version") != Some(&Value::Unsigned(2))
        || get(v, "command")? != get(&intent, "command")?
    {
        return Err(Error::Conflict);
    }
    if get(v, "outcome")? == "COMMITTED" {
        for k in ["account_id", "character_id"] {
            if get(v, k)? != get(&intent, k)? {
                return Err(Error::Conflict);
            }
        }
        if counter(get(v, "character_revision")?, false)?
            <= counter(get(&intent, "expected_character_revision")?, false)?
        {
            return Err(Error::Conflict);
        }
        match get(v, "command")? {
            CANCEL
                if get(v, "deletion_operation_id")? != get(&intent, "deletion_operation_id")? =>
            {
                return Err(Error::Conflict);
            }
            RENAME if get(v, "name")? != get(&intent, "requested_name")? => {
                return Err(Error::Conflict);
            }
            _ => {}
        }
    }
    Ok(())
}

pub use super::wire::{
    service_counter as number, service_decode as decode_value, service_encode as encode_value,
    service_exact as exact_fields, service_get as string, service_uuid as identifier,
};
