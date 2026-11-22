use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Method, Request},
};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ROUTE: &str = "/en/account/bazaar/activity";

#[tokio::test]
async fn activity_disabled_and_anonymous_refusal() -> TestResult {
    let db = sqlx::postgres::PgPoolOptions::new().connect_lazy("postgresql://localhost/unused")?;
    for (enabled, expected) in [
        (false, StatusCode::NOT_FOUND),
        (true, StatusCode::SEE_OTHER),
    ] {
        let response = router(db.clone(), enabled)
            .oneshot(Request::get(ROUTE).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), expected);
        if enabled {
            assert_eq!(response.headers()[header::LOCATION], "/en/login");
        }
    }
    Ok(())
}

async fn signed_in(pool: &PgPool) -> Result<(Uuid, String), sqlx::Error> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO accounts(email,password_hash,status,email_verified_at,locale)
         VALUES ($1,'test-only','active',now(),'en') RETURNING id",
    )
    .bind(format!("{}@example.test", Uuid::now_v7()))
    .fetch_one(pool)
    .await?;
    let token = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
    sqlx::query(
        "INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token)
         VALUES ($1,$2,now()+interval '15 min',now()+interval '1 hour','test',repeat('a',64))",
    )
    .bind(Sha256::digest(token.as_bytes()).to_vec())
    .bind(id)
    .execute(pool)
    .await?;
    Ok((id, format!("__Host-oteryn_session={token}")))
}

#[tokio::test]
async fn activity_reads_only_session_owned_rows_and_rejects_invalid_requests() -> TestResult {
    let Ok(database_url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Bazaar activity integration test requires PORTAL_TEST_DATABASE_URL");
        return Ok(());
    };
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let pool = PgPool::connect(&database_url).await?;
    oteryn_db::migrate(&pool).await?;
    let (seller, seller_cookie) = signed_in(&pool).await?;
    let (buyer, buyer_cookie) = signed_in(&pool).await?;
    let (_, stranger_cookie) = signed_in(&pool).await?;
    let listing = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO bazaar_listings(id, owner, kind, form, currency, price, accept_offers, ends_at, state)
         VALUES ($1,$2,'character','auction','coins',100,false,now(),'awaiting_delivery')",
    ).bind(listing).bind(seller).execute(&pool).await?;
    sqlx::query(
        "INSERT INTO bazaar_bids(id, listing_id, account, maximum, idempotency_key, leader, price, ends_at)
         VALUES ($1,$2,$3,371,'activity-test',$3,100,now())",
    ).bind(Uuid::now_v7()).bind(listing).bind(buyer).execute(&pool).await?;
    sqlx::query(
        "INSERT INTO bazaar_settlements(id, listing_id, buyer, seller, currency, amount, commission, seller_net, reservation, idempotency_key)
         VALUES ($1,$2,$3,$4,'coins',100,10,90,$5,'activity-test')",
    ).bind(Uuid::now_v7()).bind(listing).bind(buyer).bind(seller).bind(Uuid::now_v7())
        .execute(&pool).await?;

    let app = router(pool.clone(), true);
    for (cookie, expected, own_listing, bid) in [
        (seller_cookie.as_str(), StatusCode::OK, true, false),
        (buyer_cookie.as_str(), StatusCode::OK, false, true),
        (
            stranger_cookie.as_str(),
            StatusCode::NOT_FOUND,
            false,
            false,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("{ROUTE}?listing={listing}"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), expected);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        if expected == StatusCode::OK {
            let html = String::from_utf8(to_bytes(response.into_body(), 32768).await?.to_vec())?;
            assert_eq!(html.contains("Listed price 100 Coins"), own_listing);
            assert_eq!(html.contains("Maximum bid 371 Coins"), bid);
            assert!(html.contains("Settlement pending"));
            assert!(html.contains("<time datetime="));
        }
    }
    sqlx::query(
        "INSERT INTO bazaar_authority_assets
         (listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,
          session_hash,quantity,publication_consent,input_binding,refusal_code)
         VALUES ($1,$1,$1,$1,$1,1,$2,1,true,'activity-test','CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE')",
    )
    .bind(listing)
    .bind([0_u8; 32].as_slice())
    .execute(&pool)
    .await?;
    let refused = app
        .clone()
        .oneshot(
            Request::get(format!("{ROUTE}?listing={listing}"))
                .header(header::COOKIE, &buyer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(refused.status(), StatusCode::OK);
    let html = String::from_utf8(to_bytes(refused.into_body(), 32768).await?.to_vec())?;
    assert!(html.contains("Delivery refused"));
    assert!(!html.contains("Settlement pending"));
    assert!(!html.contains("CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE"));

    for (method, uri, expected) in [
        (
            Method::GET,
            format!("{ROUTE}?listing=invalid"),
            StatusCode::BAD_REQUEST,
        ),
        (
            Method::GET,
            format!("{ROUTE}?account={seller}"),
            StatusCode::BAD_REQUEST,
        ),
        (Method::POST, ROUTE.to_owned(), StatusCode::FORBIDDEN),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header(header::COOKIE, &buyer_cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), expected);
    }
    Ok(())
}
