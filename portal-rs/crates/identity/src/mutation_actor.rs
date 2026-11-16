//! Browser mutation identity. A cached account ID alone is not browser authorization.
use crate::AccountId;
use sqlx::PgConnection;

pub struct MutationActor {
    pub(crate) account: AccountId,
    binding: Option<String>,
}
impl MutationActor {
    /// Use the server-side session binding, never a submitted form value.
    pub fn browser(account: AccountId, binding: String) -> Self {
        Self {
            account,
            binding: Some(binding),
        }
    }
    /// Caller must hold the account row lock through mutation and commit.
    pub async fn live(&self, conn: &mut PgConnection) -> Result<bool, sqlx::Error> {
        let Some(binding) = &self.binding else {
            return Ok(true);
        };
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM web_sessions WHERE account_id=$1 AND csrf_token=$2 AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp())")
            .bind(self.account).bind(binding).fetch_one(conn).await
    }
}
/// Account-only service calls are trusted internal operations, not browser entry points.
impl From<AccountId> for MutationActor {
    fn from(account: AccountId) -> Self {
        Self {
            account,
            binding: None,
        }
    }
}
impl std::fmt::Debug for MutationActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MutationActor")
            .field("account", &self.account)
            .finish_non_exhaustive()
    }
}
