//! ADM-01 receipts selected by the stored operation and version.
use super::{signer, store};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::LazyLock};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const PINNED: &str = include_str!("fixtures/admin-control-v1/result.schema.json");

static ENVELOPE: LazyLock<Option<jsonschema::Validator>> = LazyLock::new(|| {
    let mut schema: Value = serde_json::from_str(PINNED).ok()?;
    enforce_integer_formats(&mut schema);
    jsonschema::validator_for(&schema).ok()
});

static V1: LazyLock<Option<HashMap<&'static str, jsonschema::Validator>>> = LazyLock::new(|| {
    let mut pinned: Value = serde_json::from_str(PINNED).ok()?;
    enforce_integer_formats(&mut pinned["$defs"]);
    super::catalog::READ_OPERATIONS
        .into_iter()
        .map(|operation| {
            let definition = match operation {
                "ops.overview.get" => "Overview",
                "world.list" => "WorldPage",
                "world.get" => "World",
                "node.list" => "NodePage",
                "node.get" => "Node",
                "db.status" => "DatabaseStatus",
                "db.migrations.list" => "MigrationPage",
                "player.search" => "PlayerPage",
                "player.character.get" => "Character",
                "player.inventory.get" => "Inventory",
                "sanction.list" => "SanctionPage",
                "admin.ledger.query" => "LedgerPage",
                "admin.policy.get" => "PolicyView",
                "admin.agent.list" => "AgentPage",
                "analytics.catalog" => "AnalyticsCatalog",
                _ => return None,
            };
            let schema = json!({
                "$schema": pinned["$schema"],
                "$defs": pinned["$defs"],
                "$ref": format!("#/$defs/{definition}"),
            });
            let validator = jsonschema::validator_for(&schema).ok()?;
            Some((operation, validator))
        })
        .collect()
});

// The generated schema uses format annotations for Rust integer widths.
// JSON Schema treats these as annotations, so add their numeric constraints.
fn enforce_integer_formats(schema: &mut Value) {
    match schema {
        Value::Object(fields) => {
            let bounds = match fields.get("format").and_then(Value::as_str) {
                Some("uint16") => Some((0, i64::from(u16::MAX))),
                Some("uint32") => Some((0, i64::from(u32::MAX))),
                Some("int32") => Some((i64::from(i32::MIN), i64::from(i32::MAX))),
                _ => None,
            };
            if let Some((min, max)) = bounds {
                fields.entry("minimum").or_insert_with(|| json!(min));
                fields.entry("maximum").or_insert_with(|| json!(max));
            }
            fields.values_mut().for_each(enforce_integer_formats);
        }
        Value::Array(items) => items.iter_mut().for_each(enforce_integer_formats),
        _ => {}
    }
}

pub(super) fn initialize() -> bool {
    ENVELOPE.is_some() && V1.is_some()
}

pub(crate) fn valid_success(operation: &str, version: i32, payload: &Value) -> bool {
    version == 1
        && V1
            .as_ref()
            .and_then(|schemas| schemas.get(operation))
            .is_some_and(|validator| validator.is_valid(payload))
}

pub(crate) fn qualified_read(request: &store::Request) -> bool {
    request.kind == store::Kind::Query
        && !request.dry_run
        && super::catalog::advertised_kind(&request.operation, request.operation_version)
            == Some(signer::Kind::Query)
}

pub(crate) fn valid_receipt(
    request: &store::Request,
    result: &signer::ResultEnvelope,
    payload: &Value,
    now: OffsetDateTime,
) -> bool {
    use signer::Status::*;
    if !qualified_read(request)
        || result.request_id != request.request_id
        || result.admin_operation_id != request.admin_operation_id
        || result.dry_run_preview.is_some()
        || !ENVELOPE
            .as_ref()
            .is_some_and(|schema| schema.is_valid(payload))
    {
        return false;
    }
    let completed = result
        .completed_at
        .as_deref()
        .and_then(|stamp| OffsetDateTime::parse(stamp, &Rfc3339).ok());
    if result.completed_at.is_some()
        && completed.is_none_or(|at| at < request.issued_at || at > now)
    {
        return false;
    }
    match result.status {
        Succeeded => {
            result.error.is_none()
                && result.progress.is_none()
                && completed.is_some_and(|at| at < request.expires_at)
                && valid_success(
                    &request.operation,
                    request.operation_version,
                    &result.result.0,
                )
                && matches_detail_target(&request.operation, &request.target.0, &result.result.0)
        }
        Accepted | InProgress => {
            result.error.is_none()
                && result.result.0.is_null()
                && completed.is_none()
                && if result.status == Accepted {
                    result.progress.is_none()
                } else {
                    result
                        .progress
                        .as_ref()
                        .is_some_and(|p| p.of > 0 && p.step <= p.of)
                }
        }
        Failed | Rejected | Expired | Cancelled => {
            result.result.0.is_null()
                && result.progress.is_none()
                && (matches!(result.status, Expired | Cancelled) || result.error.is_some())
        }
    }
}

fn matches_detail_target(operation: &str, target: &super::Target, payload: &Value) -> bool {
    let matches = |field, expected| payload.get(field).and_then(Value::as_str) == Some(expected);
    match operation {
        "world.get" => target
            .world_id
            .as_deref()
            .is_some_and(|id| matches("world_id", id)),
        "node.get" => target
            .node_id
            .as_deref()
            .is_some_and(|id| matches("node_id", id)),
        "player.character.get" => {
            target
                .character_id
                .as_deref()
                .is_some_and(|id| matches("character_id", id))
                && target
                    .world_id
                    .as_deref()
                    .is_none_or(|id| matches("world_id", id))
        }
        _ => true,
    }
}
