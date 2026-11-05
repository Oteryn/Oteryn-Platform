#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
use crate::{AccountState, Accounts};
use sqlx::{PgConnection, postgres::PgPoolOptions};
use uuid::Uuid;
const HOLD: &str = include_str!("fixtures/hold.json");
const SNAPSHOT: &str = include_str!("fixtures/snapshot.json");
fn uid(suffix: &str) -> Uuid {
    Uuid::parse_str(&format!("0192a000-0000-7000-8000-{suffix:0>12}")).unwrap()
}
#[test]
fn command_golden_and_invalid_input() {
    for raw in [
        HOLD,
        include_str!("fixtures/escrow.json"),
        include_str!("fixtures/deliver.json"),
        include_str!("fixtures/return.json"),
    ] {
        assert!(PreparedIntent::parse(raw).is_ok());
    }
    for raw in [
        HOLD.replace("\"10000\"", "10000"),
        HOLD.replace("\"10000\"", "\"010000\""),
        HOLD.replace("\"10000\"", "\"1000000000000\""),
        HOLD.replace("\"10000\"", "null"),
        HOLD.replace("\"10000\"", "\"1\""),
        HOLD.replace("\"17\"", "\"017\""),
        format!("{HOLD}\n"),
        HOLD.replace(
            "\"contract_version\":2",
            "\"contract_version\":2,\"contract_version\":2",
        ),
        HOLD.replace("\"contract_version\":2", "\"contract_version\":1"),
        HOLD.replace("\"offer_id\"", "\"unknown\""),
        HOLD.replace("000000000013", "000000000011"),
    ] {
        assert!(PreparedIntent::parse(&raw).is_err(), "{raw}");
    }
    assert_eq!(gold_payout(10000).unwrap(), (250, 9750));
    assert_eq!(gold_payout(41).unwrap(), (2, 39));
    assert_eq!(
        gold_payout(999_999_999_999).unwrap(),
        (25_000_000_000, 974_999_999_999)
    );
    for n in [0, 1, 1_000_000_000_000, u64::MAX] {
        assert!(gold_payout(n).is_err());
    }
    assert!(!Settings::default().dispatch_available());
    assert!(
        !Settings {
            preparation_enabled: true,
            game_enabled: true
        }
        .dispatch_available()
    );
}
fn binding() -> SnapshotBinding<'static> {
    SnapshotBinding {
        source_authority: "oteryn:game:primary",
        listing: uid("30"),
        seller: uid("11"),
        character: uid("31"),
        world: uid("12"),
        lock: uid("32"),
        lock_revision: 1,
        character_revision: 7,
        eligibility_revision: 4,
        projection_epoch: 3,
        now: 1790000003,
    }
}
#[test]
fn signed_snapshot_matches_then_denies_tampering_and_wrong_bindings() {
    let key_bytes: [u8; 32] = [
        0xe8, 0xc3, 0x03, 0x90, 0x44, 0x1a, 0x54, 0x64, 0x23, 0x82, 0xf1, 0x5a, 0x78, 0x40, 0x87,
        0xa4, 0x2d, 0xc7, 0x5f, 0x85, 0xf7, 0x75, 0xfd, 0xfc, 0x9e, 0x2a, 0x25, 0xb0, 0xf0, 0x8a,
        0x98, 0x06,
    ];
    let key = ed25519_dalek::VerifyingKey::from_bytes(&key_bytes).unwrap();
    assert!(verify_snapshot(SNAPSHOT.as_bytes(), &binding(), "fixture-example-1", &key).is_ok());
    let tampered = SNAPSHOT.replace("\"100\"", "\"101\"");
    assert!(verify_snapshot(tampered.as_bytes(), &binding(), "fixture-example-1", &key).is_err());
    let mut expected = binding();
    expected.projection_epoch = 4;
    assert!(verify_snapshot(SNAPSHOT.as_bytes(), &expected, "fixture-example-1", &key).is_err());
    expected = binding();
    expected.seller = uid("13");
    assert!(verify_snapshot(SNAPSHOT.as_bytes(), &expected, "fixture-example-1", &key).is_err());
    assert!(verify_snapshot(SNAPSHOT.as_bytes(), &binding(), "revoked", &key).is_err());
    assert!(verify_snapshot(b"{}", &binding(), "fixture-example-1", &key).is_err());
}
struct Account(bool);
impl Accounts for Account {
    async fn state(
        &self,
        _db: &mut PgConnection,
        _account: Uuid,
    ) -> Result<Option<AccountState>, crate::Error> {
        Ok(self.0.then_some(AccountState {
            active: true,
            email_verified: true,
            mfa: true,
            sudo: true,
            bazaar_blocked: false,
        }))
    }
}
#[tokio::test]
async fn postgres_staged_outbox_replay_refusal_conflict_and_restore() {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIP PostgreSQL T58: PORTAL_TEST_DATABASE_URL unset");
        return;
    };
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    // Transaction rollback isolates this issuer fixture from other tests.
    sqlx::query("DELETE FROM bazaar_game_outbox")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE bazaar_game_issuer SET source_revision=0,projection_epoch=0,recovery_generation=0",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    Outbox::reopen(&mut tx, 3, 1).await.unwrap();
    let outbox = Outbox {
        settings: Settings {
            preparation_enabled: true,
            game_enabled: false,
        },
    };
    let intent = PreparedIntent::parse(HOLD).unwrap();
    assert!(matches!(
        Outbox::default()
            .stage(&mut tx, &Account(true), uid("13"), uid("90"), &intent, 3, 1)
            .await,
        Err(Error::Unavailable)
    ));
    assert!(matches!(
        outbox
            .stage(
                &mut tx,
                &Account(false),
                uid("13"),
                uid("90"),
                &intent,
                3,
                1
            )
            .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        outbox
            .stage(&mut tx, &Account(true), uid("11"), uid("90"), &intent, 3, 1)
            .await,
        Err(Error::Denied)
    ));
    let first = outbox
        .stage(&mut tx, &Account(true), uid("13"), uid("90"), &intent, 3, 1)
        .await
        .unwrap();
    assert_eq!(first.state, "STAGED");
    assert_eq!(
        first,
        outbox
            .stage(&mut tx, &Account(true), uid("13"), uid("90"), &intent, 3, 1)
            .await
            .unwrap()
    );
    let changed = PreparedIntent::parse(&HOLD.replace("\"10000\"", "\"10001\"")).unwrap();
    assert!(matches!(
        outbox
            .stage(
                &mut tx,
                &Account(true),
                uid("13"),
                uid("90"),
                &changed,
                3,
                1
            )
            .await,
        Err(Error::Conflict)
    ));
    Outbox::reopen(&mut tx, 4, 2).await.unwrap();
    assert!(matches!(
        Outbox::reopen(&mut tx, 3, 1).await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        outbox
            .stage(&mut tx, &Account(true), uid("13"), uid("90"), &intent, 3, 1)
            .await,
        Err(Error::Denied)
    ));
    let state: String =
        sqlx::query_scalar("SELECT state FROM bazaar_game_outbox WHERE operation_id=$1")
            .bind(first.operation_id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(state, "REOPENED");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_game_outbox")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(count, 1);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn postgres_projection_ordering_poison_watermark_and_epoch_fences() {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIP PostgreSQL T58: PORTAL_TEST_DATABASE_URL unset");
        return;
    };
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    // All issuer users serialize on this row, including other concurrent tests.
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=3,recovery_generation=1")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("DELETE FROM bazaar_game_projections")
        .execute(&mut *tx)
        .await
        .unwrap();
    let outbox = Outbox {
        settings: Settings {
            preparation_enabled: true,
            game_enabled: false,
        },
    };
    let authority = "oteryn:game:primary";
    let policy = r#"{"contract_version":2,"operation":"PublishBazaarTransferabilityV1","source_authority":"oteryn:game:primary","world_id":"0192a000-0000-7000-8000-000000000012","projection_epoch":"3","projection_revision":"4","source_observed_at":"1790000002","eligibility_revision":"4","definitions":[]}"#;
    assert!(matches!(
        outbox
            .ingest(&mut tx, "foreign", policy.as_bytes(), 1790000003)
            .await,
        Err(Error::Denied)
    ));
    assert_eq!(
        outbox
            .ingest(&mut tx, authority, policy.as_bytes(), 1790000003)
            .await
            .unwrap(),
        IngestResult::Applied
    );
    assert_eq!(
        outbox
            .ingest(&mut tx, authority, policy.as_bytes(), 1790000003)
            .await
            .unwrap(),
        IngestResult::Duplicate
    );
    let changed = policy.replace(
        "\"eligibility_revision\":\"4\"",
        "\"eligibility_revision\":\"5\"",
    );
    assert_eq!(
        outbox
            .ingest(&mut tx, authority, changed.as_bytes(), 1790000003)
            .await
            .unwrap(),
        IngestResult::Quarantined
    );
    let old = policy.replace(
        "\"projection_revision\":\"4\"",
        "\"projection_revision\":\"3\"",
    );
    assert_eq!(
        outbox
            .ingest(&mut tx, authority, old.as_bytes(), 1790000003)
            .await
            .unwrap(),
        IngestResult::Superseded
    );
    let malformed = policy.replace(
        "\"projection_revision\":\"4\"",
        "\"projection_revision\":\"04\"",
    );
    assert!(matches!(
        outbox
            .ingest(&mut tx, authority, malformed.as_bytes(), 1790000003)
            .await,
        Err(Error::Invalid)
    ));
    let watermark = r#"{"contract_version":2,"operation":"PublishBazaarWatermarkV1","source_authority":"oteryn:game:primary","projection_epoch":"3","complete_through":"1790000000","observed_at":"1790000003"}"#;
    assert_eq!(
        outbox
            .ingest(&mut tx, authority, watermark.as_bytes(), 1790000003)
            .await
            .unwrap(),
        IngestResult::Applied
    );
    let heartbeat = watermark.replace(
        "\"observed_at\":\"1790000003\"",
        "\"observed_at\":\"1790000004\"",
    );
    assert_eq!(
        outbox
            .ingest(&mut tx, authority, heartbeat.as_bytes(), 1790000004)
            .await
            .unwrap(),
        IngestResult::Duplicate
    );
    let observed: String = sqlx::query_scalar(
        "SELECT source_observed_at::text FROM bazaar_game_projections WHERE kind='WATERMARK'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(observed, "1790000003");
    let future = watermark.replace(
        "\"complete_through\":\"1790000000\"",
        "\"complete_through\":\"1790000004\"",
    );
    assert!(matches!(
        outbox
            .ingest(&mut tx, authority, future.as_bytes(), 1790000003)
            .await,
        Err(Error::Invalid)
    ));
    let newer = policy.replace("\"projection_epoch\":\"3\"", "\"projection_epoch\":\"4\"");
    assert_eq!(
        outbox
            .ingest(&mut tx, authority, newer.as_bytes(), 1790000003)
            .await
            .unwrap(),
        IngestResult::Applied
    );
    assert_eq!(
        outbox
            .ingest(&mut tx, authority, watermark.as_bytes(), 1790000003)
            .await
            .unwrap(),
        IngestResult::Superseded
    );
    tx.rollback().await.unwrap();
}

#[test]
fn remaining_command_variants_validate_exact_closed_members_and_payout() {
    let prefix = HOLD.split("\"buyer_account_id\"").next().unwrap();
    let tail = HOLD.split("\"issued_at_source\"").nth(1).unwrap();
    for (command, fields) in [
        (
            Command::Lock,
            r#""character_id":"0192a000-0000-7000-8000-000000000031","expected_character_revision":"7","eligibility_revision":"4","snapshot_schema_version":1,"#,
        ),
        (
            Command::Unlock,
            r#""character_id":"0192a000-0000-7000-8000-000000000031","lock_id":"0192a000-0000-7000-8000-000000000032","expected_lock_revision":"1","#,
        ),
        (
            Command::Bind,
            r#""character_id":"0192a000-0000-7000-8000-000000000031","lock_id":"0192a000-0000-7000-8000-000000000032","expected_lock_revision":"1","transfer_operation_id":"0192a000-0000-7000-8000-000000000033","transfer_binding_digest":"1111111111111111111111111111111111111111111111111111111111111111","purpose":"BAZAAR_LISTING_ESCROW","sale_id":null,"funding":null,"#,
        ),
        (
            Command::Release,
            r#""buyer_account_id":"0192a000-0000-7000-8000-000000000013","hold_id":"0192a000-0000-7000-8000-000000000035","expected_hold_revision":"1","release_reason":"EXCESS","amount":"1","#,
        ),
        (
            Command::Payout,
            r#""buyer_account_id":"0192a000-0000-7000-8000-000000000013","hold_id":"0192a000-0000-7000-8000-000000000035","expected_hold_revision":"1","sale_id":"0192a000-0000-7000-8000-000000000036","gross_amount":"10000","fee_bps":250,"fee_amount":"250","seller_amount":"9750","delivery_operation_id":"0192a000-0000-7000-8000-000000000037","delivery_receipt_digest":"1111111111111111111111111111111111111111111111111111111111111111","#,
        ),
    ] {
        let raw = format!(
            "{}{fields}\"issued_at_source\"{tail}",
            prefix.replace(Command::Hold.wire(), command.wire())
        );
        assert_eq!(PreparedIntent::parse(&raw).unwrap().command(), command);
        assert!(
            PreparedIntent::parse(
                &raw.replace("\"source_revision\":\"17\"", "\"source_revision\":\"0\"")
            )
            .is_err()
        );
        if command == Command::Payout {
            assert!(
                PreparedIntent::parse(
                    &raw.replace("\"250\"", "\"249\"")
                        .replace("\"9750\"", "\"9751\"")
                )
                .is_err()
            );
        }
        if command == Command::Bind {
            assert!(
                PreparedIntent::parse(&raw.replace("BAZAAR_LISTING_ESCROW", "BAZAAR_SETTLEMENT"))
                    .is_err()
            );
        }
    }
}
