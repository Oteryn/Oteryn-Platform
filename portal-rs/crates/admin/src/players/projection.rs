//! Only the public projection API may read game character data, including its freshness gates.
use super::{Character, Error};
use oteryn_characters::lcfa::{self, AccountState, AccountStateLookup, Projection, ReadState};
use sqlx::PgPool;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;
#[derive(Debug)]
struct Identity(oteryn_identity::accounts::Accounts);
impl AccountStateLookup for Identity {
    fn state(
        &self,
        id: lcfa::AccountId,
    ) -> Pin<Box<dyn Future<Output = AccountState> + Send + '_>> {
        Box::pin(async move {
            match self.0.find_by_id(id.as_uuid()).await {
                Ok(Some(a)) if a.status == "active" && a.email_verified_at.is_some() => {
                    AccountState::Active
                }
                Ok(Some(_)) => AccountState::Denied,
                Ok(None) => AccountState::Unknown,
                Err(_) => AccountState::Unavailable,
            }
        })
    }
}
pub(super) fn configured(
    pool: PgPool,
    enabled: bool,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Projection {
    let environment = match lookup("OTERYN_ENV").as_deref() {
        Some("testing") => oteryn_core::Environment::Testing,
        Some("preprod") => oteryn_core::Environment::Preprod,
        _ => oteryn_core::Environment::Local,
    };
    Projection::new(
        pool.clone(),
        lcfa::Settings {
            enabled,
            environment,
            source_authority: lookup("OTERYN_ACCOUNT_CHARACTERS_AUTHORITY").unwrap_or_default(),
            ..lcfa::Settings::default()
        },
        Arc::new(Identity(oteryn_identity::accounts::Accounts { pool })),
    )
}
pub(super) async fn characters(projection: &Projection, id: Uuid) -> Result<Vec<Character>, Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Unavailable)?;
    let now = i64::try_from(now.as_secs()).map_err(|_| Error::Unavailable)?;
    let account = lcfa::AccountId::parse(&id.to_string()).map_err(|_| Error::BadInput)?;
    let view = projection
        .list_characters(account, now)
        .await
        .map_err(|_| Error::Unavailable)?;
    match view.state {
        ReadState::Ready => Ok(view
            .characters
            .into_iter()
            .map(|c| Character {
                id: c.character_id.as_uuid(),
                world: c.world_id.as_uuid(),
                name: c.name,
            })
            .collect()),
        ReadState::Denied => Err(Error::Denied),
        _ => Err(Error::Unavailable),
    }
}
