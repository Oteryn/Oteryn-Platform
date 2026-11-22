//! Successful ADM-01 payloads, selected by the stored operation and version.
use serde_json::{Value, json};
use std::{collections::HashMap, sync::LazyLock};

static V1: LazyLock<Option<HashMap<&'static str, jsonschema::Validator>>> = LazyLock::new(|| {
    let pinned: Value =
        serde_json::from_str(include_str!("fixtures/admin-control-v1/result.schema.json")).ok()?;
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

pub(super) fn initialize() -> bool {
    V1.is_some()
}

pub(super) fn valid_success(operation: &str, version: i32, payload: &Value) -> bool {
    version == 1
        && V1
            .as_ref()
            .and_then(|schemas| schemas.get(operation))
            .is_some_and(|validator| validator.is_valid(payload))
}
