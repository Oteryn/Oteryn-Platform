//! Successful ADM-01 payloads, selected by the stored operation and version.
use serde_json::{Value, json};
use std::{collections::HashMap, sync::LazyLock};

static V1: LazyLock<Option<HashMap<&'static str, jsonschema::Validator>>> = LazyLock::new(|| {
    let mut pinned: Value =
        serde_json::from_str(include_str!("fixtures/admin-control-v1/result.schema.json")).ok()?;
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
    V1.is_some()
}

pub(crate) fn valid_success(operation: &str, version: i32, payload: &Value) -> bool {
    version == 1
        && V1
            .as_ref()
            .and_then(|schemas| schemas.get(operation))
            .is_some_and(|validator| validator.is_valid(payload))
}

pub(crate) fn matches_detail_target(
    operation: &str,
    target: &super::Target,
    payload: &Value,
) -> bool {
    let (field, expected) = match operation {
        "world.get" => ("world_id", target.world_id.as_deref()),
        "node.get" => ("node_id", target.node_id.as_deref()),
        _ => return true,
    };
    expected.is_some_and(|id| payload.get(field).and_then(Value::as_str) == Some(id))
}
