//! Authentication configuration schema. Locator-to-node metadata is not a purpose binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Map,
    Single,
}
/// Add new authentication settings here when wiring a purpose. Do not register
/// metadata maps merely because their names end in `_IDENTITIES`.
pub fn shape(key: &str) -> Option<Shape> {
    match key {
        "GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY" => Some(Shape::Single),
        "GAME_AUTH_NATIVE_EVIDENCE_IDENTITIES" => Some(Shape::Map),
        "GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY" => Some(Shape::Single),
        "GAME_AUTH_NATIVE_RECOVERY_LOCATOR_IDENTITIES" => Some(Shape::Map),
        "GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES" => Some(Shape::Map),
        "GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES" => Some(Shape::Map),
        "OTERYN_ADMIN_ANALYTICS_PEER_SAN" => Some(Shape::Single),
        "OTERYN_GATEWAY_SAN" => Some(Shape::Single),
        "PORTAL_CHARACTER_COMMAND_IDENTITIES" => Some(Shape::Map),
        "PORTAL_CHARACTER_SERVICES_IDENTITY" => Some(Shape::Single),
        "PORTAL_GAME_COINS_IDENTITIES" => Some(Shape::Map),
        "PORTAL_GAME_ENFORCEMENT_READER_IDENTITY" => Some(Shape::Single),
        "PORTAL_GAME_ENFORCEMENT_RESULTS_IDENTITY" => Some(Shape::Single),
        "PORTAL_GAME_RESTORE_NOTICE_IDENTITY" => Some(Shape::Single),
        "PORTAL_LCFA_INTERNAL_IDENTITY" => Some(Shape::Single),
        "PORTAL_LOYALTY_STREAM_IDENTITY" => Some(Shape::Single),
        "PORTAL_PLAYERDATA_JOURNAL_ITEMS_SAN" => Some(Shape::Single),
        "PORTAL_PLAYERDATA_JOURNAL_PROGRESS_SAN" => Some(Shape::Single),
        "PORTAL_PLAYERDATA_JOURNAL_SESSIONS_SAN" => Some(Shape::Single),
        "PORTAL_PUBLIC_PROJECTIONS_SAN" => Some(Shape::Single),
        "PORTAL_SANCTION_EXPORT_IDENTITY" => Some(Shape::Single),
        "PORTAL_SECURITY_REVOCATION_EXPORT_IDENTITY" => Some(Shape::Single),
        "PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_MTLS_CLIENT_IDENTITY" => Some(Shape::Single),
        "PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_MTLS_CLIENT_IDENTITY" => Some(Shape::Single),
        "PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_MTLS_CLIENT_IDENTITY" => Some(Shape::Single),
        "PRODUCTS_ENTITLEMENTS_PROFILE_B_AUTHORITY_MTLS_CLIENT_IDENTITY" => Some(Shape::Single),
        "PRODUCTS_ENTITLEMENTS_PROFILE_B_CHANGES_MTLS_CLIENT_IDENTITY" => Some(Shape::Single),
        "PRODUCTS_ENTITLEMENTS_STORE_MTLS_CLIENT_IDENTITY" => Some(Shape::Single),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locator_node_metadata_is_not_an_authentication_purpose() {
        assert_eq!(
            shape("GAME_AUTH_NATIVE_RECOVERY_LOCATOR_IDENTITIES"),
            Some(Shape::Map)
        );
        assert_eq!(shape("PORTAL_NATIVE_RECOVERY_LOCATOR_IDENTITIES"), None);
    }
}
