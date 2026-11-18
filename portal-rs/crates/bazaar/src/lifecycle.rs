//! Bazaar's commercial, projection, and command transition rules.

use crate::{Error, State};
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    Listing(State),
    Create,
    Stage,
    GoldBid(GoldPhase),
    Dispatch,
    RetireTransfer,
    ReadIntent,
    Advance,
    Receipt { committed: bool },
    Bid,
    CancelRequest,
    Suspend,
    Reopen,
    Compensated,
    Projection,
    ResyncIntent,
    ResyncCustody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Current {
    Absent,
    Fence { old: (u64, u64), new: (u64, u64) },
    Listing(State),
    GoldBid(GoldPhase),
    Projection(Option<Projection>),
    Intent { dispatched: bool, admitted: bool },
    Custody { completed_item: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Projection {
    pub epoch: u64,
    pub revision: u64,
    pub observed: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Context {
    pub incoming: Option<Projection>,
    pub same_content: bool,
    pub watermark: bool,
    pub retained: bool,
    pub delivered: bool,
    pub has_leader: bool,
    pub may_suspend_settlement: bool,
    pub reopened: bool,
    pub committed: bool,
    pub held: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Listing(State),
    Apply,
    Duplicate,
    Superseded,
    Quarantine,
    Continue,
    Receipt(&'static str),
    GoldBid(GoldPhase),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    Illegal,
    EvidenceMissing,
    CustodyDrift,
    ModerationHold,
}

pub(crate) fn transition(
    current: Current,
    event: Event,
    context: Context,
) -> Result<Outcome, Refusal> {
    use Current::{Custody, Intent};
    use Event::{Compensated, ResyncCustody, ResyncIntent};
    use State::{Active, AwaitingDelivery, Cancelled, Completed, Draft, Expired, Matched};

    match (current, event) {
        (Current::Listing(Draft | Active), Event::Advance) if context.held => {
            Err(Refusal::ModerationHold)
        }
        (
            Current::Listing(Draft | Active | AwaitingDelivery | Completed | Cancelled),
            Event::Advance,
        ) => Ok(Outcome::Continue),
        (
            Current::Listing(Matched | Expired)
            | Current::Absent
            | Current::GoldBid(_)
            | Current::Projection(_)
            | Current::Fence { old: _, new: _ }
            | Intent {
                dispatched: _,
                admitted: _,
            }
            | Custody { completed_item: _ },
            Event::Advance,
        ) => Err(Refusal::Illegal),
        (Current::Listing(Draft), Event::Listing(next @ (Active | Cancelled)))
        | (
            Current::Listing(Active),
            Event::Listing(next @ (Matched | AwaitingDelivery | Cancelled | Expired)),
        )
        | (Current::Listing(Matched), Event::Listing(next @ AwaitingDelivery))
        | (Current::Listing(AwaitingDelivery), Event::Listing(next @ Completed)) => {
            Ok(Outcome::Listing(next))
        }
        (Current::Listing(Active), Event::Bid)
        | (Current::Listing(Draft | Active), Event::Suspend) => Ok(Outcome::Continue),
        (Current::Listing(Draft | Active), Event::CancelRequest) if !context.has_leader => {
            Ok(Outcome::Continue)
        }
        (Current::Listing(AwaitingDelivery), Event::Suspend) if context.may_suspend_settlement => {
            Ok(Outcome::Continue)
        }
        (Current::Absent, Event::GoldBid(GoldPhase::Pending)) => {
            Ok(Outcome::GoldBid(GoldPhase::Pending))
        }
        (
            Current::GoldBid(GoldPhase::Pending),
            Event::GoldBid(
                next @ (GoldPhase::Accepted | GoldPhase::Refused | GoldPhase::Releasing),
            ),
        )
        | (
            Current::GoldBid(GoldPhase::Releasing),
            Event::GoldBid(next @ (GoldPhase::Accepted | GoldPhase::Refused)),
        ) => Ok(Outcome::GoldBid(next)),
        (
            Current::GoldBid(_),
            Event::GoldBid(_)
            | Event::Listing(_)
            | Event::Create
            | Event::Stage
            | Event::Dispatch
            | Event::Receipt { committed: _ }
            | Event::Bid
            | Event::CancelRequest
            | Event::Suspend
            | Event::Reopen
            | Event::Compensated
            | Event::Projection
            | Event::ResyncIntent
            | Event::ResyncCustody,
        )
        | (
            Current::Absent,
            Event::GoldBid(GoldPhase::Accepted | GoldPhase::Refused | GoldPhase::Releasing),
        )
        | (
            Current::Listing(_)
            | Current::Fence { old: _, new: _ }
            | Current::Projection(_)
            | Intent {
                dispatched: _,
                admitted: _,
            }
            | Custody { completed_item: _ },
            Event::GoldBid(_),
        ) => Err(Refusal::Illegal),
        (
            Intent {
                admitted: false,
                dispatched: _,
            },
            Event::RetireTransfer,
        ) => Ok(Outcome::Apply),
        (
            Intent {
                admitted: true,
                dispatched: _,
            },
            Event::RetireTransfer,
        )
        | (
            Current::Absent
            | Current::Listing(_)
            | Current::GoldBid(_)
            | Current::Projection(_)
            | Current::Fence { old: _, new: _ }
            | Custody { completed_item: _ },
            Event::RetireTransfer,
        ) => Err(Refusal::Illegal),
        (
            Intent {
                dispatched: false,
                admitted: _,
            },
            Event::ReadIntent,
        ) => Err(Refusal::Illegal),
        (
            Intent {
                dispatched: true,
                admitted: true,
            },
            Event::ReadIntent,
        ) => Ok(Outcome::Receipt(if context.committed {
            "COMMITTED"
        } else {
            "REJECTED"
        })),
        (
            Intent {
                dispatched: true,
                admitted: false,
            },
            Event::ReadIntent,
        ) => Ok(Outcome::Receipt(if context.reopened {
            "REOPENED"
        } else {
            "PENDING"
        })),
        (
            Current::Absent
            | Current::Listing(_)
            | Current::GoldBid(_)
            | Current::Projection(_)
            | Current::Fence { old: _, new: _ }
            | Custody { completed_item: _ },
            Event::ReadIntent,
        ) => Err(Refusal::Illegal),
        (Current::Absent, Event::Stage) => Ok(Outcome::Apply),
        (
            Intent {
                dispatched,
                admitted: _,
            },
            Event::Dispatch,
        ) => Ok(if dispatched {
            Outcome::Duplicate
        } else {
            Outcome::Apply
        }),
        (
            Intent {
                dispatched: _,
                admitted: false,
            },
            Event::Receipt { committed },
        ) => Ok(Outcome::Receipt(if committed {
            "COMMITTED"
        } else {
            "REJECTED"
        })),
        (Current::Absent, Event::Create) => Ok(Outcome::Listing(Draft)),
        (Current::Fence { old, new }, Event::Reopen) => {
            if new.0 == 0 || new.1 == 0 || new.0 < old.0 || new.1 < old.1 {
                return Err(Refusal::Illegal);
            }
            Ok(if old == new {
                Outcome::Duplicate
            } else {
                Outcome::Apply
            })
        }
        (Current::Listing(AwaitingDelivery), Compensated) => Ok(Outcome::Listing(Cancelled)),
        (Current::Listing(Cancelled), Compensated) => Ok(Outcome::Duplicate),
        (Current::Listing(_), Event::Listing(_) | Compensated) => Err(Refusal::Illegal),
        (Current::Projection(old), Event::Projection) => {
            let incoming = context.incoming.ok_or(Refusal::Illegal)?;
            if let Some(old) = old {
                let key = (incoming.epoch, incoming.revision);
                let old_key = (old.epoch, old.revision);
                if key < old_key
                    || (context.watermark
                        && incoming.epoch == old.epoch
                        && incoming.observed < old.observed)
                {
                    return Ok(Outcome::Superseded);
                }
                if key == old_key {
                    return Ok(if context.same_content {
                        Outcome::Duplicate
                    } else {
                        Outcome::Quarantine
                    });
                }
            }
            Ok(Outcome::Apply)
        }
        (
            Intent {
                dispatched,
                admitted,
            },
            ResyncIntent,
        ) => {
            if (dispatched || admitted) && !context.retained {
                return Err(Refusal::EvidenceMissing);
            }
            Ok(Outcome::Continue)
        }
        (Custody { completed_item }, ResyncCustody) => {
            if completed_item && !context.delivered {
                return Err(Refusal::CustodyDrift);
            }
            Ok(Outcome::Continue)
        }
        (
            Current::Listing(_),
            Event::Projection
            | Event::ResyncIntent
            | Event::ResyncCustody
            | Event::Create
            | Event::Reopen
            | Event::Stage
            | Event::Dispatch
            | Event::Receipt { committed: _ }
            | Event::Bid
            | Event::CancelRequest
            | Event::Suspend,
        )
        | (
            Current::Projection(_),
            Event::Listing(_)
            | Event::Compensated
            | Event::ResyncIntent
            | Event::ResyncCustody
            | Event::Create
            | Event::Reopen
            | Event::Stage
            | Event::Dispatch
            | Event::Receipt { committed: _ }
            | Event::Bid
            | Event::CancelRequest
            | Event::Suspend,
        )
        | (
            Intent {
                dispatched: _,
                admitted: _,
            },
            Event::Listing(_)
            | Event::Compensated
            | Event::Projection
            | Event::ResyncCustody
            | Event::Create
            | Event::Reopen
            | Event::Stage
            | Event::Bid
            | Event::CancelRequest
            | Event::Suspend,
        )
        | (
            Intent {
                dispatched: _,
                admitted: true,
            },
            Event::Receipt { committed: _ },
        )
        | (
            Custody { completed_item: _ },
            Event::Listing(_)
            | Event::Compensated
            | Event::Projection
            | Event::ResyncIntent
            | Event::Create
            | Event::Reopen
            | Event::Stage
            | Event::Dispatch
            | Event::Receipt { committed: _ }
            | Event::Bid
            | Event::CancelRequest
            | Event::Suspend,
        )
        | (
            Current::Absent,
            Event::Listing(_)
            | Event::Compensated
            | Event::Projection
            | Event::ResyncIntent
            | Event::ResyncCustody
            | Event::Reopen
            | Event::Dispatch
            | Event::Receipt { committed: _ }
            | Event::Bid
            | Event::CancelRequest
            | Event::Suspend,
        )
        | (
            Current::Fence { old: _, new: _ },
            Event::Listing(_)
            | Event::Compensated
            | Event::Projection
            | Event::ResyncIntent
            | Event::ResyncCustody
            | Event::Create
            | Event::Stage
            | Event::Dispatch
            | Event::Receipt { committed: _ }
            | Event::Bid
            | Event::CancelRequest
            | Event::Suspend,
        ) => Err(Refusal::Illegal),
    }
}

pub(crate) async fn write_listing(
    db: &mut PgConnection,
    id: Uuid,
    event: Event,
) -> Result<(), Error> {
    write_listing_with(db, id, event, ListingChange::StateOnly).await
}

pub(crate) enum ListingChange {
    StateOnly,
    Match {
        buyer: Uuid,
        amount: i64,
        reservation: Uuid,
    },
    Invalidate,
}

pub(crate) async fn write_listing_with(
    db: &mut PgConnection,
    id: Uuid,
    event: Event,
    change: ListingChange,
) -> Result<(), Error> {
    let old: State = sqlx::query_scalar("SELECT state FROM bazaar_listings WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *db)
        .await?
        .ok_or(Error::NotFound)?;
    let next = match transition(Current::Listing(old), event, Context::default()) {
        Ok(Outcome::Listing(next)) => next,
        Ok(Outcome::Duplicate) => return Ok(()),
        Ok(
            Outcome::Apply
            | Outcome::Superseded
            | Outcome::Quarantine
            | Outcome::Continue
            | Outcome::Receipt(_)
            | Outcome::GoldBid(_),
        )
        | Err(_) => return Err(Error::IllegalTransition),
    };
    let updated = match change {
        ListingChange::StateOnly => sqlx::query(
            "UPDATE bazaar_listings SET state=$2,updated_at=clock_timestamp() WHERE id=$1 AND state=$3",
        ).bind(id).bind(next).bind(old).execute(db).await?,
        ListingChange::Match { buyer, amount, reservation } => sqlx::query(
            "UPDATE bazaar_listings SET state=$2,updated_at=clock_timestamp(),
             leader=$4,leader_max=$5,price=$5,reservation=$6 WHERE id=$1 AND state=$3",
        ).bind(id).bind(next).bind(old).bind(buyer).bind(amount).bind(reservation).execute(db).await?,
        ListingChange::Invalidate => sqlx::query(
            "UPDATE bazaar_listings SET state=$2,updated_at=now(),
             leader=NULL,leader_max=NULL,reservation=NULL WHERE id=$1 AND state=$3",
        ).bind(id).bind(next).bind(old).execute(db).await?,
    };
    if updated.rows_affected() != 1 {
        return Err(Error::IllegalTransition);
    }
    Ok(())
}

pub(crate) async fn create_listing(
    db: &mut PgConnection,
    id: Uuid,
    input: &crate::NewListing,
) -> Result<crate::Listing, Error> {
    let Ok(Outcome::Listing(next)) = transition(Current::Absent, Event::Create, Context::default())
    else {
        return Err(Error::IllegalTransition);
    };
    Ok(sqlx::query_as(
        "INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,ends_at,state)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING *",
    )
    .bind(id)
    .bind(input.owner)
    .bind(input.kind)
    .bind(input.form)
    .bind(input.currency)
    .bind(input.price)
    .bind(input.accept_offers)
    .bind(input.ends_at)
    .bind(next)
    .fetch_one(db)
    .await?)
}

pub(crate) async fn reopen_intents(
    db: &mut PgConnection,
    old: (u64, u64),
    new: (u64, u64),
) -> Result<(), crate::game::Error> {
    use crate::game::Error;
    let outcome = transition(
        Current::Fence { old, new },
        Event::Reopen,
        Context::default(),
    )
    .map_err(|_| Error::Denied)?;
    if outcome == Outcome::Duplicate {
        return Ok(());
    }
    sqlx::query("UPDATE bazaar_game_issuer SET projection_epoch=$1::text::numeric,recovery_generation=$2::text::numeric WHERE singleton AND projection_epoch=$3::text::numeric AND recovery_generation=$4::text::numeric")
        .bind(new.0.to_string()).bind(new.1.to_string()).bind(old.0.to_string()).bind(old.1.to_string()).execute(&mut *db).await?;
    // The caller holds the issuer lock used by issuance. Lock the affected old
    // rows and recheck their state when updating.
    sqlx::query(
        "WITH locked AS (
        SELECT operation_id,state FROM bazaar_game_outbox
        WHERE projection_epoch<$1::text::numeric OR recovery_generation<$2::text::numeric
        FOR UPDATE
    )
    UPDATE bazaar_game_outbox o SET state='REOPENED',updated_at=now()
    FROM locked WHERE locked.operation_id=o.operation_id AND o.state=locked.state",
    )
    .bind(new.0.to_string())
    .bind(new.1.to_string())
    .execute(db)
    .await?;
    Ok(())
}

pub(crate) struct ProjectionWrite<'a> {
    pub cursor: Projection,
    pub digest: &'a str,
    pub body: &'a str,
    pub watermark: bool,
}

pub(crate) async fn write_projection(
    db: &mut PgConnection,
    authority: &str,
    kind: &str,
    subject: Uuid,
    incoming: ProjectionWrite<'_>,
) -> Result<crate::game::IngestResult, crate::game::Error> {
    use crate::game::{Error, IngestResult};
    let saved: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT projection_epoch::text,projection_revision::text,source_observed_at::text,content_digest
         FROM bazaar_game_projections WHERE source_authority=$1 AND kind=$2 AND subject=$3 FOR UPDATE",
    )
    .bind(authority).bind(kind).bind(subject).fetch_optional(&mut *db).await?;
    let old = if let Some((epoch, revision, observed, _digest)) = &saved {
        Some(Projection {
            epoch: epoch.parse().map_err(|_| Error::Invalid)?,
            revision: revision.parse().map_err(|_| Error::Invalid)?,
            observed: observed.parse().map_err(|_| Error::Invalid)?,
        })
    } else {
        None
    };
    let outcome = transition(
        Current::Projection(old),
        Event::Projection,
        Context {
            incoming: Some(incoming.cursor),
            same_content: saved
                .as_ref()
                .is_some_and(|(_, _, _, digest)| digest == incoming.digest),
            watermark: incoming.watermark,
            ..Context::default()
        },
    )
    .map_err(|_| Error::Invalid)?;
    match outcome {
        Outcome::Superseded => Ok(IngestResult::Superseded),
        Outcome::Duplicate => Ok(IngestResult::Duplicate),
        Outcome::Quarantine => {
            sqlx::query("INSERT INTO bazaar_game_quarantine(source_authority,kind,subject,incoming_digest) VALUES($1,$2,$3,$4)")
                .bind(authority).bind(kind).bind(subject).bind(incoming.digest).execute(db).await?;
            Ok(IngestResult::Quarantined)
        }
        Outcome::Apply => {
            let updated = sqlx::query(
                "INSERT INTO bazaar_game_projections(source_authority,kind,subject,projection_epoch,projection_revision,content_digest,canonical_body,source_observed_at)
                 VALUES($1,$2,$3,$4::text::numeric,$5::text::numeric,$6,$7,$8::text::numeric)
                 ON CONFLICT(source_authority,kind,subject) DO UPDATE SET
                    projection_epoch=EXCLUDED.projection_epoch,projection_revision=EXCLUDED.projection_revision,
                    content_digest=EXCLUDED.content_digest,canonical_body=EXCLUDED.canonical_body,
                    source_observed_at=EXCLUDED.source_observed_at,updated_at=now()
                 WHERE bazaar_game_projections.projection_epoch=$9::text::numeric
                   AND bazaar_game_projections.projection_revision=$10::text::numeric
                   AND bazaar_game_projections.content_digest=$11",
            )
            .bind(authority).bind(kind).bind(subject).bind(incoming.cursor.epoch.to_string())
            .bind(incoming.cursor.revision.to_string()).bind(incoming.digest).bind(incoming.body)
            .bind(incoming.cursor.observed.to_string())
            .bind(saved.as_ref().map(|(epoch, _, _, _)| epoch))
            .bind(saved.as_ref().map(|(_, revision, _, _)| revision))
            .bind(saved.as_ref().map(|(_, _, _, digest)| digest))
            .execute(db).await?;
            if updated.rows_affected() != 1 {
                return Err(Error::Conflict);
            }
            Ok(IngestResult::Applied)
        }
        Outcome::Listing(_) | Outcome::Continue | Outcome::Receipt(_) | Outcome::GoldBid(_) => {
            Err(Error::Invalid)
        }
    }
}

pub(crate) struct Issuance<'a> {
    pub operation: Uuid,
    pub key: String,
    pub listing: Uuid,
    pub actor: Uuid,
    pub intent: &'a crate::game::PreparedIntent,
    pub revision: u64,
    pub epoch: u64,
    pub generation: u64,
}

pub(crate) async fn write_intent(
    db: &mut PgConnection,
    issuance: Issuance<'_>,
) -> Result<crate::game::StagedCommand, crate::game::Error> {
    use crate::game::Error;
    transition(Current::Absent, Event::Stage, Context::default()).map_err(|_| Error::Conflict)?;
    let Issuance {
        operation,
        key,
        listing,
        actor,
        intent,
        revision,
        epoch,
        generation,
    } = issuance;
    let staged=sqlx::query_as::<_,crate::game::StagedCommand>("INSERT INTO bazaar_game_outbox(operation_id,issuance_key,listing_id,actor_id,command,source_revision,intent,binding_digest,projection_epoch,recovery_generation) VALUES($1,$2,$3,$4,$5,$6::text::numeric,$7,$8,$9::text::numeric,$10::text::numeric) RETURNING operation_id,binding_digest,intent,state")
            .bind(operation).bind(key).bind(listing).bind(actor).bind(intent.command().wire()).bind(revision.to_string()).bind(std::str::from_utf8(intent.bytes()).map_err(|_| Error::Invalid)?).bind(intent.binding_digest()).bind(epoch.to_string()).bind(generation.to_string()).fetch_one(db).await?;
    Ok(staged)
}

pub(crate) async fn write_dispatch(
    db: &mut PgConnection,
    operation: Uuid,
    fence: crate::game::RecoveryFence,
) -> Result<(), crate::game::Error> {
    use crate::game::Error;
    let dispatched: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_authority_dispatch WHERE operation_id=o.operation_id)
         FROM bazaar_game_outbox o WHERE o.operation_id=$1 FOR UPDATE",
    )
    .bind(operation)
    .fetch_one(&mut *db)
    .await?;
    if transition(
        Current::Intent {
            dispatched,
            admitted: false,
        },
        Event::Dispatch,
        Context::default(),
    )
    .map_err(|_| Error::Conflict)?
        == Outcome::Apply
    {
        sqlx::query("INSERT INTO bazaar_authority_dispatch(operation_id,projection_epoch,recovery_generation) VALUES($1,$2::text::numeric,$3::text::numeric) ON CONFLICT DO NOTHING")
            .bind(operation).bind(fence.projection_epoch.to_string()).bind(fence.recovery_generation.to_string()).execute(db).await?;
    }
    Ok(())
}

pub(crate) async fn write_receipt(
    db: &mut PgConnection,
    peer: &str,
    intent: &crate::game::PreparedIntent,
    receipt: &crate::game::Receipt,
) -> Result<(), crate::game::Error> {
    use crate::game::Error;
    let admitted: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_authority_receipts
                       WHERE operation_id=o.operation_id AND recovery_generation=$2::text::numeric)
         FROM bazaar_game_outbox o WHERE o.operation_id=$1 FOR UPDATE",
    )
    .bind(receipt.operation)
    .bind(receipt.generation.to_string())
    .fetch_one(&mut *db)
    .await?;
    let Ok(Outcome::Receipt(outcome)) = transition(
        Current::Intent {
            dispatched: false,
            admitted,
        },
        Event::Receipt {
            committed: receipt.committed,
        },
        Context::default(),
    ) else {
        return Err(Error::Conflict);
    };
    sqlx::query("INSERT INTO bazaar_authority_receipts(operation_id,recovery_generation,source_authority,intent_digest,receipt_digest,receipt,outcome) VALUES($1,$2::text::numeric,$3,$4,$5,$6,$7)")
        .bind(receipt.operation).bind(receipt.generation.to_string()).bind(peer)
        .bind(intent.binding_digest()).bind(receipt.digest())
        .bind(std::str::from_utf8(receipt.bytes()).map_err(|_| Error::Invalid)?)
        .bind(outcome).execute(db).await?;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GoldPhase {
    Pending,
    Releasing,
    Accepted,
    Refused,
}

impl GoldPhase {
    fn wire(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Releasing => "releasing",
            Self::Accepted => "accepted",
            Self::Refused => "refused",
        }
    }
}

pub(crate) struct GoldBid {
    pub request: Uuid,
    pub listing: Uuid,
    pub buyer: Uuid,
    pub character: Uuid,
    pub maximum: i64,
    pub operation: Uuid,
}

pub(crate) async fn create_gold_bid(
    db: &mut PgConnection,
    bid: GoldBid,
) -> Result<(), crate::game::Error> {
    use crate::game::Error;
    let Ok(Outcome::GoldBid(phase)) = transition(
        Current::Absent,
        Event::GoldBid(GoldPhase::Pending),
        Context::default(),
    ) else {
        return Err(Error::Conflict);
    };
    sqlx::query("INSERT INTO bazaar_authority_gold_bids(request_id,listing_id,buyer,character_id,maximum,hold_operation_id,phase) VALUES($1,$2,$3,$4,$5,$6,$7)")
        .bind(bid.request).bind(bid.listing).bind(bid.buyer).bind(bid.character).bind(bid.maximum)
        .bind(bid.operation).bind(phase.wire()).execute(db).await?;
    Ok(())
}

pub(crate) async fn write_gold_phase(
    db: &mut PgConnection,
    request: Uuid,
    next: GoldPhase,
    release: Option<Uuid>,
    release_accepted: Option<bool>,
) -> Result<(), crate::game::Error> {
    use crate::game::Error;
    let old: String = sqlx::query_scalar(
        "SELECT phase FROM bazaar_authority_gold_bids WHERE request_id=$1 FOR UPDATE",
    )
    .bind(request)
    .fetch_one(&mut *db)
    .await?;
    let prior = [
        GoldPhase::Pending,
        GoldPhase::Releasing,
        GoldPhase::Accepted,
        GoldPhase::Refused,
    ]
    .into_iter()
    .find(|phase| phase.wire() == old)
    .ok_or(Error::Invalid)?;
    let Ok(Outcome::GoldBid(phase)) = transition(
        Current::GoldBid(prior),
        Event::GoldBid(next),
        Context::default(),
    ) else {
        return Err(Error::Conflict);
    };
    let updated = sqlx::query(
        "UPDATE bazaar_authority_gold_bids SET phase=$2,
           release_operation_id=CASE WHEN $4::boolean IS NOT NULL THEN $3 ELSE COALESCE($3,release_operation_id) END,
           release_accepted=COALESCE($4,release_accepted)
         WHERE request_id=$1 AND phase=$5",
    ).bind(request).bind(phase.wire()).bind(release).bind(release_accepted).bind(old)
    .execute(db).await?;
    if updated.rows_affected() != 1 {
        return Err(Error::Conflict);
    }
    Ok(())
}

/// The issuer lock serializes the projection and resync writers.
pub(crate) async fn quarantine_custody(
    db: &mut PgConnection,
    authority: &str,
    listing: Uuid,
    digest: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO bazaar_game_quarantine(source_authority,kind,subject,incoming_digest)
         SELECT $1,'STATUS',$2,$3 WHERE NOT EXISTS(
             SELECT 1 FROM bazaar_game_quarantine
             WHERE source_authority=$1 AND kind='STATUS' AND subject=$2 AND incoming_digest=$3
         )",
    )
    .bind(authority)
    .bind(listing)
    .bind(digest)
    .execute(db)
    .await?;
    Ok(())
}

/// The issuer lock serializes retry records and the terminal report.
pub(crate) async fn audit_delivery_evidence(
    db: &mut PgConnection,
    listing: Uuid,
    key: &str,
    generation: u64,
    action: &str,
    reason: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO bazaar_audit(relation_name,row_id,after_row)
         VALUES('bazaar_delivery_evidence',$1,jsonb_build_object(
                    'issuance_key',$2::text,'generation',$3::text,'action',$4::text,'reason',$5::text))",
    )
    .bind(listing).bind(key).bind(generation.to_string()).bind(action).bind(reason)
    .execute(db).await?;
    Ok(())
}

/// The listing lock serializes the operator report with transfer recovery.
pub(crate) async fn report_refused_unpublished_bind(
    db: &mut PgConnection,
    listing: Uuid,
    bind: Uuid,
    transfer: Uuid,
    generation: u64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO bazaar_audit(relation_name,row_id,after_row)
         SELECT 'bazaar_authority_transfers',$1,jsonb_build_object(
            'action','refused_unpublished_bind_conflict','bind_operation_id',$2::text,
            'transfer_operation_id',$3::text,'generation',$4::text)
         WHERE NOT EXISTS(SELECT 1 FROM bazaar_audit WHERE relation_name='bazaar_authority_transfers'
            AND row_id=$1 AND after_row->>'action'='refused_unpublished_bind_conflict'
            AND after_row->>'transfer_operation_id'=$3::text
            AND after_row->>'generation'=$4::text)",
    )
    .bind(listing).bind(bind.to_string()).bind(transfer.to_string()).bind(generation.to_string())
    .execute(db).await?;
    Ok(())
}

pub(crate) struct TransferStage<'a> {
    pub operation: Uuid,
    pub listing: Uuid,
    pub purpose: &'a str,
    pub decision: Uuid,
    pub revision: i64,
    pub intent: &'a str,
    pub digest: &'a str,
    pub binding: &'a str,
    pub fence: crate::game::RecoveryFence,
}

pub(crate) async fn stage_transfer(
    db: &mut PgConnection,
    staged: TransferStage<'_>,
) -> Result<(), crate::game::Error> {
    use crate::game::Error;
    transition(Current::Absent, Event::Stage, Context::default()).map_err(|_| Error::Conflict)?;
    let retry: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM bazaar_authority_transfers WHERE listing_id=$1 AND purpose=$2)")
        .bind(staged.listing).bind(staged.purpose).fetch_one(&mut *db).await?;
    let sql = if retry {
        "INSERT INTO bazaar_authority_transfer_retries(operation_id,listing_id,purpose,decision_id,source_revision,intent,intent_digest,binding,projection_epoch,recovery_generation) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9::text::numeric,$10::text::numeric)"
    } else {
        "INSERT INTO bazaar_authority_transfers(operation_id,listing_id,purpose,decision_id,source_revision,intent,intent_digest,binding,projection_epoch,recovery_generation) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9::text::numeric,$10::text::numeric)"
    };
    sqlx::query(sql)
        .bind(staged.operation)
        .bind(staged.listing)
        .bind(staged.purpose)
        .bind(staged.decision)
        .bind(staged.revision)
        .bind(staged.intent)
        .bind(staged.digest)
        .bind(staged.binding)
        .bind(staged.fence.projection_epoch.to_string())
        .bind(staged.fence.recovery_generation.to_string())
        .execute(db)
        .await?;
    Ok(())
}

pub(crate) struct TransferPublication {
    pub operation: Uuid,
    pub revision: i64,
    pub decision: Uuid,
    pub key: String,
    pub from: Uuid,
    pub to: Uuid,
    pub binding: String,
    pub intent: String,
}

pub(crate) async fn publish_transfer_intent(
    db: &mut PgConnection,
    transfer: TransferPublication,
) -> Result<(), crate::game::Error> {
    transition(
        Current::Intent {
            dispatched: false,
            admitted: false,
        },
        Event::Dispatch,
        Context::default(),
    )
    .map_err(|_| crate::game::Error::Conflict)?;
    sqlx::query("INSERT INTO character_commands(id,source_revision,issuer_decision_id,request_key,command,account_id,target_account_id,binding_json,intent_json) OVERRIDING SYSTEM VALUE VALUES($1,$2,$3,$4,'TransferCharacterOwnership',$5,$6,$7,$8)")
        .bind(transfer.operation).bind(transfer.revision).bind(transfer.decision).bind(transfer.key)
        .bind(transfer.from).bind(transfer.to).bind(transfer.binding).bind(transfer.intent).execute(db).await?;
    Ok(())
}

pub(crate) async fn retire_transfer(
    db: &mut PgConnection,
    operation: Uuid,
    evidence: Uuid,
    generation: u64,
) -> Result<(), crate::game::Error> {
    // The caller verifies terminal no-effect under the character publication lock.
    let dispatched =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM character_commands WHERE id=$1)")
            .bind(operation)
            .fetch_one(&mut *db)
            .await?;
    transition(
        Current::Intent {
            dispatched,
            admitted: false,
        },
        Event::RetireTransfer,
        Context::default(),
    )
    .map_err(|_| crate::game::Error::Conflict)?;
    sqlx::query("INSERT INTO bazaar_authority_transfer_retirements(operation_id,evidence_operation_id,recovery_generation) VALUES($1,$2,$3::text::numeric) ON CONFLICT DO NOTHING")
        .bind(operation).bind(evidence).bind(generation.to_string()).execute(db).await?;
    Ok(())
}
