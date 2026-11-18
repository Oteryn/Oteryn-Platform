#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use oteryn_wallet::{AccountState, AdjustmentAudit, TransferPermission, WalletFlags};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct Policy;
impl AccountStateReader for Policy {
    async fn state(&self, _: &mut sqlx::PgConnection, account: Uuid) -> AccountState {
        if account.as_bytes()[15] == 0 {
            AccountState::Inactive
        } else {
            AccountState::Active
        }
    }
}
impl TransferBlockCheck for Policy {
    async fn permission(&self, _: &mut sqlx::PgConnection, _: Uuid) -> TransferPermission {
        TransferPermission::Allowed
    }
}
impl AuditHook for Policy {
    async fn adjustment(&self, _: &mut sqlx::PgConnection, _: &AdjustmentAudit) -> bool {
        true
    }
}
impl premium::Accounts for Policy {
    fn state<'a>(
        &'a self,
        account: Uuid,
        _: &'a mut sqlx::PgConnection,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<premium::AccountState, premium::Error>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            Ok(if account.as_bytes()[15] == 0 {
                premium::AccountState::Unknown
            } else {
                premium::AccountState::Known
            })
        })
    }
}
impl AttemptHook for Policy {
    async fn check(&self, account: Uuid) -> Result<(), Error> {
        if account.as_bytes()[15] == 254 {
            Err(Error::Denied)
        } else {
            Ok(())
        }
    }
}
type Service = Codes<Policy, Policy, Policy, Policy>;
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn create() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED codes database test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = PgPool::connect(&url).await.unwrap();
        let schema = format!("codes_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let search = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(25)
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
        for migration in [
            include_str!("../../../migrations/20261105151420_t26_wallet.sql"),
            include_str!("../../../migrations/20261105160000_t27_premium_ledger.sql"),
            include_str!("../../../migrations/20261105155204_t55_codes.sql"),
            NO_COIN_DEBT_STUB,
        ] {
            sqlx::raw_sql(migration).execute(&pool).await.unwrap();
        }
        sqlx::raw_sql(
            include_str!("../../../migrations/20261115225000_fix_a3-l6_incoming_capacity.sql")
                .split("-- Preserve")
                .next()
                .unwrap(),
        )
        .execute(&pool)
        .await
        .unwrap();
        Some(Self {
            pool,
            admin,
            schema,
        })
    }
    fn service(&self) -> Service {
        Codes::new(
            self.pool.clone(),
            CodeFlags { enabled: true },
            self.wallet(),
            self.premium(),
            Policy,
        )
    }
    fn wallet(&self) -> oteryn_wallet::Ledger<Policy, Policy, Policy> {
        oteryn_wallet::Ledger::new(
            self.pool.clone(),
            WalletFlags { enabled: true },
            Policy,
            Policy,
            Policy,
        )
    }
    fn premium(&self) -> premium::repository::Ledger {
        premium::repository::Ledger::new(self.pool.clone(), Arc::new(Policy), true)
    }
    async fn count(&self, table: &str) -> i64 {
        assert!(
            [
                "wallet_entries",
                "premium_ledger",
                "code_redemptions",
                "code_game_commands"
            ]
            .contains(&table)
        );
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(&self.pool)
            .await
            .unwrap()
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
fn account() -> Uuid {
    let mut bytes = *Uuid::now_v7().as_bytes();
    bytes[15] = 1;
    Uuid::from_bytes(bytes)
}
fn staff() -> AuthorizedStaff {
    AuthorizedStaff { actor: account() }
}
fn batch(kind: Kind) -> Batch {
    let now = OffsetDateTime::now_utc();
    Batch {
        request_id: Uuid::now_v7(),
        kind,
        amount: 7,
        max_per_code: 1,
        max_per_account: 1,
        valid_from: now - time::Duration::hours(1),
        valid_until: now + time::Duration::hours(1),
        note: "Test gift batch".into(),
    }
}
fn request(code: &str) -> Redemption {
    Redemption {
        code: code.into(),
        account: account(),
        character_ref: None,
        key: Uuid::now_v7(),
    }
}
#[test]
fn normalization_bad_input_and_defaults() {
    assert_eq!(
        normalized_hash("2345-6789-ABCD-EFGH"),
        normalized_hash(" 23456789abcdefgh ")
    );
    for bad in [
        "",
        "2345-6789-ABCD-EFGI",
        "23456789ABCDEFGH1",
        "2345 6789ABCDEFGH",
        "é23456789ABCDEFG",
    ] {
        assert_eq!(normalized_hash(bad), Err(Error::Invalid));
    }
    assert!(!CodeFlags::default().enabled);
    let generated = Generated {
        batch_id: account(),
        codes: vec!["secret plaintext".into()],
    };
    assert!(!format!("{generated:?}").contains("secret"));
}
#[tokio::test]
async fn db_generate_redeem_coins_premium_and_plaintext_absent() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let service = db.service();
    for kind in [Kind::Coins, Kind::PremiumDays] {
        let generated = service.generate(&staff(), &batch(kind), 2).await.unwrap();
        for code in &generated.codes {
            assert_eq!(code.len(), 19);
            assert!(code.split('-').all(|s| s.len() == 4));
        }
        let r = request(&generated.codes[0].to_lowercase());
        let receipt = service.redeem(&r).await.unwrap();
        assert_eq!(receipt.kind, kind);
        assert_eq!(service.redeem(&r).await.unwrap(), receipt);
        let mut conflict = r.clone();
        conflict.account = account();
        assert_eq!(service.redeem(&conflict).await, Err(Error::Conflict));
        conflict = r.clone();
        conflict.character_ref = Some(account());
        assert_eq!(service.redeem(&conflict).await, Err(Error::Conflict));
        conflict = r.clone();
        conflict.code = generated.codes[1].clone();
        assert_eq!(service.redeem(&conflict).await, Err(Error::Conflict));
        let stored: Vec<String> =
            sqlx::query_scalar("SELECT row_to_json(c)::text FROM gift_codes c")
                .fetch_all(&db.pool)
                .await
                .unwrap();
        for text in stored {
            for code in &generated.codes {
                assert!(!text.contains(code));
                assert!(!text.contains(&code.replace('-', "")));
            }
        }
        if kind == Kind::Coins {
            assert_eq!(db.wallet().balance(r.account).await.unwrap().available, 7);
        } else {
            let seconds: i64 = sqlx::query_scalar(
                "SELECT sum(delta_seconds)::bigint FROM premium_ledger WHERE account_id=$1",
            )
            .bind(r.account)
            .fetch_one(&db.pool)
            .await
            .unwrap();
            assert_eq!(seconds, 7 * 86400);
        }
    }
    assert_eq!(
        (
            db.count("wallet_entries").await,
            db.count("premium_ledger").await
        ),
        (1, 1)
    );
    db.close().await;
}
#[tokio::test]
async fn db_limits_across_codes_and_accounts() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let service = db.service();
    let mut b = batch(Kind::Coins);
    b.max_per_code = 2;
    let g = service.generate(&staff(), &b, 2).await.unwrap();
    let r = request(&g.codes[0]);
    service.redeem(&r).await.unwrap();
    let mut again = r.clone();
    again.key = Uuid::now_v7();
    assert_eq!(service.redeem(&again).await, Err(Error::Refused));
    again.code = g.codes[1].clone();
    assert_eq!(service.redeem(&again).await, Err(Error::Refused));
    service.redeem(&request(&g.codes[0])).await.unwrap();
    assert_eq!(
        service.redeem(&request(&g.codes[0])).await,
        Err(Error::Refused)
    );
    assert_eq!(db.count("code_redemptions").await, 2);
    db.close().await;
}
#[tokio::test]
async fn db_expired_future_bad_input_and_disabled() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let mut service = db.service();
    for shift in [-3, 3] {
        let mut b = batch(Kind::Coins);
        b.valid_from += time::Duration::hours(shift);
        b.valid_until += time::Duration::hours(shift);
        let g = service.generate(&staff(), &b, 1).await.unwrap();
        assert_eq!(
            service.redeem(&request(&g.codes[0])).await,
            Err(Error::Refused)
        );
    }
    assert_eq!(service.redeem(&request("bad")).await, Err(Error::Invalid));
    assert_eq!(
        service.redeem(&request("2345-6789-ABCD-EFGH")).await,
        Err(Error::Refused)
    );
    assert!(matches!(
        service.generate(&staff(), &batch(Kind::Coins), 0).await,
        Err(Error::Invalid)
    ));
    let mut b = batch(Kind::PremiumDays);
    b.amount = 367;
    assert!(matches!(
        service.generate(&staff(), &b, 1).await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        service
            .generate(
                &AuthorizedStaff { actor: Uuid::nil() },
                &batch(Kind::Coins),
                1
            )
            .await,
        Err(Error::Invalid)
    ));
    service.flags = CodeFlags::default();
    assert_eq!(service.redeem(&request("bad")).await, Err(Error::Disabled));
    assert!(matches!(
        service.generate(&staff(), &batch(Kind::Coins), 1).await,
        Err(Error::Disabled)
    ));
    assert_eq!(db.count("code_redemptions").await, 0);
    db.close().await;
}
#[tokio::test]
async fn db_twenty_concurrent_redemptions_have_one_success() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let service = Arc::new(db.service());
    let g = service
        .generate(&staff(), &batch(Kind::Coins), 1)
        .await
        .unwrap();
    let barrier = Arc::new(tokio::sync::Barrier::new(20));
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..20 {
        let service = service.clone();
        let barrier = barrier.clone();
        let r = request(&g.codes[0]);
        tasks.spawn(async move {
            barrier.wait().await;
            service.redeem(&r).await
        });
    }
    let mut successes = 0;
    while let Some(result) = tasks.join_next().await {
        match result.unwrap() {
            Ok(_) => successes += 1,
            Err(e) => assert_eq!(e, Error::Refused),
        }
    }
    assert_eq!(successes, 1);
    assert_eq!(db.count("wallet_entries").await, 1);
    assert_eq!(db.count("code_redemptions").await, 1);
    db.close().await;
}
#[tokio::test]
async fn db_item_requires_character_and_only_enqueues_pending_command() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let service = db.service();
    let g = service
        .generate(&staff(), &batch(Kind::ItemGrant), 1)
        .await
        .unwrap();
    let mut r = request(&g.codes[0]);
    assert_eq!(service.redeem(&r).await, Err(Error::Invalid));
    r.character_ref = Some(account());
    let first = service.redeem(&r).await.unwrap();
    assert_eq!(service.redeem(&r).await.unwrap(), first);
    let (character, amount, status): (Uuid, i64, String) =
        sqlx::query_as("SELECT character_ref,amount,status FROM code_game_commands")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(
        (Some(character), amount, status.as_str()),
        (r.character_ref, 7, "pending")
    );
    assert_eq!(
        db.count("wallet_entries").await + db.count("premium_ledger").await,
        0
    );
    db.close().await;
}
#[tokio::test]
async fn db_ledger_refusal_and_final_insert_failure_roll_back_everything() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let service = db.service();
    for kind in [Kind::Coins, Kind::PremiumDays] {
        let g = service.generate(&staff(), &batch(kind), 1).await.unwrap();
        let mut denied = request(&g.codes[0]);
        let mut bytes = *denied.account.as_bytes();
        bytes[15] = 0;
        denied.account = Uuid::from_bytes(bytes);
        assert_eq!(service.redeem(&denied).await, Err(Error::Refused));
    }
    assert_eq!(db.count("code_redemptions").await, 0);
    // Deferred failure occurs at commit, after the ledger APIs have succeeded.
    sqlx::raw_sql("CREATE FUNCTION reject_redemption() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test rejection'; END; $$; CREATE CONSTRAINT TRIGGER reject_redemption AFTER INSERT ON code_redemptions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reject_redemption();").execute(&db.pool).await.unwrap();
    for kind in [Kind::Coins, Kind::PremiumDays] {
        let g = service.generate(&staff(), &batch(kind), 1).await.unwrap();
        assert_eq!(
            service.redeem(&request(&g.codes[0])).await,
            Err(Error::Unavailable)
        );
    }
    assert_eq!(
        db.count("code_redemptions").await
            + db.count("wallet_entries").await
            + db.count("premium_ledger").await,
        0
    );
    db.close().await;
}
#[tokio::test]
async fn db_wallet_adjust_in_commit_rollback_and_replay() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let ledger = db.wallet();
    let actor = account();
    let who = account();
    let key = Uuid::now_v7();
    let mut tx = db.pool.begin().await.unwrap();
    ledger
        .adjust_in(&mut tx, key, who, 5, actor, "Test reason")
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(db.count("wallet_entries").await, 0);
    let mut tx = db.pool.begin().await.unwrap();
    assert!(
        !ledger
            .adjust_in(&mut tx, key, who, 5, actor, "Test reason")
            .await
            .unwrap()
            .replayed
    );
    tx.commit().await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    assert!(
        ledger
            .adjust_in(&mut tx, key, who, 5, actor, "Test reason")
            .await
            .unwrap()
            .replayed
    );
    tx.commit().await.unwrap();
    assert_eq!(db.count("wallet_entries").await, 1);
    assert_eq!(ledger.balance(who).await.unwrap().available, 5);
    db.close().await;
}
#[tokio::test]
async fn db_premium_mutate_in_commit_rollback_and_replay() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let ledger = db.premium();
    let op = premium::Operator {
        actor: account(),
        confirmed_mfa: true,
        permission: "products.premium.manage".into(),
    };
    let m = premium::Mutation {
        account: account(),
        request: Uuid::now_v7(),
        change: premium::Change::Grant(2),
        reason: "Test premium reason".into(),
    };
    let mut tx = db.pool.begin().await.unwrap();
    ledger.mutate_in(&mut tx, &op, &m).await.unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(db.count("premium_ledger").await, 0);
    let mut tx = db.pool.begin().await.unwrap();
    let result = ledger.mutate_in(&mut tx, &op, &m).await.unwrap();
    tx.commit().await.unwrap();
    let mut tx = db.pool.begin().await.unwrap();
    assert_eq!(ledger.mutate_in(&mut tx, &op, &m).await.unwrap(), result);
    tx.commit().await.unwrap();
    assert_eq!(db.count("premium_ledger").await, 1);
    db.close().await;
}

#[tokio::test]
async fn db_concurrent_batch_account_limit_and_same_key_retry() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let service = Arc::new(db.service());
    let g = service
        .generate(&staff(), &batch(Kind::Coins), 2)
        .await
        .unwrap();
    let who = account();
    let mut tasks = tokio::task::JoinSet::new();
    for code in g.codes {
        let service = service.clone();
        let mut r = request(&code);
        r.account = who;
        tasks.spawn(async move { service.redeem(&r).await });
    }
    let mut wins = 0;
    while let Some(result) = tasks.join_next().await {
        match result.unwrap() {
            Ok(_) => wins += 1,
            Err(e) => assert_eq!(e, Error::Refused),
        }
    }
    assert_eq!(wins, 1);
    let g = service
        .generate(&staff(), &batch(Kind::PremiumDays), 1)
        .await
        .unwrap();
    let r = request(&g.codes[0]);
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..20 {
        let service = service.clone();
        let r = r.clone();
        tasks.spawn(async move { service.redeem(&r).await.unwrap() });
    }
    let mut receipts = Vec::new();
    while let Some(result) = tasks.join_next().await {
        receipts.push(result.unwrap());
    }
    assert!(receipts.iter().all(|r| *r == receipts[0]));
    assert_eq!(db.count("premium_ledger").await, 1);
    let mut denied = request("bad");
    let mut bytes = *denied.account.as_bytes();
    bytes[15] = 254;
    denied.account = Uuid::from_bytes(bytes);
    assert_eq!(service.redeem(&denied).await, Err(Error::Denied));
    db.close().await;
}

/// Wallet credits read the coin-debt view, which spans the payments tables.
/// A schema built from a migration subset gets an empty stand-in: no debt.
pub(crate) const NO_COIN_DEBT_STUB: &str = "CREATE TABLE coin_purchase_reversals(id uuid PRIMARY KEY,payer_account_id uuid NOT NULL,outcome text NOT NULL,created_at timestamptz NOT NULL DEFAULT now());CREATE VIEW coin_outstanding_debts AS SELECT id AS reversal_id,id AS order_id,payer_account_id AS account_id,created_at,0::bigint AS owed FROM coin_purchase_reversals";

#[tokio::test]
async fn postgres_a3_browser_premium_redemption_one_connection_and_value_restrictions() {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let started = std::time::Instant::now();
    for maximum in [1, 16] {
        let url = std::env::var("PORTAL_TEST_DATABASE_URL").expect("PostgreSQL required");
        let root = PgPool::connect(&url).await.unwrap();
        let schema = format!("a3_codes_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await
            .unwrap();
        let search = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(maximum)
            .acquire_timeout(std::time::Duration::from_secs(1))
            .after_connect(move |connection, _| {
                let search = search.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path',$1,false)")
                        .bind(search)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        let issuer = account();
        let player = account();
        for id in [issuer, player] {
            sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'code-test-password','active','en',now())")
            .bind(id).bind(format!("{id}@example.invalid")).execute(&pool).await.unwrap();
        }
        let codes = Codes::new(
            pool.clone(),
            CodeFlags { enabled: true },
            oteryn_wallet::Ledger::new(
                pool.clone(),
                WalletFlags { enabled: true },
                Policy,
                Policy,
                Policy,
            ),
            premium::repository::Ledger::new(
                pool.clone(),
                Arc::new(crate::loyalty::stream::http::DatabaseAccounts),
                true,
            ),
            Policy,
        );
        let mut generous = batch(Kind::PremiumDays);
        generous.max_per_account = 30;
        generous.max_per_code = 30;
        let generated = codes
            .generate(&AuthorizedStaff { actor: issuer }, &generous, 1)
            .await
            .unwrap();
        let request = Redemption {
            account: player,
            ..request(&generated.codes[0])
        };
        let receipt = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            codes.redeem_player(&request),
        )
        .await
        .unwrap()
        .unwrap();
        let changes = [
            "UPDATE accounts SET email_change_blocked_until=clock_timestamp()+interval '1 hour' WHERE id=$1",
            "INSERT INTO account_terminations(account_id,scheduled_for) VALUES($1,now()+interval '2 days')",
            "INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by) VALUES($1,'restriction','active','Redemption regression',now()-interval '1 hour',now()+interval '1 hour',$1)",
        ];
        for change in changes {
            sqlx::query(change)
                .bind(player)
                .execute(&pool)
                .await
                .unwrap();
            // An exact replay is distinct from acquisition of new value.
            assert_eq!(codes.redeem_player(&request).await.unwrap(), receipt);
            let new_request = Redemption {
                key: Uuid::now_v7(),
                ..request.clone()
            };
            assert_eq!(codes.redeem_player(&new_request).await, Err(Error::Denied));
            let counts: (i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM code_redemptions),(SELECT count(*) FROM premium_ledger)",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
            assert_eq!(counts, (1, 1));
            sqlx::query("UPDATE accounts SET email_change_blocked_until='-infinity' WHERE id=$1")
                .bind(player)
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("UPDATE account_terminations SET cancelled_at=now() WHERE account_id=$1 AND cancelled_at IS NULL").bind(player).execute(&pool).await.unwrap();
            sqlx::query("UPDATE sanctions SET status='revoked' WHERE account_id=$1")
                .bind(player)
                .execute(&pool)
                .await
                .unwrap();
        }
        // Fill the entire pool with contenders. Each operation must finish using
        // its sole transaction instead of retaining one connection to request two.
        let mut tasks = Vec::new();
        let codes = Arc::new(codes);
        for _ in 0..16 {
            let codes = codes.clone();
            let request = Redemption {
                key: Uuid::now_v7(),
                ..request.clone()
            };
            tasks.push(tokio::spawn(
                async move { codes.redeem_player(&request).await },
            ));
        }
        for task in tasks {
            tokio::time::timeout(std::time::Duration::from_secs(5), task)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        }
        pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
            .execute(&root)
            .await
            .unwrap();
        root.close().await;
    }
    eprintln!(
        "postgres_a3_browser_premium_redemption_one_connection_and_value_restrictions duration: {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn browser_redemption_locks_code_before_account_and_rechecks_status()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let db = Database::create().await.ok_or("PostgreSQL is required")?;
    sqlx::raw_sql("CREATE TABLE accounts(id uuid PRIMARY KEY,status text NOT NULL,email_verified_at timestamptz); CREATE FUNCTION account_value_eligible(uuid) RETURNS boolean LANGUAGE sql AS 'SELECT true'").execute(&db.pool).await?;
    let player = account();
    sqlx::query("INSERT INTO accounts VALUES($1,'active',clock_timestamp())")
        .bind(player)
        .execute(&db.pool)
        .await?;
    let service = db.service();
    let generated = service
        .generate(
            &AuthorizedStaff { actor: account() },
            &batch(Kind::Coins),
            1,
        )
        .await?;
    let redemption = Redemption {
        account: player,
        ..request(&generated.codes[0])
    };
    let mut blocker = db.pool.begin().await?;
    sqlx::query("SELECT id FROM gift_codes WHERE batch_id=$1 FOR UPDATE")
        .bind(generated.batch_id)
        .execute(&mut *blocker)
        .await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;
    let pending = tokio::spawn(async move { service.redeem_player(&redemption).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(pid)
            .fetch_one(&db.pool)
            .await?;
            if waiting {
                break Ok::<_, sqlx::Error>(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await??;
    let account_lock = sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE NOWAIT")
        .bind(player)
        .execute(&mut *blocker)
        .await;
    if account_lock.is_ok() {
        sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
            .bind(player)
            .execute(&mut *blocker)
            .await?;
        blocker.commit().await?;
    } else {
        blocker.rollback().await?;
    }
    let result = pending.await?;
    account_lock?;
    assert_eq!(result, Err(Error::Denied));
    assert_eq!(db.count("code_redemptions").await, 0);
    db.close().await;
    Ok(())
}

#[tokio::test]
async fn drc_gift_request_budget() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let service = db.service();
    let request = request("2222-2222-2222-2222");
    let mut blocker = db.pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,55))")
        .bind(request.key.to_string())
        .execute(&mut *blocker)
        .await
        .unwrap();
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(5), service.redeem(&request)).await;
    assert!(
        result.is_ok(),
        "gift request lock exceeded the shared three-second budget"
    );
    assert!(matches!(result.unwrap(), Err(Error::Unavailable)));
    blocker.rollback().await.unwrap();
    assert!(matches!(
        service.redeem(&request).await,
        Err(Error::Refused)
    ));
    db.close().await;
}

/// Holds `statement` (schema-qualified by `{schema}`) on a separate connection.
async fn hold(db: &Database, statement: &str, id: Uuid) -> sqlx::PgConnection {
    use sqlx::Connection;
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL").unwrap();
    let mut blocker = sqlx::PgConnection::connect(&url).await.unwrap();
    sqlx::query("BEGIN").execute(&mut blocker).await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(statement.replace("{schema}", &schema)))
        .bind(id)
        .execute(&mut blocker)
        .await
        .unwrap();
    blocker
}

async fn release(mut blocker: sqlx::PgConnection) {
    use sqlx::Connection;
    sqlx::query("ROLLBACK").execute(&mut blocker).await.unwrap();
    blocker.close().await.unwrap();
}

/// Takes the staff account row as the production adapter does, without arming
/// a budget of its own: the issuance transaction must bound the wait.
struct SharedAccountAuthorization;
impl StaffAuthorization for SharedAccountAuthorization {
    async fn authorize(
        &self,
        connection: &mut sqlx::PgConnection,
        actor: Uuid,
    ) -> Result<(), Error> {
        sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR SHARE")
            .bind(actor)
            .execute(connection)
            .await?;
        Ok(())
    }
}

#[tokio::test]
async fn postgres_generate_staff_held_account_lock_sheds_within_mutation_budget() {
    use std::time::{Duration, Instant};
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    sqlx::query("CREATE TABLE accounts(id uuid PRIMARY KEY)")
        .execute(&db.pool)
        .await
        .unwrap();
    let actor = account();
    sqlx::query("INSERT INTO accounts VALUES($1)")
        .bind(actor)
        .execute(&db.pool)
        .await
        .unwrap();
    let blocker = hold(
        &db,
        "SELECT id FROM {schema}.accounts WHERE id=$1 FOR UPDATE",
        actor,
    )
    .await;
    let service = db.service();
    let started = Instant::now();
    let observed = tokio::time::timeout(
        Duration::from_secs(10),
        service.generate_staff(actor, &SharedAccountAuthorization, &batch(Kind::Coins), 1),
    )
    .await;
    let elapsed = started.elapsed();
    release(blocker).await;
    db.close().await;
    let observed = observed.expect("generate_staff still waiting at 10 s on a held account lock");
    assert!(matches!(observed, Err(Error::Unavailable)), "{observed:?}");
    assert!(
        elapsed < Duration::from_secs(4),
        "budget exceeded: {elapsed:?}"
    );
}

/// `redeem` and `redeem_player` share `redeem_in`, which armed the budget
/// before this change; the explicit arm at their `begin()` is a no-op, so this
/// test covers both entry points and is green with or without it.
#[tokio::test]
async fn postgres_redemptions_held_code_lock_sheds_within_mutation_budget() {
    use std::time::{Duration, Instant};
    let _database_guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    sqlx::raw_sql("CREATE TABLE accounts(id uuid PRIMARY KEY,status text NOT NULL,email_verified_at timestamptz); CREATE FUNCTION account_value_eligible(uuid) RETURNS boolean LANGUAGE sql AS 'SELECT true'").execute(&db.pool).await.unwrap();
    let player = account();
    sqlx::query("INSERT INTO accounts VALUES($1,'active',clock_timestamp())")
        .bind(player)
        .execute(&db.pool)
        .await
        .unwrap();
    let service = db.service();
    let generated = service
        .generate(&staff(), &batch(Kind::Coins), 1)
        .await
        .unwrap();
    let blocker = hold(
        &db,
        "SELECT id FROM {schema}.gift_codes WHERE batch_id=$1 FOR UPDATE",
        generated.batch_id,
    )
    .await;
    let redemption = Redemption {
        account: player,
        ..request(&generated.codes[0])
    };
    let mut observed = Vec::new();
    for browser in [false, true] {
        let started = Instant::now();
        let result = tokio::time::timeout(Duration::from_secs(10), async {
            if browser {
                service.redeem_player(&redemption).await
            } else {
                service.redeem(&redemption).await
            }
        })
        .await;
        observed.push((browser, result, started.elapsed()));
    }
    release(blocker).await;
    let redeemed = db.count("code_redemptions").await;
    db.close().await;
    for (browser, result, elapsed) in observed {
        let result = result.expect("redemption still waiting at 10 s on a held code lock");
        assert_eq!(result, Err(Error::Unavailable), "browser={browser}");
        assert!(
            elapsed < Duration::from_secs(4),
            "browser={browser}: {elapsed:?}"
        );
    }
    assert_eq!(redeemed, 0);
}

#[tokio::test]
async fn r8_issuance_retry_preserves_batch() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let service = db.service();
    let issuer = staff();
    let terms = batch(Kind::Coins);
    let (first, retry) = tokio::join!(
        service.generate(&issuer, &terms, 1),
        service.generate(&issuer, &terms, 1)
    );
    let first = first.unwrap();
    let retry = retry.unwrap();
    assert_eq!(
        first.batch_id, retry.batch_id,
        "retry created another batch"
    );
    assert_eq!(first.codes.len() + retry.codes.len(), 1);
    assert!(matches!(
        service.generate(&issuer, &terms, 2).await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        service.generate(&staff(), &terms, 1).await,
        Err(Error::Conflict)
    ));
    db.close().await;
}

#[tokio::test]
async fn r8_distinct_issuance_requests_create_two_batches() {
    let _guard = oteryn_db::test_support::global_lock_guard().await.unwrap();
    let Some(db) = Database::create().await else {
        return;
    };
    let service = db.service();
    let issuer = staff();
    let mut terms = batch(Kind::Coins);
    let first = service.generate(&issuer, &terms, 1).await.unwrap();
    terms.request_id = Uuid::now_v7();
    let second = service.generate(&issuer, &terms, 1).await.unwrap();
    assert_ne!(first.batch_id, second.batch_id);
    assert_eq!((first.codes.len(), second.codes.len()), (1, 1));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM code_batches")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        2
    );
    db.close().await;
}
