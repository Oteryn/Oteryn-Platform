//! Every transaction in this crate bounds its lock waits, or is listed here
//! with the reason it does not need to at its `begin()`.
use std::collections::BTreeMap;

/// Sites whose budget is armed elsewhere or which take no request-path lock.
const EXEMPT: &[(&str, usize, &str)] = &[
    (
        "analytics/economy.rs",
        1,
        "read: its first statement sets statement_timeout 3s, then only pg_try_advisory_xact_lock",
    ),
    (
        "analytics/web.rs",
        2,
        "read: first statement sets statement_timeout 3s then try-lock only; denial: one audit_log INSERT",
    ),
    (
        "console/dashboard.rs",
        1,
        "metrics: READ ONLY snapshot of plain SELECTs, takes no row or advisory lock",
    ),
    (
        "console/players.rs",
        1,
        "record_read: one audit_log INSERT (no FK, unique or trigger), takes no lock it can wait on",
    ),
    (
        "control/controls.rs",
        1,
        "step-up verified: one audit_log INSERT (no FK, unique or trigger), takes no lock it can wait on",
    ),
    (
        "control/store.rs",
        1,
        "expire_due: worker job admin.control.expire; rows taken FOR UPDATE SKIP LOCKED, never waits",
    ),
    (
        "players/mod.rs",
        1,
        "record: one audit_log INSERT (no FK, unique or trigger), takes no lock it can wait on",
    ),
    (
        "web.rs",
        1,
        "refusal: one audit_log INSERT (no FK, unique or trigger), takes no lock it can wait on",
    ),
];

const BUDGETS: &[&str] = &["mutation_budget", "statement_timeout", "lock_timeout"];

fn sources(
    dir: &std::path::Path,
    root: &std::path::Path,
    out: &mut Vec<(String, String)>,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            sources(&path, root, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            let name = path
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            if !name.contains("test") {
                out.push((name, std::fs::read_to_string(&path)?));
            }
        }
    }
    Ok(())
}

#[test]
fn every_transaction_bounds_its_lock_waits() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &root, &mut files)?;
    let mut unbudgeted: BTreeMap<String, usize> = BTreeMap::new();
    for (name, text) in &files {
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if !line.contains(".begin()") {
                continue;
            }
            let window = lines[index..lines.len().min(index + 4)].join("\n");
            if !BUDGETS.iter().any(|budget| window.contains(budget)) {
                *unbudgeted.entry(name.clone()).or_default() += 1;
            }
        }
    }
    let exempt: BTreeMap<String, usize> = EXEMPT
        .iter()
        .map(|(name, count, _)| ((*name).to_owned(), *count))
        .collect();
    assert_eq!(
        unbudgeted, exempt,
        "a begin() without a lock budget must arm one or be listed in EXEMPT with its reason"
    );
    Ok(())
}

#[test]
fn account_role_deletions_check_protected_role_survival() -> Result<(), Box<dyn std::error::Error>>
{
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&root, &root, &mut files)?;
    let mut checked = Vec::new();
    for (name, source) in &files {
        for (index, _) in source.match_indices("DELETE FROM account_roles") {
            let prior = &source[..index];
            let guard = prior
                .rsplit("protected_role_survives")
                .next()
                .unwrap_or_default();
            assert!(
                prior.contains("protected_role_survives") && guard.lines().count() < 12,
                "{name}"
            );
            checked.push(name.as_str());
        }
    }
    checked.sort_unstable();
    assert_eq!(checked, ["rbac/storage.rs", "web.rs"]);
    Ok(())
}
