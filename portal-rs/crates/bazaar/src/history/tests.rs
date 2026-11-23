use super::*;

#[test]
fn bounded_filters_and_selections() -> Result<(), serde_urlencoded::de::Error> {
    for query in [
        "min_price=-1",
        "max_price=1000000000000",
        "min_price=25&max_price=24",
        "currency=us_dollars",
        "form=trade",
        "from=2026-02-29",
        "from=2024-02-29&to=2024-02-28",
    ] {
        let search: Search = serde_urlencoded::from_str(query)?;
        assert!(
            matches!(search.filters.validate(), Err(Error::InvalidInput)),
            "{query}"
        );
    }
    let search: Search = serde_urlencoded::from_str("from=2024-02-29&max_price=99&page=2")?;
    assert_eq!(search.filters.from, "2024-02-29");
    assert_eq!(search.page, 2);
    assert!(search.filters.validate().is_ok());
    let id = Uuid::now_v7();
    let search = Search {
        left: Some(id),
        right: Some(id),
        ..Default::default()
    };
    assert!(matches!(search.selection(), Err(Error::InvalidInput)));
    let form: Fields =
        serde_urlencoded::from_str("action=save&name=Example&min_price=123&_csrf=x")?;
    assert!(
        matches!(form.change(), Ok(Change::Save { filters, .. }) if filters.min_price == "123")
    );
    Ok(())
}

async fn seed_account(pool: &PgPool) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at)
         VALUES(uuidv7(),uuidv7()::text||'@example.invalid','test-only','active','en',now()) RETURNING id",
    ).fetch_one(pool).await
}

async fn seed_sale(
    pool: &PgPool,
    seller: Uuid,
    buyer: Uuid,
    amount: i64,
    completed: bool,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "WITH listing AS (
          INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at)
          VALUES(uuidv7(),$1,'item','fixed_price','coins',$3,false,
                 CASE WHEN $4 THEN 'completed'::bazaar_state ELSE 'awaiting_delivery'::bazaar_state END,now()) RETURNING id
         )
         INSERT INTO bazaar_settlements(id,listing_id,buyer,seller,currency,amount,commission,seller_net,reservation,idempotency_key,completed_at)
         SELECT uuidv7(),id,$2,$1,'coins',$3,$3/10,$3-$3/10,uuidv7(),id::text,CASE WHEN $4 THEN now() END FROM listing
         RETURNING listing_id",
    ).bind(seller).bind(buyer).bind(amount).bind(completed).fetch_one(pool).await
}

#[tokio::test]
async fn participant_history_and_market_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("bazaar_history_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .after_connect(move |db, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(db)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let seller = seed_account(&pool).await?;
    let buyer = seed_account(&pool).await?;
    let shopper = seed_account(&pool).await?;
    let amount = 31337;
    let listing = seed_sale(&pool, seller, buyer, amount, true).await?;
    let selection = [listing];
    let pending = seed_sale(&pool, seller, buyer, amount, false).await?;
    assert!(matches!(
        store::change(&pool, seller, Change::Share { sale: pending }).await,
        Err(Error::NotFound)
    ));
    let filters = Filters {
        min_price: amount.to_string(),
        max_price: amount.to_string(),
        ..Default::default()
    };
    let mut conn = pool.acquire().await?;
    assert_eq!(
        store::sales(&mut conn, Some(seller), &filters, Some(&selection), 0).await?[0].role,
        "sold"
    );
    assert_eq!(
        store::sales(&mut conn, Some(buyer), &filters, Some(&selection), 0).await?[0].role,
        "bought"
    );
    assert!(
        store::sales(&mut conn, Some(shopper), &filters, Some(&selection), 0)
            .await?
            .is_empty()
    );
    assert!(matches!(
        store::change(&pool, shopper, Change::Share { sale: listing }).await,
        Err(Error::NotFound)
    ));

    store::change(&pool, shopper, Change::EnableAlerts).await?;
    for name in ["Mine", "Also mine"] {
        store::change(
            &pool,
            shopper,
            Change::Save {
                name: name.to_owned(),
                filters: Filters {
                    min_price: amount.to_string(),
                    max_price: amount.to_string(),
                    ..Default::default()
                },
            },
        )
        .await?;
    }
    store::change(&pool, seller, Change::Share { sale: listing }).await?;
    assert!(store::account(&mut conn, shopper).await?.alerts.is_empty());
    store::change(&pool, buyer, Change::Share { sale: listing }).await?;
    let account = store::account(&mut conn, shopper).await?;
    let [notice] = account.alerts.as_slice() else {
        return Err("expected one saved-search alert".into());
    };
    assert_ne!(notice.id, listing);
    assert_eq!(notice.amount, amount);

    store::change(&pool, shopper, Change::Watch { sale: notice.id }).await?;
    store::change(&pool, shopper, Change::Watch { sale: notice.id }).await?;
    assert_eq!(store::account(&mut conn, shopper).await?.watched.len(), 1);
    store::change(&pool, shopper, Change::Dismiss { sale: notice.id }).await?;
    assert!(store::account(&mut conn, shopper).await?.alerts.is_empty());
    store::change(&pool, seller, Change::Unshare { sale: listing }).await?;
    assert!(
        store::sales(&mut conn, None, &filters, Some(&[notice.id]), 0)
            .await?
            .is_empty()
    );
    assert!(store::account(&mut conn, shopper).await?.watched.is_empty());
    let remaining: (bool, bool) = sqlx::query_as(
        "SELECT seller_at IS NOT NULL, buyer_at IS NOT NULL
         FROM bazaar_history_shares WHERE listing=$1",
    )
    .bind(listing)
    .fetch_one(&mut *conn)
    .await?;
    assert_eq!(remaining, (false, true));
    for table in ["bazaar_history_watches", "bazaar_history_dismissals"] {
        let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM {table} WHERE sale=$1"
        )))
        .bind(notice.id)
        .fetch_one(&mut *conn)
        .await?;
        assert_eq!(count, 0);
    }

    sqlx::query(
        "INSERT INTO bazaar_history_searches(account,name,criteria)
         SELECT $1,sequence::text,jsonb_build_object('currency','gold')
         FROM generate_series(1,48) sequence",
    )
    .bind(shopper)
    .execute(&pool)
    .await?;
    assert!(matches!(
        store::change(
            &pool,
            shopper,
            Change::Save {
                name: "Over limit".into(),
                filters: Filters::default()
            }
        )
        .await,
        Err(Error::Denied)
    ));
    assert!(matches!(
        store::change(
            &pool,
            shopper,
            Change::Save {
                name: "Invalid".into(),
                filters: Filters {
                    from: "2026-13-10".into(),
                    ..Default::default()
                }
            }
        )
        .await,
        Err(Error::InvalidInput)
    ));
    assert!(matches!(
        store::change(&pool, Uuid::now_v7(), Change::EnableAlerts).await,
        Err(Error::Denied)
    ));

    for value in 1..=5 {
        let id = seed_sale(&pool, seller, buyer, value * 100, true).await?;
        store::change(&pool, seller, Change::Share { sale: id }).await?;
        store::change(&pool, buyer, Change::Share { sale: id }).await?;
    }
    let prices = store::prices(&mut conn).await?;
    let reference = prices
        .iter()
        .find(|point| {
            point.form == "fixed_price" && point.currency == "coins" && point.month.is_none()
        })
        .ok_or("missing price reference")?;
    assert_eq!((reference.samples, reference.median), (5, 300));
    assert!(
        prices
            .iter()
            .any(|point| point.month.is_some() && point.median == 300)
    );

    store::change(&pool, seller, Change::Share { sale: listing }).await?;
    let account = store::account(&mut conn, shopper).await?;
    let [relisted] = account.alerts.as_slice() else {
        return Err("expected a match for the new publication".into());
    };
    assert_ne!(relisted.id, notice.id);
    for _ in 0..24 {
        let id = seed_sale(&pool, seller, buyer, amount, true).await?;
        store::change(&pool, seller, Change::Share { sale: id }).await?;
        store::change(&pool, buyer, Change::Share { sale: id }).await?;
    }
    assert!(
        store::account(&mut conn, shopper)
            .await?
            .alerts
            .iter()
            .all(|alert| alert.id != relisted.id)
    );
    store::change(&pool, shopper, Change::Dismiss { sale: relisted.id }).await?;
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM bazaar_history_dismissals WHERE account=$1 AND sale=$2)"
        )
        .bind(shopper)
        .bind(relisted.id)
        .fetch_one(&mut *conn)
        .await?
    );

    drop(conn);
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}
