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
        [false, false, true],
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

fn scan(dir: &std::path::Path, offenders: &mut Vec<String>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            scan(&path, offenders)?;
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with(".rs") || name.ends_with("tests.rs") || name == "lifecycle.rs" {
            continue;
        }
        let source = std::fs::read_to_string(&path)?;
        // These modules place their inline PostgreSQL fixtures after production code.
        let source = if matches!(name, "browser.rs" | "worker.rs" | "mod.rs")
            && path
                .parent()
                .and_then(std::path::Path::file_name)
                .is_some_and(|dir| dir == "runtime")
        {
            source
                .split_once("#[cfg(test)]")
                .map_or(source.as_str(), |(production, _)| production)
        } else {
            &source
        };
        for sql in source.split('"').skip(1).step_by(2) {
            let normalized = sql
                .to_ascii_lowercase()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
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
            let forbidden = insert
                .iter()
                .any(|table| normalized.contains(&format!("insert into {table}(")))
                || normalized.contains("update bazaar_game_outbox ")
                || normalized.contains("update bazaar_game_projections ")
                || normalized.contains("update bazaar_authority_gold_bids set phase")
                || normalized
                    .split_once("update bazaar_listings ")
                    .is_some_and(|(_, tail)| {
                        tail.split_once(" set ").is_some_and(|(_, set)| {
                            set.split(" where ").next().is_some_and(|assignments| {
                                assignments.split(',').any(|assignment| {
                                    assignment
                                        .split_once('=')
                                        .is_some_and(|(column, _)| column.trim() == "state")
                                })
                            })
                        })
                    });
            if forbidden {
                offenders.push(format!("{}: {normalized}", path.display()));
            }
        }
    }
    Ok(())
}

#[test]
fn lifecycle_state_writes_use_writers() -> std::io::Result<()> {
    let mut offenders = Vec::new();
    scan(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut offenders,
    )?;
    assert!(offenders.is_empty(), "{offenders:#?}");
    Ok(())
}
