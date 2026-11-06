use super::*;
use axum::{
    Extension, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use oteryn_internal_api::guard::{self, Binding, Identity, IdentityMap};
use std::{
    sync::Mutex,
    time::{Duration as StdDuration, Instant},
};
/// Inserted only by our TLS adapter after chain/EKU/SPKI/purpose verification.
#[derive(Clone, Debug)]
pub(super) struct AuditPeer;
#[derive(Clone, Debug)]
struct EventState {
    runtime: Runtime,
    rate: Arc<Mutex<(Instant, u32)>>,
}
impl Runtime {
    pub(super) fn private_router(&self) -> Result<Router, Error> {
        let identities = Arc::new(
            IdentityMap::new(vec![
                Binding {
                    identity: Identity::spki_sha256(self.config.bazaar_spki),
                    group: super::super::authority_http::BAZAAR_GROUP.into(),
                },
                Binding {
                    identity: Identity::spki_sha256(self.config.receipt_spki),
                    group: oteryn_characters::commands::GROUP.into(),
                },
            ])
            .map_err(|_| Error::Invalid)?,
        );
        let commands = oteryn_characters::commands::Commands::new(
            self.pool.clone(),
            oteryn_characters::commands::Settings {
                enabled: true,
                create_enabled: false,
                transfer_enabled: true,
                source_authority: self.config.authority.config.receipt_authority.clone(),
                ..Default::default()
            },
        );
        let legacy = oteryn_characters::commands::router(commands, identities.clone());
        let receipts = super::super::authority_http::extend_receipt_facility(
            legacy,
            self.authority().clone(),
            self.pool.clone(),
            identities.clone(),
        );
        let bazaar = super::super::authority_http::router(
            self.authority().clone(),
            self.pool.clone(),
            identities,
        );
        let events = Router::new()
            .route("/internal/v1/game-events", post(event))
            .layer(DefaultBodyLimit::max(8192))
            .with_state(EventState {
                runtime: self.clone(),
                rate: Arc::new(Mutex::new((Instant::now(), 0))),
            });
        Ok(bazaar.merge(receipts).merge(events))
    }
}
async fn event(
    State(state): State<EventState>,
    peer: Option<Extension<AuditPeer>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let result = async {
        if peer.is_none() {
            return Err(Error::Denied);
        }
        if headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            != Some("application/x-protobuf")
        {
            return Err(Error::Invalid);
        }
        let allowed = state.rate.lock().is_ok_and(|mut rate| {
            if rate.0.elapsed() >= StdDuration::from_secs(60) {
                *rate = (Instant::now(), 0);
            }
            if rate.1 >= 120 {
                false
            } else {
                rate.1 += 1;
                true
            }
        });
        if !allowed {
            return Err(Error::Backlog);
        }
        state.runtime.config.current()?;
        let mut tx = state.runtime.pool.begin().await?;
        let time = now(&mut tx).await?;
        let result = state
            .runtime
            .authority()
            .ingest_decision(
                &mut tx,
                &state.runtime.authority().config.projection_authority,
                &body,
                time,
            )
            .await?;
        tx.commit().await?;
        if result == super::super::ReceiptResult::Quarantined {
            return Err(Error::Conflict);
        }
        Ok(())
    }
    .await;
    let mut response = match result {
        Ok(()) => (
            [(header::CONTENT_TYPE, "application/json")],
            "{\"contract_version\":1,\"result\":\"accepted\"}",
        )
            .into_response(),
        Err(e) => guard::failure(match e {
            Error::Denied => StatusCode::UNAUTHORIZED,
            Error::Conflict => StatusCode::CONFLICT,
            Error::Invalid => StatusCode::BAD_REQUEST,
            Error::Backlog => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        }),
    };
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store, private"),
    );
    response
}
