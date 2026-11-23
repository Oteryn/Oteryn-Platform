use std::{collections::BTreeMap, path::Path};

use async_trait::async_trait;
use axum::Router;
use oteryn_admin::{
    downloads::{PublicationFailure, ReleasePublisher, SignedBundle},
    rbac::{CheckContext, MutationAuthorization, Permission},
};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{AccountState, Channel, Downloads, Error, FeatureFlags, PublishAccess, Upload};

pub fn configured_router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    let flags = FeatureFlags {
        downloads: std::env::var("OTERYN_DOWNLOADS_ENABLED").as_deref() == Ok("true"),
    };
    let mut roots = BTreeMap::new();
    if flags.downloads {
        for (channel, key) in [
            (Channel::Stable, "OTERYN_DOWNLOADS_ROOT_STABLE"),
            (Channel::Beta, "OTERYN_DOWNLOADS_ROOT_BETA"),
        ] {
            if let Some(path) = std::env::var_os(key)
                && let Ok(root) = crate::command::read_bounded(Path::new(&path), 1024 * 1024)
                && crate::verification::anchor_version(&root).is_ok()
            {
                roots.insert(channel, root);
            }
        }
    }
    oteryn_admin::downloads::configured_router(pool.clone(), Downloads::new(pool, flags, roots))
}

impl ReleasePublisher for Downloads {
    fn ready(&self, channel: &str) -> bool {
        self.publication_ready(channel)
    }

    async fn publish(
        &self,
        context: CheckContext,
        authority: MutationAuthorization,
        bundle: SignedBundle,
    ) -> Result<Uuid, PublicationFailure> {
        let channel = Channel::parse(&bundle.channel).map_err(|_| PublicationFailure::Invalid)?;
        if !self.ready(channel.as_str()) {
            return Err(PublicationFailure::Unavailable);
        }
        let access = BrowserAccess { context, authority };
        Downloads::publish(
            self,
            &access,
            context.account,
            Upload {
                channel,
                policy: bundle.policy,
                metadata: bundle.metadata,
            },
        )
        .await
        .map_err(|error| match error {
            Error::Invalid(_) => PublicationFailure::Invalid,
            Error::Verification => PublicationFailure::Verification,
            Error::Conflict => PublicationFailure::Conflict,
            Error::Denied => PublicationFailure::Denied,
            Error::Disabled | Error::NotFound | Error::Storage(_) => {
                PublicationFailure::Unavailable
            }
        })
    }
}

/// The shared staff guard has already checked the account and permission.
/// The publication service checks the durable browser proof again under its locks.
struct BrowserAccess {
    context: CheckContext,
    authority: MutationAuthorization,
}

#[async_trait]
impl PublishAccess for BrowserAccess {
    async fn account_state(&self, actor: Uuid) -> AccountState {
        if actor == self.context.account && self.context.mfa_verified {
            AccountState::Active
        } else {
            AccountState::Unknown
        }
    }

    async fn may_publish(&self, actor: Uuid, _: Channel) -> Option<bool> {
        Some(actor == self.context.account)
    }

    async fn authorize_in(&self, connection: &mut PgConnection, actor: Uuid, _: Channel) -> bool {
        actor == self.context.account
            && self
                .authority
                .check_in(connection, self.context, Permission::ManageDownloads)
                .await
                .is_ok()
    }
}
