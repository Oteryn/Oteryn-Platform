#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::commands::{self, AccountState, AccountStateReader, Commands, Intent};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_internal_api::{
    canonical, fixtures,
    guard::{Binding, Identity, IdentityMap, VerifiedClient},
};
use sqlx::{Row, postgres::PgPoolOptions};
use tower::ServiceExt;
const NOTICE: &str = include_str!("fixtures/notice.json");
const RECEIPT: &str = include_str!("../commands/fixtures/create-receipt.json");
const OP: &str = "0192a000-0000-7000-8000-000000000001";
fn settings() -> Settings {
    Settings {
        enabled: true,
        source_authority: "oteryn-game-eu1".into(),
        exchange_available: true,
        ..Settings::default()
    }
}
fn command_settings() -> commands::Settings {
    commands::Settings {
        enabled: true,
        create_enabled: true,
        source_authority: "oteryn-game-eu1".into(),
        ..commands::Settings::default()
    }
}
fn identities(command: bool) -> Arc<IdentityMap> {
    let mut bindings = vec![
        Binding {
            identity: Identity::spki_sha256([1; 32]),
            group: GROUP.into(),
        },
        Binding {
            identity: Identity::spki_sha256([3; 32]),
            group: "other".into(),
        },
    ];
    if command {
        bindings.push(Binding {
            identity: Identity::spki_sha256([2; 32]),
            group: commands::GROUP.into(),
        });
    }
    Arc::new(IdentityMap::new(bindings).unwrap())
}
async fn response(
    app: axum::Router,
    path: &str,
    raw: &str,
    peer: Option<u8>,
) -> (StatusCode, String) {
    let mut request = Request::post(path)
        .header("content-type", "application/json")
        .body(Body::from(raw.to_owned()))
        .unwrap();
    if let Some(peer) = peer {
        request
            .extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(
                Identity::spki_sha256([peer; 32]),
            ));
    }
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    if status == StatusCode::TOO_MANY_REQUESTS {
        assert_eq!(response.headers()["retry-after"], "60");
    }
    assert_eq!(response.headers()["cache-control"], "no-store, private");
    (
        status,
        String::from_utf8(to_bytes(response.into_body(), 8192).await.unwrap().to_vec()).unwrap(),
    )
}
fn app(pool: PgPool) -> axum::Router {
    router(
        Restore::new(pool, settings()),
        command_settings(),
        identities(true),
    )
}
const RECEIPT_PATH: &str = "/internal/v1/game-auth/character-commands/receipt";
fn receipt(generation: u64) -> String {
    RECEIPT
        .replace("oteryn:character-authority:primary", "oteryn-game-eu1")
        .replace(
            "\"recovery_generation\":\"3\"",
            &format!("\"recovery_generation\":\"{generation}\""),
        )
}
fn notice(h: u64, new: u64) -> String {
    NOTICE
        .replace(
            "\"restored_generation\":\"3\"",
            &format!("\"restored_generation\":\"{h}\""),
        )
        .replace(
            "\"new_recovery_generation\":\"4\"",
            &format!("\"new_recovery_generation\":\"{new}\""),
        )
        .replace(
            "018f2a10-0000-7000-8000-000000000001",
            &Uuid::now_v7().to_string(),
        )
}
#[test]
fn golden_vectors_match_game_and_schema_members() {
    let vectors =
        fixtures::load(concat!(env!("CARGO_MANIFEST_DIR"), "/src/restore/fixtures")).unwrap();
    assert_eq!(vectors.len(), 3);
    for f in vectors {
        assert!(f.was_canonical);
        assert_eq!(canonical::encode(&f.value).unwrap(), f.original);
    }
    let n = wire::decode(NOTICE.as_bytes()).unwrap();
    assert_eq!(
        wire::status(n.id, 2, 1),
        include_str!("fixtures/reconciling.json")
    );
    assert_eq!(
        wire::status(n.id, 2, 0),
        include_str!("fixtures/acknowledged.json")
    );
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/contracts/fixtures/store-wire-v1/restore-notice.schema.json"
    ))
    .unwrap();
    let value: serde_json::Value = serde_json::from_str(NOTICE).unwrap();
    assert_eq!(
        value.as_object().unwrap().len(),
        schema["required"].as_array().unwrap().len()
    );
    for field in schema["required"].as_array().unwrap() {
        assert!(value.get(field.as_str().unwrap()).is_some());
    }
}
#[test]
fn invalid_notices_and_uint64_boundaries() {
    for raw in [
        format!("{NOTICE}\n"),
        NOTICE.replace("\"3\"", "\"03\""),
        NOTICE.replace("\"3\"", "3"),
        NOTICE.replace("\"4\"", "\"3\""),
        NOTICE.replace("1790000000", "18446744073709551616"),
        NOTICE.replace("nas-2026-11-05.base", "bad label"),
        NOTICE.replace("018f2a10", "018F2A10"),
        NOTICE.replace("-7000-", "-4000-"),
        NOTICE.replace("\"contract_version\":1", "\"contract_version\":2"),
        NOTICE.replace("\"backup_label\"", "\"unknown\""),
        NOTICE.replace("\"operation\":", "\"extra\":0,\"operation\":"),
        NOTICE.replace(",\"operation\"", ", \"operation\""),
        NOTICE.replace("1790000000", "0"),
        NOTICE.replace("\"3\"", "\"-1\""),
    ] {
        assert!(
            matches!(wire::decode(raw.as_bytes()), Err(Error::BadInput)),
            "{raw}"
        );
    }
    assert_eq!(wire::counter("18446744073709551615").unwrap(), u64::MAX);
    assert!(
        wire::decode(
            NOTICE
                .replace("\"4\"", "\"18446744073709551615\"")
                .as_bytes()
        )
        .is_ok()
    );
}
#[tokio::test]
async fn guards_refuse_wrong_identity_bad_schema_source_and_disabled() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unreachable")
        .unwrap();
    let router = app(pool.clone());
    for peer in [None, Some(2), Some(3), Some(4)] {
        assert_eq!(
            response(router.clone(), PATH, NOTICE, peer).await,
            (StatusCode::UNAUTHORIZED, String::new())
        );
    }
    for raw in [
        "{}".to_owned(),
        NOTICE.replace("\"4\"", "\"04\""),
        format!("{NOTICE}\n"),
    ] {
        assert_eq!(
            response(router.clone(), PATH, &raw, Some(1)).await,
            (StatusCode::BAD_REQUEST, String::new())
        );
    }
    assert_eq!(
        response(router.clone(), PATH, &"x".repeat(513), Some(1))
            .await
            .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        response(
            router,
            PATH,
            &NOTICE.replace("oteryn-game-eu1", "other:game"),
            Some(1)
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let disabled = super::router(
        Restore::new(pool, Settings::default()),
        command_settings(),
        identities(true),
    );
    assert_eq!(
        response(disabled, PATH, NOTICE, Some(1)).await,
        (StatusCode::SERVICE_UNAVAILABLE, String::new())
    );
}
const MIGRATION: &str =
    include_str!("../../../../migrations/20261105160020_t23_restore_reconciliation.sql");
async fn database() -> Option<(PgPool, PgPool, String)> {
    database_at(true).await
}
async fn database_at(apply_restore: bool) -> Option<(PgPool, PgPool, String)> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED DB test: PORTAL_TEST_DATABASE_URL is unset");
        return None;
    };
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    let schema = format!("t23_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await
        .unwrap();
    let search = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .after_connect(move |conn, _| {
            let search = search.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(search)
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105160000_t22_character_commands.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    if apply_restore {
        sqlx::raw_sql(MIGRATION).execute(&pool).await.unwrap();
    }
    Some((pool, admin, schema))
}
async fn cleanup(pool: PgPool, admin: PgPool, schema: String) {
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
async fn seed(pool: &PgPool) {
    let intent = include_str!("../commands/fixtures/create-intent.json");
    let v: serde_json::Value = serde_json::from_str(intent).unwrap();
    let binding = format!(
        "{{\"account_id\":{},\"target_world_id\":{},\"requested_name\":{}}}",
        v["account_id"], v["target_world_id"], v["requested_name"]
    );
    sqlx::query("INSERT INTO character_commands(id,issuer_decision_id,request_key,command,account_id,binding_json,intent_json) VALUES ($1,$2,'restore-test','CreateCharacter',$3,$4,$5)").bind(Uuid::parse_str(OP).unwrap()).bind(Uuid::parse_str(v["issuer_decision_id"].as_str().unwrap()).unwrap()).bind(Uuid::parse_str(v["account_id"].as_str().unwrap()).unwrap()).bind(binding).bind(intent).execute(pool).await.unwrap();
}
#[tokio::test]
async fn db_notice_replay_conflict_stale_and_acknowledgement_once() {
    let Some((pool, admin, schema)) = database().await else {
        return;
    };
    let restore = Restore::new(pool.clone(), settings());
    let expected = wire::status(wire::decode(NOTICE.as_bytes()).unwrap().id, 0, 0);
    let (a, b) = tokio::join!(
        restore.notice(NOTICE.as_bytes()),
        restore.notice(NOTICE.as_bytes())
    );
    assert_eq!(a.unwrap(), expected);
    assert_eq!(b.unwrap(), expected);
    let before: String =
        sqlx::query_scalar("SELECT acknowledged_at::text FROM character_restore_notices")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(restore.notice(NOTICE.as_bytes()).await.unwrap(), expected);
    let after: String =
        sqlx::query_scalar("SELECT acknowledged_at::text FROM character_restore_notices")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(before, after);
    assert!(matches!(
        restore
            .notice(NOTICE.replace("nas-2026-11-05.base", "changed").as_bytes())
            .await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        restore.notice(notice(2, 4).as_bytes()).await,
        Err(Error::Conflict)
    ));
    assert_eq!(restore.queue().await.unwrap().len(), 2);
    cleanup(pool, admin, schema).await;
}
#[tokio::test]
async fn db_reopens_receipts_preserves_history_and_queues_reversed_outcome() {
    let Some((pool, admin, schema)) = database().await else {
        return;
    };
    seed(&pool).await;
    let app = app(pool.clone());
    assert_eq!(
        response(app.clone(), RECEIPT_PATH, &receipt(3), Some(2))
            .await
            .0,
        StatusCode::OK
    );
    let expected = wire::status(wire::decode(NOTICE.as_bytes()).unwrap().id, 1, 1);
    assert_eq!(
        response(app.clone(), PATH, NOTICE, Some(1)).await,
        (StatusCode::OK, expected.clone())
    );
    assert_eq!(
        response(app.clone(), PATH, NOTICE, Some(1)).await,
        (StatusCode::OK, expected)
    );
    let commands = Commands::new(pool.clone(), command_settings());
    assert!(
        commands
            .status(Uuid::parse_str(OP).unwrap())
            .await
            .unwrap()
            .receipt_json
            .is_none()
    );
    assert_eq!(
        response(app.clone(), RECEIPT_PATH, &receipt(3), Some(2))
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        response(
            app.clone(),
            RECEIPT_PATH,
            &receipt(4).replace("Aldric", "Baldric"),
            Some(2)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let rejected = include_str!("../commands/fixtures/rejected-receipt.json")
        .replace("oteryn:character-authority:primary", "oteryn-game-eu1")
        .replace(
            "\"recovery_generation\":\"3\"",
            "\"recovery_generation\":\"4\"",
        );
    for _ in 0..2 {
        assert_eq!(
            response(app.clone(), RECEIPT_PATH, &rejected, Some(2))
                .await
                .0,
            StatusCode::OK
        );
    }
    assert_eq!(
        response(app, PATH, NOTICE, Some(1)).await,
        (
            StatusCode::OK,
            wire::status(wire::decode(NOTICE.as_bytes()).unwrap().id, 1, 0)
        )
    );
    let rows=sqlx::query("SELECT receipt_json,superseded_at IS NOT NULL AS superseded FROM character_restore_receipts ORDER BY id").fetch_all(&pool).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get::<String, _>("receipt_json"), receipt(3));
    assert!(rows[0].get::<bool, _>("superseded"));
    assert!(!rows[1].get::<bool, _>("superseded"));
    let queue = Restore::new(pool.clone(), settings())
        .queue()
        .await
        .unwrap();
    assert!(queue.iter().any(
        |q| q.reason == "character_authority.restore_outcome_reversed" && q.account_id.is_some()
    ));
    assert!(
        queue
            .iter()
            .any(|q| q.reason == "character_authority.command_receipt_lost_to_restore")
    );
    cleanup(pool, admin, schema).await;
}
#[tokio::test]
async fn db_chained_restores_carry_original_receipt_until_new_decision() {
    let Some((pool, admin, schema)) = database().await else {
        return;
    };
    seed(&pool).await;
    let app = app(pool.clone());
    assert_eq!(
        response(app.clone(), RECEIPT_PATH, &receipt(3), Some(2))
            .await
            .0,
        StatusCode::OK
    );
    let notices = [NOTICE.to_owned(), notice(2, 5), notice(5, 6)];
    for raw in &notices {
        let event = wire::decode(raw.as_bytes()).unwrap().id;
        assert_eq!(
            response(app.clone(), PATH, raw, Some(1)).await,
            (StatusCode::OK, wire::status(event, 1, 1))
        );
    }
    let distinct: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT superseded_receipt_id) FROM character_restore_reopens",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(distinct, 1);
    assert_eq!(
        response(app.clone(), RECEIPT_PATH, &receipt(5), Some(2))
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        response(app.clone(), RECEIPT_PATH, &receipt(6), Some(2))
            .await
            .0,
        StatusCode::OK
    );
    for raw in &notices {
        let event = wire::decode(raw.as_bytes()).unwrap().id;
        assert_eq!(
            response(app.clone(), PATH, raw, Some(1)).await,
            (StatusCode::OK, wire::status(event, 1, 0))
        );
    }
    cleanup(pool, admin, schema).await;
}
#[tokio::test]
async fn db_selection_is_numeric_inclusive_and_ignores_restore_time() {
    let Some((pool, admin, schema)) = database().await else {
        return;
    };
    seed(&pool).await;
    let app = app(pool.clone());
    assert_eq!(
        response(app.clone(), RECEIPT_PATH, &receipt(9), Some(2))
            .await
            .0,
        StatusCode::OK
    );
    let raw = notice(10, 11).replace("1790000000", "1");
    assert_eq!(
        response(app.clone(), PATH, &raw, Some(1)).await.1,
        wire::status(wire::decode(raw.as_bytes()).unwrap().id, 0, 0)
    );
    // A decision below H stays current, including exact republication below the floor.
    assert_eq!(
        response(app.clone(), RECEIPT_PATH, &receipt(9), Some(2))
            .await
            .0,
        StatusCode::OK
    );
    let raw = notice(9, 12).replace("1790000000", "18446744073709551615");
    assert_eq!(
        response(app.clone(), PATH, &raw, Some(1)).await.1,
        wire::status(wire::decode(raw.as_bytes()).unwrap().id, 1, 1)
    );
    assert_eq!(
        response(app, RECEIPT_PATH, &receipt(12), Some(2)).await.0,
        StatusCode::OK
    );
    cleanup(pool, admin, schema).await;
}
#[tokio::test]
async fn db_unavailable_exchange_rolls_back_but_empty_notices_acknowledge() {
    let Some((pool, admin, schema)) = database().await else {
        return;
    };
    let mut s = settings();
    s.exchange_available = false;
    let restore = Restore::new(pool.clone(), s);
    assert!(
        restore
            .notice(NOTICE.as_bytes())
            .await
            .unwrap()
            .contains("ACKNOWLEDGED")
    );
    seed(&pool).await;
    assert_eq!(
        response(app(pool.clone()), RECEIPT_PATH, &receipt(4), Some(2))
            .await
            .0,
        StatusCode::OK
    );
    let raw = notice(4, 5);
    assert!(matches!(
        restore.notice(raw.as_bytes()).await,
        Err(Error::Unavailable)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM character_restore_notices")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    assert!(
        Commands::new(pool.clone(), command_settings())
            .status(Uuid::parse_str(OP).unwrap())
            .await
            .unwrap()
            .receipt_json
            .is_some()
    );
    let missing = router(
        Restore::new(pool.clone(), settings()),
        command_settings(),
        identities(false),
    );
    assert_eq!(
        response(missing, PATH, &raw, Some(1)).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(restore.queue().await.unwrap().len(), 1);
    cleanup(pool, admin, schema).await;
}
#[tokio::test]
async fn db_unknown_history_is_denied_and_queued_without_notice() {
    let Some((pool, admin, schema)) = database().await else {
        return;
    };
    seed(&pool).await;
    let app = app(pool.clone());
    assert_eq!(
        response(app.clone(), RECEIPT_PATH, &receipt(3), Some(2))
            .await
            .0,
        StatusCode::OK
    );
    sqlx::query("DELETE FROM character_restore_receipts")
        .execute(&pool)
        .await
        .unwrap();
    for _ in 0..2 {
        assert_eq!(
            response(app.clone(), PATH, NOTICE, Some(1)).await.0,
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
    let restore = Restore::new(pool.clone(), settings());
    let queued = restore.queue().await.unwrap();
    assert_eq!(queued.len(), 1);
    assert_eq!(
        queued[0].reason,
        "character_authority.restore_state_ambiguous"
    );
    assert!(queued[0].account_id.is_some());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM character_restore_notices")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    cleanup(pool, admin, schema).await;
}
#[tokio::test]
async fn db_recovery_exchange_does_not_enable_issuance_and_rates_are_per_identity() {
    let Some((pool, admin, schema)) = database().await else {
        return;
    };
    seed(&pool).await;
    let mut cs = command_settings();
    cs.enabled = false;
    let commands = Commands::new(pool.clone(), cs.clone());
    struct Accounts;
    impl AccountStateReader for Accounts {
        async fn state(&self, _: Uuid) -> AccountState {
            AccountState::Unknown
        }
    }
    assert!(matches!(
        commands
            .enqueue(
                &Accounts,
                Uuid::now_v7(),
                "new",
                Intent::Create {
                    account_id: Uuid::now_v7(),
                    world_id: Uuid::now_v7(),
                    name: "Aldric".into()
                }
            )
            .await,
        Err(commands::Error::Unavailable)
    ));
    let mut s = settings();
    s.requests_per_minute = 1;
    let map = Arc::new(
        IdentityMap::new(vec![
            Binding {
                identity: Identity::spki_sha256([1; 32]),
                group: GROUP.into(),
            },
            Binding {
                identity: Identity::spki_sha256([4; 32]),
                group: GROUP.into(),
            },
            Binding {
                identity: Identity::spki_sha256([2; 32]),
                group: commands::GROUP.into(),
            },
        ])
        .unwrap(),
    );
    let app = router(Restore::new(pool.clone(), s), cs, map);
    assert_eq!(
        response(app.clone(), RECEIPT_PATH, &receipt(3), Some(2))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        response(app.clone(), PATH, NOTICE, Some(1)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        response(app.clone(), PATH, NOTICE, Some(1)).await,
        (StatusCode::TOO_MANY_REQUESTS, String::new())
    );
    assert_eq!(response(app, PATH, NOTICE, Some(4)).await.0, StatusCode::OK);
    cleanup(pool, admin, schema).await;
}
#[tokio::test]
async fn db_concurrent_receipt_and_notice_never_lose_a_reopen() {
    let Some((pool, admin, schema)) = database().await else {
        return;
    };
    seed(&pool).await;
    let app = app(pool.clone());
    let raw_receipt = receipt(3);
    let (receipt_result, notice_result) = tokio::join!(
        response(app.clone(), RECEIPT_PATH, &raw_receipt, Some(2)),
        response(app.clone(), PATH, NOTICE, Some(1))
    );
    assert_eq!(notice_result.0, StatusCode::OK);
    assert!(matches!(
        receipt_result.0,
        StatusCode::OK | StatusCode::CONFLICT
    ));
    let row=sqlx::query("SELECT c.state,c.receipt_json,(SELECT count(*) FROM character_restore_reopens WHERE new_receipt_id IS NULL) AS outstanding FROM character_commands c").fetch_one(&pool).await.unwrap();
    assert_eq!(row.get::<String, _>("state"), "PENDING");
    assert!(row.get::<Option<String>, _>("receipt_json").is_none());
    assert_eq!(
        row.get::<i64, _>("outstanding"),
        if receipt_result.0 == StatusCode::OK {
            1
        } else {
            0
        }
    );
    assert_eq!(
        response(app, RECEIPT_PATH, &receipt(4), Some(2)).await.0,
        StatusCode::OK
    );
    cleanup(pool, admin, schema).await;
}
#[tokio::test]
async fn db_migration_preserves_existing_receipts_and_reopens_them() {
    let Some((pool, admin, schema)) = database_at(false).await else {
        return;
    };
    seed(&pool).await;
    let legacy = commands::router(
        Commands::new(pool.clone(), command_settings()),
        identities(true),
    );
    assert_eq!(
        response(legacy, RECEIPT_PATH, &receipt(3), Some(2)).await.0,
        StatusCode::OK
    );
    sqlx::raw_sql(MIGRATION).execute(&pool).await.unwrap();
    let app = app(pool.clone());
    assert_eq!(
        response(app.clone(), PATH, NOTICE, Some(1)).await.1,
        wire::status(wire::decode(NOTICE.as_bytes()).unwrap().id, 1, 1)
    );
    assert_eq!(
        response(app, RECEIPT_PATH, &receipt(4), Some(2)).await.0,
        StatusCode::OK
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM character_restore_receipts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
    cleanup(pool, admin, schema).await;
}
#[tokio::test]
async fn db_invalid_legacy_generation_is_queued_instead_of_guessed() {
    let Some((pool, admin, schema)) = database_at(false).await else {
        return;
    };
    seed(&pool).await;
    sqlx::query("UPDATE character_commands SET receipt_json=$1,state='TERMINAL'")
        .bind(receipt(3).replace(
            "\"recovery_generation\":\"3\"",
            "\"recovery_generation\":\"03\"",
        ))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(MIGRATION).execute(&pool).await.unwrap();
    let restore = Restore::new(pool.clone(), settings());
    assert!(matches!(
        restore.notice(NOTICE.as_bytes()).await,
        Err(Error::Unavailable)
    ));
    assert_eq!(
        restore.queue().await.unwrap()[0].reason,
        "character_authority.restore_state_ambiguous"
    );
    cleanup(pool, admin, schema).await;
}
