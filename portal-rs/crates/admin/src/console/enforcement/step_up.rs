// Reuse ADM-01's account-wide rate limit, fresh TOTP and rotated staff session.
use super::{Controls, Refusal, STEP_UP, SessionAuthority, recent};
use oteryn_identity::mfa::Mfa;
use oteryn_web::session::BrowserSession;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub(crate) async fn verify(
    pool: &PgPool,
    session: &BrowserSession,
    code: &str,
) -> Result<(), Refusal> {
    let mfa = Mfa::configured(pool.clone()).map_err(|_| Refusal::Unavailable)?;
    Controls::new(
        pool.clone(),
        SessionAuthority {
            pool: pool.clone(),
            mfa: mfa.clone(),
        },
        mfa,
        true,
    )
    .step_up(session, code)
    .await
}

pub(crate) async fn confirmed(
    connection: &mut PgConnection,
    session: &BrowserSession,
    account: Uuid,
) -> Result<bool, sqlx::Error> {
    let proof = session.get_in(connection, STEP_UP).await?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(connection)
            .await?;
    Ok(proof.is_some_and(|proof| proof.account == account && recent(proof.step_up_at, now, 300)))
}
