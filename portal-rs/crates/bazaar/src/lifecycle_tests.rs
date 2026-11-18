use crate::State::{self, Active, AwaitingDelivery, Cancelled, Completed, Draft, Expired, Matched};
use crate::lifecycle::{Context, Current, Event, Outcome, Projection, Refusal, transition};

const STATES: [State; 7] = [
    Draft,
    Active,
    Matched,
    AwaitingDelivery,
    Completed,
    Cancelled,
    Expired,
];

#[test]
fn listing_transition_table() {
    let allowed: [[bool; 7]; 7] = [
        [false, true, false, false, false, true, false],
        [false, false, true, true, false, true, true],
        [false, false, false, true, false, false, false],
        [false, false, false, false, true, false, false],
        [false; 7],
        [false; 7],
        [false; 7],
    ];
    for (old, row) in STATES.into_iter().zip(allowed) {
        for (new, allowed) in STATES.into_iter().zip(row) {
            let expected = if allowed {
                Ok(Outcome::Listing(new))
            } else {
                Err(Refusal::Illegal)
            };
            assert_eq!(
                transition(
                    Current::Listing(old),
                    Event::Listing(new),
                    Context::default()
                ),
                expected,
                "{old:?} -> {new:?}"
            );
        }
    }
    for (state, expected) in STATES.into_iter().zip([
        [false, true, true],
        [true, true, true],
        [false, false, false],
        [false, false, false],
        [false, false, false],
        [false, false, false],
        [false, false, false],
    ]) {
        for (event, allowed) in [Event::Bid, Event::CancelRequest, Event::Suspend]
            .into_iter()
            .zip(expected)
        {
            assert_eq!(
                transition(Current::Listing(state), event, Context::default()).is_ok(),
                allowed,
                "{state:?}, {event:?}"
            );
        }
        for held in [false, true] {
            let expected = match (state, held) {
                (Matched | Expired, false | true) => Err(Refusal::Illegal),
                (Draft | Active, true) => Err(Refusal::ModerationHold),
                (Draft | Active, false)
                | (AwaitingDelivery | Completed | Cancelled, false | true) => Ok(Outcome::Continue),
            };
            assert_eq!(
                transition(
                    Current::Listing(state),
                    Event::Advance,
                    Context {
                        held,
                        ..Default::default()
                    }
                ),
                expected
            );
        }
        assert_eq!(
            transition(
                Current::Listing(state),
                Event::Compensated,
                Context::default()
            )
            .is_ok(),
            matches!(state, AwaitingDelivery | Cancelled),
        );
    }
    assert_eq!(
        transition(Current::Absent, Event::Create, Context::default()),
        Ok(Outcome::Listing(Draft))
    );
}

#[test]
fn authority_transition_table() {
    let cursor = |epoch, revision, observed| Projection {
        epoch,
        revision,
        observed,
    };
    let old = Current::Projection(Some(cursor(3, 5, 10)));
    for (incoming, same_content, watermark, expected) in [
        (cursor(3, 6, 11), false, false, Outcome::Apply),
        (cursor(3, 4, 10), false, false, Outcome::Superseded),
        (cursor(3, 5, 10), true, false, Outcome::Duplicate),
        (cursor(3, 5, 10), false, false, Outcome::Quarantine),
        (cursor(3, 6, 9), false, true, Outcome::Superseded),
        (cursor(4, 1, 11), false, false, Outcome::Apply),
        (cursor(2, 90, 11), false, false, Outcome::Superseded),
    ] {
        assert_eq!(
            transition(
                old,
                Event::Projection,
                Context {
                    incoming: Some(incoming),
                    same_content,
                    watermark,
                    ..Default::default()
                }
            ),
            Ok(expected)
        );
    }
    for (dispatched, admitted, retained, expected) in [
        (false, false, false, Ok(Outcome::Continue)),
        (true, false, false, Err(Refusal::EvidenceMissing)),
        (false, true, false, Err(Refusal::EvidenceMissing)),
        (true, true, true, Ok(Outcome::Continue)),
    ] {
        assert_eq!(
            transition(
                Current::Intent {
                    dispatched,
                    admitted
                },
                Event::ResyncIntent,
                Context {
                    retained,
                    ..Default::default()
                }
            ),
            expected
        );
    }
    for (completed_item, delivered, expected) in [
        (false, false, Ok(Outcome::Continue)),
        (true, false, Err(Refusal::CustodyDrift)),
        (true, true, Ok(Outcome::Continue)),
    ] {
        assert_eq!(
            transition(
                Current::Custody { completed_item },
                Event::ResyncCustody,
                Context {
                    delivered,
                    ..Default::default()
                }
            ),
            expected
        );
    }
    for (dispatched, admitted, reopened, committed, expected) in [
        (false, false, false, false, Err(Refusal::Illegal)),
        (true, false, false, false, Ok(Outcome::Receipt("PENDING"))),
        (true, false, true, false, Ok(Outcome::Receipt("REOPENED"))),
        (true, true, false, true, Ok(Outcome::Receipt("COMMITTED"))),
        (true, true, false, false, Ok(Outcome::Receipt("REJECTED"))),
    ] {
        assert_eq!(
            transition(
                Current::Intent {
                    dispatched,
                    admitted
                },
                Event::ReadIntent,
                Context {
                    reopened,
                    committed,
                    ..Context::default()
                }
            ),
            expected
        );
    }
    let staged = Current::Intent {
        dispatched: false,
        admitted: false,
    };
    assert_eq!(
        transition(Current::Absent, Event::Stage, Context::default()),
        Ok(Outcome::Apply)
    );
    assert_eq!(
        transition(staged, Event::Dispatch, Context::default()),
        Ok(Outcome::Apply)
    );
    for (committed, outcome) in [(true, "COMMITTED"), (false, "REJECTED")] {
        assert_eq!(
            transition(staged, Event::Receipt { committed }, Context::default()),
            Ok(Outcome::Receipt(outcome))
        );
    }
    for (new, expected) in [
        ((3, 3), Ok(Outcome::Duplicate)),
        ((4, 3), Ok(Outcome::Apply)),
        ((3, 4), Ok(Outcome::Apply)),
        ((2, 4), Err(Refusal::Illegal)),
        ((4, 2), Err(Refusal::Illegal)),
    ] {
        assert_eq!(
            transition(
                Current::Fence { old: (3, 3), new },
                Event::Reopen,
                Context::default()
            ),
            expected
        );
    }
}

fn normalize_sql(sql: &str) -> String {
    sql.to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn forbidden_state_write(sql: &str) -> bool {
    let insert = [
        "bazaar_listings",
        "bazaar_game_outbox",
        "bazaar_game_projections",
        "bazaar_authority_dispatch",
        "bazaar_authority_receipts",
        "bazaar_game_quarantine",
        "bazaar_authority_gold_bids",
        "bazaar_authority_transfers",
        "bazaar_authority_transfer_retries",
        "bazaar_authority_transfer_retirements",
        "character_commands",
    ];
    sql.split_once("insert into ").is_some_and(|(_, tail)| {
        tail.split_once('(')
            .is_some_and(|(table, _)| insert.contains(&table.trim_end()))
    }) || sql.contains("update bazaar_game_outbox ")
        || sql.contains("update bazaar_game_projections ")
        || sql.contains("update bazaar_authority_gold_bids set phase")
        || sql
            .split_once("update bazaar_listings set ")
            .is_some_and(|(_, set)| {
                let assignments = set
                    .split_once(" where ")
                    .map_or(set, |(assignments, _)| assignments);
                assignments.split(',').any(|assignment| {
                    assignment
                        .split_once('=')
                        .is_some_and(|(column, _)| column.trim() == "state")
                })
            })
}

// Inline PostgreSQL fixtures seed catalogue rows, model a completed CC1
// handoff, or build a worker backlog. Exempt only these statements in the
// cfg(test) portion of their source files.
const FIXTURE_WRITES: &[(&str, &str)] = &[
    (
        "game/runtime/browser.rs",
        "INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at,created_at) VALUES($1,$2,'item','auction','coins',100,false,CASE WHEN $3=0 THEN 'active'::bazaar_state ELSE 'completed'::bazaar_state END,now()+interval '1 day',now()+make_interval(secs=>$3))",
    ),
    (
        "game/runtime/browser.rs",
        "UPDATE bazaar_listings SET state='active'",
    ),
    (
        "game/runtime/browser.rs",
        "UPDATE bazaar_listings SET state='awaiting_delivery'",
    ),
    (
        "game/runtime/browser.rs",
        "UPDATE bazaar_listings SET state='completed',kind='character'",
    ),
    (
        "game/runtime/worker.rs",
        "INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at) VALUES($1,$2,'item','auction','coins',100,false,'awaiting_delivery',now())",
    ),
    (
        "game/runtime/worker.rs",
        "INSERT INTO bazaar_authority_gold_bids(request_id,listing_id,buyer,character_id,maximum,hold_operation_id) VALUES($1,$1,$1,$1,100,$1)",
    ),
    (
        "game/runtime/mod.rs",
        "INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at) VALUES($1,$2,'item','auction','coins',100,false,now()+interval '1 day')",
    ),
];

fn scan_source(path: &std::path::Path, source: &str, offenders: &mut Vec<String>) {
    if path == std::path::Path::new("lifecycle.rs") {
        return;
    }
    let (production, fixtures) = source.split_once("#[cfg(test)]").unwrap_or((source, ""));
    for (source, may_exempt_fixture) in [(production, false), (fixtures, true)] {
        for sql in source.split('"').skip(1).step_by(2) {
            let normalized = normalize_sql(sql);
            if forbidden_state_write(&normalized)
                && !(may_exempt_fixture
                    && FIXTURE_WRITES.iter().any(|(file, fixture)| {
                        path == std::path::Path::new(file) && normalized == normalize_sql(fixture)
                    }))
            {
                offenders.push(format!("{}: {normalized}", path.display()));
            }
        }
    }
}

fn scan(
    root: &std::path::Path,
    dir: &std::path::Path,
    offenders: &mut Vec<String>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(root.join(dir))? {
        let entry = entry?;
        let path = dir.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            scan(root, &path, offenders)?;
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.ends_with(".rs") && !name.ends_with("tests.rs") {
            scan_source(&path, &std::fs::read_to_string(entry.path())?, offenders);
        }
    }
    Ok(())
}

#[test]
fn lifecycle_state_writes_use_writers() -> std::io::Result<()> {
    let mut offenders = Vec::new();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    scan(&root, std::path::Path::new(""), &mut offenders)?;
    assert!(offenders.is_empty(), "{offenders:#?}");
    Ok(())
}

#[test]
fn lifecycle_guard_detects_forbidden_sql_and_accepts_writer() {
    use std::path::Path;
    let cases = [
        ("update bazaar_listings set state='cancelled'", true),
        (
            "UpDaTe  bazaar_listings\n SET updated_at=now(),\t STATE = $1 WHERE id=$2",
            true,
        ),
        ("INSERT INTO bazaar_listings(id) VALUES($1)", true),
        ("InSeRt\t INTO bazaar_listings \n (id) VALUES($1)", true),
        (
            "INSERT INTO bazaar_game_outbox (operation_id) VALUES($1)",
            true,
        ),
        (
            "UPDATE bazaar_listings SET updated_at=now() WHERE state='active'",
            false,
        ),
    ];
    for (sql, expected) in cases {
        let mut offenders = Vec::new();
        scan_source(
            Path::new("outside_writer.rs"),
            &format!(r#"query("{sql}")"#),
            &mut offenders,
        );
        assert_eq!(!offenders.is_empty(), expected, "{sql}");
    }
    let writer = include_str!("lifecycle.rs");
    let mut offenders = Vec::new();
    scan_source(Path::new("lifecycle.rs"), writer, &mut offenders);
    assert!(offenders.is_empty());
    scan_source(Path::new("outside_writer.rs"), writer, &mut offenders);
    assert!(!offenders.is_empty());
}
