//! Every transaction in this crate bounds its lock waits, or is listed here
//! with the reason it does not need to at its `begin()`.
use std::collections::BTreeMap;

/// Sites whose budget is armed elsewhere or which take no request-path lock.
const EXEMPT: &[(&str, usize, &str)] = &[
    (
        "account.rs",
        1,
        "premium panel: one REPEATABLE READ READ ONLY select, no row or advisory lock",
    ),
    (
        "codes.rs",
        1,
        "generate: no production caller; code_routes issues through generate_staff",
    ),
    (
        "delivery.rs",
        2,
        "begin through Delivery::begin, which sets lock_timeout 3s and statement_timeout 5s",
    ),
    (
        "premium_profile_b.rs",
        1,
        "change feed: plain reads of the head and journal, no row or advisory lock",
    ),
    (
        "purchase_repository.rs",
        2,
        "begin through Purchases::begin, which sets lock_timeout 3s and statement_timeout 5s",
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
