//! Durable notification seam for listing transitions (G-7 §1, T62).
//!
//! T61 has no implementation in this base. Its adapter must verify subscriptions,
//! saved-search matches and transition recipients inside the supplied transaction.
//! T36 currently has no Bazaar preference category; no production sink is registered.
//! Neither adapter may infer a sale from polling a delivery or an HTTP response.
use std::future::Future;

use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Error;

/// Explicit opt-in; recording is permitted while dispatch is disabled.
#[derive(Debug, Default, Clone, Copy)]
pub struct NotificationFlags {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Outbid,
    NewOffer,
    EndingSoon,
    Won,
    MatchingSearch,
}
impl NoticeKind {
    fn key(self) -> &'static str {
        match self {
            Self::Outbid => "outbid",
            Self::NewOffer => "new-offer",
            Self::EndingSoon => "ending-soon",
            Self::Won => "won",
            Self::MatchingSearch => "matching-search",
        }
    }
    fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "outbid" => Ok(Self::Outbid),
            "new-offer" => Ok(Self::NewOffer),
            "ending-soon" => Ok(Self::EndingSoon),
            "won" => Ok(Self::Won),
            "matching-search" => Ok(Self::MatchingSearch),
            _ => Err(Error::Denied),
        }
    }
}

/// Stable transition identity, never a delivery attempt ID. Contains no private bid maximum,
/// item attributes, account address or untrusted text. `evidence` identifies the durable
/// bid, offer, ending deadline, settlement or saved-search match respectively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BazaarNotice {
    pub transition: Uuid,
    pub listing: Uuid,
    pub recipient: Uuid,
    pub evidence: Uuid,
    pub kind: NoticeKind,
}
impl BazaarNotice {
    fn validate(&self) -> Result<(), Error> {
        if [self.transition, self.listing, self.recipient, self.evidence]
            .iter()
            .any(Uuid::is_nil)
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}

/// Read only durable, verified Platform state, on the same transaction/connection.
/// Return false for missing, stale, conflicting or ambiguous evidence. Implementations
/// must lock the relevant listing/subscription rows against concurrent changes.
///
/// Outbid: recipient was displaced by this accepted bid, never by their own raise.
/// NewOffer: recipient owns the listing and the accepted offer is from another account.
/// EndingSoon: watched active listing ends in (now, now + 1 hour], bound to its current
/// deadline; an anti-sniping extension invalidates old deadline evidence.
/// Won: recipient is the buyer of a COMPLETED settlement with verified durable Game
/// CHAR_CMD receipt evidence and its current character_revision, projection_revision,
/// projection_epoch and recovery_generation cut, never a provisional leader.
/// MatchingSearch: active public listing matches this recipient's current saved search.
/// All kinds require an active, verified, unrestricted recipient and current visibility.
/// Verification runs both at recording and dispatch, so revoked subscriptions deny.
pub trait NoticeEvidence: Send + Sync {
    fn verify(
        &self,
        db: &mut PgConnection,
        notice: &BazaarNotice,
        now: OffsetDateTime,
    ) -> impl Future<Output = Result<bool, Error>> + Send;
}

/// T36 integration boundary. The adapter must read current channel preferences, render
/// via the i18n layer and enqueue inbox/email effects on this transaction only. Email
/// defaults off. Use `delivery_id` for idempotency, with no network I/O or commit here.
/// Unknown account, locale, category or preference state must deny. A declined channel
/// is a successful no-op. No production adapter is available until T36 supports Bazaar.
pub trait NoticeSink: Send + Sync {
    fn enqueue(
        &self,
        db: &mut PgConnection,
        delivery_id: Uuid,
        notice: &BazaarNotice,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}

/// Safe composition default for both absent dependencies.
#[derive(Debug)]
pub struct DenyNotices;
impl NoticeEvidence for DenyNotices {
    async fn verify(
        &self,
        _: &mut PgConnection,
        _: &BazaarNotice,
        _: OffsetDateTime,
    ) -> Result<bool, Error> {
        Ok(false)
    }
}
impl NoticeSink for DenyNotices {
    async fn enqueue(&self, _: &mut PgConnection, _: Uuid, _: &BazaarNotice) -> Result<(), Error> {
        Err(Error::Denied)
    }
}

#[derive(Debug)]
pub struct BazaarNotifications<E, S> {
    pub pool: PgPool,
    pub evidence: E,
    pub sink: S,
    pub flags: NotificationFlags,
}
impl<E: NoticeEvidence, S: NoticeSink> BazaarNotifications<E, S> {
    /// Call in the transaction that accepts the domain transition. Caller owns commit.
    /// Reuse with different input refuses instead of silently keeping a different event.
    pub async fn record(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        notice: &BazaarNotice,
    ) -> Result<Uuid, Error> {
        notice.validate()?;
        if !self
            .evidence
            .verify(tx, notice, OffsetDateTime::now_utc())
            .await?
        {
            return Err(Error::Denied);
        }
        let row: (Uuid, Uuid, Uuid) = sqlx::query_as(
            "INSERT INTO bazaar_notification_outbox(transition_id,listing_id,recipient_id,evidence_id,kind)
             VALUES($1,$2,$3,$4,$5)
             ON CONFLICT(transition_id,recipient_id,kind) DO UPDATE
             SET transition_id=bazaar_notification_outbox.transition_id
             RETURNING id,listing_id,evidence_id",
        )
        .bind(notice.transition)
        .bind(notice.listing)
        .bind(notice.recipient)
        .bind(notice.evidence)
        .bind(notice.kind.key())
        .fetch_one(&mut **tx)
        .await?;
        if (row.1, row.2) != (notice.listing, notice.evidence) {
            return Err(Error::IdempotencyConflict);
        }
        Ok(row.0)
    }

    /// One bounded transaction per event. Disabled dispatch does not claim or alter rows.
    /// Failed evidence is retained as suppressed; sink errors roll back and remain pending.
    /// SKIP LOCKED allows concurrent workers without duplicate channel effects.
    pub async fn dispatch_one(&self) -> Result<bool, Error> {
        if !self.flags.enabled {
            return Err(Error::Disabled);
        }
        let mut tx = self.pool.begin().await?;
        let row: Option<(Uuid, Uuid, Uuid, Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT id,transition_id,listing_id,recipient_id,evidence_id,kind
             FROM bazaar_notification_outbox WHERE state='pending'
             ORDER BY created_at,id FOR UPDATE SKIP LOCKED LIMIT 1",
        )
        .fetch_optional(&mut *tx)
        .await?;
        let Some((id, transition, listing, recipient, evidence, kind)) = row else {
            return Ok(false);
        };
        let notice = BazaarNotice {
            transition,
            listing,
            recipient,
            evidence,
            kind: NoticeKind::parse(&kind)?,
        };
        let state = if self
            .evidence
            .verify(&mut tx, &notice, OffsetDateTime::now_utc())
            .await?
        {
            self.sink.enqueue(&mut tx, id, &notice).await?;
            "delivered"
        } else {
            "suppressed"
        };
        sqlx::query("UPDATE bazaar_notification_outbox SET state=$2,updated_at=now() WHERE id=$1")
            .bind(id)
            .bind(state)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NoticeLocale {
    Pl,
    En,
}

/// Plain text content for T36's escaped HTML and text templates. No arbitrary listing
/// text is interpolated; the sink can use `listing_path` for an authenticated deep link.
#[derive(Debug)]
pub struct NoticeText {
    pub subject: String,
    pub body: String,
    pub listing_path: String,
}
impl BazaarNotice {
    pub fn render(&self, locale: NoticeLocale) -> Result<NoticeText, Error> {
        self.validate()?;
        let (language, source) = match locale {
            NoticeLocale::Pl => ("pl", include_str!("notify/pl.ftl")),
            NoticeLocale::En => ("en", include_str!("notify/en.ftl")),
        };
        let language = language
            .parse::<unic_langid::LanguageIdentifier>()
            .map_err(|_| Error::Denied)?;
        let mut bundle = fluent_bundle::FluentBundle::new(vec![language]);
        let resource =
            fluent_bundle::FluentResource::try_new(source.to_owned()).map_err(|_| Error::Denied)?;
        bundle.add_resource(resource).map_err(|_| Error::Denied)?;
        let translate = |key: &str| {
            let pattern = bundle
                .get_message(key)
                .and_then(|message| message.value())
                .ok_or(Error::Denied)?;
            let mut errors = Vec::new();
            let text = bundle
                .format_pattern(pattern, None, &mut errors)
                .into_owned();
            if !errors.is_empty() {
                return Err(Error::Denied);
            }
            Ok(text)
        };
        Ok(NoticeText {
            subject: translate(self.kind.key())?,
            body: translate(&format!("{}-body", self.kind.key()))?,
            listing_path: format!("/bazaar/{}", self.listing),
        })
    }
}

#[cfg(test)]
mod tests;
