//! Successful ADM-01 payloads, selected by the stored operation and version.
use super::catalog::{ECONOMY_QUERIES, GAME_OPERATIONS};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::LazyLock};

type Validators = HashMap<(&'static str, Option<&'static str>), jsonschema::Validator>;

static V1: LazyLock<Option<Validators>> = LazyLock::new(|| {
    GAME_OPERATIONS
        .iter()
        .map(|op| ((op.name, None), op.result_schema))
        .chain(
            ECONOMY_QUERIES
                .iter()
                .map(|query| (("analytics.query", Some(query.id)), query.result_schema)),
        )
        .map(|(key, schema)| {
            let mut schema = serde_json::from_str(schema).ok()?;
            enforce_integer_formats(&mut schema);
            let validator = jsonschema::validator_for(&schema).ok()?;
            Some((key, validator))
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
    valid_payload(operation, version, None, payload)
}

/// Validate against the schema for the operation, or its signed named-query form.
pub(super) fn valid_payload(
    operation: &str,
    version: i32,
    query_id: Option<&str>,
    payload: &Value,
) -> bool {
    version == 1
        && V1
            .as_ref()
            .and_then(|schemas| schemas.get(&(operation, query_id)))
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
