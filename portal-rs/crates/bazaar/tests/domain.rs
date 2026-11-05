use oteryn_bazaar::*;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[test]
fn increment_and_commission_use_integer_smallest_units() -> Result<(), Error> {
    assert_eq!(Currency::Coins.increment(101)?, 2);
    assert_eq!(Currency::Gold.increment(100)?, 1_000);
    assert_eq!(Currency::Coins.commission(i64::MAX)?, i64::MAX / 10);
    assert_eq!(Currency::Gold.commission(i64::MAX)?, i64::MAX / 40);
    assert!(matches!(
        Currency::Coins.increment(0),
        Err(Error::InvalidInput)
    ));
    assert!(matches!(
        Currency::Gold.commission(-1),
        Err(Error::InvalidInput)
    ));
    Ok(())
}

#[test]
fn proxy_ties_favor_earlier_bid_and_large_values_do_not_overflow() -> Result<(), Error> {
    let leader = Uuid::now_v7();
    let challenger = Uuid::now_v7();
    let mut listing = Listing {
        id: Uuid::now_v7(),
        owner: Uuid::now_v7(),
        kind: Kind::Item,
        form: Form::Auction,
        currency: Currency::Coins,
        price: 100,
        accept_offers: false,
        state: State::Active,
        ends_at: OffsetDateTime::now_utc() + Duration::days(1),
        leader: Some(leader),
        leader_max: Some(500),
        reservation: Some(Uuid::now_v7()),
    };
    assert_eq!(proxy_bid(&listing, challenger, 500)?, (leader, 500));
    assert_eq!(proxy_bid(&listing, leader, 700)?, (leader, 100));
    listing.leader_max = Some(i64::MAX);
    assert_eq!(
        proxy_bid(&listing, challenger, i64::MAX)?,
        (leader, i64::MAX)
    );
    listing.price = i64::MAX;
    assert!(matches!(
        proxy_bid(&listing, challenger, i64::MAX),
        Err(Error::Overflow)
    ));
    Ok(())
}

#[test]
fn validation_denies_invalid_kind_duration_and_account_state() -> Result<(), Error> {
    let now = OffsetDateTime::now_utc();
    let mut input = NewListing {
        owner: Uuid::now_v7(),
        kind: Kind::Character,
        form: Form::Auction,
        currency: Currency::Coins,
        price: 1,
        accept_offers: false,
        ends_at: now + Duration::days(1),
    };
    input.validate(now)?;
    input.ends_at = now + Duration::days(15);
    assert!(matches!(input.validate(now), Err(Error::InvalidInput)));
    input.ends_at = now + Duration::days(2);
    input.form = Form::WantToBuy;
    assert!(matches!(input.validate(now), Err(Error::InvalidInput)));
    assert!(!AccountState::default().allows_trade());
    assert!(
        !AccountState {
            active: true,
            email_verified: true,
            ..AccountState::default()
        }
        .allows_listing()
    );
    assert!(!Flags::default().enabled);
    assert!(!Flags::default().gold_enabled);
    assert!(matches!(
        State::Completed.transition(State::Active),
        Err(Error::IllegalTransition)
    ));
    Ok(())
}
