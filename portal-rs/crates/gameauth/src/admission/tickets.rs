//! Database-backed admission authority: native ticket redemption (gateway login
//! contract §4.2) and the character issuance check (§5.4). Grant issuance (§7
//! route records, §8 claims, §9 signer) is not configured in this process, so
//! `issue` ends with `Unavailable` after every Platform check has passed.
use super::{AdmissionAuthority, AdmissionRequest, AuthorizedAccount, Error, Future, Material};
use sqlx::{PgConnection, Row};
use uuid::Uuid;

const AUDIENCE: &str = "oteryn-native-game-gateway";
/// LCFA feed freshness bound S and clock uncertainty (contract §5.2), the same
/// bounds the device-session character read uses.
const FRESHNESS_SECONDS: i64 = 30;
const CLOCK_UNCERTAINTY_SECONDS: i64 = 1;

#[derive(Debug, Clone, Default)]
pub struct NativeTickets {
    /// `source_authority` of the LCFA producer; empty means no character source,
    /// which fails issuance closed with `RouteUnavailable` (contract §17, 33a off).
    pub projection_authority: String,
}

impl NativeTickets {
    async fn character_world(
        &self,
        db: &mut PgConnection,
        account: Uuid,
        character: Uuid,
    ) -> Result<Uuid, Error> {
        if self.projection_authority.is_empty() {
            return Err(Error::RouteUnavailable);
        }
        let live: Option<bool> = sqlx::query_scalar("SELECT highest_epoch IS NOT NULL AND highest_epoch=watermark_epoch AND source_authority=$1 AND complete_through<=extract(epoch FROM clock_timestamp())+$2 AND extract(epoch FROM clock_timestamp())-complete_through+$2<=$3 FROM account_character_feed WHERE id=1 FOR SHARE")
            .bind(&self.projection_authority).bind(CLOCK_UNCERTAINTY_SECONDS).bind(FRESHNESS_SECONDS)
            .fetch_optional(&mut *db).await.map_err(|_| Error::Unavailable)?;
        if live != Some(true) {
            return Err(Error::RouteUnavailable);
        }
        let valid: Option<bool> = sqlx::query_scalar("SELECT NOT s.invalid AND s.projection_epoch=f.highest_epoch AND s.source_authority=$2 FROM account_character_snapshots s CROSS JOIN account_character_feed f WHERE s.account_id=$1 AND f.id=1")
            .bind(account).bind(&self.projection_authority)
            .fetch_optional(&mut *db).await.map_err(|_| Error::Unavailable)?;
        if valid != Some(true) {
            return Err(Error::RouteUnavailable);
        }
        let row = sqlx::query("SELECT world_id, availability FROM account_characters WHERE account_id=$1 AND character_id=$2")
            .bind(account).bind(character)
            .fetch_optional(db).await.map_err(|_| Error::Unavailable)?;
        match row {
            Some(row)
                if row
                    .try_get::<String, _>("availability")
                    .map_err(|_| Error::Unavailable)?
                    == "AVAILABLE" =>
            {
                row.try_get("world_id").map_err(|_| Error::Unavailable)
            }
            _ => Err(Error::CharacterConflict),
        }
    }
}

impl AdmissionAuthority for NativeTickets {
    fn redeem<'a>(
        &'a self,
        db: &'a mut PgConnection,
        request: &'a AdmissionRequest,
    ) -> Future<'a, AuthorizedAccount> {
        Box::pin(async move {
            let ticket = sqlx::query("SELECT account_id, game_generation, native_generation, audience, used_at IS NULL AND expires_at > clock_timestamp() AS usable FROM game_login_tickets WHERE ticket_hash=$1 FOR UPDATE")
                .bind(request.ticket_hash()).fetch_optional(&mut *db).await.map_err(|_| Error::Unavailable)?
                .ok_or(Error::TicketRejected)?;
            let field = |name: &str| -> Result<String, Error> {
                ticket.try_get(name).map_err(|_| Error::Unavailable)
            };
            let usable: bool = ticket.try_get("usable").map_err(|_| Error::Unavailable)?;
            if field("audience")? != AUDIENCE || !usable {
                return Err(Error::TicketRejected);
            }
            let account: Uuid = ticket
                .try_get("account_id")
                .map_err(|_| Error::Unavailable)?;
            let current = sqlx::query("SELECT status, game_auth_generation::text AS game, native_security_generation::text AS native FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(account).fetch_optional(&mut *db).await.map_err(|_| Error::Unavailable)?
                .ok_or(Error::SecurityDenied)?;
            let get = |name: &str| -> Result<String, Error> {
                current.try_get(name).map_err(|_| Error::Unavailable)
            };
            let native = field("native_generation")?;
            if get("status")? != "active"
                || get("game")? != field("game_generation")?
                || get("native")? != native
            {
                return Err(Error::SecurityDenied);
            }
            let security_generation = native.parse().map_err(|_| Error::SecurityDenied)?;
            sqlx::query("UPDATE game_login_tickets SET used_at=clock_timestamp(), attempt_ref=$2, updated_at=clock_timestamp() WHERE ticket_hash=$1")
                .bind(request.ticket_hash()).bind(request.attempt_ref)
                .execute(db).await.map_err(|_| Error::Unavailable)?;
            Ok(AuthorizedAccount {
                id: account,
                security_generation,
            })
        })
    }
    /// No grant is ever stored by this authority, so a stored attempt cannot be
    /// re-signed here.
    fn validate_replay<'a>(
        &'a self,
        _: &'a mut PgConnection,
        _: &'a AdmissionRequest,
        _: &'a [u8],
    ) -> Future<'a, ()> {
        Box::pin(async { Err(Error::ReconciliationRequired) })
    }
    fn issue<'a>(
        &'a self,
        db: &'a mut PgConnection,
        request: &'a AdmissionRequest,
        account: AuthorizedAccount,
    ) -> Future<'a, Material> {
        Box::pin(async move {
            self.character_world(db, account.id, request.character_id)
                .await?;
            // TODO(T19c, plan §6 "Granty admission", contract §7.3/§8/§9): route
            // records, claim assembly and the issuer signer await owner decisions
            // (FND-04B-AMEND-1, U3, U4, U7). Without a signer the issuer signs
            // nothing (§9.2 self-check).
            Err(Error::Unavailable)
        })
    }
}
