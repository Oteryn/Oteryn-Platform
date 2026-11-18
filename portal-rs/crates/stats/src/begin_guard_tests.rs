//! Every transaction in this crate bounds its lock waits, or is listed here
//! with the reason it does not need to at its `begin()`.
use std::collections::BTreeMap;

/// Sites whose budget is armed elsewhere or which take no request-path lock.
const EXEMPT: &[(&str, usize, &str)] = &[
    (
        "online/storage.rs",
        2,
        "Game ingest apply and the daily purge worker, serialized on the projection row",
    ),
    (
        "world/kills.rs",
        1,
        "Game ingest apply with its own retry contract",
    ),
    (
        "worker.rs",
        2,
        "background jobs off the request path, serialized by worker advisory lock 65001",
    ),
    (
        "watch.rs",
        1,
        "dispatch: event fanout with no production caller",
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
fn death_expiry_has_one_cast_safe_predicate() {
    let worker = include_str!("worker.rs");
    assert_eq!(
        worker.matches("DELETE FROM stats_character_events").count(),
        2
    );
    assert_eq!(worker.matches("details->>'occurred_at'").count(), 3);
    assert!(worker.contains("purge_death_details(&mut tx, false)"));
    assert!(worker.contains("purge_death_details(&mut tx, true)"));
}
