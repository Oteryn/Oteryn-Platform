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
    async fn deck(&self, _: &mut PgConnection, owner: Uuid, deck: Uuid) -> Result<Value> {
        if owner != deck {
            return Err(Error::Refused);
        }
        Ok(json!({"owner":owner}))
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
    async fn grant(&self, conn: &mut PgConnection, owner: Uuid, cards: &Value) -> Result<()> {
        sqlx::query("INSERT INTO t91_test_grants(owner,cards) VALUES($1,$2)")
            .bind(owner)
            .bind(cards)
            .execute(conn)
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
    // Isolated account contract fixture; production migrations are exercised separately.
    sqlx::raw_sql("CREATE TABLE accounts(id uuid PRIMARY KEY,status text NOT NULL,email_verified_at timestamptz); CREATE TABLE t91_test_balances(owner uuid PRIMARY KEY,balance bigint NOT NULL); CREATE TABLE t91_test_grants(owner uuid,cards jsonb);").execute(&pool).await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105231211_t91_collection_pvp.sql"
    ))
    .execute(&pool)
    .await?;
    Ok(Some(pool))
}
async fn account(pool: &PgPool) -> Result<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts VALUES($1,'active',now())")
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
        failing.buy_set(a, set, 1, 10, Uuid::now_v7()).await,
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
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105144852_t02_web_sessions.sql"
    ))
    .execute(&pool)
    .await?;
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
