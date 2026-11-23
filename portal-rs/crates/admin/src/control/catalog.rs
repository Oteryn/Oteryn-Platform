//! Versioned ADM operation definitions, separate from dispatch and agent advertisement.
use super::{controls, signer};
use crate::rbac::Permission;
use serde::{Deserialize, Deserializer};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub const MAX_SANCTION_REVISION: u64 = 9_007_199_254_740_991;
pub const MAX_EVIDENCE_REFS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaffOperation {
    Warning,
    Notation,
    Namelock,
    Ban,
    Unban,
}
impl StaffOperation {
    pub const ALL: [Self; 5] = [
        Self::Warning,
        Self::Notation,
        Self::Namelock,
        Self::Ban,
        Self::Unban,
    ];
    pub const fn name(self) -> &'static str {
        match self {
            Self::Warning => "staff.warning",
            Self::Notation => "staff.notation",
            Self::Namelock => "staff.namelock",
            Self::Ban => "staff.ban",
            Self::Unban => "staff.unban",
        }
    }
    pub const fn kind(self) -> signer::Kind {
        signer::Kind::Command
    }
    pub const fn operation_version(self) -> u32 {
        1
    }
    pub fn lookup(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|op| op.name() == name)
    }

    /// Validate closed params before deriving policy. No client risk is consulted.
    pub fn policy(
        self,
        params: &StaffParams,
        target: &signer::Target,
        dry_run: bool,
    ) -> Result<controls::Operation, controls::Refusal> {
        use controls::{Refusal::InvalidInput, RiskClass};
        if params.sanction_id.is_empty()
            || !(1..=MAX_SANCTION_REVISION).contains(&params.sanction_revision)
            || params.evidence_refs.len() > MAX_EVIDENCE_REFS
            || params.evidence_refs.iter().any(String::is_empty)
            || target
                .account_ref
                .as_ref()
                .is_none_or(|s| uuid::Uuid::parse_str(s).is_err())
            || (self == Self::Namelock && target.character_id.as_ref().is_none_or(String::is_empty))
        {
            return Err(InvalidInput);
        }
        match (&params.expires_at, self) {
            (Expiry::Absent, Self::Ban | Self::Namelock) => return Err(InvalidInput),
            (Expiry::Absent, _) => {}
            (Expiry::Null | Expiry::At(_), Self::Ban | Self::Namelock) => {}
            _ => return Err(InvalidInput),
        }
        if let Expiry::At(at) = &params.expires_at {
            let at = OffsetDateTime::parse(at, &Rfc3339).map_err(|_| InvalidInput)?;
            if at.offset() != time::UtcOffset::UTC {
                return Err(InvalidInput);
            }
        }
        let risk = match self {
            Self::Warning | Self::Notation => RiskClass::R1,
            Self::Namelock => RiskClass::R2,
            Self::Ban if params.expires_at == Expiry::Null => RiskClass::R4,
            Self::Ban | Self::Unban => RiskClass::R3,
        };
        let executing_r4 = risk == RiskClass::R4 && !dry_run;
        if executing_r4 != params.preview_digest.is_some()
            || params.preview_digest.as_ref().is_some_and(String::is_empty)
        {
            return Err(InvalidInput);
        }
        Ok(controls::Operation {
            name: self.name().into(),
            permission: Permission::for_staff_operation(self).as_str().into(),
            risk,
        })
    }
}

/// Absent and explicit null must remain distinct for the signed params contract.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum Expiry {
    #[default]
    Absent,
    Null,
    At(String),
}
impl<'de> Deserialize<'de> for Expiry {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match Option::<String>::deserialize(d)? {
            Some(s) => Self::At(s),
            None => Self::Null,
        })
    }
}
fn present_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}

/// Structural fields only; opaque refs are never interpreted as player text.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaffParams {
    sanction_id: String,
    #[serde(default)]
    expires_at: Expiry,
    sanction_revision: u64,
    evidence_refs: Vec<String>,
    #[serde(default, deserialize_with = "present_string")]
    preview_digest: Option<String>,
}

/// Pure structural validation for pinned fixtures and future catalog consumers.
/// This neither authorizes execution nor verifies signature/preview evidence.
/// The returned policy is derived from operation + validated params; signed policy
/// claims must agree, so lowering the requested risk cannot bypass controls.
pub fn validate_staff_request(
    request: &signer::RequestEnvelope,
) -> Result<controls::Operation, controls::Refusal> {
    use controls::{Refusal::InvalidInput, RiskClass};
    let op = StaffOperation::lookup(&request.operation).ok_or(InvalidInput)?;
    if request.contract != "oteryn.admin-control.v1"
        || request.kind != op.kind()
        || request.operation_version != op.operation_version()
        || request.reason.is_none()
    {
        return Err(InvalidInput);
    }
    let params: StaffParams =
        serde_json::from_value(request.params.0.clone()).map_err(|_| InvalidInput)?;
    let policy = op.policy(&params, &request.target, request.dry_run)?;
    let risk = match policy.risk {
        RiskClass::R0 => signer::RiskClass::R0,
        RiskClass::R1 => signer::RiskClass::R1,
        RiskClass::R2 => signer::RiskClass::R2,
        RiskClass::R3 => signer::RiskClass::R3,
        RiskClass::R4 => signer::RiskClass::R4,
    };
    if request.risk_class != risk
        || request.actor.permission != policy.permission
        || (policy.risk >= RiskClass::R3 && request.actor.step_up_at.is_none())
        || (policy.risk == RiskClass::R4 && !request.dry_run && request.approvals.is_empty())
    {
        return Err(InvalidInput);
    }
    if let Expiry::At(at) = params.expires_at {
        let expiry = OffsetDateTime::parse(&at, &Rfc3339).map_err(|_| InvalidInput)?;
        let decision =
            OffsetDateTime::parse(&request.issued_at, &Rfc3339).map_err(|_| InvalidInput)?;
        if expiry <= decision {
            return Err(InvalidInput);
        }
    }
    Ok(policy)
}

/// Metadata for a pinned Game operation. Runtime support is selected separately.
#[derive(Debug, Clone, Copy)]
pub struct GameOperation {
    pub name: &'static str,
    pub phase: u8,
    pub kind: signer::Kind,
    pub minimum: signer::RiskClass,
    pub permission: &'static str,
    pub params_schema: &'static str,
    pub result_schema: &'static str,
}

macro_rules! game_operations {
    ($($name:literal, $phase:literal, $kind:ident, $minimum:ident, $permission:literal;)+) => {
        &[
            $(GameOperation {
                name: $name,
                phase: $phase,
                kind: signer::Kind::$kind,
                minimum: signer::RiskClass::$minimum,
                permission: $permission,
                params_schema: include_str!(concat!("fixtures/admin-control-v1/", $name, ".params.schema.json")),
                result_schema: include_str!(concat!("fixtures/admin-control-v1/", $name, ".result.schema.json")),
            },)+
        ]
    };
}

/// Oteryn-Game `crates/admin/contracts/src/catalog.rs` `Operation::ALL` at
/// 3d4ea6ed913359c2e5aafb7ab6b890a894ff32fa.
pub const GAME_OPERATIONS: &[GameOperation] = game_operations! {
    "ops.overview.get", 1, Query, R0, "game.ops.read";
    "world.list", 1, Query, R0, "game.world.read";
    "world.get", 1, Query, R0, "game.world.read";
    "node.list", 1, Query, R0, "game.node.read";
    "node.get", 1, Query, R0, "game.node.read";
    "db.status", 1, Query, R0, "game.db.read";
    "db.migrations.list", 1, Query, R0, "game.db.read";
    "player.search", 1, Query, R1, "game.player.read";
    "player.character.get", 1, Query, R1, "game.player.read";
    "player.inventory.get", 1, Query, R1, "game.player.read";
    "sanction.list", 1, Query, R1, "game.sanction.read";
    "admin.ledger.query", 1, Query, R1, "game.audit.read";
    "admin.policy.get", 1, Query, R0, "game.ops.read";
    "admin.agent.list", 1, Query, R0, "game.ops.read";
    "analytics.catalog", 1, Query, R0, "game.analytics.read";
    "analytics.query", 2, Query, R0, "game.analytics.read";
    "node.drain", 2, Command, R2, "game.node.ops";
    "ops.alerts.ack", 2, Command, R2, "game.ops.write";
    "ops.alerts.list", 2, Query, R0, "game.ops.read";
    "ops.logs.tail", 2, Query, R1, "game.ops.logs.read";
    "ops.metrics.series", 2, Query, R0, "game.ops.read";
    "player.kick", 2, Command, R2, "game.player.moderate";
    "player.message", 2, Command, R2, "game.player.moderate";
    "player.teleport", 2, Command, R2, "game.player.moderate";
    "sanction.ban.account", 2, Command, R3, "game.sanction.ban";
    "sanction.ban.character", 2, Command, R3, "game.sanction.ban";
    "sanction.mute", 2, Command, R2, "game.sanction.write";
    "sanction.notation.add", 2, Command, R2, "game.sanction.write";
    "sanction.revoke", 2, Command, R3, "game.sanction.ban";
    "sanction.unmute", 2, Command, R2, "game.sanction.write";
    "world.broadcast", 2, Command, R2, "game.world.ops";
    "world.motd.set", 2, Command, R2, "game.world.ops";
    "world.restart", 2, Command, R3, "game.world.ops";
    "world.save_now", 2, Command, R2, "game.world.ops";
    "world.state.set", 2, Command, R3, "game.world.config";
};

#[derive(Debug, Clone, Copy)]
pub struct EconomyQuery {
    pub id: &'static str,
    pub minimum: signer::RiskClass,
    pub params_schema: &'static str,
    pub result_schema: &'static str,
}

macro_rules! economy_queries {
    ($($id:literal, $minimum:ident;)+) => {
        &[
            $(EconomyQuery {
                id: $id,
                minimum: signer::RiskClass::$minimum,
                params_schema: include_str!(concat!("fixtures/admin-control-v1/analytics.query.", $id, ".params.schema.json")),
                result_schema: include_str!(concat!("fixtures/admin-control-v1/analytics.query.", $id, ".result.schema.json")),
            },)+
        ]
    };
}

/// Named forms of `analytics.query` from the pinned Game `phase2.rs`.
pub const ECONOMY_QUERIES: &[EconomyQuery] = economy_queries! {
    "gold_sources_sinks", R0;
    "gold_supply", R0;
    "item_supply_by_type", R0;
    "market_price_series", R0;
    "market_volume", R0;
    "npc_trade_flow", R0;
    "top_wealth", R1;
};

/// Phase-1 query schemas.
pub const READ_OPERATIONS: [&str; 15] = [
    "ops.overview.get",
    "world.list",
    "world.get",
    "node.list",
    "node.get",
    "db.status",
    "db.migrations.list",
    "player.search",
    "player.character.get",
    "player.inventory.get",
    "sanction.list",
    "admin.ledger.query",
    "admin.policy.get",
    "admin.agent.list",
    "analytics.catalog",
];

/// Game registrations from `crates/admin/agent/src/read.rs` at the pinned revision.
pub const REGISTERED_READS: [&str; 13] = [
    "admin.ledger.query",
    "admin.policy.get",
    "db.migrations.list",
    "db.status",
    "node.get",
    "node.list",
    "ops.overview.get",
    "player.character.get",
    "player.inventory.get",
    "player.search",
    "sanction.list",
    "world.get",
    "world.list",
];

pub const GLOBAL_READS: [&str; 3] = ["admin.policy.get", "db.migrations.list", "db.status"];

/// Target selectors accepted by the pinned Game read contract.
pub(crate) fn valid_read_target(operation: &str, target: &super::Target) -> bool {
    let world = target.world_id.is_some();
    let node = target.node_id.is_some();
    let character = target.character_id.is_some();
    let account = target.account_ref.is_some();
    if GLOBAL_READS.contains(&operation) {
        return !world && !node && !character && !account;
    }
    match operation {
        "world.get" => world && !node && !character && !account,
        "world.list" => !node && !character && !account,
        "node.get" => node && !character && !account,
        "node.list" | "ops.overview.get" => !character && !account,
        "player.character.get" | "player.inventory.get" => world && character && !node,
        "player.search" => !node,
        "sanction.list" => (character || account) && !node,
        _ => true,
    }
}

/// Trusted kind of an operation version an agent advertises in hello. The
/// agent never supplies the kind; an unknown operation or version has none.
/// A kind is not an advertisement, authorization or enablement.
pub fn advertised_kind(operation: &str, version: i32) -> Option<signer::Kind> {
    if REGISTERED_READS.contains(&operation) {
        return (version == 1).then_some(signer::Kind::Query);
    }
    StaffOperation::lookup(operation)
        .filter(|op| u32::try_from(version).is_ok_and(|v| v == op.operation_version()))
        .map(StaffOperation::kind)
}

/// Local opt-in allowlist, default empty like other ADM feature gates. A catalog
/// entry alone is never an advertisement, authorization, or executable command.
#[derive(Debug, Default)]
pub struct Catalog {
    staff_allowlist: Vec<StaffOperation>,
}
impl Catalog {
    pub fn with_staff_allowlist(staff_allowlist: Vec<StaffOperation>) -> Self {
        Self { staff_allowlist }
    }
    pub fn enabled(&self, operation: StaffOperation) -> bool {
        self.staff_allowlist.contains(&operation)
    }
    pub fn validate(
        &self,
        request: &signer::RequestEnvelope,
    ) -> Result<controls::Operation, controls::Refusal> {
        let op =
            StaffOperation::lookup(&request.operation).ok_or(controls::Refusal::InvalidInput)?;
        if !self.enabled(op) {
            return Err(controls::Refusal::Disabled);
        }
        validate_staff_request(request)
    }
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
