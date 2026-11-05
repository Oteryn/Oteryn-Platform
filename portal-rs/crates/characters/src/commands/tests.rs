#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
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
const AUTHORITY: &str = "oteryn:character-authority:primary";
const OP: &str = "0192a000-0000-7000-8000-000000000001";
const ACCOUNT: &str = "0190f2a1-3b4c-7d5e-8f60-718293a4b5c6";
const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
const DECISION: &str = "0192a000-0000-7000-8000-0000000000a1";
const LIST: &str = include_str!("fixtures/pending-request.json");
const READ: &str = include_str!("fixtures/read-request.json");
const RECEIPT: &str = include_str!("fixtures/create-receipt.json");
const REJECTED: &str = include_str!("fixtures/rejected-receipt.json");
fn settings() -> Settings {
    Settings {
        enabled: true,
        create_enabled: true,
        transfer_enabled: true,
        source_authority: AUTHORITY.into(),
        ..Settings::default()
    }
}
fn create() -> Intent {
    Intent::Create {
        account_id: Uuid::parse_str(ACCOUNT).unwrap(),
        world_id: Uuid::parse_str(WORLD).unwrap(),
        name: "Aldric".into(),
    }
}
#[derive(Debug)]
struct Accounts(AccountState);
impl AccountStateReader for Accounts {
    async fn state(&self, _: Uuid) -> AccountState {
        self.0
    }
}
fn identities() -> Arc<IdentityMap> {
    Arc::new(
        IdentityMap::new(vec![
            Binding {
                identity: Identity::spki_sha256([1; 32]),
                group: GROUP.into(),
            },
            Binding {
                identity: Identity::spki_sha256([2; 32]),
                group: "other".into(),
            },
        ])
        .unwrap(),
    )
}
fn request(path: &str, body: &str, peer: Option<u8>) -> Request<Body> {
    let mut r = Request::post(format!("/internal/v1/game-auth/character-commands/{path}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    if let Some(peer) = peer {
        r.extensions_mut()
            .insert(VerifiedClient::from_verified_tls13_http11(
                Identity::spki_sha256([peer; 32]),
            ));
    }
    r
}
async fn response(
    app: axum::Router,
    path: &str,
    body: &str,
    peer: Option<u8>,
) -> (StatusCode, String) {
    let r = app.oneshot(request(path, body, peer)).await.unwrap();
    let status = r.status();
    assert_eq!(r.headers()["cache-control"], "no-store, private");
    (
        status,
        String::from_utf8(to_bytes(r.into_body(), 8192).await.unwrap().to_vec()).unwrap(),
    )
}
async fn database() -> Option<(PgPool, PgPool, String)> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED DB test: PORTAL_TEST_DATABASE_URL is unset");
        return None;
    };
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    let schema = format!("t22_{}", Uuid::now_v7().simple());
    // Schema identifier contains only a fixed prefix and generated UUID hex.
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await
        .unwrap();
    let search = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(8)
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
async fn seed(commands: &Commands, fixture: &str) {
    let v = wire::decode(fixture.as_bytes(), 4096).unwrap();
    let fields = if wire::get(&v, "command").unwrap() == CREATE {
        ["account_id", "target_world_id", "requested_name"].as_slice()
    } else {
        [
            "character_id",
            "expected_from_account_id",
            "to_account_id",
            "purpose",
        ]
        .as_slice()
    };
    let binding = wire::object(
        fields
            .iter()
            .map(|k| (*k, v.get(k).unwrap().clone()))
            .collect(),
    );
    sqlx::query("INSERT INTO character_commands (id,source_revision,issuer_decision_id,request_key,command,account_id,target_account_id,binding_json,intent_json) OVERRIDING SYSTEM VALUE VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)")
        .bind(wire::uuid(wire::get(&v,"operation_id").unwrap()).unwrap()).bind(wire::get(&v,"source_revision").unwrap().parse::<i64>().unwrap())
        .bind(wire::uuid(wire::get(&v,"issuer_decision_id").unwrap()).unwrap()).bind(wire::get(&v,"operation_id").unwrap()).bind(wire::get(&v,"command").unwrap())
        .bind(wire::uuid(wire::get(&binding,"account_id").or_else(|_| wire::get(&binding,"expected_from_account_id")).unwrap()).unwrap())
        .bind(binding.get("to_account_id").map(|_| wire::uuid(wire::get(&binding,"to_account_id").unwrap()).unwrap()))
        .bind(wire::encode(&binding).unwrap()).bind(fixture).execute(&commands.pool).await.unwrap();
}
#[test]
fn game_golden_vectors_round_trip_and_match_intent_encoder() {
    let vectors = fixtures::load(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/fixtures"
    ))
    .unwrap();
    assert_eq!(vectors.len(), 9);
    for f in vectors {
        assert!(f.was_canonical);
        assert_eq!(canonical::encode(&f.value).unwrap(), f.original);
    }
    let raw = wire::intent(
        CREATE,
        Uuid::parse_str(DECISION).unwrap(),
        17,
        Uuid::parse_str(OP).unwrap(),
        wire::binding(&create()),
        1790000000,
        1790000300,
    )
    .unwrap();
    assert_eq!(raw, include_str!("fixtures/create-intent.json"));
    let transfer = Intent::Transfer {
        character_id: Uuid::parse_str("01934f10-7c04-7001-805b-3b1122334401").unwrap(),
        from_account_id: Uuid::parse_str(ACCOUNT).unwrap(),
        to_account_id: Uuid::parse_str("0190f2a1-3b4c-7d5e-8f60-718293a4b5c7").unwrap(),
        purpose: TransferPurpose::Settlement,
    };
    let encoded = wire::intent(
        TRANSFER,
        Uuid::parse_str("0192a000-0000-7000-8000-0000000000b1").unwrap(),
        18,
        Uuid::parse_str("0192a000-0000-7000-8000-000000000002").unwrap(),
        wire::binding(&transfer),
        1790000000,
        1790000300,
    )
    .unwrap();
    assert_eq!(encoded, include_str!("fixtures/transfer-intent.json"));
    assert!(wire::receipt(RECEIPT.as_bytes()).is_ok());
    assert!(wire::receipt(REJECTED.as_bytes()).is_ok());
    assert!(wire::receipt(include_bytes!("fixtures/transfer-receipt.json")).is_ok());
}
#[test]
fn malformed_wire_and_cross_command_codes_are_refused() {
    for raw in [
        RECEIPT.replace("\"1\"", "\"01\""),
        RECEIPT.replace("Aldric", "A1"),
        RECEIPT.replace("COMMITTED", "PENDING"),
        REJECTED.replace("CHAR_CMD_NAME_UNAVAILABLE", "CHAR_CMD_OWNER_MISMATCH"),
        RECEIPT.replace("0190f2a1", "0190F2A1"),
        RECEIPT.replace("1790000002", "18446744073709551616"),
        RECEIPT.replace(",\"command\"", ", \"command\""),
        format!("{RECEIPT}\n"),
        RECEIPT.replace("Aldric", "Ald\\u0072ic"),
    ] {
        assert!(wire::receipt(raw.as_bytes()).is_err(), "{raw}");
    }
    assert!(wire::counter("18446744073709551615", false).is_ok());
    assert!(wire::counter("0", false).is_err());
    let mut invalid = create();
    if let Intent::Create { name, .. } = &mut invalid {
        *name = " A".into();
    }
    assert!(invalid.validate().is_err());
}
#[tokio::test]
async fn guard_denies_wrong_identity_disabled_surface_and_bad_input() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unreachable")
        .unwrap();
    let service = Commands::new(pool.clone(), settings());
    let app = router(service, identities());
    for peer in [None, Some(2), Some(3)] {
        assert_eq!(
            response(app.clone(), "pending", LIST, peer).await,
            (StatusCode::UNAUTHORIZED, String::new())
        );
    }
    for raw in [
        LIST.replace("max_entries\":32", "max_entries\":33"),
        format!("{LIST}\n"),
        LIST.replace("\"0\"", "\"00\""),
        LIST.replace("ListPendingCharacterCommandsV1", "Unknown"),
    ] {
        assert_eq!(
            response(app.clone(), "pending", &raw, Some(1)).await,
            (StatusCode::BAD_REQUEST, String::new())
        );
    }
    assert_eq!(
        response(app.clone(), "pending", &"x".repeat(513), Some(1))
            .await
            .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        response(
            app,
            "pending",
            &LIST.replace(AUTHORITY, "another:authority"),
            Some(1)
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let disabled = router(Commands::new(pool, Settings::default()), identities());
    assert_eq!(
        response(disabled, "pending", LIST, Some(1)).await,
        (StatusCode::SERVICE_UNAVAILABLE, String::new())
    );
}
#[tokio::test]
async fn account_state_and_invalid_input_deny_before_storage() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unreachable")
        .unwrap();
    let c = Commands::new(pool, settings());
    let decision = Uuid::now_v7();
    for state in [
        AccountState::Denied,
        AccountState::Unknown,
        AccountState::Ambiguous,
    ] {
        assert!(matches!(
            c.enqueue(&Accounts(state), decision, "key", create()).await,
            Err(Error::Denied)
        ));
    }
    assert!(matches!(
        c.enqueue(&Accounts(AccountState::Eligible), decision, "", create())
            .await,
        Err(Error::BadInput)
    ));
    assert!(matches!(
        c.enqueue(
            &Accounts(AccountState::Eligible),
            Uuid::nil(),
            "key",
            create()
        )
        .await,
        Err(Error::BadInput)
    ));
}
#[tokio::test]
async fn db_golden_exchange_replay_conflicts_and_unknown_operations() {
    let Some((pool, admin, schema)) = database().await else {
        return;
    };
    let c = Commands::new(pool.clone(), settings());
    seed(&c, include_str!("fixtures/create-intent.json")).await;
    let app = router(c.clone(), identities());
    assert_eq!(
        response(app.clone(), "pending", LIST, Some(1)).await,
        (
            StatusCode::OK,
            include_str!("fixtures/pending-response.json").into()
        )
    );
    assert_eq!(
        response(app.clone(), "read", READ, Some(1)).await,
        (
            StatusCode::OK,
            include_str!("fixtures/create-intent.json").into()
        )
    );
    assert_eq!(
        response(
            app.clone(),
            "read",
            &READ.replace(CREATE, "RenameCharacter"),
            Some(1)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        response(
            app.clone(),
            "read",
            &READ.replace(OP, &Uuid::now_v7().to_string()),
            Some(1)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        response(
            app.clone(),
            "receipt",
            &RECEIPT.replace("Aldric", "Baldric"),
            Some(1)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    for _ in 0..2 {
        assert_eq!(
            response(app.clone(), "receipt", RECEIPT, Some(1)).await,
            (
                StatusCode::OK,
                "{\"contract_version\":1,\"result\":\"accepted\"}".into()
            )
        );
    }
    let updated: String =
        sqlx::query_scalar("SELECT updated_at::text FROM character_commands WHERE id=$1")
            .bind(Uuid::parse_str(OP).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    c.publish(RECEIPT.as_bytes()).await.unwrap();
    let after: String =
        sqlx::query_scalar("SELECT updated_at::text FROM character_commands WHERE id=$1")
            .bind(Uuid::parse_str(OP).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(updated, after);
    assert_eq!(
        response(app.clone(), "receipt", REJECTED, Some(1)).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        response(
            app.clone(),
            "receipt",
            &RECEIPT.replace(OP, &Uuid::now_v7().to_string()),
            Some(1)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        c.pending(LIST.as_bytes()).await.unwrap(),
        "{\"contract_version\":1,\"pending\":[]}"
    );
    assert_eq!(
        c.read(READ.as_bytes()).await.unwrap(),
        include_str!("fixtures/create-intent.json")
    );
    seed(&c, include_str!("fixtures/transfer-intent.json")).await;
    c.publish(include_bytes!("fixtures/transfer-receipt.json"))
        .await
        .unwrap();
    assert_eq!(
        c.status(Uuid::parse_str(OP).unwrap())
            .await
            .unwrap()
            .receipt_json
            .as_deref(),
        Some(RECEIPT)
    );
    cleanup(pool, admin, schema).await;
}
#[tokio::test]
async fn db_enqueue_retries_paging_and_backlog_are_serialized() {
    let Some((pool, admin, schema)) = database().await else {
        return;
    };
    let mut s = settings();
    s.max_pending = 2;
    let c = Commands::new(pool.clone(), s);
    let accounts = Accounts(AccountState::Eligible);
    let decision = Uuid::now_v7();
    let (a, b) = tokio::join!(
        c.enqueue(&accounts, decision, "same", create()),
        c.enqueue(&accounts, decision, "same", create())
    );
    let a = a.unwrap();
    assert_eq!(a, b.unwrap());
    let mut changed = create();
    if let Intent::Create { name, .. } = &mut changed {
        *name = "Baldric".into();
    }
    assert!(matches!(
        c.enqueue(&accounts, decision, "same", changed).await,
        Err(Error::Conflict)
    ));
    let second = c
        .enqueue(&accounts, decision, "second", create())
        .await
        .unwrap();
    assert_ne!(a.operation_id, second.operation_id);
    assert!(matches!(
        c.enqueue(&accounts, decision, "third", create()).await,
        Err(Error::BacklogFull)
    ));
    let first_page = c
        .pending(
            LIST.replace("max_entries\":32", "max_entries\":1")
                .as_bytes(),
        )
        .await
        .unwrap();
    let page = wire::decode(first_page.as_bytes(), 8192).unwrap();
    let canonical::Value::Array(entries) = page.get("pending").unwrap() else {
        panic!("pending entries");
    };
    let rev = wire::get(&entries[0], "source_revision").unwrap();
    let cursor = LIST
        .replace(
            "\"after_source_revision\":\"0\"",
            &format!("\"after_source_revision\":\"{rev}\""),
        )
        .replace(
            "00000000-0000-0000-0000-000000000000",
            &a.operation_id.to_string(),
        );
    let next = c.pending(cursor.as_bytes()).await.unwrap();
    assert!(next.contains(&second.operation_id.to_string()));
    assert!(!next.contains(&a.operation_id.to_string()));
    assert_eq!(
        c.pending(LIST.replace("\"0\"", "\"18446744073709551615\"").as_bytes())
            .await
            .unwrap(),
        "{\"contract_version\":1,\"pending\":[]}"
    );
    let rejected = REJECTED.replace(OP, &a.operation_id.to_string());
    c.publish(rejected.as_bytes()).await.unwrap();
    c.publish(rejected.as_bytes()).await.unwrap();
    assert_eq!(
        c.enqueue(&accounts, decision, "same", create())
            .await
            .unwrap()
            .operation_id,
        a.operation_id
    );
    assert!(
        c.enqueue(&accounts, decision, "third", create())
            .await
            .is_ok()
    );
    cleanup(pool, admin, schema).await;
}
#[tokio::test]
async fn db_transfer_enqueue_checks_both_accounts_and_sequence_regression() {
    let Some((pool, admin, schema)) = database().await else {
        return;
    };
    let c = Commands::new(pool.clone(), settings());
    let from = Uuid::now_v7();
    let to = Uuid::now_v7();
    let decision = Uuid::now_v7();
    let transfer = Intent::Transfer {
        character_id: Uuid::now_v7(),
        from_account_id: from,
        to_account_id: to,
        purpose: TransferPurpose::Settlement,
    };
    #[derive(Debug)]
    struct SourceOnly(Uuid);
    impl AccountStateReader for SourceOnly {
        async fn state(&self, id: Uuid) -> AccountState {
            if id == self.0 {
                AccountState::Eligible
            } else {
                AccountState::Unknown
            }
        }
    }
    assert!(matches!(
        c.enqueue(&SourceOnly(from), decision, "transfer", transfer.clone())
            .await,
        Err(Error::Denied)
    ));
    let issued = c
        .enqueue(
            &Accounts(AccountState::Eligible),
            decision,
            "transfer",
            transfer,
        )
        .await
        .unwrap();
    assert!(issued.intent_json.contains("BAZAAR_SETTLEMENT"));
    seed(&c, include_str!("fixtures/create-intent.json")).await;
    sqlx::query("ALTER TABLE character_commands ALTER COLUMN source_revision RESTART WITH 2")
        .execute(&pool)
        .await
        .unwrap();
    // A restored sequence that collides or falls behind must never create a listed intent.
    assert!(matches!(
        c.enqueue(
            &Accounts(AccountState::Eligible),
            decision,
            "create",
            create()
        )
        .await,
        Err(Error::Unavailable)
    ));
    let row = sqlx::query("SELECT count(*) AS count FROM character_commands")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<i64, _>("count"), 2);
    cleanup(pool, admin, schema).await;
}

#[tokio::test]
async fn invalid_configuration_and_disabled_profiles_fail_closed() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unreachable")
        .unwrap();
    for s in [
        Settings {
            max_pending: 41,
            ..settings()
        },
        Settings {
            ttl_seconds: 301,
            ..settings()
        },
        Settings {
            requests_per_minute: 0,
            ..settings()
        },
        Settings {
            source_authority: "bad authority".into(),
            ..settings()
        },
    ] {
        let app = router(Commands::new(pool.clone(), s), identities());
        assert_eq!(
            response(app, "pending", LIST, Some(1)).await.0,
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
    let c = Commands::new(
        pool.clone(),
        Settings {
            create_enabled: false,
            ..settings()
        },
    );
    assert!(matches!(
        c.enqueue(
            &Accounts(AccountState::Eligible),
            Uuid::now_v7(),
            "key",
            create()
        )
        .await,
        Err(Error::Unavailable)
    ));
    let c = Commands::new(
        pool,
        Settings {
            requests_per_minute: 1,
            ..settings()
        },
    );
    let app = router(c, identities());
    assert_eq!(
        response(app.clone(), "pending", "{}", Some(1)).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        response(app, "read", READ, Some(1)).await,
        (StatusCode::TOO_MANY_REQUESTS, String::new())
    );
}
