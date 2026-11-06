mod support;

use oteryn_bazaar::*;
use std::sync::atomic::Ordering;
use support::*;
use time::Duration;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[tokio::test]
async fn proxy_sequence_reserves_full_maximum_and_replays_original_response() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let l = h.listing(Form::Auction, Currency::Coins, 100).await?;
    let first = h.api.bid(l.id, h.a, 500, "first").await?;
    assert_eq!((first.leader, first.price), (h.a, 100));
    let second = h.api.bid(l.id, h.b, 200, "second").await?;
    assert_eq!((second.leader, second.price), (h.a, 201));
    let raised = h.api.bid(l.id, h.a, 700, "raise").await?;
    assert_eq!(raised.price, 201);
    assert_eq!(h.api.bid(l.id, h.a, 500, "first").await?, first);
    assert!(matches!(
        h.api.bid(l.id, h.a, 501, "first").await,
        Err(Error::IdempotencyConflict)
    ));
    let winner = h.api.bid(l.id, h.b, 900, "win").await?;
    assert_eq!((winner.leader, winner.price), (h.b, 703));
    let ledger = h.ledger().await?;
    assert_eq!(ledger.holds.len(), 1);
    assert_eq!(
        ledger.holds.values().next().map(|(_, amount)| *amount),
        Some(900)
    );
    assert!(matches!(
        h.api.bid(l.id, h.a, 1_000_001, "poor").await,
        Err(Error::InsufficientFunds)
    ));
    assert_eq!(h.api.get(l.id).await?.leader, Some(h.b));
    h.finish().await
}

#[tokio::test]
async fn anti_sniping_adds_five_minutes_once_per_request() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let l = h.listing(Form::Auction, Currency::Gold, 1_000).await?;
    let before = h.end_in(l.id, 60).await?;
    let bid = h.api.bid(l.id, h.a, 10_000, "late").await?;
    assert_eq!(bid.ends_at, before + Duration::minutes(5));
    assert_eq!(h.api.bid(l.id, h.a, 10_000, "late").await?, bid);
    assert!(matches!(
        h.api.bid(l.id, h.b, 1_999, "low").await,
        Err(Error::InvalidInput)
    ));
    h.finish().await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_bids_and_retries_keep_one_winner() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let l = h.listing(Form::Auction, Currency::Coins, 100).await?;
    let (a, b, retry) = tokio::join!(
        h.api.bid(l.id, h.a, 500, "a"),
        h.api.bid(l.id, h.b, 800, "b"),
        h.api.bid(l.id, h.b, 800, "b")
    );
    a?;
    assert_eq!(b?, retry?);
    let listing = h.api.get(l.id).await?;
    assert_eq!(listing.leader, Some(h.b));
    assert_eq!(listing.price, 501);
    assert_eq!(h.ledger().await?.holds.len(), 1);
    let bids: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_bids WHERE listing_id=$1")
        .bind(l.id)
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(bids, 2);
    h.finish().await
}

#[tokio::test]
async fn settlement_commission_is_exact_and_proceeds_wait_for_delivery() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    for (currency, amount, fee) in [(Currency::Coins, 101, 10), (Currency::Gold, 10_039, 250)] {
        let l = h.listing(Form::Auction, currency, amount).await?;
        h.api.bid(l.id, h.a, amount + 1_000, "bid").await?;
        assert!(matches!(
            h.api.settle(l.id, "settle").await,
            Err(Error::IllegalTransition)
        ));
        h.end_in(l.id, -1).await?;
        let (one, two) = tokio::join!(h.api.settle(l.id, "settle"), h.api.settle(l.id, "settle"));
        let result = one?;
        assert_eq!(result, two?);
        assert_eq!((result.commission, result.seller_net), (fee, amount - fee));
        assert!(matches!(
            h.api.settle(l.id, "other").await,
            Err(Error::IdempotencyConflict)
        ));
        let ledger = h.ledger().await?;
        assert!(ledger.holds.is_empty());
        assert_eq!(
            ledger.balances[&MemoryLedger::key(h.a, currency)],
            1_000_000 - amount
        );
        assert_eq!(
            ledger.balances[&MemoryLedger::key(h.owner, currency)],
            1_000_000
        );
        h.evidence.delivery.store(false, Ordering::SeqCst);
        assert!(matches!(
            h.api.complete_delivery(l.id).await,
            Err(Error::Denied)
        ));
        h.evidence.delivery.store(true, Ordering::SeqCst);
        h.api.complete_delivery(l.id).await?;
        h.api.complete_delivery(l.id).await?;
        assert_eq!(
            h.ledger().await?.balances[&MemoryLedger::key(h.owner, currency)],
            1_000_000 + amount - fee
        );
        assert_eq!(h.api.get(l.id).await?.state, State::Completed);
    }
    h.finish().await
}

#[tokio::test]
async fn adapter_failure_rolls_back_ledger_settlement_listing_and_audit() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let l = h.listing(Form::Auction, Currency::Coins, 100).await?;
    h.api.bid(l.id, h.a, 500, "bid").await?;
    h.end_in(l.id, -1).await?;
    let before = serde_json::to_string(&h.ledger().await?)?;
    let audit: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_audit")
        .fetch_one(&h.pool)
        .await?;
    h.funds.fail_settlement.store(true, Ordering::SeqCst);
    assert!(matches!(
        h.api.settle(l.id, "settle").await,
        Err(Error::Denied)
    ));
    assert_eq!(serde_json::to_string(&h.ledger().await?)?, before);
    assert_eq!(h.api.get(l.id).await?.state, State::Active);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_settlements")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(count, 0);
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM bazaar_audit")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(after, audit);
    h.funds.fail_settlement.store(false, Ordering::SeqCst);
    h.api.settle(l.id, "settle").await?;
    h.finish().await
}

#[tokio::test]
async fn offers_accept_reject_release_and_reverse_want_to_buy_roles() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let l = h.listing(Form::FixedPrice, Currency::Coins, 200).await?;
    let a = h.api.offer(l.id, h.a, 180, "a").await?;
    assert_eq!(h.api.offer(l.id, h.a, 180, "a").await?.id, a.id);
    assert!(matches!(
        h.api.offer(l.id, h.a, 181, "a").await,
        Err(Error::IdempotencyConflict)
    ));
    let b = h.api.offer(l.id, h.b, 190, "b").await?;
    h.api.reject_offer(l.id, h.owner, a.id).await?;
    assert_eq!(h.ledger().await?.holds.len(), 1);
    h.api.accept_offer(l.id, h.owner, b.id).await?;
    assert!(matches!(
        h.api.accept_offer(l.id, h.owner, b.id).await,
        Err(Error::IllegalTransition)
    ));
    let s = h.api.settle(l.id, "sale").await?;
    assert_eq!((s.buyer, s.seller, s.amount), (h.b, h.owner, 190));
    let w = h.listing(Form::WantToBuy, Currency::Coins, 300).await?;
    let response = h.api.offer(w.id, h.a, 250, "sell").await?;
    assert!(h.ledger().await?.holds.is_empty());
    h.api.accept_offer(w.id, h.owner, response.id).await?;
    let s = h.api.settle(w.id, "buy").await?;
    assert_eq!((s.buyer, s.seller, s.amount), (h.owner, h.a, 250));
    h.finish().await
}

#[tokio::test]
async fn illegal_transitions_and_bad_inputs_are_refused() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let mut bad = h.input(Form::Auction, Currency::Coins, 0);
    assert!(matches!(
        h.api.create(bad.clone()).await,
        Err(Error::InvalidInput)
    ));
    bad.price = 1;
    bad.kind = Kind::Character;
    bad.form = Form::FixedPrice;
    assert!(matches!(h.api.create(bad).await, Err(Error::InvalidInput)));
    let l = h
        .api
        .create(h.input(Form::Auction, Currency::Coins, 100))
        .await?;
    assert!(matches!(
        h.api.bid(l.id, h.a, 200, "draft").await,
        Err(Error::IllegalTransition)
    ));
    assert!(matches!(
        h.api.complete_delivery(l.id).await,
        Err(Error::IllegalTransition)
    ));
    h.api.publish(l.id, h.owner).await?;
    assert!(matches!(
        h.api.bid(l.id, h.owner, 200, "self").await,
        Err(Error::IllegalTransition)
    ));
    for (amount, key) in [
        (0, "zero"),
        (-1, "negative"),
        (200, ""),
        (200, " bad"),
        (200, "\n"),
    ] {
        assert!(matches!(
            h.api.bid(l.id, h.a, amount, key).await,
            Err(Error::InvalidInput)
        ));
    }
    h.api.bid(l.id, h.a, 200, "valid").await?;
    assert!(matches!(
        h.api.cancel(l.id, h.owner).await,
        Err(Error::IllegalTransition)
    ));
    assert!(matches!(
        h.api.expire(l.id).await,
        Err(Error::IllegalTransition)
    ));
    assert!(
        sqlx::query("UPDATE bazaar_listings SET state='completed' WHERE id=$1")
            .bind(l.id)
            .execute(&h.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM bazaar_bids WHERE listing_id=$1")
            .bind(l.id)
            .execute(&h.pool)
            .await
            .is_err()
    );
    h.finish().await
}

#[tokio::test]
async fn flags_account_state_and_escrow_fail_closed() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    assert!(matches!(
        h.flags(Flags::default())
            .create(h.input(Form::Auction, Currency::Coins, 100))
            .await,
        Err(Error::Disabled)
    ));
    assert!(matches!(
        h.flags(Flags {
            enabled: true,
            ..Flags::default()
        })
        .create(h.input(Form::Auction, Currency::Gold, 100))
        .await,
        Err(Error::Disabled)
    ));
    let l = h
        .api
        .create(h.input(Form::Auction, Currency::Coins, 100))
        .await?;
    h.evidence.escrow.store(false, Ordering::SeqCst);
    assert!(matches!(
        h.api.publish(l.id, h.owner).await,
        Err(Error::Denied)
    ));
    h.evidence.escrow.store(true, Ordering::SeqCst);
    h.api.publish(l.id, h.owner).await?;
    assert!(matches!(
        h.api.bid(l.id, Uuid::now_v7(), 200, "unknown").await,
        Err(Error::Denied)
    ));
    for state in [
        AccountState::default(),
        AccountState {
            active: true,
            email_verified: true,
            bazaar_blocked: true,
            ..AccountState::default()
        },
    ] {
        h.accounts
            .0
            .write()
            .map_err(|_| "poisoned accounts")?
            .insert(h.a, state);
        assert!(matches!(
            h.api.bid(l.id, h.a, 200, "blocked").await,
            Err(Error::Denied)
        ));
    }
    let mut owner = *h
        .accounts
        .0
        .read()
        .map_err(|_| "poisoned accounts")?
        .get(&h.owner)
        .ok_or("missing owner")?;
    owner.sudo = false;
    h.accounts
        .0
        .write()
        .map_err(|_| "poisoned accounts")?
        .insert(h.owner, owner);
    assert!(matches!(
        h.api
            .create(h.input(Form::Auction, Currency::Coins, 100))
            .await,
        Err(Error::Denied)
    ));
    h.finish().await
}

#[tokio::test]
async fn expiry_releases_offers_and_cancellation_requires_no_activity() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let l = h.listing(Form::FixedPrice, Currency::Coins, 100).await?;
    let a = h.api.offer(l.id, h.a, 100, "a").await?;
    h.api.offer(l.id, h.b, 101, "b").await?;
    h.end_in(l.id, -1).await?;
    h.api.expire(l.id).await?;
    assert!(h.ledger().await?.holds.is_empty());
    assert_eq!(h.api.get(l.id).await?.state, State::Expired);
    assert!(matches!(
        h.api.accept_offer(l.id, h.owner, a.id).await,
        Err(Error::IllegalTransition)
    ));
    let empty = h.listing(Form::Auction, Currency::Coins, 100).await?;
    h.api.cancel(empty.id, h.owner).await?;
    assert!(matches!(
        h.api.publish(empty.id, h.owner).await,
        Err(Error::IllegalTransition)
    ));
    let expired = h.listing(Form::Auction, Currency::Coins, 100).await?;
    h.end_in(expired.id, -1).await?;
    assert!(matches!(
        h.api.bid(expired.id, h.a, 200, "late").await,
        Err(Error::IllegalTransition)
    ));
    h.api.expire(expired.id).await?;
    h.finish().await
}

#[tokio::test]
async fn concurrent_listings_cannot_over_reserve_one_account() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let one = h.listing(Form::Auction, Currency::Coins, 100).await?;
    let two = h.listing(Form::Auction, Currency::Coins, 100).await?;
    let (a, b) = tokio::join!(
        h.api.bid(one.id, h.a, 600_000, "one"),
        h.api.bid(two.id, h.a, 600_000, "two")
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(
        matches!(a, Err(Error::InsufficientFunds)) || matches!(b, Err(Error::InsufficientFunds))
    );
    let ledger = h.ledger().await?;
    assert_eq!(ledger.holds.len(), 1);
    assert_eq!(ledger.holds.values().next().map(|(_, n)| *n), Some(600_000));
    h.finish().await
}

#[tokio::test]
async fn sql_failure_after_reservation_rolls_back_the_whole_bid() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let l = h.listing(Form::Auction, Currency::Coins, 100).await?;
    let before = serde_json::to_string(&h.ledger().await?)?;
    sqlx::raw_sql("CREATE FUNCTION fail_bid() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
        RAISE EXCEPTION 'injected test failure'; END; $$;
        CREATE TRIGGER fail_bid_insert BEFORE INSERT ON bazaar_bids FOR EACH ROW EXECUTE FUNCTION fail_bid();")
        .execute(&h.pool).await?;
    assert!(matches!(
        h.api.bid(l.id, h.a, 500, "fail").await,
        Err(Error::Database(_))
    ));
    assert_eq!(serde_json::to_string(&h.ledger().await?)?, before);
    assert_eq!(h.api.get(l.id).await?.leader, None);
    sqlx::query("DROP TRIGGER fail_bid_insert ON bazaar_bids")
        .execute(&h.pool)
        .await?;
    h.api.bid(l.id, h.a, 500, "fail").await?;
    h.finish().await
}

#[tokio::test]
async fn offer_limits_fixed_prices_and_acceptance_release_other_holds() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let mut input = h.input(Form::FixedPrice, Currency::Coins, 100);
    input.accept_offers = false;
    let l = h.api.create(input).await?;
    h.api.publish(l.id, h.owner).await?;
    assert!(matches!(
        h.api.offer(l.id, h.a, 99, "low").await,
        Err(Error::IllegalTransition)
    ));
    assert!(matches!(
        h.api.offer(l.id, h.a, -1, "negative").await,
        Err(Error::InvalidInput)
    ));
    let one = h.api.offer(l.id, h.a, 100, "one").await?;
    assert!(matches!(
        h.api.offer(l.id, h.a, 100, "duplicate").await,
        Err(Error::Denied)
    ));
    h.api.offer(l.id, h.b, 100, "two").await?;
    h.api.accept_offer(l.id, h.owner, one.id).await?;
    assert_eq!(h.ledger().await?.holds.len(), 1);
    // Twenty pending offers per responding account, serialized across listings.
    for index in 0..21 {
        let ad = h.listing(Form::FixedPrice, Currency::Coins, 1).await?;
        let result = h.api.offer(ad.id, h.a, 1, &format!("offer-{index}")).await;
        if index < 20 {
            result?;
        } else {
            assert!(matches!(result, Err(Error::Denied)));
        }
    }
    h.finish().await
}

#[tokio::test]
async fn terminal_cancellation_and_expiry_require_return_evidence() -> TestResult {
    let Some(h) = Harness::new().await? else {
        return Ok(());
    };
    let listing = h.listing(Form::Auction, Currency::Coins, 100).await?;
    h.evidence.escrow.store(false, Ordering::SeqCst);
    assert!(matches!(
        h.api.cancel(listing.id, h.owner).await,
        Err(Error::Denied)
    ));
    assert_eq!(h.api.get(listing.id).await?.state, State::Active);
    h.end_in(listing.id, -1).await?;
    assert!(matches!(h.api.expire(listing.id).await, Err(Error::Denied)));
    assert_eq!(h.api.get(listing.id).await?.state, State::Active);
    h.finish().await
}
