//! Reusable axum extractor. Install a trusted `CheckContext` in request extensions
//! after authenticating the request. Missing context always rejects the request.

use super::{AccountStateReader, Authorizer, CheckContext, Error, Permission};
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
};
use std::{marker::PhantomData, sync::Arc};

/// A route chooses a permission in its type, rather than accepting one from the client.
pub trait RequiredPermission: Send + Sync {
    const PERMISSION: Permission;
}

#[derive(Debug)]
pub struct Require<P, A> {
    context: CheckContext,
    marker: PhantomData<(P, A)>,
}

impl<P, A> Require<P, A> {
    pub fn context(&self) -> CheckContext {
        self.context
    }
}

impl<S, A, P> FromRequestParts<S> for Require<P, A>
where
    S: Send + Sync,
    A: AccountStateReader + 'static,
    P: RequiredPermission,
    Arc<Authorizer<A>>: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let context = parts
            .extensions
            .get::<CheckContext>()
            .copied()
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let authorizer = Arc::<Authorizer<A>>::from_ref(state);
        authorizer
            .check(context, P::PERMISSION)
            .await
            .map_err(|error| match error {
                Error::Storage(_) => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::FORBIDDEN,
            })?;
        Ok(Self {
            context,
            marker: PhantomData,
        })
    }
}
