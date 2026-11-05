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
        ] {
            sqlx::raw_sql(migration).execute(&pool).await.unwrap();
        }
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
