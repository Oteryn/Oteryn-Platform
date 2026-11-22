//! Allowlisted presentation of the pinned ADM-01 OperationResult union.
use super::{
    Field, FieldLabel as L, FieldValue, Operation, Record, RecordName,
    access::{Error, Policy},
    audit::LedgerRow,
    players,
};
use serde_json::Value;
use uuid::Uuid;
fn bad() -> Error {
    Error::Unavailable
}
fn text(v: &Value, key: &str) -> Result<String, Error> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| s.len() <= 512 && !s.chars().any(char::is_control))
        .map(str::to_owned)
        .ok_or_else(bad)
}
fn optional(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_owned)
}
fn array<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>, Error> {
    v.get(key).and_then(Value::as_array).ok_or_else(bad)
}
fn number(v: &Value, key: &str) -> Result<String, Error> {
    v.get(key)
        .filter(|n| n.is_number())
        .map(Value::to_string)
        .ok_or_else(bad)
}
fn boolean(v: &Value, key: &str) -> Result<bool, Error> {
    v.get(key).and_then(Value::as_bool).ok_or_else(bad)
}
fn list(v: &Value, key: &str) -> Result<String, Error> {
    array(v, key)?
        .iter()
        .map(|v| v.as_str().ok_or_else(bad))
        .collect::<Result<Vec<_>, _>>()
        .map(|v| v.join(", "))
}
fn field(label: L, value: String) -> Field {
    Field {
        label,
        value: FieldValue::Text(value),
    }
}
fn flag(label: L, value: bool, yes: &'static str, no: &'static str) -> Field {
    Field {
        label,
        value: FieldValue::Label(if value { yes } else { no }),
    }
}
fn world(v: &Value) -> Result<Record, Error> {
    Ok(Record {
        name: RecordName::Text(format!("{} · {}", text(v, "world_id")?, text(v, "name")?)),
        fields: vec![
            field(L::State, text(v, "state")?),
            field(L::MapRevision, text(v, "map_revision")?),
            field(L::ContentRevision, text(v, "content_revision")?),
            field(L::RulesetRevision, text(v, "ruleset_revision")?),
            field(L::PolicyRevision, text(v, "policy_revision")?),
            field(L::Channels, list(v, "channels")?),
        ],
    })
}
fn node(v: &Value) -> Result<Record, Error> {
    Ok(Record {
        name: RecordName::Text(text(v, "node_id")?),
        fields: vec![
            flag(
                L::Health,
                boolean(v, "ready")?,
                "node-ready",
                "node-not-ready",
            ),
            field(L::Build, text(v, "build")?),
            field(L::Cpu, number(v, "cpu_percent")?),
            field(L::Memory, text(v, "memory_bytes")?),
            field(L::Assignment, list(v, "worlds")?),
        ],
    })
}
pub(super) fn records(operation: Operation, v: &Value) -> Result<Vec<Record>, Error> {
    match operation {
        Operation::Worlds => array(v, "worlds")?.iter().map(world).collect(),
        Operation::World => Ok(vec![world(v)?]),
        Operation::Nodes => array(v, "nodes")?.iter().map(node).collect(),
        Operation::Node => Ok(vec![node(v)?]),
        Operation::Overview => Ok(vec![Record {
            name: RecordName::Label("record-overview"),
            fields: vec![
                field(L::Players, number(v, "players_online")?),
                flag(
                    L::Health,
                    boolean(v, "db_available")?,
                    "available",
                    "unavailable",
                ),
                field(L::Alerts, array(v, "alerts")?.len().to_string()),
            ],
        }]),
        Operation::Database => {
            let connections = array(v, "connections")?
                .iter()
                .map(|r| {
                    Ok(format!(
                        "{}: {}",
                        text(r, "role")?,
                        number(r, "connections")?
                    ))
                })
                .collect::<Result<Vec<_>, Error>>()?
                .join(", ");
            let mut fields = vec![
                field(L::Connections, connections),
                field(L::Locks, number(v, "locks_waiting")?),
                field(L::CacheHit, number(v, "cache_hit_ratio")?),
            ];
            for (key, label) in [
                ("oldest_transaction_seconds", L::OldestTransaction),
                ("replication_lag_seconds", L::Lag),
                ("bloat_estimate_bytes", L::Bloat),
            ] {
                if let Some(value) = v.get(key).filter(|v| !v.is_null()) {
                    let value = if let Some(text) = value.as_str() {
                        text.to_owned()
                    } else if value.is_number() {
                        value.to_string()
                    } else {
                        return Err(Error::Unavailable);
                    };
                    fields.push(field(label, value));
                }
            }
            Ok(vec![Record {
                name: RecordName::Label("database"),
                fields,
            }])
        }
        Operation::Migrations => array(v, "migrations")?
            .iter()
            .map(|m| {
                Ok(Record {
                    name: RecordName::Text(text(m, "version")?),
                    fields: vec![
                        flag(L::Applied, boolean(m, "applied")?, "yes", "no"),
                        flag(L::Drift, boolean(m, "checksum_drift")?, "yes", "no"),
                    ],
                })
            })
            .collect(),
    }
}
fn character(v: &Value, dossier: bool) -> Result<players::Character, Error> {
    let details = if dossier {
        let skills = array(v, "skills")?
            .iter()
            .map(|s| {
                Ok((
                    text(s, "name")?,
                    s.get("level")
                        .and_then(Value::as_u64)
                        .and_then(|n| u32::try_from(n).ok())
                        .ok_or_else(bad)?,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let position = v.get("position").ok_or_else(bad)?;
        Some(players::Dossier {
            vocation: text(v, "vocation")?,
            skills,
            position: format!(
                "{}, {}, {}",
                number(position, "x")?,
                number(position, "y")?,
                number(position, "z")?
            ),
            online: boolean(v, "online")?,
            premium: boolean(v, "premium")?,
            guild: optional(v, "guild"),
            house: optional(v, "house"),
            bank_balance: text(v, "bank_balance")?,
            last_login_times: array(v, "last_logins")?
                .iter()
                .map(|v| v.as_str().map(str::to_owned).ok_or_else(bad))
                .collect::<Result<_, _>>()?,
        })
    } else {
        None
    };
    Ok(players::Character {
        id: text(v, "character_id")?,
        name: text(v, "name")?,
        world: text(v, "world_id")?,
        level: v
            .get("level")
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(bad)?,
        dossier: details,
    })
}
pub(super) fn snapshot(
    operation: &str,
    request_id: Uuid,
    observed_at: String,
    v: &Value,
) -> Result<players::Snapshot, Error> {
    let mut snapshot = players::Snapshot {
        request_id,
        observed_at,
        characters: None,
        inventory: None,
        sanctions: None,
    };
    match operation {
        "player.search" => {
            snapshot.characters = Some(
                array(v, "players")?
                    .iter()
                    .map(|c| character(c, false))
                    .collect::<Result<_, _>>()?,
            )
        }
        "player.character.get" => snapshot.characters = Some(vec![character(v, true)?]),
        "player.inventory.get" => {
            snapshot.inventory = Some(
                array(v, "items")?
                    .iter()
                    .map(|i| {
                        Ok(players::Item {
                            instance_id: text(i, "instance_id")?,
                            type_id: number(i, "type_id")?,
                            location: text(i, "location")?,
                            count: text(i, "count")?,
                        })
                    })
                    .collect::<Result<_, Error>>()?,
            )
        }
        "sanction.list" => {
            snapshot.sanctions = Some(
                array(v, "sanctions")?
                    .iter()
                    .map(|s| {
                        Ok(players::Sanction {
                            sanction_id: text(s, "sanction_id")?,
                            kind: text(s, "kind")?,
                            status: boolean(s, "active")?.to_string(),
                        })
                    })
                    .collect::<Result<_, Error>>()?,
            )
        }
        _ => return Err(Error::Unavailable),
    }
    Ok(snapshot)
}
pub(super) fn ledger(v: &Value) -> Result<Vec<LedgerRow>, Error> {
    array(v, "entries")?
        .iter()
        .map(|r| {
            Ok(LedgerRow {
                request_id: text(r, "request_id")?.parse().map_err(|_| bad())?,
                ledger_id: text(r, "ledger_id")?,
                admin: text(r, "admin_id")?,
                operation: text(r, "operation")?,
                target: target(r.get("target").ok_or_else(bad)?)?,
                status: text(r, "status")?,
                observed_at: text(r, "finished_at")?,
            })
        })
        .collect()
}
pub(super) fn policy(v: &Value) -> Result<Vec<Policy>, Error> {
    array(v, "operations")?
        .iter()
        .map(|p| {
            Ok(Policy {
                operation: text(p, "operation")?,
                minimum_risk: text(p, "min_risk_class")?,
                permission: list(p, "allowed_permissions")?,
            })
        })
        .collect()
}
pub(super) fn target(v: &Value) -> Result<String, Error> {
    ["character_id", "world_id", "node_id", "account_ref"]
        .into_iter()
        .find_map(|key| optional(v, key))
        .ok_or_else(bad)
}
pub(super) fn preview(v: &Value) -> Result<Vec<String>, Error> {
    let effects = array(v, "effects")?;
    if effects.len() > 100 {
        return Err(Error::Unavailable);
    }
    effects
        .iter()
        .map(|e| {
            Ok(format!(
                "{} · {} · {} → {}",
                text(e, "aggregate")?,
                text(e, "id")?,
                optional(e, "before_revision").unwrap_or_default(),
                optional(e, "after_revision").unwrap_or_default()
            ))
        })
        .collect()
}
