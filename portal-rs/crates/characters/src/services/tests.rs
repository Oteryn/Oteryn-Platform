#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod unit {
    use super::super::*;
    #[test]
    fn flags_default_off_and_schedule_requires_cancellation() {
        let mut s = Settings::default();
        assert!(!s.permits(&Action::Schedule));
        assert!(!s.permits(&Action::Rename {
            requested_name: "Aldric".into()
        }));
        s.enabled = true;
        s.schedule_deletion = true;
        s.projection_enabled = true;
        s.exchange_enabled = true;
        s.command_authority = "commands".into();
        s.projection_authority = "services".into();
        s.clock_uncertainty_seconds = Some(0);
        s.freshness_seconds = Some(30);
        assert!(!s.permits(&Action::Schedule));
        s.cancel_deletion = true;
        assert!(s.permits(&Action::Schedule));
        s.schedule_deletion = false;
        assert!(s.permits(&Action::Cancel {
            deletion_operation_id: Uuid::now_v7()
        }));
    }
}
use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request as HttpRequest, StatusCode},
};
use oteryn_internal_api::guard::{Binding, Identity, IdentityMap, VerifiedClient};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower::ServiceExt;
const COMMAND_AUTHORITY: &str = "oteryn:character-authority:primary";
const PROJECTION_AUTHORITY: &str = "oteryn:character-services:primary";
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED PostgreSQL T46 test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = PgPool::connect(&url).await.unwrap();
        let schema = format!("t46_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let search = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .after_connect(move |c, _| {
                let search = search.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(search)
                        .execute(&mut *c)
                        .await?;
                    sqlx::query("SET statement_timeout='10s'")
                        .execute(c)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        for raw in [
            include_str!("../../../../migrations/20261105145653_t18_account_characters.sql"),
            include_str!("../../../../migrations/20261105151420_t26_wallet.sql"),
            include_str!("../../../../migrations/20261105160001_t22_character_commands.sql"),
            include_str!("../../../../migrations/20261105160020_t23_restore_reconciliation.sql"),
            include_str!("../../../../migrations/20261105231501_t46_character_services.sql"),
            "CREATE TABLE accounts(id uuid PRIMARY KEY,status text NOT NULL)",
        ] {
            sqlx::raw_sql(raw).execute(&pool).await.unwrap();
        }
        Some(Self {
            pool,
            admin,
            schema,
        })
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await
        .unwrap();
        self.admin.close().await;
    }
}
fn settings() -> Settings {
    Settings {
        enabled: true,
        schedule_deletion: true,
        cancel_deletion: true,
        rename: true,
        projection_enabled: true,
        exchange_enabled: true,
        wallet_enabled: true,
        command_authority: COMMAND_AUTHORITY.into(),
        projection_authority: PROJECTION_AUTHORITY.into(),
        clock_uncertainty_seconds: Some(0),
        freshness_seconds: Some(30),
        rename_price: Some(Price {
            product_id: "character.rename".into(),
            version: 1,
            coins: 70,
        }),
    }
}
async fn current(pool: &PgPool) -> u64 {
    let n: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
        .fetch_one(pool)
        .await
        .unwrap();
    n as u64
}
fn snapshot(
    account: Uuid,
    character: Uuid,
    revision: u64,
    cr: u64,
    lifecycle: &str,
    deletion: Option<(Uuid, u64, u64)>,
    at: u64,
) -> String {
    let (id, start, end) = deletion
        .map(|(id, s, e)| (format!("\"{id}\""), format!("\"{s}\""), format!("\"{e}\"")))
        .unwrap_or(("null".into(), "null".into(), "null".into()));
    format!(
        "{{\"contract_version\":2,\"operation\":\"PublishCharacterServiceStateV2\",\"source_authority\":\"{PROJECTION_AUTHORITY}\",\"account_id\":\"{account}\",\"character_id\":\"{character}\",\"projection_epoch\":\"1\",\"projection_revision\":\"{revision}\",\"source_observed_at\":\"{at}\",\"character_revision\":\"{cr}\",\"world_id\":\"01934f10-7c02-7001-805b-3b1122334401\",\"name\":\"Aldric\",\"sex\":null,\"lifecycle\":\"{lifecycle}\",\"deletion_operation_id\":{id},\"deletion_scheduled_at\":{start},\"deletion_effective_at\":{end}}}"
    )
}
async fn seed(s: &Services, account: Uuid, character: Uuid, coins: i64) {
    sqlx::query("INSERT INTO accounts VALUES ($1,'active') ON CONFLICT DO NOTHING")
        .bind(account)
        .execute(&s.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='lcfa',complete_through=floor(extract(epoch FROM clock_timestamp())),observed_at=floor(extract(epoch FROM clock_timestamp()))").execute(&s.pool).await.unwrap();
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES ($1,'lcfa','1','1',1,$2) ON CONFLICT DO NOTHING").bind(account).bind(vec![0u8;32]).execute(&s.pool).await.unwrap();
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES ($1,$2,$3,'Aldric','AVAILABLE')").bind(account).bind(character).bind(Uuid::now_v7()).execute(&s.pool).await.unwrap();
    let at = current(&s.pool).await;
    s.ingest(
        snapshot(account, character, 1, 8, "ACTIVE", None, at).as_bytes(),
        false,
    )
    .await
    .unwrap();
    s.ingest(format!("{{\"contract_version\":2,\"operation\":\"PublishCharacterServiceWatermarkV2\",\"source_authority\":\"{PROJECTION_AUTHORITY}\",\"projection_epoch\":\"1\",\"complete_through\":\"{at}\",\"observed_at\":\"{at}\"}}").as_bytes(),true).await.unwrap();
    sqlx::query("INSERT INTO wallet_accounts(account_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(account)
        .execute(&s.pool)
        .await
        .unwrap();
    if coins > 0 {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES ($1,'adjustment',$2,$3)").bind(id).bind(account).bind(coins).execute(&s.pool).await.unwrap();
        sqlx::query("INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta) VALUES ($1,$2,$3,$4,0)").bind(Uuid::now_v7()).bind(id).bind(account).bind(coins).execute(&s.pool).await.unwrap();
    }
}
fn rename(character: Uuid) -> super::Request {
    super::Request {
        accepted_price: Some(70),
        request_id: Uuid::now_v7(),
        character_id: character,
        action: Action::Rename {
            requested_name: "Aldric Vale".into(),
        },
    }
}
fn receipt(c: &commands::CommandStatus, committed: bool, generation: u64) -> String {
    let v = wire::decode_value(c.intent_json.as_bytes(), 4096).unwrap();
    let cmd = wire::string(&v, "command").unwrap();
    let code = if committed {
        "CHAR_CMD_OK"
    } else {
        "CHAR_CMD_INTENT_EXPIRED"
    };
    let outcome = if committed { "COMMITTED" } else { "REJECTED" };
    let mut raw = format!(
        "{{\"contract_version\":2,\"operation\":\"PublishCharacterCommandReceiptV2\",\"source_authority\":\"{COMMAND_AUTHORITY}\",\"command\":\"{cmd}\",\"operation_id\":\"{}\",\"outcome\":\"{outcome}\",\"result_code\":\"{code}\"",
        c.operation_id
    );
    if committed {
        let account = wire::string(&v, "account_id").unwrap();
        let character = wire::string(&v, "character_id").unwrap();
        let cr = wire::number(
            wire::string(&v, "expected_character_revision").unwrap(),
            false,
        )
        .unwrap()
            + 1;
        raw += &format!(
            ",\"character_id\":\"{character}\",\"account_id\":\"{account}\",\"character_revision\":\"{cr}\",\"projection_epoch\":\"1\",\"projection_revision\":\"2\""
        );
        if cmd == wire::RENAME {
            raw += ",\"previous_name\":\"Aldric\",\"name\":\"Aldric Vale\"";
        }
        if cmd == wire::SCHEDULE {
            raw += &format!(
                ",\"lifecycle\":\"DELETION_PENDING\",\"deletion_operation_id\":\"{}\",\"deletion_scheduled_at\":\"1790000000\",\"deletion_effective_at\":\"1790000100\"",
                c.operation_id
            );
        }
        if cmd == wire::CANCEL {
            raw += &format!(
                ",\"lifecycle\":\"ACTIVE\",\"deletion_operation_id\":\"{}\"",
                wire::string(&v, "deletion_operation_id").unwrap()
            );
        }
    }
    raw += &format!(",\"recovery_generation\":\"{generation}\",\"decided_at\":\"1790000002\"}}");
    raw
}
async fn balance(s: &Services, account: Uuid) -> (i64, i64) {
    let b = s.wallet().balance(account).await.unwrap();
    (b.available, b.reserved)
}
#[tokio::test]
async fn postgres_reservation_insufficient_funds_and_transaction_rollback() {
    let Some(db) = Database::open().await else {
        return;
    };
    let s = Services::new(db.pool.clone(), settings());
    let a = Uuid::now_v7();
    let c = Uuid::now_v7();
    seed(&s, a, c, 60).await;
    assert!(matches!(s.request(a, rename(c)).await, Err(Error::Denied)));
    assert_eq!(balance(&s, a).await, (60, 0));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM character_commands")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        0
    );
    let mut tx = db.pool.begin().await.unwrap();
    s.wallet()
        .reserve_in(&mut tx, Uuid::now_v7(), a, 20)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(balance(&s, a).await, (60, 0));
    db.close().await;
}
#[tokio::test]
async fn postgres_concurrent_reservations_cannot_overspend() {
    let Some(db) = Database::open().await else {
        return;
    };
    let s = Services::new(db.pool.clone(), settings());
    let a = Uuid::now_v7();
    seed(&s, a, Uuid::now_v7(), 100).await;
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let mut tasks = vec![];
    for _ in 0..2 {
        let w = s.wallet();
        let pool = db.pool.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            let mut tx = pool.begin().await.unwrap();
            let result = w.reserve_in(&mut tx, Uuid::now_v7(), a, 70).await;
            match result {
                Ok(_) => {
                    tx.commit().await.unwrap();
                    true
                }
                Err(oteryn_wallet::Error::Refused(oteryn_wallet::Refusal::InsufficientFunds)) => {
                    tx.rollback().await.unwrap();
                    false
                }
                e => panic!("unexpected reservation result: {e:?}"),
            }
        }));
    }
    let mut count = 0;
    for task in tasks {
        count += usize::from(task.await.unwrap());
    }
    assert_eq!(count, 1);
    assert_eq!(balance(&s, a).await, (30, 70));
    db.close().await;
}
#[tokio::test]
async fn postgres_settlement_replay_conflict_and_concurrency() {
    let Some(db) = Database::open().await else {
        return;
    };
    let s = Services::new(db.pool.clone(), settings());
    let a = Uuid::now_v7();
    seed(&s, a, Uuid::now_v7(), 100).await;
    let reservation = Uuid::now_v7();
    let key = Uuid::now_v7();
    let mut tx = db.pool.begin().await.unwrap();
    s.wallet()
        .reserve_in(&mut tx, reservation, a, 70)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let mut tasks = vec![];
    for _ in 0..2 {
        let w = s.wallet();
        let pool = db.pool.clone();
        tasks.push(tokio::spawn(async move {
            let mut tx = pool.begin().await.unwrap();
            let receipt = w
                .settle_in(&mut tx, key, a, reservation, true)
                .await
                .unwrap();
            tx.commit().await.unwrap();
            receipt.replayed
        }));
    }
    let r1 = tasks.remove(0).await.unwrap();
    let r2 = tasks.remove(0).await.unwrap();
    assert_ne!(r1, r2);
    assert_eq!(balance(&s, a).await, (30, 0));
    let mut tx = db.pool.begin().await.unwrap();
    assert!(
        s.wallet()
            .settle_in(&mut tx, key, a, reservation, false)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    assert!(
        s.wallet()
            .settle_in(&mut tx, Uuid::now_v7(), a, reservation, true)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    assert_eq!(balance(&s, a).await, (30, 0));
    db.close().await;
}
#[tokio::test]
async fn postgres_rename_receipt_capture_refusal_release_and_exact_replay() {
    let Some(db) = Database::open().await else {
        return;
    };
    let s = Services::new(db.pool.clone(), settings());
    let a = Uuid::now_v7();
    let c = Uuid::now_v7();
    seed(&s, a, c, 200).await;
    let r = rename(c);
    let cmd = s.request(a, r.clone()).await.unwrap();
    assert_eq!(s.request(a, r).await.unwrap(), cmd);
    assert_eq!(balance(&s, a).await, (130, 70));
    let raw = receipt(&cmd, true, 3);
    s.receive(raw.as_bytes()).await.unwrap();
    s.receive(raw.as_bytes()).await.unwrap();
    assert_eq!(balance(&s, a).await, (130, 0));
    assert!(matches!(
        s.receive(receipt(&cmd, false, 3).as_bytes()).await,
        Err(Error::Conflict)
    ));
    let other = Uuid::now_v7();
    seed(&s, a, other, 0).await;
    let cmd = s.request(a, rename(other)).await.unwrap();
    s.receive(receipt(&cmd, false, 3).as_bytes()).await.unwrap();
    s.receive(receipt(&cmd, false, 3).as_bytes()).await.unwrap();
    assert_eq!(balance(&s, a).await, (130, 0));
    db.close().await;
}
#[tokio::test]
async fn postgres_owner_stale_unknown_flags_and_bad_input_deny() {
    let Some(db) = Database::open().await else {
        return;
    };
    let s = Services::new(db.pool.clone(), settings());
    let a = Uuid::now_v7();
    let b = Uuid::now_v7();
    let c = Uuid::now_v7();
    seed(&s, a, c, 100).await;
    sqlx::query("INSERT INTO accounts VALUES ($1,'active')")
        .bind(b)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(matches!(s.request(b, rename(c)).await, Err(Error::Denied)));
    for action in [
        Action::Schedule,
        Action::Cancel {
            deletion_operation_id: Uuid::now_v7(),
        },
    ] {
        let request = super::Request {
            request_id: Uuid::now_v7(),
            character_id: c,
            action,
            accepted_price: None,
        };
        assert!(matches!(s.request(b, request).await, Err(Error::Denied)));
    }
    let mut quote = rename(c);
    quote.accepted_price = Some(69);
    assert!(matches!(s.request(a, quote).await, Err(Error::Conflict)));
    let mut req = rename(c);
    req.action = Action::Rename {
        requested_name: "Bad  Name".into(),
    };
    assert!(matches!(s.request(a, req).await, Err(Error::BadInput)));
    let mut off = s.clone();
    off.settings.rename = false;
    assert!(matches!(
        off.request(a, rename(c)).await,
        Err(Error::Unavailable)
    ));
    sqlx::query("UPDATE character_service_feed SET complete_through=complete_through-31")
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(matches!(
        s.request(a, rename(c)).await,
        Err(Error::Unavailable)
    ));
    sqlx::query("UPDATE character_service_feed SET complete_through=observed_at")
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE accounts SET status='locked' WHERE id=$1")
        .bind(a)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(matches!(s.request(a, rename(c)).await, Err(Error::Denied)));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM character_commands")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        0
    );
    db.close().await;
}
#[tokio::test]
async fn postgres_schedule_game_deadline_cancel_boundary_and_no_charge() {
    let Some(db) = Database::open().await else {
        return;
    };
    let s = Services::new(db.pool.clone(), settings());
    let a = Uuid::now_v7();
    let c = Uuid::now_v7();
    seed(&s, a, c, 100).await;
    let command = s
        .request(
            a,
            super::Request {
                accepted_price: Some(70),
                request_id: Uuid::now_v7(),
                character_id: c,
                action: Action::Schedule,
            },
        )
        .await
        .unwrap();
    let raw = receipt(&command, true, 3);
    s.receive(raw.as_bytes()).await.unwrap();
    assert_eq!(balance(&s, a).await, (100, 0));
    let at = current(&db.pool).await;
    s.ingest(
        snapshot(
            a,
            c,
            2,
            9,
            "DELETION_PENDING",
            Some((command.operation_id, at - 1, at + 60)),
            at,
        )
        .as_bytes(),
        false,
    )
    .await
    .unwrap();
    let mut draining = s.clone();
    draining.settings.schedule_deletion = false;
    let cancel = draining
        .request(
            a,
            super::Request {
                accepted_price: Some(70),
                request_id: Uuid::now_v7(),
                character_id: c,
                action: Action::Cancel {
                    deletion_operation_id: command.operation_id,
                },
            },
        )
        .await
        .unwrap();
    draining
        .receive(receipt(&cancel, true, 3).as_bytes())
        .await
        .unwrap();
    s.ingest(
        snapshot(
            a,
            c,
            3,
            10,
            "DELETION_PENDING",
            Some((command.operation_id, at - 1, at)),
            at,
        )
        .as_bytes(),
        false,
    )
    .await
    .unwrap();
    assert!(matches!(
        s.request(
            a,
            super::Request {
                accepted_price: Some(70),
                request_id: Uuid::now_v7(),
                character_id: c,
                action: Action::Cancel {
                    deletion_operation_id: command.operation_id
                }
            }
        )
        .await,
        Err(Error::Denied)
    ));
    assert!(wire::receipt(raw.replace("1790000100", "1790000000").as_bytes()).is_err());
    db.close().await;
}
fn identities() -> Arc<IdentityMap> {
    Arc::new(
        IdentityMap::new(vec![
            Binding {
                identity: Identity::spki_sha256([1; 32]),
                group: commands::GROUP.into(),
            },
            Binding {
                identity: Identity::spki_sha256([2; 32]),
                group: super::transport::PROJECTION_GROUP.into(),
            },
        ])
        .unwrap(),
    )
}
async fn http(app: axum::Router, path: &str, body: &str, peer: u8) -> (StatusCode, String) {
    let mut request = HttpRequest::post(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    request
        .extensions_mut()
        .insert(VerifiedClient::from_verified_tls13_http11(
            Identity::spki_sha256([peer; 32]),
        ));
    let r = app.oneshot(request).await.unwrap();
    let status = r.status();
    assert!(r.headers().get("cache-control").is_some());
    let body = String::from_utf8(to_bytes(r.into_body(), 8192).await.unwrap().to_vec()).unwrap();
    (status, body)
}
fn list(version: u8) -> String {
    format!(
        "{{\"contract_version\":{version},\"operation\":\"ListPendingCharacterCommandsV{version}\",\"source_authority\":\"{COMMAND_AUTHORITY}\",\"after_source_revision\":\"0\",\"after_operation_id\":\"00000000-0000-0000-0000-000000000000\",\"max_entries\":32}}"
    )
}
#[tokio::test]
async fn postgres_v1_v2_isolation_identity_and_receipt_binding() {
    let Some(db) = Database::open().await else {
        return;
    };
    let s = Services::new(db.pool.clone(), settings());
    let a = Uuid::now_v7();
    let c = Uuid::now_v7();
    seed(&s, a, c, 100).await;
    let cmd = s.request(a, rename(c)).await.unwrap();
    let v1 = commands::Commands::new(
        db.pool.clone(),
        commands::Settings {
            enabled: true,
            source_authority: COMMAND_AUTHORITY.into(),
            ..commands::Settings::default()
        },
    );
    let v1_router = commands::router(v1, identities());
    let (_, body) = http(
        v1_router.clone(),
        "/internal/v1/game-auth/character-commands/pending",
        &list(1),
        1,
    )
    .await;
    assert_eq!(body, "{\"contract_version\":1,\"pending\":[]}");
    let read = format!(
        "{{\"contract_version\":1,\"operation\":\"ReadCharacterCommandIntentV1\",\"source_authority\":\"{COMMAND_AUTHORITY}\",\"issuer_authority\":\"{}\",\"command\":\"RenameCharacter\",\"operation_id\":\"{}\"}}",
        commands::ISSUER,
        cmd.operation_id
    );
    assert_eq!(
        http(
            v1_router.clone(),
            "/internal/v1/game-auth/character-commands/read",
            &read,
            1
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        http(
            v1_router,
            "/internal/v1/game-auth/character-commands/receipt",
            &receipt(&cmd, true, 3),
            1
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let app = router(s.clone(), identities());
    let path = "/internal/v2/game-auth/character-commands/pending";
    assert_eq!(
        http(app.clone(), path, &list(2), 2).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        http(app.clone(), path, &list(1), 1).await.0,
        StatusCode::BAD_REQUEST
    );
    let (_, body) = http(app.clone(), path, &list(2), 1).await;
    assert!(body.contains(&cmd.operation_id.to_string()));
    let raw = receipt(&cmd, true, 3).replace("\"name\":\"Aldric Vale\"", "\"name\":\"Other Name\"");
    assert!(matches!(
        s.receive(raw.as_bytes()).await,
        Err(Error::Conflict)
    ));
    assert_eq!(balance(&s, a).await, (30, 70));
    let snapshot = snapshot(a, c, 2, 9, "ACTIVE", None, current(&db.pool).await);
    assert_eq!(
        http(
            app.clone(),
            "/internal/v2/game-auth/character-services/state",
            &snapshot,
            1
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        http(
            app,
            "/internal/v2/game-auth/character-services/state",
            &snapshot,
            2
        )
        .await
        .0,
        StatusCode::OK
    );
    // Drain existing work even after new submissions have stopped.
    let mut drain = s.clone();
    drain.settings.enabled = false;
    drain
        .receive(receipt(&cmd, true, 3).as_bytes())
        .await
        .unwrap();
    assert_eq!(balance(&s, a).await, (30, 0));
    db.close().await;
}
#[tokio::test]
async fn postgres_projection_conflict_epoch_invalidation_and_removal_deny() {
    let Some(db) = Database::open().await else {
        return;
    };
    let s = Services::new(db.pool.clone(), settings());
    let a = Uuid::now_v7();
    let c = Uuid::now_v7();
    seed(&s, a, c, 100).await;
    let raw: String = sqlx::query_scalar("SELECT snapshot_json FROM character_service_states")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(s.ingest(raw.as_bytes(), false).await.unwrap(), "accepted");
    assert!(matches!(
        s.ingest(raw.replace("Aldric", "Other").as_bytes(), false)
            .await,
        Err(Error::Conflict)
    ));
    assert!(s.request(a, rename(c)).await.is_err());
    let at = current(&db.pool).await;
    s.ingest(
        snapshot(a, c, 2, 9, "REMOVED_FROM_ACCOUNT", None, at).as_bytes(),
        false,
    )
    .await
    .unwrap();
    assert!(matches!(s.request(a, rename(c)).await, Err(Error::Denied)));
    let newer = snapshot(a, c, 3, 10, "ACTIVE", None, at)
        .replace("\"projection_epoch\":\"1\"", "\"projection_epoch\":\"2\"");
    s.ingest(newer.as_bytes(), false).await.unwrap();
    assert!(s.request(a, rename(c)).await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT highest_epoch FROM account_character_feed")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        "2"
    );
    assert!(
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT watermark_epoch FROM account_character_feed"
        )
        .fetch_one(&db.pool)
        .await
        .unwrap()
        .is_none()
    );
    db.close().await;
}
#[tokio::test]
async fn postgres_restore_reopens_and_grants_lost_rename_for_free() {
    let Some(db) = Database::open().await else {
        return;
    };
    let s = Services::new(db.pool.clone(), settings());
    let a = Uuid::now_v7();
    let c = Uuid::now_v7();
    seed(&s, a, c, 100).await;
    let cmd = s.request(a, rename(c)).await.unwrap();
    s.receive(receipt(&cmd, true, 3).as_bytes()).await.unwrap();
    assert_eq!(balance(&s, a).await, (30, 0));
    let restore = crate::restore::Restore::new(
        db.pool.clone(),
        crate::restore::Settings {
            enabled: true,
            source_authority: COMMAND_AUTHORITY.into(),
            exchange_available: true,
            ..crate::restore::Settings::default()
        },
    );
    let event = Uuid::now_v7();
    // Exact existing restore wire; this reopens the same immutable service operation.
    let raw = include_str!("../restore/fixtures/notice.json");
    let v = wire::decode_value(raw.as_bytes(), 512).unwrap();
    let original = wire::string(&v, "restore_event_id").unwrap();
    let notice = raw
        .replace(original, &event.to_string())
        .replace("oteryn-game-eu1", COMMAND_AUTHORITY);
    restore.notice(notice.as_bytes()).await.unwrap();
    let new_generation =
        wire::number(wire::string(&v, "new_recovery_generation").unwrap(), false).unwrap();
    s.receive(receipt(&cmd, false, new_generation).as_bytes())
        .await
        .unwrap();
    assert_eq!(balance(&s, a).await, (30, 0));
    let state: String = sqlx::query_scalar(
        "SELECT financial_state FROM character_service_orders WHERE operation_id=$1",
    )
    .bind(cmd.operation_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(state, "RECOVERY_REQUIRED");
    assert!(s.request(a, rename(c)).await.is_err());
    restore.notice(notice.as_bytes()).await.unwrap();
    // Simulate the independently acknowledged post-restore LCFA/service resync.
    sqlx::query("UPDATE character_service_feed SET updated_at=clock_timestamp()")
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE account_character_feed SET updated_at=clock_timestamp()")
        .execute(&db.pool)
        .await
        .unwrap();
    let mut request = rename(c);
    request.accepted_price = Some(0);
    let free = s.request(a, request).await.unwrap();
    assert_eq!(balance(&s, a).await, (30, 0));
    s.receive(receipt(&free, true, new_generation).as_bytes())
        .await
        .unwrap();
    assert_eq!(balance(&s, a).await, (30, 0));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM wallet_reservations")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        1
    );
    db.close().await;
}
#[tokio::test]
async fn postgres_browser_owner_forms_csrf_recent_auth_and_actor_to_receipt() {
    use axum::{
        Extension, Form, Router, extract::State, http::HeaderMap, middleware, routing::get,
    };
    use oteryn_web::session::{BrowserSession, SessionLayer};
    let Some(db) = Database::open().await else {
        return;
    };
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105144852_t02_web_sessions.sql"
    ))
    .execute(&db.pool)
    .await
    .unwrap();
    let s = Services::new(db.pool.clone(), settings());
    let a = Uuid::now_v7();
    let other = Uuid::now_v7();
    let c = Uuid::now_v7();
    seed(&s, a, c, 100).await;
    sqlx::query("INSERT INTO accounts VALUES ($1,'active')")
        .bind(other)
        .execute(&db.pool)
        .await
        .unwrap();
    let token = "a".repeat(64);
    let csrf = "b".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at,sudo_until) VALUES ($1,$2,$3,'test',now()+interval '30 minutes',now()+interval '1 day',now()+interval '5 minutes')").bind(Sha256::digest(token.as_bytes()).to_vec()).bind(a).bind(&csrf).execute(&db.pool).await.unwrap();
    let character = c.to_string();
    let get_character = character.clone();
    let post_character = character.clone();
    let app = Router::new()
        .route(
            "/services",
            get(
                move |State(s): State<Services>, Extension(session): Extension<BrowserSession>| {
                    let c = get_character.clone();
                    async move {
                        super::browser::show_with(
                            &s,
                            session.account_id().await,
                            &c,
                            "en",
                            session.csrf_token().await.unwrap_or_default(),
                        )
                        .await
                    }
                },
            )
            .post(
                move |State(s): State<Services>,
                      Extension(session): Extension<BrowserSession>,
                      headers: HeaderMap,
                      Form(fields): Form<Vec<(String, String)>>| {
                    let c = post_character.clone();
                    async move {
                        let actor = session.account_id().await;
                        let recent = super::browser::recent(&s.pool, actor, &headers).await;
                        super::browser::submit_with(&s, actor, &c, "en", recent, fields).await
                    }
                },
            ),
        )
        .route_layer(middleware::from_fn_with_state(
            SessionLayer::new(db.pool.clone()),
            oteryn_web::session::middleware,
        ))
        .with_state(s.clone());
    let request = HttpRequest::get("/services")
        .header("cookie", format!("__Host-oteryn_session={token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let html = String::from_utf8(
        to_bytes(response.into_body(), 65536)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(html.contains(&csrf));
    assert!(html.contains("Change sex"));
    assert!(html.contains("Transfer world"));
    let fields = |csrf: &str| {
        format!(
            "_csrf={csrf}&request_id={}&action=rename&requested_name=Aldric+Vale&confirm=yes&accepted_price=70",
            Uuid::now_v7()
        )
    };
    let post = |body: String| {
        HttpRequest::post("/services")
            .header("content-type", "application/x-www-form-urlencoded")
            .header("cookie", format!("__Host-oteryn_session={token}"))
            .body(Body::from(body))
            .unwrap()
    };
    assert_eq!(
        app.clone()
            .oneshot(post(fields(&"0".repeat(64))))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE web_sessions SET sudo_until=NULL")
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        app.clone()
            .oneshot(post(fields(&csrf)))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE web_sessions SET sudo_until=now()+interval '5 minutes',account_id=$1")
        .bind(other)
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        app.clone()
            .oneshot(post(fields(&csrf)))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE web_sessions SET account_id=$1")
        .bind(a)
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        app.clone()
            .oneshot(post(fields(&csrf).replace("Aldric+Vale", "Bad++Name")))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        app.oneshot(post(fields(&csrf))).await.unwrap().status(),
        StatusCode::ACCEPTED
    );
    assert_eq!(balance(&s, a).await, (30, 70));
    let cmd = commands::Commands::new(db.pool.clone(), commands::Settings::default())
        .status(
            sqlx::query_scalar("SELECT id FROM character_commands")
                .fetch_one(&db.pool)
                .await
                .unwrap(),
        )
        .await
        .unwrap();
    s.receive(receipt(&cmd, true, 3).as_bytes()).await.unwrap();
    assert_eq!(balance(&s, a).await, (30, 0));
    db.close().await;
}
#[tokio::test]
async fn postgres_service_and_ownership_transfer_cannot_queue_together() {
    struct Accounts;
    impl commands::AccountStateReader for Accounts {
        async fn state(&self, _: Uuid) -> commands::AccountState {
            commands::AccountState::Eligible
        }
    }
    let Some(db) = Database::open().await else {
        return;
    };
    let s = Services::new(db.pool.clone(), settings());
    let a = Uuid::now_v7();
    let c = Uuid::now_v7();
    seed(&s, a, c, 100).await;
    let v1 = commands::Commands::new(
        db.pool.clone(),
        commands::Settings {
            enabled: true,
            transfer_enabled: true,
            source_authority: COMMAND_AUTHORITY.into(),
            ..commands::Settings::default()
        },
    );
    let service = s.request(a, rename(c));
    let transfer = v1.enqueue(
        &Accounts,
        Uuid::now_v7(),
        "listing-step",
        commands::Intent::Transfer {
            character_id: c,
            from_account_id: a,
            to_account_id: Uuid::now_v7(),
            purpose: commands::TransferPurpose::ListingEscrow,
        },
    );
    let (service, transfer) = tokio::join!(service, transfer);
    assert_eq!(
        usize::from(service.is_ok()) + usize::from(transfer.is_ok()),
        1
    );
    assert!(matches!(
        (&service, &transfer),
        (Ok(_), Err(Error::Conflict)) | (Err(Error::Conflict), Ok(_))
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM character_commands")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        balance(&s, a).await,
        if service.is_ok() { (30, 70) } else { (100, 0) }
    );
    db.close().await;
}
