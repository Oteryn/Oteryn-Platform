//! Database-backed admission authority: native ticket redemption (gateway login
//! contract §4.2), the character issuance check (§5.4) and, in testing and
//! preproduction with `Issuance` permitted, route selection (§7.4 over the §17
//! mode 34a route records), the §8 claims and the §9 signer. Otherwise `issue`
//! ends with `Unavailable` after every Platform check has passed.
use super::{
    AdmissionAuthority, AdmissionRequest, AuthorizedAccount, Error, Future, Issuance, Material,
    grant, wire,
};
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
    /// Grant issuance policy and signer; the default issues nothing.
    pub issuance: Issuance,
}
async fn seconds(db: &mut PgConnection) -> Result<i64, Error> {
    sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
        .fetch_one(db)
        .await
        .map_err(|_| Error::Unavailable)
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
    /// A retry re-signs only while issuance is permitted and the route record
    /// bound in the grant is unchanged (§6.2, §17); the store re-signs.
    fn validate_replay<'a>(
        &'a self,
        db: &'a mut PgConnection,
        _: &'a AdmissionRequest,
        body: &'a [u8],
    ) -> Future<'a, ()> {
        Box::pin(async move {
            // A retry whose stored kid cannot be re-signed (§6.2 step 5, §6.3).
            self.issuance
                .signer()
                .map_err(|_| Error::ReconciliationRequired)?;
            let (world, channel, endpoint) = wire::scope(body)?;
            let revision = grant::claim(&wire::token(body)?, "route_revision")
                .ok_or(Error::ReconciliationRequired)?;
            grant::current(db, world, channel, &endpoint, &revision).await
        })
    }
    fn issue<'a>(
        &'a self,
        db: &'a mut PgConnection,
        request: &'a AdmissionRequest,
        account: AuthorizedAccount,
    ) -> Future<'a, Material> {
        Box::pin(async move {
            let world = self
                .character_world(db, account.id, request.character_id)
                .await?;
            let signer = self.issuance.signer()?;
            let at = seconds(db).await?;
            let (route, report) =
                grant::select(db, &self.issuance.runtime, world, request.channel_id, at).await?;
            // The trust self-check is refreshed before the issuer clock is read.
            signer.prepare().await?;
            let iat = u64::try_from(seconds(db).await?).map_err(|_| Error::Unavailable)?;
            let ttl = self.issuance.ttl_seconds;
            let payload = grant::payload(&grant::Claims {
                iat,
                ttl,
                jti: grant::jti()?,
                request,
                account,
                world,
                route: &route,
                report: &report,
            })?;
            let expires_at = iat + u64::from(ttl);
            let token = signer.sign(&payload, expires_at, iat)?;
            Ok(Material {
                world_id: world,
                channel_id: route.channel,
                endpoint: route.endpoint,
                token,
                expires_at_ms: i64::try_from(expires_at)
                    .ok()
                    .and_then(|s| s.checked_mul(1000))
                    .ok_or(Error::Unavailable)?,
            })
        })
    }
}
