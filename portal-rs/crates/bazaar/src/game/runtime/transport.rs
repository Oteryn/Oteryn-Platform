use super::*;
use axum::{
    Extension, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, FromRequest, Request, State},
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
    /// Builds the routes served by the dedicated Bazaar listener.
    pub fn private_router(&self) -> Result<Router, Error> {
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
        let legacy = oteryn_characters::restore::router(
            oteryn_characters::restore::Restore::new(self.pool.clone(), Default::default()),
            oteryn_characters::commands::Settings {
                enabled: true,
                create_enabled: false,
                transfer_enabled: true,
                source_authority: self.config.authority.config.receipt_authority.clone(),
                ..Default::default()
            },
            identities.clone(),
        );
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
    request: Request,
) -> Response {
    let mut body_failure = None;
    let result = async {
        state.runtime.config.current()?;
        if peer.is_none() {
            return Err(Error::Denied);
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
        if headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            != Some("application/x-protobuf")
        {
            return Err(Error::Invalid);
        }
        let body = Bytes::from_request(request, &state)
            .await
            .map_err(|rejection| {
                body_failure = Some(rejection.status());
                Error::Invalid
            })?;
        state.runtime.config.current()?;
        let mut tx = crate::game::authority_http::begin_exchange(&state.runtime.pool).await?;
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
            Error::Conflict | Error::CustodyDrift => StatusCode::CONFLICT,
            Error::Invalid => body_failure.unwrap_or(StatusCode::BAD_REQUEST),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{AuthorityConfig, RecoveryFence};
    use tower::ServiceExt;

    #[tokio::test]
    async fn v1_transfer_receipt_requires_restore_lock_row()
    -> Result<(), Box<dyn std::error::Error>> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIPPED receipt restore lock: PORTAL_TEST_DATABASE_URL unset");
            return Ok(());
        };
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await?;
        // Session-local tables shadow production data, including the singleton lock.
        sqlx::raw_sql("CREATE TEMP TABLE character_restore_locks(id smallint);
            CREATE TEMP TABLE character_restore_notices(new_recovery_generation text);
            CREATE TEMP TABLE character_commands(id uuid,command text,binding_json text,receipt_json text)")
            .execute(&pool).await?;
        let authority = Authority {
            config: AuthorityConfig {
                receipt_authority: "oteryn:character-authority:primary".into(),
                ..Default::default()
            },
            fence: RecoveryFence {
                projection_epoch: 1,
                recovery_generation: 3,
            },
        };
        let directory = std::env::temp_dir().join(format!("bazaar-transport-{}", Uuid::now_v7()));
        std::fs::create_dir(&directory)?;
        let config = Config::fixture(authority, &directory);
        std::fs::remove_dir_all(directory)?;
        let peer = guard::VerifiedClient::from_verified_tls13_http11(Identity::spki_sha256(
            config.receipt_spki,
        ));
        let router = Runtime {
            pool: pool.clone(),
            config: Arc::new(config),
        }
        .private_router()?;
        let request = || {
            Request::post("/internal/v1/game-auth/character-commands/receipt")
                .header(header::CONTENT_TYPE, "application/json")
                .extension(peer.clone())
                .body(axum::body::Body::from(
                    include_str!(
                        "../../../../characters/src/commands/fixtures/transfer-receipt.json"
                    )
                    .trim(),
                ))
        };
        assert_eq!(
            router.clone().oneshot(request()?).await?.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "missing restore lock must fence v1 receipts"
        );
        sqlx::query("INSERT INTO character_restore_locks VALUES(1)")
            .execute(&pool)
            .await?;
        assert_eq!(
            router.oneshot(request()?).await?.status(),
            StatusCode::NOT_FOUND
        );
        Ok(())
    }
}
