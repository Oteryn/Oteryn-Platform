use super::*;
use crate::{oauth, tickets};

/// Live OAuth allowlist plus database-locked native-client policy.
#[derive(Debug, Clone)]
pub struct PlatformAuthority {
    pub oauth: oauth::Config,
    pub projection_authority: String,
}
pub(crate) async fn lock_client(
    db: &mut PgConnection,
    config: &oauth::Config,
    client: Uuid,
) -> Result<()> {
    if !oauth::native_client(config, client) {
        return Err(Failure::Denied);
    }
    let allowed: Option<bool> = sqlx::query_scalar("SELECT enabled AND first_party AND public_client AND native_ticket FROM oauth_native_clients WHERE client_id=$1 FOR UPDATE")
        .bind(client).fetch_optional(db).await.map_err(unavailable)?;
    if allowed == Some(true) {
        Ok(())
    } else {
        Err(Failure::Denied)
    }
}
impl NativeAuthority for PlatformAuthority {
    fn enroll<'a>(
        &'a self,
        db: &'a mut PgConnection,
        bearer: &'a str,
    ) -> NativeFuture<'a, Enrollment> {
        Box::pin(async move {
            oauth::native_enrollment(db, &self.oauth, bearer)
                .await
                .map_err(|e| match e {
                    oauth::Error::Unavailable => Failure::Unavailable,
                    _ => Failure::Denied,
                })
        })
    }
    fn verify<'a>(
        &'a self,
        db: &'a mut PgConnection,
        auth: &'a Authorization,
    ) -> NativeFuture<'a, ()> {
        Box::pin(async move {
            let current: Option<(String,i64,i64,bool)> = sqlx::query_as("SELECT status,game_auth_generation,native_security_generation,email_verified_at IS NOT NULL FROM accounts WHERE id=$1 FOR UPDATE")
                .bind(auth.account_id).fetch_optional(&mut *db).await.map_err(unavailable)?;
            if !current.is_some_and(|(status, game, native, verified)| {
                status == "active"
                    && verified
                    && game == auth.game_generation
                    && native == auth.native_generation
            }) {
                return Err(Failure::Denied);
            }
            lock_client(db, &self.oauth, auth.client_id).await
        })
    }
    fn use_native<'a>(
        &'a self,
        db: &'a mut PgConnection,
        auth: &'a Authorization,
        family: Uuid,
        purpose: Purpose,
    ) -> NativeFuture<'a, NativeResult> {
        Box::pin(async move {
            match purpose {
                Purpose::Ticket => {
                    let issued = tickets::issue_on_connection(
                        db,
                        tickets::Principal {
                            account_id: auth.account_id,
                            status: tickets::AccountStatus::Active,
                            game_generation: auth
                                .game_generation
                                .try_into()
                                .map_err(|_| Failure::Denied)?,
                            native_generation: auth
                                .native_generation
                                .try_into()
                                .map_err(|_| Failure::Denied)?,
                        },
                        Some(family),
                    )
                    .await
                    .map_err(|e| match e {
                        tickets::Error::Unauthorized => Failure::Denied,
                        _ => Failure::Unavailable,
                    })?;
                    let expires_at: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM expires_at))::bigint FROM game_login_tickets WHERE ticket_hash=$1")
                        .bind(hash(&issued.ticket)).fetch_one(db).await.map_err(unavailable)?;
                    Ok(NativeResult::Ticket {
                        ticket: issued.ticket,
                        expires_in: i64::from(issued.expires_in),
                        expires_at,
                    })
                }
                Purpose::Characters => {
                    if self.projection_authority.is_empty() {
                        return Err(Failure::Unavailable);
                    }
                    // Matches the native projection's 30-second freshness and 1-second
                    // uncertainty bounds; the shared lock excludes concurrent replacement.
                    let live: Option<bool> = sqlx::query_scalar("SELECT highest_epoch IS NOT NULL AND highest_epoch=watermark_epoch AND source_authority=$1 AND complete_through<=extract(epoch FROM clock_timestamp())+1 AND extract(epoch FROM clock_timestamp())-complete_through+1<=30 FROM account_character_feed WHERE id=1 FOR SHARE")
                        .bind(&self.projection_authority).fetch_optional(&mut *db).await.map_err(unavailable)?;
                    if live != Some(true) {
                        return Err(Failure::Unavailable);
                    }
                    let valid: Option<bool> = sqlx::query_scalar("SELECT NOT s.invalid AND s.projection_epoch=f.highest_epoch AND s.source_authority=$2 FROM account_character_snapshots s CROSS JOIN account_character_feed f WHERE s.account_id=$1 AND f.id=1")
                        .bind(auth.account_id).bind(&self.projection_authority).fetch_optional(&mut *db).await.map_err(unavailable)?;
                    if valid != Some(true) {
                        return Err(Failure::Unavailable);
                    }
                    let rows = sqlx::query("SELECT character_id,world_id,name,availability FROM account_characters WHERE account_id=$1 ORDER BY character_id LIMIT 65")
                        .bind(auth.account_id).fetch_all(db).await.map_err(unavailable)?;
                    let characters = rows
                        .into_iter()
                        .map(|row| {
                            Ok(Character {
                                character_id: row.try_get("character_id").map_err(unavailable)?,
                                world_id: row.try_get("world_id").map_err(unavailable)?,
                                name: row.try_get("name").map_err(unavailable)?,
                                availability: row.try_get("availability").map_err(unavailable)?,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    Ok(NativeResult::Characters(characters))
                }
            }
        })
    }
}

/// Allows the normal ticket issuer to consume the original enrolled OAuth family
/// using the same verifier and locks as enrollment.
impl tickets::OAuthAuthority for PlatformAuthority {
    fn verify<'a>(
        &'a self,
        db: &'a mut PgConnection,
        bearer: &'a str,
        consume: bool,
    ) -> tickets::ResultFuture<'a, tickets::Principal> {
        Box::pin(async move {
            let enrollment = oauth::native_enrollment(db, &self.oauth, bearer)
                .await
                .map_err(|e| match e {
                    oauth::Error::Unavailable => tickets::Error::Unavailable,
                    _ => tickets::Error::Unauthorized,
                })?;
            if consume {
                sqlx::query("UPDATE oauth_token_families SET revoked_at=clock_timestamp(),updated_at=now() WHERE id=(SELECT family_id FROM oauth_tokens WHERE id::text=$1)")
                    .bind(enrollment.token_id).execute(&mut *db).await.map_err(|_| tickets::Error::Unavailable)?;
            }
            let auth = enrollment.authorization;
            Ok(tickets::Principal {
                account_id: auth.account_id,
                status: tickets::AccountStatus::Active,
                game_generation: auth
                    .game_generation
                    .try_into()
                    .map_err(|_| tickets::Error::Unauthorized)?,
                native_generation: auth
                    .native_generation
                    .try_into()
                    .map_err(|_| tickets::Error::Unauthorized)?,
            })
        })
    }
}
