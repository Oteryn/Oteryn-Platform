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

#[path = "lifecycle_sql_guard.rs"]
mod sql_guard;
use sql_guard::scan_source;

#[test]
fn lifecycle_state_writes_use_writers() -> std::io::Result<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let offenders = sql_guard::scan(&root)?;
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
        (
            "UPDATE bazaar_settlements s SET completed_at=clock_timestamp() WHERE s.listing_id=$1",
            true,
        ),
        (
            "UPDATE bazaar_authority_gold_sales s SET completed_at=now() WHERE s.id=$1",
            true,
        ),
        (
            "INSERT INTO bazaar_authority_compensations(listing_id,return_operation_id,recovery_generation) SELECT $1,$2,$3",
            true,
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

#[test]
fn lifecycle_guard_checks_aliases_and_every_cte_write() {
    let mut misses = Vec::new();
    for sql in [
        "UPDATE bazaar_listings AS l SET state=$1 WHERE l.id=$2",
        r#"UPDATE ONLY public."bazaar_listings" l SET (leader,state)=($1,$2) WHERE l.id=$3"#,
        "WITH moved AS (UPDATE bazaar_listings l SET state=$1 RETURNING id)
         INSERT INTO other_table (id) SELECT id FROM moved",
        "WITH log AS (INSERT INTO other_table (id) VALUES($1) RETURNING id)
         INSERT INTO bazaar_listings (id) SELECT id FROM log",
    ] {
        let mut offenders = Vec::new();
        scan_source(
            std::path::Path::new("outside_writer.rs"),
            &format!("query(r##\"{sql}\"##)"),
            &mut offenders,
        );
        if offenders.len() != 1 {
            misses.push(sql);
        }
    }
    assert!(misses.is_empty(), "{misses:?}");
    let mut offenders = Vec::new();
    scan_source(
        std::path::Path::new("outside_writer.rs"),
        r#"query("UPDATE \"bazaar_listings\" AS l\nSET state=$1")"#,
        &mut offenders,
    );
    assert_eq!(offenders.len(), 1);
}

#[test]
fn lifecycle_guard_limits_fixture_exemption_to_test_item() {
    let sql = "UPDATE bazaar_listings SET state='active'";
    let fixture = format!(
        r#"
        #[cfg(test)]
        mod tests {{ fn seed() {{ query("{sql}"); }} }}
        "#
    );
    let path = std::path::Path::new("game/runtime/browser.rs");
    let mut offenders = Vec::new();
    scan_source(path, &fixture, &mut offenders);
    assert!(offenders.is_empty());
    let source = format!(r#"{fixture} fn production() {{ query("{sql}"); }}"#);
    scan_source(path, &source, &mut offenders);
    assert_eq!(offenders.len(), 1);
}

#[test]
fn lifecycle_guard_handles_postgres_escape_strings() {
    let source = r##"fn write() { query(r#"WITH note AS (SELECT E'can\'t') UPDATE bazaar_listings SET state='active'"#); }"##;
    let mut offenders = Vec::new();
    scan_source(
        std::path::Path::new("outside_writer.rs"),
        source,
        &mut offenders,
    );
    assert_eq!(offenders.len(), 1);
}

#[test]
fn lifecycle_guard_keeps_adjacent_array_element_in_production() {
    let source = r#"fn write() { let sql = [#[cfg(test)] "fixture", "UPDATE bazaar_listings SET state='active'"]; query(sql[0]); }"#;
    let mut offenders = Vec::new();
    scan_source(
        std::path::Path::new("game/runtime/browser.rs"),
        source,
        &mut offenders,
    );
    assert_eq!(offenders.len(), 1);
}

#[test]
fn bz07_guard_checks_assembled_and_included_sql() -> std::io::Result<()> {
    let root = std::env::temp_dir().join(format!("bazaar-sql-guard-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir(&root)?;
    std::fs::write(
        root.join("write.sql"),
        "UPDATE bazaar_listings SET state=$1 WHERE id=$2",
    )?;
    std::fs::write(
        root.join("lib.rs"),
        r#"
        fn assembled() {
            sqlx::query(concat!("UPDATE bazaar_listings ", "SET state=$1 WHERE id=$2"));
        }
        fn included() {
            sqlx::query(include_str!("write.sql"));
        }
    "#,
    )?;
    let result = sql_guard::scan(&root);
    std::fs::remove_dir_all(root)?;
    assert_eq!(result?.len(), 2);
    Ok(())
}

#[test]
pub(crate) fn bz07_bz13_guard_checks_nested_sql_in_macro_definitions() -> std::io::Result<()> {
    let root = std::env::temp_dir().join(format!("bazaar-sql-guard-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir(&root)?;
    std::fs::write(root.join("target.sql"), "UPDATE bazaar_listings ")?;
    std::fs::write(
        root.join("lib.rs"),
        r#"
        macro_rules! write_listing {
            () => {
                sqlx::query(concat!(include_str!(concat!("target", ".sql")), "SET state=$1 WHERE id=$2"))
            };
        }
        fn write() { write_listing!(); }
    "#,
    )?;
    let result = sql_guard::scan(&root);
    std::fs::remove_dir_all(root)?;
    assert_eq!(result?.len(), 1);
    Ok(())
}
