//! Guards the embedded migration set. sqlx keys applied migrations by their
//! numeric version, so two files with the same version prefix make
//! `migrate run` fail on a real database even though everything compiles.

use std::collections::BTreeMap;

#[test]
fn migration_versions_are_unique() {
    let mut seen: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    for migration in oteryn_db::MIGRATOR.iter() {
        // A reversible pair shares one version; only up migrations count.
        if migration.migration_type.is_down_migration() {
            continue;
        }
        seen.entry(migration.version)
            .or_default()
            .push(migration.description.to_string());
    }
    let duplicates: Vec<_> = seen.iter().filter(|(_, names)| names.len() > 1).collect();
    assert!(
        duplicates.is_empty(),
        "duplicate migration versions in portal-rs/migrations: {duplicates:?}"
    );
}

#[test]
fn migrations_are_embedded() {
    assert!(oteryn_db::MIGRATOR.iter().next().is_some());
}

/// Versions are 14-digit UTC timestamps (`YYYYMMDDhhmmss`). A shorter or
/// longer prefix still compiles but sorts out of order, so the migration
/// would run before or after the schema it was written against.
#[test]
fn migration_versions_are_fourteen_digit_timestamps() {
    let malformed: Vec<_> = oteryn_db::MIGRATOR
        .iter()
        .filter(|migration| !(10_000_000_000_000..100_000_000_000_000).contains(&migration.version))
        .map(|migration| (migration.version, migration.description.to_string()))
        .collect();
    assert!(
        malformed.is_empty(),
        "migration versions must be 14-digit timestamps: {malformed:?}"
    );
}
