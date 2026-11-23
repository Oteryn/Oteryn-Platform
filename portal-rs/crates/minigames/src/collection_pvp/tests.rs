use super::*;
use serde_json::json;
#[derive(Debug, Clone, Copy)]
struct Rules;
impl Collection for Rules {
    fn available(&self) -> bool {
        true
    }
    fn view(&self, state: &Value, _: Side) -> Result<Value> {
        Ok(json!({"turn":state["turn"]}))
    }
    async fn deck(
        &self,
        _: &mut Transaction<'_, Postgres>,
        owner: Uuid,
        deck: Uuid,
    ) -> Result<Value> {
        if owner != deck {
            return Err(Error::Refused);
        }
        Ok(json!({"owner":owner}))
    }
    async fn revalidate(
        &self,
        _: &mut Transaction<'_, Postgres>,
        owner: Uuid,
        snapshot: &Value,
    ) -> Result<()> {
        if snapshot["owner"] == json!(owner) {
            Ok(())
        } else {
            Err(Error::Refused)
        }
    }
    fn begin(&self, left: &Value, right: &Value) -> Result<Value> {
        Ok(json!({"left":left,"right":right,"turn":0}))
    }
    fn turn(&self, state: &Value, side: Side, action: &Value) -> Result<Turn> {
        if side != Side::Left || action != &json!({"attack":1}) || state["turn"] != 0 {
            return Err(Error::Invalid);
        }
        Ok(Turn {
            state: json!({"turn":1}),
            outcome: Some(Outcome::Left),
        })
    }
    async fn grant(
        &self,
        conn: &mut Transaction<'_, Postgres>,
        owner: Uuid,
        cards: &Value,
    ) -> Result<()> {
        sqlx::query("INSERT INTO t91_test_grants(owner,cards) VALUES($1,$2)")
            .bind(owner)
            .bind(cards)
            .execute(&mut **conn)
            .await?;
        Ok(())
    }
}
impl SetPayment for Rules {
    async fn debit(
        &self,
        conn: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        owner: Uuid,
        amount: i64,
        _: Uuid,
    ) -> Result<()> {
        let changed = sqlx::query(
            "UPDATE t91_test_balances SET balance=balance-$2 WHERE owner=$1 AND balance >= $2",
        )
        .bind(owner)
        .bind(amount)
        .execute(&mut **conn)
        .await?
        .rows_affected();
        if changed == 1 {
            Ok(())
        } else {
            Err(Error::Refused)
        }
    }
}
async fn database() -> std::result::Result<Option<PgPool>, Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("PostgreSQL test skipped: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(None);
    };
    let admin = PgPool::connect(&url).await?;
    let schema = format!("t91_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    admin.close().await;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |c, _| {
            let search = format!("SET search_path TO {schema}");
            Box::pin(async move {
                sqlx::query(sqlx::AssertSqlSafe(search)).execute(c).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    // Real identity and value eligibility are part of the purchase contract.
    oteryn_db::migrate(&pool).await?;
    sqlx::raw_sql("CREATE TABLE t91_test_balances(owner uuid PRIMARY KEY,balance bigint NOT NULL); CREATE TABLE t91_test_grants(owner uuid,cards jsonb);").execute(&pool).await?;
    Ok(Some(pool))
}
async fn account(pool: &PgPool) -> Result<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale) VALUES($1,$1::text||'@example.invalid','test','active',now(),'en')")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(id)
}
async fn cleanup(pool: PgPool) -> std::result::Result<(), sqlx::Error> {
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&pool)
        .await?;
    assert!(
        schema.starts_with("t91_")
            && schema
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    );
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(())
}
#[tokio::test]
async fn postgres_battle_consent_authorization_replay_and_rankings()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let a = account(&pool).await?;
    let b = account(&pool).await?;
    let outsider = account(&pool).await?;
    let s = Service::new(
        pool.clone(),
        Flags {
            pvp: true,
            sets: false,
        },
        Rules,
        Rules,
    );
    assert!(matches!(s.challenge(a, a, a).await, Err(Error::Invalid)));
    assert!(matches!(s.challenge(a, b, b).await, Err(Error::Refused)));
    let id = s.challenge(a, b, a).await?;
    assert!(matches!(s.battle(outsider, id).await, Err(Error::Refused)));
    assert!(matches!(s.accept(a, id, a).await, Err(Error::Refused)));
    s.accept(b, id, b).await?;
    let visible = s.battle(a, id).await?;
    assert!(visible.state.get("left").is_none());
    assert!(matches!(
        s.play(a, id, 0, json!({"attack":1})).await,
        Err(Error::Refused)
    ));
    assert!(matches!(
        s.play(a, id, 1, json!({"attack":9})).await,
        Err(Error::Invalid)
    ));
    let (first, second) = tokio::join!(
        s.play(a, id, 1, json!({"attack":1})),
        s.play(a, id, 1, json!({"attack":1}))
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let rank = s.rankings(None).await?;
    assert_eq!(rank.len(), 2);
    assert_eq!(rank[0].account_id, a);
    assert_eq!(rank[0].points, 3);
    let declined = s.challenge(a, b, a).await?;
    s.decline(b, declined).await?;
    assert!(matches!(
        s.accept(b, declined, b).await,
        Err(Error::Refused)
    ));
    cleanup(pool).await?;
    Ok(())
}
#[tokio::test]
async fn postgres_tournament_capacity_ownership_and_completion()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let a = account(&pool).await?;
    let b = account(&pool).await?;
    let c = account(&pool).await?;
    let s = Service::new(
        pool.clone(),
        Flags {
            pvp: true,
            sets: false,
        },
        Rules,
        Rules,
    );
    assert!(matches!(
        s.create_tournament(a, 1).await,
        Err(Error::Invalid)
    ));
    let id = s.create_tournament(a, 2).await?;
    s.join(a, id, a).await?;
    assert!(matches!(s.start(a, id).await, Err(Error::Refused)));
    s.join(b, id, b).await?;
    assert!(matches!(s.join(c, id, c).await, Err(Error::Refused)));
    assert!(matches!(s.start(b, id).await, Err(Error::Refused)));
    s.start(a, id).await?;
    assert!(matches!(s.start(a, id).await, Err(Error::Refused)));
    let (battle, left): (Uuid, Uuid) =
        sqlx::query_as("SELECT id,left_account FROM collection_pvp_battles WHERE tournament_id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await?;
    s.play(left, battle, 0, json!({"attack":1})).await?;
    let status: String =
        sqlx::query_scalar("SELECT status FROM collection_pvp_tournaments WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(status, "completed");
    assert_eq!(s.rankings(Some(id)).await?.len(), 2);
    cleanup(pool).await?;
    Ok(())
}
#[tokio::test]
async fn postgres_fixed_contents_price_replay_and_atomic_refusal()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let a = account(&pool).await?;
    let set = Uuid::now_v7();
    let request = Uuid::now_v7();
    sqlx::query("INSERT INTO t91_test_balances VALUES($1,20)")
        .bind(a)
        .execute(&pool)
        .await?;
    sqlx::query(
        "INSERT INTO collection_pvp_sets(id,version,price,cards,published) VALUES($1,1,10,$2,true)",
    )
    .bind(set)
    .bind(json!({"monster-a":2}))
    .execute(&pool)
    .await?;
    let s = Service::new(
        pool.clone(),
        Flags {
            pvp: false,
            sets: true,
        },
        Rules,
        Rules,
    );
    assert_eq!(s.sets().await?[0].cards, json!({"monster-a":2}));
    assert!(matches!(
        s.buy_set(a, set, 1, 9, request).await,
        Err(Error::Refused)
    ));
    assert!(matches!(
        s.buy_set(a, set, 1, 10, Uuid::nil()).await,
        Err(Error::Invalid)
    ));
    let (first, second) = tokio::join!(
        s.buy_set(a, set, 1, 10, request),
        s.buy_set(a, set, 1, 10, request)
    );
    first?;
    second?;
    assert!(
        sqlx::query("UPDATE collection_pvp_sets SET price=11 WHERE id=$1")
            .bind(set)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(matches!(
        s.buy_set(a, set, 2, 10, request).await,
        Err(Error::Refused)
    ));
    let balance: i64 = sqlx::query_scalar("SELECT balance FROM t91_test_balances WHERE owner=$1")
        .bind(a)
        .fetch_one(&pool)
        .await?;
    assert_eq!(balance, 10);
    let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM t91_test_grants")
        .fetch_one(&pool)
        .await?;
    assert_eq!(grants, 1);
    assert!(matches!(
        s.buy_set(a, set, 1, 10, Uuid::now_v7()).await,
        Err(Error::Refused)
    ));
    let unavailable_set = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO collection_pvp_sets(id,version,price,cards,published) VALUES($1,1,10,$2,true)",
    )
    .bind(unavailable_set)
    .bind(json!({"monster-b":1}))
    .execute(&pool)
    .await?;
    let failing = Service::new(
        pool.clone(),
        Flags {
            pvp: false,
            sets: true,
        },
        Unavailable,
        Rules,
    );
    assert!(matches!(
        failing
            .buy_set(a, unavailable_set, 1, 10, Uuid::now_v7())
            .await,
        Err(Error::Unavailable)
    ));
    let balance: i64 = sqlx::query_scalar("SELECT balance FROM t91_test_balances WHERE owner=$1")
        .bind(a)
        .fetch_one(&pool)
        .await?;
    assert_eq!(balance, 10);
    cleanup(pool).await?;
    Ok(())
}
#[tokio::test]
async fn flags_and_missing_adapters_deny_without_io() -> Result<()> {
    let pool = sqlx::postgres::PgPoolOptions::new().connect_lazy("postgres://localhost/unused")?;
    let s = Service::new(pool, Flags::default(), Unavailable, Unavailable);
    assert!(matches!(
        s.challenge(Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7())
            .await,
        Err(Error::Unavailable)
    ));
    assert!(matches!(s.sets().await, Err(Error::Unavailable)));
    Ok(())
}

#[test]
fn fixed_contents_reject_unknown_random_and_bad_quantities() {
    for value in [
        json!({}),
        json!([]),
        json!({"rat":0}),
        json!({"rat":-1}),
        json!({"rat":1.5}),
        json!({"rat":"random"}),
        json!({"rat":101}),
    ] {
        assert!(validate_cards(&value).is_err());
    }
    assert!(validate_cards(&json!({"rat":2,"wolf":1})).is_ok());
}

#[tokio::test]
async fn postgres_browser_success_refusal_csrf_and_bad_input()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    use axum::{
        Extension, Router,
        body::{Body, to_bytes},
        extract::State,
        http::{Request, StatusCode, header},
        middleware,
        routing::get,
    };
    use oteryn_web::{
        csrf::CsrfConfig,
        session::{self, BrowserSession, PrivilegeChange, SessionLayer},
    };
    use tower::ServiceExt;
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let a = account(&pool).await?;
    async fn login(
        State(actor): State<Uuid>,
        Extension(session): Extension<BrowserSession>,
    ) -> std::result::Result<String, StatusCode> {
        session::rotate(&session, PrivilegeChange::Login(actor))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        session
            .csrf_token()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
    let login = Router::new()
        .route("/login-test", get(login))
        .route_layer(middleware::from_fn_with_state(
            SessionLayer::new(pool.clone()),
            session::middleware,
        ))
        .with_state(a);
    let response = login
        .oneshot(Request::get("/login-test").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()?
        .split(';')
        .next()
        .ok_or("missing session cookie")?
        .to_owned();
    let csrf = String::from_utf8(to_bytes(response.into_body(), 4096).await?.to_vec())?;
    let app = http::router(
        Service::new(
            pool.clone(),
            Flags {
                pvp: true,
                sets: true,
            },
            Rules,
            Rules,
        ),
        CsrfConfig {
            trusted_origins: vec!["https://portal.example.invalid".into()],
            ..CsrfConfig::default()
        },
    );
    for (lang, title) in [("pl", "Kolekcja potworów"), ("en", "Monster collection")] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/{lang}/collection-pvp"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::REFERRER_POLICY], "same-origin");
        let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
        assert!(html.contains(title));
        assert!(html.contains(&csrf));
        assert!(html.contains("width=device-width"));
        if let Ok(directory) = std::env::var("PORTAL_TEST_RENDER_DIR") {
            std::fs::write(
                std::path::Path::new(&directory).join(format!("collection-pvp-{lang}.html")),
                html,
            )?;
        }
    }
    for (body, status) in [
        (
            format!("op=create&capacity=2&_csrf={csrf}"),
            StatusCode::SEE_OTHER,
        ),
        (
            format!("op=create&capacity=bad&_csrf={csrf}"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            format!("op=start&id={}&_csrf={csrf}", Uuid::now_v7()),
            StatusCode::FORBIDDEN,
        ),
        ("op=create&capacity=2".into(), StatusCode::FORBIDDEN),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post("/en/collection-pvp")
                    .header(header::COOKIE, &cookie)
                    .header(header::ORIGIN, "https://portal.example.invalid")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body))?,
            )
            .await?;
        assert_eq!(response.status(), status);
    }
    let unsupported = app
        .clone()
        .oneshot(
            Request::put("/en/collection-pvp")
                .header(header::COOKIE, &cookie)
                .header(header::ORIGIN, "https://portal.example.invalid")
                .header("x-csrf-token", &csrf)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(unsupported.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert!(
        unsupported.headers()[header::ALLOW]
            .to_str()?
            .contains("POST")
    );

    use sha2::{Digest, Sha256};
    sqlx::query(
        "INSERT INTO web_rate_limits(limiter_name,key_hash,window_start,count)
         SELECT 'collection-pvp-action',$1,date_trunc('minute',statement_timestamp())+delta,60
         FROM unnest(ARRAY[interval '0',interval '1 minute']) AS offsets(delta)
         ON CONFLICT(limiter_name,key_hash,window_start) DO UPDATE SET count=60",
    )
    .bind(Sha256::digest(a.as_bytes()).to_vec())
    .execute(&pool)
    .await?;
    let response = app
        .oneshot(
            Request::post("/en/collection-pvp")
                .header(header::COOKIE, &cookie)
                .header(header::ORIGIN, "https://portal.example.invalid")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!("op=create&capacity=2&_csrf={csrf}")))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after: u32 = response.headers()[header::RETRY_AFTER].to_str()?.parse()?;
    assert!((1..=60).contains(&retry_after));
    let tournaments: i64 =
        sqlx::query_scalar("SELECT count(*) FROM collection_pvp_tournaments WHERE organizer=$1")
            .bind(a)
            .fetch_one(&pool)
            .await?;
    assert_eq!(tournaments, 1);
    let disabled = http::router(
        Service::new(pool.clone(), Flags::default(), Unavailable, Unavailable),
        CsrfConfig::default(),
    );
    assert_eq!(
        disabled
            .oneshot(Request::get("/en/collection-pvp").body(Body::empty())?)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    cleanup(pool).await?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct WalletAuthority;
impl oteryn_wallet::AccountStateReader for WalletAuthority {
    async fn state(&self, _: &mut sqlx::PgConnection, _: Uuid) -> oteryn_wallet::AccountState {
        oteryn_wallet::AccountState::Active
    }
}
impl oteryn_wallet::TransferBlockCheck for WalletAuthority {
    async fn permission(
        &self,
        _: &mut sqlx::PgConnection,
        _: Uuid,
    ) -> oteryn_wallet::TransferPermission {
        oteryn_wallet::TransferPermission::Allowed
    }
}
impl oteryn_wallet::AuditHook for WalletAuthority {
    async fn adjustment(
        &self,
        _: &mut sqlx::PgConnection,
        _: &oteryn_wallet::AdjustmentAudit,
    ) -> bool {
        true
    }
}

#[tokio::test]
async fn postgres_store_wallet_receipt_cannot_buy_a_set()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("PostgreSQL test skipped: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = PgPool::connect(&url).await?;
    let schema = format!("t91_wallet_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    admin.close().await;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |c, _| {
            let search = schema.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(search)
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    sqlx::query("CREATE TABLE t91_test_grants(owner uuid,cards jsonb)")
        .execute(&pool)
        .await?;
    let owner = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,email_verified_at,locale) VALUES($1,'set@example.invalid','unused','active',now(),'en')").bind(owner).execute(&pool).await?;
    let ledger = oteryn_wallet::Ledger::new(
        pool.clone(),
        oteryn_wallet::WalletFlags { enabled: true },
        WalletAuthority,
        WalletAuthority,
        WalletAuthority,
    );
    ledger
        .adjust(Uuid::now_v7(), owner, 200, owner, "synthetic test credit")
        .await?;
    let operation = Uuid::now_v7();
    let mut store_tx = pool.begin().await?;
    let receipt = ledger
        .clone()
        .with_adjustment_kind(oteryn_wallet::AdjustmentKind::StorePurchase)
        .adjust_in(
            &mut store_tx,
            operation,
            owner,
            -10,
            owner,
            &format!("store-purchase:{operation}"),
        )
        .await?;
    assert!(!receipt.replayed);
    store_tx.commit().await?;
    let set = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO collection_pvp_sets(id,version,price,cards,published) VALUES($1,1,10,$2,true)",
    )
    .bind(set)
    .bind(json!({"monster-a":2}))
    .execute(&pool)
    .await?;
    let service = Service::new(
        pool.clone(),
        Flags {
            pvp: false,
            sets: true,
        },
        Rules,
        WalletPayment::new(ledger.clone()),
    );
    assert!(matches!(
        service.buy_set(owner, set, 1, 10, operation).await,
        Err(Error::Refused)
    ));
    let grants: i64 = sqlx::query_scalar("SELECT count(*) FROM t91_test_grants")
        .fetch_one(&pool)
        .await?;
    assert_eq!(grants, 0);
    assert_eq!(ledger.balance(owner).await?.available, 190);
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use sha2::{Digest, Sha256};
    use tower::ServiceExt;
    let token = "a".repeat(64);
    let csrf = "b".repeat(64);
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at)
        VALUES($1,$2,$3,'card set test',now()+interval '1 hour',now()+interval '1 day')")
        .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(owner).bind(&csrf).execute(&pool).await?;
    oteryn_db::test_support::coin_chargeback_hold(&pool, owner).await?;
    let app = http::router(
        Service::new(
            pool.clone(),
            Flags {
                pvp: false,
                sets: true,
            },
            Rules,
            WalletPayment::new(ledger.clone()),
        ),
        oteryn_web::csrf::CsrfConfig {
            trusted_origins: vec!["https://portal.example.invalid".into()],
            ..Default::default()
        },
    );
    for held in [true, false] {
        if !held {
            sqlx::query("DELETE FROM coin_purchase_holds WHERE account_id=$1")
                .bind(owner)
                .execute(&pool)
                .await?;
        }
        let response = app
            .clone()
            .oneshot(
                Request::post("/en/collection-pvp")
                    .header("cookie", format!("__Host-oteryn_session={token}"))
                    .header("origin", "https://portal.example.invalid")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!(
                        "_csrf={csrf}&op=buy&id={set}&version=1&price=10&request={}",
                        Uuid::now_v7()
                    )))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    ledger
        .adjust(Uuid::now_v7(), owner, 10, owner, "Synthetic debt repayment")
        .await?;
    // A fresh local purchase and its own retry remain valid.
    let fresh = Uuid::now_v7();
    service.buy_set(owner, set, 1, 10, fresh).await?;
    service.buy_set(owner, set, 1, 10, fresh).await?;
    assert_eq!(ledger.balance(owner).await?.available, 180);
    cleanup(pool).await?;
    Ok(())
}

#[tokio::test]
async fn lock_order_challenge_prepares_all_accounts_before_waiting()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let pool = database()
        .await?
        .ok_or("PostgreSQL is required for lock-order regression")?;
    let earlier = account(&pool).await?;
    let later = account(&pool).await?;
    assert!(earlier < later);
    let mut writer = pool.begin().await?;
    let blocker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(earlier)
        .execute(&mut *writer)
        .await?;
    let service = Service::new(
        pool.clone(),
        Flags {
            pvp: true,
            sets: false,
        },
        Rules,
        Rules,
    );
    let challenge = tokio::spawn(async move { service.challenge(later, earlier, later).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)) AND wait_event_type='Lock')")
                .bind(blocker).fetch_one(&pool).await?;
            if waiting { return Ok::<_, sqlx::Error>(()); }
            tokio::task::yield_now().await;
        }
    }).await.map_err(|_| "challenge did not reach the observable account lock wait within 2 seconds")??;
    sqlx::query("SET LOCAL lock_timeout='250ms'")
        .execute(&mut *writer)
        .await?;
    let later_lock = sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(later)
        .execute(&mut *writer)
        .await;
    writer.rollback().await?;
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(4), challenge)
        .await
        .map_err(|_| "challenge did not finish after releasing account blocker")??;
    cleanup(pool).await?;
    assert!(
        later_lock.is_ok(),
        "challenge retained the later account while waiting for the earlier one: {later_lock:?}"
    );
    outcome?;
    Ok(())
}

#[tokio::test]
async fn postgres_published_cards_fund_a_qualifying_tournament()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    use crate::collection;
    use oteryn_wallet::{Ledger, WalletFlags, wire::DatabasePolicy};
    use oteryn_world::{WorldOptions, WorldStore};

    let Some(pool) = database().await? else {
        return Ok(());
    };
    let left = account(&pool).await?;
    let right = account(&pool).await?;
    let world = Uuid::now_v7();
    let card = Uuid::now_v7();
    let set = Uuid::now_v7();
    let bad_set = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO world_versions(id,source_commit,status) VALUES($1,repeat('a',40),'ready')",
    )
    .bind(world)
    .execute(&pool)
    .await?;
    sqlx::query("UPDATE world_active_version SET version_id=$1 WHERE singleton")
        .bind(world)
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES($1,'Creature','rat',$2,true)")
        .bind(world).bind(json!({
            "definition":{"identity":{"family":"Creature","key":"rat"},"client_projection":"ClientSafe"},
            "authoring":{"profile":{"details":{"display_name":"Rat","bestiary":{}}}}
        })).execute(&pool).await?;
    sqlx::query("INSERT INTO world_visibility(kind,key,allowed) VALUES('Creature','rat',true)")
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO collection_cards(id,definition,sprite) VALUES($1,$2,decode('01','hex'))")
        .bind(card).bind(json!({
            "id":card,"world_version":world,"kind":"Creature","key":"rat","race_key":"rat",
            "race_revision":"1","final_kill_threshold":1,"name":{"en":"Rat","pl":"Szczur"},"appearance_id":1
        })).execute(&pool).await?;
    for (sku, contents) in [
        (set, json!({card.to_string():2})),
        (bad_set, json!({"rat":1})),
    ] {
        sqlx::query("INSERT INTO collection_pvp_sets(id,version,price,cards,published) VALUES($1,1,10,$2,true)")
            .bind(sku).bind(contents).execute(&pool).await?;
    }
    let catalogue = collection::Collection::new(
        pool.clone(),
        WorldStore::new(pool.clone(), WorldOptions { enabled: true }),
        oteryn_playerdata::progress::ProgressStore::new(
            pool.clone(),
            Default::default(),
            Vec::new(),
        ),
        collection::Flags { collection: true },
        None,
    );
    let collection = PlatformCollection::new(catalogue, true);
    let service = Service::new(
        pool.clone(),
        Flags {
            pvp: true,
            sets: true,
        },
        collection,
        platform::payment(pool.clone(), true),
    );
    let wallet = Ledger::new(
        pool.clone(),
        WalletFlags { enabled: true },
        DatabasePolicy,
        DatabasePolicy,
        WalletAuthority,
    );
    assert!(matches!(
        service.buy_set(left, set, 1, 10, Uuid::now_v7()).await,
        Err(Error::Refused)
    ));
    assert!(matches!(
        service.save_deck(left, "Rat", &[card]).await,
        Err(Error::Refused)
    ));
    for actor in [left, right] {
        wallet
            .adjust(Uuid::now_v7(), actor, 40, actor, "Test credit")
            .await?;
        assert!(matches!(
            service.buy_set(actor, bad_set, 1, 10, Uuid::now_v7()).await,
            Err(Error::Invalid)
        ));
        assert_eq!(wallet.balance(actor).await?.available, 40);
        let operation = Uuid::now_v7();
        service.buy_set(actor, set, 1, 10, operation).await?;
        service.buy_set(actor, set, 1, 10, operation).await?;
        assert!(matches!(
            service.buy_set(actor, set, 1, 10, Uuid::now_v7()).await,
            Err(Error::Refused)
        ));
        assert_eq!(wallet.balance(actor).await?.available, 30);
        let entitlement: Value = sqlx::query_scalar(
            "SELECT cards FROM collection_pvp_purchases WHERE id=$1 AND account_id=$2",
        )
        .bind(operation)
        .bind(actor)
        .fetch_one(&pool)
        .await?;
        assert_eq!(entitlement, json!({card.to_string():2}));
        assert!(matches!(
            service.save_deck(actor, "Rat", &[card, card]).await,
            Err(Error::Invalid)
        ));
        service.save_deck(actor, "Rat", &[card]).await?;
    }
    let deck_id = async |owner| -> std::result::Result<Uuid, sqlx::Error> {
        sqlx::query_scalar("SELECT id FROM collection_pvp_owned_decks WHERE account_id=$1")
            .bind(owner)
            .fetch_one(&pool)
            .await
    };
    let left_deck = deck_id(left).await?;
    let right_deck = deck_id(right).await?;
    assert!(matches!(
        service.challenge(left, right, right_deck).await,
        Err(Error::Refused)
    ));
    let tournament = service.create_tournament(left, 2).await?;
    service.join(left, tournament, left_deck).await?;
    service.join(right, tournament, right_deck).await?;
    assert!(matches!(
        service.start(right, tournament).await,
        Err(Error::Refused)
    ));
    service.start(left, tournament).await?;
    let id: Uuid =
        sqlx::query_scalar("SELECT id FROM collection_pvp_battles WHERE tournament_id=$1")
            .bind(tournament)
            .fetch_one(&pool)
            .await?;
    let first = service.battle(left, id).await?;
    assert!(matches!(
        service
            .play(left, id, first.revision, json!("teleport"))
            .await,
        Err(Error::Invalid)
    ));
    service
        .play(left, id, first.revision, json!("strike"))
        .await?;
    let waiting = service.battle(left, id).await?;
    assert_eq!(waiting.state["waiting"], true);
    assert_eq!(service.battle(right, id).await?.state["waiting"], false);
    assert!(matches!(
        service
            .play(left, id, waiting.revision, json!("guard"))
            .await,
        Err(Error::Refused)
    ));
    service
        .play(right, id, waiting.revision, json!("strike"))
        .await?;
    for _ in 0..4 {
        let current = service.battle(left, id).await?;
        if current.status == "completed" {
            break;
        }
        service
            .play(left, id, current.revision, json!("strike"))
            .await?;
        service
            .play(right, id, current.revision + 1, json!("strike"))
            .await?;
    }
    assert_eq!(service.battle(left, id).await?.status, "completed");
    assert_eq!(service.tournaments(left).await?[0].status, "completed");
    assert_eq!(service.rankings(Some(tournament)).await?.len(), 2);
    for _ in 0..9 {
        service.create_tournament(left, 2).await?;
    }
    assert!(matches!(
        service.create_tournament(left, 2).await,
        Err(Error::Refused)
    ));
    service.save_deck(left, "Another deck", &[card]).await?;
    cleanup(pool).await?;
    Ok(())
}
