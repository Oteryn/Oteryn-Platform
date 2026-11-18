use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use oteryn_payments::coin::{CoinOrderRequest, MAX_COINS, OrderState, parse_packages};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn audit_09_01_real_wallet_serializes_ceiling_and_holds_paid_settlement() -> TestResult {
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let admin = PgPool::connect(&url).await?;
    let schema = format!("coin_ceiling_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |connection, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let payer: Uuid = sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,'test-hash','active','en',now()) RETURNING id")
        .bind(format!("{}@example.test",Uuid::now_v7())).fetch_one(&pool).await?;
    let client = Uuid::now_v7();
    sqlx::query("INSERT INTO oauth_native_clients(client_id,enabled,first_party,public_client,native_ticket) VALUES($1,true,true,true,true)")
        .bind(client).execute(&pool).await?;
    let config = oauth::Config {
        enabled: true,
        clients: vec![oauth::Client {
            client_id: client.to_string(),
            redirect_uris: vec!["http://127.0.0.1:41000/callback".into()],
        }],
    };
    let verifier = "v".repeat(43);
    let code = oauth::authorize(
        &pool,
        &config,
        payer,
        &oauth::Authorization {
            response_type: "code".into(),
            client_id: client.to_string(),
            redirect_uri: "http://127.0.0.1:41000/callback".into(),
            scope: oauth::ACCOUNT_SERVICES_SCOPE.into(),
            state: "ceiling-regression".into(),
            code_challenge: URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())),
            code_challenge_method: "S256".into(),
        },
    )
    .await?;
    let tokens = oauth::exchange(
        &pool,
        &config,
        &oauth::TokenRequest {
            grant_type: "authorization_code".into(),
            client_id: client.to_string(),
            redirect_uri: "http://127.0.0.1:41000/callback".into(),
            code,
            code_verifier: verifier,
            ..Default::default()
        },
    )
    .await?;
    let host = Arc::new(Host {
        pool: pool.clone(),
        oauth: config,
        projection: lcfa::Projection::new(
            pool.clone(),
            lcfa::Settings::default(),
            Arc::new(lcfa::DenyAccounts),
        ),
    });
    let mut tx = pool.begin().await?;
    host.credit_in(&mut tx, Uuid::now_v7(), payer, MAX_COINS - 1)
        .await?;
    tx.commit().await?;
    let reservation = Uuid::now_v7();
    host.ledger().reserve(reservation, payer, 1).await?;
    let credit = || async {
        let mut tx = pool.begin().await?;
        let result = host.credit_in(&mut tx, Uuid::now_v7(), payer, 1).await;
        tx.commit().await?;
        Ok::<_, sqlx::Error>(result)
    };
    let (left, right) = tokio::join!(credit(), credit());
    let outcomes = [left?, right?];
    assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|r| **r == Err(CoinError::Conflict))
            .count(),
        1
    );
    // A paid credit at the ceiling is valid when it repays debt in full.
    // This exercises the production Host, including its caller-owned transaction.
    let (quote, debt_order, reversal) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
    sqlx::query("INSERT INTO coin_purchase_quotes (id,payer_account_id,package_id,coins,minor_units,currency,exponent,environment,expires_at,consumed_at) VALUES ($1,$2,$3,2,100,'EUR',2,'testing',now(),now())")
        .bind(quote).bind(payer).bind(Uuid::now_v7()).execute(&pool).await?;
    sqlx::query("INSERT INTO coin_purchase_orders (id,payer_account_id,request_id,quote_id,package_id,coins,minor_units,currency,exponent,environment,payment_intent_id,state,code,review_required) VALUES ($1,$2,$3,$4,$5,2,100,'EUR',2,'testing',$6,'processing','review',true)")
        .bind(debt_order).bind(payer).bind(Uuid::now_v7()).bind(quote).bind(Uuid::now_v7()).bind(Uuid::now_v7()).execute(&pool).await?;
    sqlx::query("INSERT INTO coin_purchase_reversals (id,order_id,payer_account_id,kind,provider,provider_event_id,coins,debited,outcome) VALUES ($1,$2,$3,'chargeback','test',$4,2,0,'held')")
        .bind(reversal).bind(debt_order).bind(payer).bind(reversal.to_string()).execute(&pool).await?;
    let mut tx = pool.begin().await?;
    host.credit_in(&mut tx, Uuid::now_v7(), payer, 2).await?;
    tx.commit().await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT owed FROM coin_outstanding_debts WHERE reversal_id=$1"
        )
        .bind(reversal)
        .fetch_one(&pool)
        .await?,
        0
    );
    assert_eq!(host.wallet(payer).await?.total, MAX_COINS);
    let settings = CoinSettings {
        enabled: true,
        environment: "testing".into(),
        origin: "https://portal.example".into(),
        packages: parse_packages(
            r#"[{"id":"01934f10-7c02-7001-805b-3b1122334402","title":"One coin","coins":1,"price":{"minor_units":1,"currency":"EUR","exponent":2},"valid_for_seconds":120}]"#,
        ).ok_or("invalid test package")?,
    };
    let coins = CoinPurchases::new(
        pool.clone(),
        settings,
        TestProvider::new(&[9; 32])?,
        host.clone(),
    );
    let catalogue = coins.catalogue(payer, None).await?;
    let package = &catalogue.packages[0];
    let order = coins
        .create_order(
            payer,
            &CoinOrderRequest {
                request_id: Uuid::now_v7(),
                package_id: package.id,
                quote_id: package.quote_id,
                character_id: None,
            },
        )
        .await?;
    let held = coins.sandbox_complete(payer, order.order_id, true).await?;
    assert_eq!(held.state, OrderState::Processing);
    assert!(held.review_required);
    assert!(held.receipt_id.is_none());
    let balance = coins.balance(payer).await?;
    assert_eq!(balance.total_coins, MAX_COINS);
    assert!(balance.review_required);
    let response = coin_http::router(pool.clone(), Some(Arc::new(coins)))
        .oneshot(
            Request::get("/api/v1/native-account/coin-purchases/balance")
                .header("Authorization", format!("Bearer {}", tokens.access_token))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    host.ledger()
        .release(Uuid::now_v7(), payer, reservation)
        .await?;
    use oteryn_identity::termination::{Error, game_closure::runtime::Config};
    let closure = crate::account_lifecycle_http::request_test_termination(&pool, payer).await?;
    let denied = matches!(closure, Err(Error::UnknownState));
    let termination = match closure {
        Ok(termination) => termination,
        Err(Error::UnknownState) => sqlx::query_as("INSERT INTO account_terminations(account_id,scheduled_for) VALUES($1,now()+interval '14 days') RETURNING *")
            .bind(payer).fetch_one(&pool).await?,
        Err(error) => return Err(error.into()),
    };
    let boundary = Config::load(|key| match key {
        "PORTAL_ACCOUNT_CLOSURE_ENABLED" => Some("true".into()),
        "PORTAL_ACCOUNT_CLOSURE_RECOVERY_GENERATION" => Some("3".into()),
        _ => None,
    })?
    .boundary(
        Arc::new(crate::account_lifecycle_http::TerminationBoundary),
        true,
    )?
    .ok_or("missing closure boundary")?;
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(payer)
        .execute(&mut *tx)
        .await?;
    oteryn_identity::termination::Boundary::requested(boundary.as_ref(), &mut tx, &termination)
        .await?;
    tx.commit().await?;
    let request = format!(
        r#"{{"contract_version":2,"operation":"ReadAccountClosureLifecycleV1","source_authority":"oteryn:character-authority:primary","account_id":"{payer}","termination_id":"{}","recovery_generation":"3","audience":"OTERYN_PLATFORM_CHARACTER_AUTHORITY"}}"#,
        termination.id,
    );
    let observation: serde_json::Value =
        serde_json::from_slice(&boundary.read_lifecycle(&pool, request.as_bytes()).await?)?;
    assert_eq!(
        (denied, observation["financial_obligations_clear"].as_bool()),
        (true, Some(false))
    );
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    eprintln!(
        "PG audit_09_01_real_wallet_serializes_ceiling_and_holds_paid_settlement: {:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
