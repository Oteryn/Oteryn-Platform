use super::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;
const GOLDENS: [&str; 5] = [
    include_str!("fixtures/staff-actions-v1/01-ban.golden.json"),
    include_str!("fixtures/staff-actions-v1/02-unban.golden.json"),
    include_str!("fixtures/staff-actions-v1/03-namelock.golden.json"),
    include_str!("fixtures/staff-actions-v1/04-warning.golden.json"),
    include_str!("fixtures/staff-actions-v1/05-notation.golden.json"),
];
fn fixture(i: usize) -> Result<signer::RequestEnvelope, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(GOLDENS[i])?)
}
fn rejected(i: usize, change: impl FnOnce(&mut Value)) -> Result<(), Box<dyn std::error::Error>> {
    let mut v: Value = serde_json::from_str(GOLDENS[i])?;
    change(&mut v);
    if let Ok(r) = serde_json::from_value::<signer::RequestEnvelope>(v) {
        assert!(validate_staff_request(&r).is_err());
    }
    Ok(())
}
fn remove(v: &mut Value, key: &str) {
    if let Some(p) = v["params"].as_object_mut() {
        p.remove(key);
    }
}
#[test]
fn five_goldens() -> Result<(), Box<dyn std::error::Error>> {
    for i in 0..5 {
        validate_staff_request(&fixture(i)?)?;
    }
    Ok(())
}
#[test]
fn unknown_param() -> Result<(), Box<dyn std::error::Error>> {
    rejected(3, |v| v["params"]["tier"] = json!("god"))
}
#[test]
fn ban_requires_expiry() -> Result<(), Box<dyn std::error::Error>> {
    rejected(0, |v| remove(v, "expires_at"))
}
#[test]
fn warning_forbids_expiry() -> Result<(), Box<dyn std::error::Error>> {
    rejected(3, |v| v["params"]["expires_at"] = Value::Null)
}
#[test]
fn preview_only_executing_r4() -> Result<(), Box<dyn std::error::Error>> {
    for i in 1..5 {
        rejected(i, |v| v["params"]["preview_digest"] = json!("placeholder"))?;
    }
    rejected(0, |v| v["dry_run"] = json!(true))
}
#[test]
fn r4_requires_preview() -> Result<(), Box<dyn std::error::Error>> {
    rejected(0, |v| remove(v, "preview_digest"))?;
    rejected(0, |v| v["params"]["preview_digest"] = Value::Null)?;
    rejected(0, |v| v["params"]["preview_digest"] = json!(""))
}
#[test]
fn namelock_requires_character() -> Result<(), Box<dyn std::error::Error>> {
    rejected(2, |v| v["target"]["character_id"] = Value::Null)?;
    rejected(2, |v| v["target"]["character_id"] = json!(""))
}
#[test]
fn revision_positive_safe_integer() -> Result<(), Box<dyn std::error::Error>> {
    for revision in [
        json!(0),
        json!(9_007_199_254_740_992u64),
        json!(-1),
        json!(1.5),
        json!("1"),
    ] {
        rejected(3, |v| v["params"]["sanction_revision"] = revision)?;
    }
    let mut r = fixture(3)?;
    r.params.0["sanction_revision"] = json!(MAX_SANCTION_REVISION);
    validate_staff_request(&r)?;
    Ok(())
}
#[test]
fn evidence_bound() -> Result<(), Box<dyn std::error::Error>> {
    let mut r = fixture(3)?;
    r.params.0["evidence_refs"] = json!(vec!["opaque"; MAX_EVIDENCE_REFS]);
    validate_staff_request(&r)?;
    rejected(3, |v| {
        v["params"]["evidence_refs"] = json!(vec!["opaque"; MAX_EVIDENCE_REFS + 1])
    })?;
    rejected(3, |v| v["params"]["evidence_refs"] = json!([""]))
}
#[test]
fn utc_expiry_after_decision() -> Result<(), Box<dyn std::error::Error>> {
    for expiry in [
        "tomorrow",
        "2026-12-01T12:00:00+01:00",
        "2026-01-01T12:00:00Z",
    ] {
        rejected(2, |v| v["params"]["expires_at"] = json!(expiry))?;
    }
    let mut r = fixture(2)?;
    r.params.0["expires_at"] = Value::Null;
    validate_staff_request(&r)?;
    Ok(())
}
#[test]
fn permission_risk_table_and_untrusted_claims() -> Result<(), Box<dyn std::error::Error>> {
    use controls::RiskClass;
    let cases = [
        (3, false, Permission::GameSanctionWrite, RiskClass::R1),
        (4, false, Permission::GameSanctionWrite, RiskClass::R1),
        (2, false, Permission::GameSanctionBan, RiskClass::R2),
        (0, true, Permission::GameSanctionBan, RiskClass::R3),
        (1, false, Permission::GameSanctionBan, RiskClass::R3),
        (0, false, Permission::GameSanctionBan, RiskClass::R4),
    ];
    for (i, temporary, permission, risk) in cases {
        let mut r = fixture(i)?;
        if temporary {
            r.params.0["expires_at"] = json!("2026-12-01T00:00:00Z");
            if let Some(p) = r.params.0.as_object_mut() {
                p.remove("preview_digest");
            }
            r.risk_class = signer::RiskClass::R3;
        }
        let policy = validate_staff_request(&r)?;
        assert_eq!(policy.permission, permission.as_str());
        assert_eq!(policy.risk, risk);
        let op = StaffOperation::lookup(&r.operation).ok_or("missing operation")?;
        assert_eq!(policy.name, op.name());
        assert_eq!(op.kind(), signer::Kind::Command);
        assert_eq!(op.operation_version(), 1);
        let claimed_risk = r.risk_class;
        r.risk_class = signer::RiskClass::R0;
        assert!(validate_staff_request(&r).is_err());
        r.risk_class = claimed_risk;
        r.actor.permission = "game.ops.read".into();
        let params = serde_json::from_value(r.params.0.clone())?;
        assert_eq!(op.policy(&params, &r.target, r.dry_run)?.risk, risk);
        assert!(validate_staff_request(&r).is_err());
    }
    Ok(())
}
#[test]
fn default_off_exact_allowlist() -> Result<(), Box<dyn std::error::Error>> {
    let default = Catalog::default();
    for op in StaffOperation::ALL {
        assert!(!default.enabled(op));
    }
    for i in 0..5 {
        assert_eq!(
            default.validate(&fixture(i)?).err(),
            Some(controls::Refusal::Disabled)
        );
    }
    let enabled = Catalog::with_staff_allowlist(vec![StaffOperation::Warning]);
    enabled.validate(&fixture(3)?)?;
    assert_eq!(
        enabled.validate(&fixture(0)?).err(),
        Some(controls::Refusal::Disabled)
    );
    Ok(())
}
#[test]
fn notation_account_or_character() -> Result<(), Box<dyn std::error::Error>> {
    let mut r = fixture(4)?;
    r.target.character_id = None;
    validate_staff_request(&r)?;
    r.target.character_id = Some("character-1".into());
    validate_staff_request(&r)?;
    Ok(())
}
#[test]
fn closed_kind_version() -> Result<(), Box<dyn std::error::Error>> {
    rejected(3, |v| v["kind"] = json!("query"))?;
    rejected(3, |v| v["operation_version"] = json!(2))
}
#[test]
fn r4_preview_free_dry_run() -> Result<(), Box<dyn std::error::Error>> {
    let mut r = fixture(0)?;
    r.dry_run = true;
    r.approvals.clear();
    if let Some(p) = r.params.0.as_object_mut() {
        p.remove("preview_digest");
    }
    assert_eq!(validate_staff_request(&r)?.risk, controls::RiskClass::R4);
    Ok(())
}

#[test]
fn committed_staff_schemas_qualify_requests_and_results() -> Result<(), Box<dyn std::error::Error>>
{
    let mut action: Value =
        serde_json::from_str(include_str!("fixtures/staff-actions-v1/action.schema.json"))?;
    let base: Value = serde_json::from_str(include_str!(
        "fixtures/admin-control-v1/request.schema.json"
    ))?;
    // Resolve the one committed relative reference locally; never retrieve schemas over the network.
    assert_eq!(
        action["allOf"][0]["$ref"],
        "../admin-control-v1/request.schema.json"
    );
    action["$defs"] = base["$defs"].clone();
    action["allOf"][0] = base;
    let validator = jsonschema::options()
        .should_validate_formats(true)
        .build(&action)
        .map_err(|e| e.to_string())?;
    for golden in GOLDENS {
        let request: Value = serde_json::from_str(golden)?;
        validator.validate(&request).map_err(|e| e.to_string())?;
        for (key, value) in [
            ("operation", json!("staff.unknown")),
            ("kind", json!("query")),
            ("operation_version", json!(2)),
        ] {
            let mut bad = request.clone();
            bad[key] = value;
            assert!(!validator.is_valid(&bad), "{key}");
        }
        let mut bad = request.clone();
        bad["params"]["sanction_revision"] = json!(0);
        assert!(!validator.is_valid(&bad));
        let mut bad = request.clone();
        bad["params"]["unexpected"] = json!(true);
        assert!(!validator.is_valid(&bad));
    }
    let schema: Value =
        serde_json::from_str(include_str!("fixtures/staff-actions-v1/result.schema.json"))?;
    let validator = jsonschema::validator_for(&schema).map_err(|e| e.to_string())?;
    for enforcement in ["applied", "pending"] {
        let result = json!({"sanction_id":"sanction-1","sanction_revision":1,"enforcement":enforcement,"evidence_refs":["case-1"]});
        validator.validate(&result).map_err(|e| e.to_string())?;
        for (key, value) in [
            ("enforcement", json!("unknown")),
            ("sanction_revision", json!(0)),
            ("sanction_id", json!("")),
            ("evidence_refs", json!([""])),
            ("unexpected", json!(true)),
        ] {
            let mut bad = result.clone();
            bad[key] = value;
            assert!(!validator.is_valid(&bad), "{key}");
        }
    }
    Ok(())
}
#[test]
fn game_catalog_matches_fixture_schema_pairs() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/control/fixtures/admin-control-v1");
    let mut files = BTreeSet::new();
    for entry in std::fs::read_dir(directory)? {
        let name = entry?
            .file_name()
            .into_string()
            .map_err(|_| "fixture filename must be UTF-8")?;
        files.insert(name);
    }
    let operations: BTreeSet<_> = GAME_OPERATIONS.iter().map(|op| op.name).collect();
    let query_forms: BTreeSet<_> = ECONOMY_QUERIES
        .iter()
        .map(|query| format!("analytics.query.{}", query.id))
        .collect();
    assert_eq!(operations.len(), GAME_OPERATIONS.len());
    assert_eq!(query_forms.len(), ECONOMY_QUERIES.len());
    for suffix in [".params.schema.json", ".result.schema.json"] {
        let stems: BTreeSet<_> = files
            .iter()
            .filter_map(|file| file.strip_suffix(suffix))
            .collect();
        let (named, operations_on_disk): (BTreeSet<_>, BTreeSet<_>) = stems
            .into_iter()
            .partition(|stem| stem.starts_with("analytics.query."));
        assert_eq!(operations, operations_on_disk, "{suffix}");
        assert_eq!(
            query_forms
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            named,
            "{suffix}"
        );
    }
    Ok(())
}

#[test]
fn game_catalog_preserves_agent_advertisement_scope() {
    let phase_1: BTreeSet<_> = GAME_OPERATIONS
        .iter()
        .filter(|op| op.phase == 1)
        .map(|op| op.name)
        .collect();
    assert_eq!(phase_1, BTreeSet::from(READ_OPERATIONS));
    for op in GAME_OPERATIONS {
        assert_eq!(
            advertised_kind(op.name, 1),
            (op.phase == 1).then_some(op.kind),
            "{}",
            op.name
        );
    }
    for query in ECONOMY_QUERIES {
        assert_eq!(
            advertised_kind(&format!("analytics.query.{}", query.id), 1),
            None
        );
    }
}
