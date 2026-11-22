// Reuse ADM-01's account-wide rate limit, fresh TOTP and rotated staff session.
use super::{Controls, Refusal, STEP_UP, STEP_UP_NAME, SessionAuthority, recent};
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
    proof_current(connection, proof, account).await
}

/// The appeal adapter uses the already transaction-locked staff session proof.
pub async fn appeal_step_up_current(
    connection: &mut PgConnection,
    context: crate::rbac::CheckContext,
) -> Result<bool, sqlx::Error> {
    let Some(binding) = context.session_proof else {
        return Ok(false);
    };
    let proof: Option<sqlx::types::Json<super::StepUp>> = sqlx::query_scalar(
        "SELECT data->($3::text) FROM web_sessions WHERE id_hash=$1 AND account_id=$2
           AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp()",
    )
    .bind(binding.session_hash.as_slice())
    .bind(context.account)
    .bind(STEP_UP_NAME)
    .fetch_optional(&mut *connection)
    .await?
    .flatten();
    proof_current(connection, proof.map(|proof| proof.0), context.account).await
}

async fn proof_current(
    connection: &mut PgConnection,
    proof: Option<super::StepUp>,
    account: Uuid,
) -> Result<bool, sqlx::Error> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(connection)
            .await?;
    Ok(proof.is_some_and(|proof| proof.account == account && recent(proof.step_up_at, now, 300)))
}
