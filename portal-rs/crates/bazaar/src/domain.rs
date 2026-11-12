use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bazaar is disabled")]
    Disabled,
    #[error("invalid input")]
    InvalidInput,
    #[error("account or integration evidence denied")]
    Denied,
    #[error("illegal bazaar transition")]
    IllegalTransition,
    #[error("listing or offer not found")]
    NotFound,
    #[error("idempotency key has different input")]
    IdempotencyConflict,
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("amount overflow")]
    Overflow,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "bazaar_kind", rename_all = "snake_case")]
pub enum Kind {
    Character,
    Item,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "bazaar_form", rename_all = "snake_case")]
pub enum Form {
    Auction,
    FixedPrice,
    WantToBuy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "bazaar_currency", rename_all = "snake_case")]
pub enum Currency {
    Coins,
    Gold,
}

impl Currency {
    /// Ceil one percent, with the currency floor, in smallest units.
    pub fn increment(self, price: i64) -> Result<i64, Error> {
        if price <= 0 {
            return Err(Error::InvalidInput);
        }
        let percent = price / 100 + i64::from(price % 100 != 0);
        Ok(percent.max(match self {
            Self::Coins => 1,
            Self::Gold => 1_000,
        }))
    }

    /// Commission rounds down to the smallest unit; the seller receives the remainder.
    pub fn commission(self, amount: i64) -> Result<i64, Error> {
        if amount <= 0 {
            return Err(Error::InvalidInput);
        }
        let divisor = match self {
            Self::Coins => 10,
            Self::Gold => 40,
        };
        Ok(amount / divisor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "bazaar_state", rename_all = "snake_case")]
pub enum State {
    Draft,
    Active,
    Matched,
    AwaitingDelivery,
    Completed,
    Cancelled,
    Expired,
}

impl State {
    pub fn transition(self, next: Self) -> Result<(), Error> {
        if matches!(
            (self, next),
            (Self::Draft, Self::Active | Self::Cancelled)
                | (
                    Self::Active,
                    Self::Matched | Self::AwaitingDelivery | Self::Cancelled | Self::Expired
                )
                | (Self::Matched, Self::AwaitingDelivery)
                | (Self::AwaitingDelivery, Self::Completed)
        ) {
            Ok(())
        } else {
            Err(Error::IllegalTransition)
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewListing {
    pub owner: Uuid,
    pub kind: Kind,
    pub form: Form,
    pub currency: Currency,
    /// Auction starting price or advertised price, per item unit.
    pub price: i64,
    pub accept_offers: bool,
    pub ends_at: OffsetDateTime,
}

impl NewListing {
    pub fn validate(&self, now: OffsetDateTime) -> Result<(), Error> {
        let duration = self.ends_at - now;
        if self.owner.is_nil()
            || self.price <= 0
            || (self.kind == Kind::Character && self.form != Form::Auction)
            || (self.form == Form::Auction && self.accept_offers)
            || !(Duration::days(1)..=Duration::days(14)).contains(&duration)
            || (self.form != Form::Auction && duration != Duration::days(14))
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Listing {
    pub id: Uuid,
    pub owner: Uuid,
    pub kind: Kind,
    pub form: Form,
    pub currency: Currency,
    pub price: i64,
    pub accept_offers: bool,
    pub state: State,
    pub ends_at: OffsetDateTime,
    pub leader: Option<Uuid>,
    /// Private proxy maximum. Never expose this field to public clients.
    pub leader_max: Option<i64>,
    pub reservation: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct BidOutcome {
    pub leader: Uuid,
    pub price: i64,
    pub ends_at: OffsetDateTime,
}

/// Pure proxy calculation. Earlier bids win ties; raising one's own maximum keeps the price.
pub fn proxy_bid(listing: &Listing, bidder: Uuid, maximum: i64) -> Result<(Uuid, i64), Error> {
    if maximum <= 0 || bidder.is_nil() {
        return Err(Error::InvalidInput);
    }
    if listing.form != Form::Auction || listing.state != State::Active || bidder == listing.owner {
        return Err(Error::IllegalTransition);
    }
    match (listing.leader, listing.leader_max) {
        (None, None) if maximum >= listing.price => Ok((bidder, listing.price)),
        (Some(leader), Some(old_max)) if leader == bidder && maximum > old_max => {
            Ok((leader, listing.price))
        }
        (Some(leader), Some(old_max)) if leader != bidder => {
            let minimum = listing
                .price
                .checked_add(listing.currency.increment(listing.price)?)
                .ok_or(Error::Overflow)?;
            if maximum < minimum {
                return Err(Error::InvalidInput);
            }
            let (winner, winner_max, loser_max) = if maximum > old_max {
                (bidder, maximum, old_max)
            } else {
                (leader, old_max, maximum)
            };
            // A capped addition avoids rejecting a valid maximum at i64::MAX.
            let price =
                i128::from(loser_max) + i128::from(listing.currency.increment(listing.price)?);
            let price = price.min(i128::from(winner_max));
            Ok((winner, i64::try_from(price).map_err(|_| Error::Overflow)?))
        }
        _ => Err(Error::InvalidInput),
    }
}

pub(crate) fn validate_key(key: &str) -> Result<(), Error> {
    if key.is_empty() || key.len() > 128 || key.chars().any(char::is_control) || key.trim() != key {
        Err(Error::InvalidInput)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Offer {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub account: Uuid,
    pub amount: i64,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Settlement {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub buyer: Uuid,
    pub seller: Uuid,
    pub currency: Currency,
    pub amount: i64,
    pub commission: i64,
    pub seller_net: i64,
    pub reservation: Uuid,
}
