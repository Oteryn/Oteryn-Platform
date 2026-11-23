//! RBAC, audit log, four-eyes approvals and the staff panel shell (design T10).

pub mod audit;
pub mod downloads;
pub mod rbac;

pub mod web;

pub mod bazaar;
pub mod commerce;
mod mail_ops;
pub use control::controls;
pub mod players;
pub mod support;

pub mod console;
pub mod control;

pub use control::control_http;

pub mod analytics;

#[cfg(test)]
mod begin_guard_tests;

#[cfg(test)]
pub mod test_staff;

#[cfg(test)]
extern crate self as oteryn_admin;

#[cfg(test)]
async fn test_staff_authenticate(
    pool: &sqlx::PgPool,
    session: &oteryn_web::session::BrowserSession,
    actor: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    if session.account_id().await == Some(actor) {
        return Ok(());
    }
    let proof = test_staff::proof(pool, actor).await?;
    let grant: String =
        sqlx::query_scalar("SELECT data->>'mfa_grant' FROM web_sessions WHERE id_hash=$1")
            .bind(proof.session_hash.as_slice())
            .fetch_one(pool)
            .await?;
    let generation: i64 =
        sqlx::query_scalar("SELECT native_security_generation FROM accounts WHERE id=$1")
            .bind(actor)
            .fetch_one(pool)
            .await?;
    oteryn_web::session::rotate(
        session,
        oteryn_web::session::PrivilegeChange::AuthorizedLogin(actor, generation),
    )
    .await?;
    session
        .set(
            oteryn_web::session::SessionKey::<String>::new(oteryn_identity::mfa::SESSION_GRANT_KEY),
            &grant,
        )
        .await
}
