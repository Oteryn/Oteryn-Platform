#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use oteryn_wallet::{AccountState, AdjustmentAudit, TransferPermission, WalletFlags};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone)]
struct Checks {
    allow: Arc<AtomicBool>,
    audit: Arc<AtomicBool>,
    notify: Arc<AtomicBool>,
}
impl Checks {
    fn new() -> Self {
        Self {
            allow: Arc::new(AtomicBool::new(true)),
            audit: Arc::new(AtomicBool::new(true)),
            notify: Arc::new(AtomicBool::new(true)),
        }
    }
}
impl Policy for Checks {
    async fn allowed(
        &self,
        _: &mut PgConnection,
        _: Uuid,
        _: Uuid,
        _: Kind,
        _: Stage,
    ) -> Result<bool, sqlx::Error> {
        Ok(self.allow.load(Ordering::SeqCst))
    }
}
impl AccountStateReader for Checks {
    async fn state(&self, _: &mut PgConnection, _: Uuid) -> AccountState {
        AccountState::Active
    }
}
impl TransferBlockCheck for Checks {
    async fn permission(&self, _: &mut PgConnection, _: Uuid) -> TransferPermission {
        TransferPermission::Allowed
    }
}
impl AuditHook for Checks {
    async fn adjustment(&self, _: &mut PgConnection, _: &AdjustmentAudit) -> bool {
        self.audit.load(Ordering::SeqCst)
    }
}
impl oteryn_notify::AccountState for Checks {
    fn active_verified(
        &self,
        _: Uuid,
    ) -> oteryn_notify::NotifyFuture<'_, Option<oteryn_notify::Account>> {
        Box::pin(async {
            Ok(self
                .notify
                .load(Ordering::SeqCst)
                .then(|| oteryn_notify::Account {
                    email: "player@example.test".into(),
                    locale: oteryn_notify::Locale::Pl,
                }))
        })
    }
}
type Service = Referrals<Checks, Checks, Checks, Checks>;
fn service(pool: PgPool, checks: Checks, enabled: bool) -> Service {
    Referrals::new(
        pool.clone(),
        Flags { enabled },
        checks.clone(),
        oteryn_wallet::Ledger::new(
            pool.clone(),
            WalletFlags { enabled: true },
            checks.clone(),
            checks.clone(),
            checks.clone(),
        ),
        oteryn_notify::Notifications {
            pool,
            accounts: Arc::new(checks),
            unsubscribe: oteryn_notify::Unsubscribe::new(
                vec![7; 32],
                "https://example.test/unsubscribe".into(),
            )
            .unwrap(),
        },
        Reward {
            actor: Uuid::now_v7(),
            amount: 25,
        },
    )
}
fn invitation(kind: Kind) -> Invitation {
    Invitation {
        id: Uuid::now_v7(),
        inviter: Uuid::now_v7(),
        invitee: Uuid::now_v7(),
        kind,
        expires_at: OffsetDateTime::now_utc() + time::Duration::days(7),
    }
}
struct Database {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    async fn create() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED referrals PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = PgPool::connect(&url).await.unwrap();
        let schema = format!("referrals_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let search = schema.clone();
        let pool = sqlx::postgres::PgPoolOptions::new()
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
        // The full schema: wallet credits read the coin-debt view, which
        // spans the payments tables.
        oteryn_db::migrate(&pool).await.unwrap();
        Some(Self {
            pool,
            admin,
            schema,
        })
    }
    async fn count(&self, table: &str) -> i64 {
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
#[tokio::test]
async fn disabled_and_invalid_before_io() {
    assert!(!Flags::default().enabled);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let mut svc = service(pool, Checks::new(), false);
    let mut request = invitation(Kind::New);
    assert_eq!(svc.invite(&request).await, Err(Error::Disabled));
    assert_eq!(
        svc.claim(request.invitee, request.id).await,
        Err(Error::Disabled)
    );
    assert_eq!(svc.statistics(request.inviter).await, Err(Error::Disabled));
    svc.flags.enabled = true;
    request.invitee = Uuid::nil();
    assert_eq!(svc.invite(&request).await, Err(Error::Invalid));
    request.invitee = request.inviter;
    assert_eq!(svc.invite(&request).await, Err(Error::Denied));
    assert_eq!(
        svc.claim(Uuid::nil(), request.id).await,
        Err(Error::Invalid)
    );
}
#[tokio::test]
async fn postgres_success_replay_and_statistics() {
    let Some(db) = Database::create().await else {
        return;
    };
    let svc = service(db.pool.clone(), Checks::new(), true);
    for kind in [Kind::New, Kind::Returning] {
        let request = invitation(kind);
        svc.invite(&request).await.unwrap();
        svc.invite(&request).await.unwrap();
        let (a, b) = tokio::join!(
            svc.claim(request.invitee, request.id),
            svc.claim(request.invitee, request.id)
        );
        assert_eq!(a.unwrap(), b.unwrap());
        assert_eq!(
            svc.wallet.balance(request.inviter).await.unwrap().available,
            25
        );
        assert_eq!(
            svc.statistics(request.inviter).await.unwrap(),
            Statistics {
                invitations: 1,
                returning: i64::from(kind == Kind::Returning),
                rewarded: 1,
                expired: 0,
                coins: 25
            }
        );
        assert_eq!(
            svc.statistics(request.invitee).await.unwrap().invitations,
            0
        );
    }
    assert_eq!(db.count("wallet_entries").await, 2);
    assert_eq!(db.count("notifications").await, 4);
    db.close().await;
}
#[tokio::test]
async fn postgres_refusals_and_bad_input() {
    let Some(db) = Database::create().await else {
        return;
    };
    let checks = Checks::new();
    let svc = service(db.pool.clone(), checks.clone(), true);
    let request = invitation(Kind::Returning);
    checks.allow.store(false, Ordering::SeqCst);
    assert_eq!(svc.invite(&request).await, Err(Error::Denied));
    checks.allow.store(true, Ordering::SeqCst);
    let mut expired = request.clone();
    expired.expires_at = OffsetDateTime::now_utc() - time::Duration::seconds(1);
    assert_eq!(svc.invite(&expired).await, Err(Error::Invalid));
    svc.invite(&request).await.unwrap();
    let mut changed = request.clone();
    changed.inviter = Uuid::now_v7();
    assert_eq!(svc.invite(&changed).await, Err(Error::Conflict));
    changed.id = Uuid::now_v7();
    assert_eq!(svc.invite(&changed).await, Err(Error::Denied));
    let mut cycle = invitation(Kind::New);
    cycle.inviter = request.invitee;
    cycle.invitee = request.inviter;
    assert_eq!(svc.invite(&cycle).await, Err(Error::Denied));
    assert_eq!(
        svc.claim(request.inviter, request.id).await,
        Err(Error::Denied)
    );
    checks.allow.store(false, Ordering::SeqCst);
    assert_eq!(
        svc.claim(request.invitee, request.id).await,
        Err(Error::Denied)
    );
    assert_eq!(svc.statistics(request.inviter).await, Err(Error::Denied));
    checks.allow.store(true, Ordering::SeqCst);
    sqlx::query("UPDATE referral_invitations SET expires_at=created_at+interval '1 microsecond'")
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        svc.claim(request.invitee, request.id).await,
        Err(Error::Denied)
    );
    assert_eq!(svc.statistics(request.inviter).await.unwrap().expired, 1);
    assert_eq!(db.count("wallet_entries").await, 0);
    db.close().await;
}
#[tokio::test]
async fn postgres_wallet_and_notification_refusal_roll_back() {
    let Some(db) = Database::create().await else {
        return;
    };
    let checks = Checks::new();
    let svc = service(db.pool.clone(), checks.clone(), true);
    let request = invitation(Kind::New);
    checks.notify.store(false, Ordering::SeqCst);
    assert_eq!(svc.invite(&request).await, Err(Error::Denied));
    assert_eq!(db.count("referral_invitations").await, 0);
    checks.notify.store(true, Ordering::SeqCst);
    svc.invite(&request).await.unwrap();
    checks.audit.store(false, Ordering::SeqCst);
    assert_eq!(
        svc.claim(request.invitee, request.id).await,
        Err(Error::Denied)
    );
    checks.audit.store(true, Ordering::SeqCst);
    checks.notify.store(false, Ordering::SeqCst);
    assert_eq!(
        svc.claim(request.invitee, request.id).await,
        Err(Error::Denied)
    );
    assert_eq!(db.count("wallet_entries").await, 0);
    assert_eq!(db.count("wallet_operations").await, 0);
    assert_eq!(db.count("notifications").await, 1);
    assert_eq!(svc.statistics(request.inviter).await.unwrap().rewarded, 0);
    checks.notify.store(true, Ordering::SeqCst);
    svc.claim(request.invitee, request.id).await.unwrap();
    assert_eq!(db.count("wallet_entries").await, 1);
    db.close().await;
}
#[tokio::test]
async fn postgres_concurrent_duplicate_invitees() {
    let Some(db) = Database::create().await else {
        return;
    };
    let svc = service(db.pool.clone(), Checks::new(), true);
    let first = invitation(Kind::New);
    let mut second = invitation(Kind::Returning);
    second.invitee = first.invitee;
    let (a, b) = tokio::join!(svc.invite(&first), svc.invite(&second));
    assert!(matches!(
        (a, b),
        (Ok(()), Err(Error::Denied)) | (Err(Error::Denied), Ok(()))
    ));
    assert_eq!(db.count("referral_invitations").await, 1);
    assert_eq!(db.count("notifications").await, 1);
    db.close().await;
}

#[test]
fn localized_notifications() {
    for locale in [oteryn_notify::Locale::Pl, oteryn_notify::Locale::En] {
        for key in ["invited", "rewarded"] {
            let rendered = notification(locale, key).unwrap().render(locale).unwrap();
            assert!(!rendered.subject.is_empty());
            assert!(!rendered.text.is_empty());
        }
    }
    assert!(notification(oteryn_notify::Locale::En, "unknown").is_err());
}

#[tokio::test]
async fn postgres_transitive_cycle_and_email_replay() {
    let Some(db) = Database::create().await else {
        return;
    };
    let svc = service(db.pool.clone(), Checks::new(), true);
    let first = invitation(Kind::New);
    for account in [first.inviter, first.invitee] {
        sqlx::query("INSERT INTO account_notification_prefs(account_id,category,email_enabled,portal_enabled) VALUES($1,'news',true,true)")
            .bind(account).execute(&db.pool).await.unwrap();
    }
    svc.invite(&first).await.unwrap();
    svc.invite(&first).await.unwrap();
    let mut second = invitation(Kind::Returning);
    second.inviter = first.invitee;
    svc.invite(&second).await.unwrap();
    let mut third = invitation(Kind::New);
    third.inviter = second.invitee;
    third.invitee = first.inviter;
    assert_eq!(svc.invite(&third).await, Err(Error::Denied));
    svc.claim(first.invitee, first.id).await.unwrap();
    svc.claim(first.invitee, first.id).await.unwrap();
    assert_eq!(db.count("email_outbox").await, 2);
    assert_eq!(db.count("outbox").await, 2);
    assert_eq!(db.count("notifications").await, 3);
    db.close().await;
}
