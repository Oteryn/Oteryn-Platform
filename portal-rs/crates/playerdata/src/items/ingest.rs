//! Adapter for T72's dedicated journal listener. Never mount on the public router.
//! The trusted listener inserts `JournalPeer` after purpose-specific mTLS checks.
//! Request headers alone never establish this extension or source authority.
use super::{model::*, store::*};
use axum::{
    Extension, Router,
    body::to_bytes,
    extract::{Request, State},
    http::{StatusCode, Version, header},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct JournalPeer {
    source_authority: String,
}
impl JournalPeer {
    /// Call only after the dedicated journal-purpose mTLS and environment check.
    pub fn from_trusted_listener(source_authority: String) -> Result<Self> {
        key(&source_authority, 128)?;
        Ok(Self { source_authority })
    }
}
#[derive(Debug, Clone)]
struct StateData {
    projection: Projection,
    rate: Arc<Mutex<HashMap<String, (Instant, u16)>>>,
}

pub fn router(projection: Projection) -> Router {
    Router::new()
        .route("/internal/v1/playerdata/journal/records", post(publish))
        .route("/internal/v1/playerdata/journal/cursor", post(cursor))
        .with_state(StateData {
            projection,
            rate: Arc::new(Mutex::new(HashMap::new())),
        })
}
fn response(status: StatusCode, body: &'static str) -> Response {
    let mut r = (status, body).into_response();
    r.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    if status == StatusCode::OK {
        r.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
    }
    if matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE
    ) {
        r.headers_mut()
            .insert(header::RETRY_AFTER, header::HeaderValue::from_static("1"));
    }
    r
}
fn error(e: Error) -> Response {
    response(
        match e {
            Error::Invalid => StatusCode::BAD_REQUEST,
            Error::Conflict => StatusCode::CONFLICT,
            Error::Denied => StatusCode::UNAUTHORIZED,
            Error::Unavailable | Error::Storage(_) => StatusCode::SERVICE_UNAVAILABLE,
        },
        "",
    )
}
async fn body(
    state: &StateData,
    peer: Option<Extension<JournalPeer>>,
    request: Request,
    max: usize,
) -> std::result::Result<(String, Vec<u8>), Response> {
    if !state.projection.flags.ingest {
        return Err(response(StatusCode::SERVICE_UNAVAILABLE, ""));
    }
    let Some(Extension(peer)) = peer else {
        return Err(response(StatusCode::UNAUTHORIZED, ""));
    };
    {
        let Ok(mut rate) = state.rate.lock() else {
            return Err(response(StatusCode::SERVICE_UNAVAILABLE, ""));
        };
        // A trusted listener has a bounded, configured peer set.
        if !rate.contains_key(&peer.source_authority) && rate.len() >= 16 {
            return Err(response(StatusCode::SERVICE_UNAVAILABLE, ""));
        }
        let entry = rate
            .entry(peer.source_authority.clone())
            .or_insert((Instant::now(), 0));
        if entry.0.elapsed().as_secs() >= 60 {
            *entry = (Instant::now(), 0);
        }
        if entry.1 >= 120 {
            return Err(response(StatusCode::TOO_MANY_REQUESTS, ""));
        }
        entry.1 += 1;
    }
    let h = request.headers();
    if request.version() != Version::HTTP_11
        || h.get_all(header::CONTENT_TYPE).iter().count() != 1
        || h.get(header::CONTENT_TYPE)
            .is_none_or(|v| v != "application/json")
        || h.contains_key(header::TRANSFER_ENCODING)
        || h.get_all(header::CONTENT_ENCODING).iter().count() > 1
        || h.get(header::CONTENT_ENCODING)
            .is_some_and(|v| v != "identity")
        || h.get_all(header::CONTENT_LENGTH).iter().count() > 1
    {
        return Err(response(StatusCode::BAD_REQUEST, ""));
    }
    let length = match h.get(header::CONTENT_LENGTH) {
        Some(v) => match v.to_str().ok().and_then(|v| counter(v, true).ok()) {
            Some(n) => Some(n),
            None => return Err(response(StatusCode::BAD_REQUEST, "")),
        },
        None => None,
    };
    if length.is_some_and(|n| n > max as u64) {
        return Err(response(StatusCode::PAYLOAD_TOO_LARGE, ""));
    }
    let bytes = to_bytes(request.into_body(), max)
        .await
        .map_err(|_| response(StatusCode::PAYLOAD_TOO_LARGE, ""))?;
    if length.is_some_and(|n| n != bytes.len() as u64) {
        return Err(response(StatusCode::BAD_REQUEST, ""));
    }
    Ok((peer.source_authority, bytes.to_vec()))
}
async fn publish(
    State(state): State<StateData>,
    peer: Option<Extension<JournalPeer>>,
    request: Request,
) -> Response {
    let (authority, bytes) = match body(&state, peer, request, 65536).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match state.projection.ingest(&bytes, &authority).await {
        Ok(()) => response(
            StatusCode::OK,
            r#"{"contract_version":1,"result":"accepted"}"#,
        ),
        Err(e) => error(e),
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CursorRequest {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub account_id: String,
    pub world_id: String,
    pub character_id: Option<String>,
    pub character_revision: Option<String>,
    pub from_projection_revision: Option<String>,
    pub to_projection_revision: Option<String>,
    pub projection_epoch: Option<String>,
    pub family: String,
    pub recovery_generation: String,
}
impl CursorRequest {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 1024 || bytes.contains(&b'\\') {
            return Err(Error::Invalid);
        }
        let r: Self = serde_json::from_slice(bytes).map_err(|_| Error::Invalid)?;
        if serde_json::to_vec(&r).map_err(|_| Error::Invalid)? != bytes
            || r.contract_version != 1
            || r.operation != "ReadGameJournalCursorV1"
        {
            return Err(Error::Invalid);
        }
        key(&r.source_authority, 128)?;
        id(&r.account_id)?;
        id(&r.world_id)?;
        counter(&r.recovery_generation, false)?;
        let fields = [
            &r.character_revision,
            &r.from_projection_revision,
            &r.to_projection_revision,
            &r.projection_epoch,
        ];
        if r.family == "BANK" {
            if r.character_id.is_some() || fields.iter().any(|v| v.is_some()) {
                return Err(Error::Invalid);
            }
        } else {
            if !matches!(r.family.as_str(), "ITEMS" | "MARKET" | "PLAYER_TRADE") {
                return Err(Error::Invalid);
            }
            id(r.character_id.as_deref().ok_or(Error::Invalid)?)?;
            for v in fields {
                counter(v.as_deref().ok_or(Error::Invalid)?, false)?;
            }
        }
        Ok(r)
    }
    fn scope(&self) -> Scope {
        let cut = self.character_revision.as_ref().and_then(|c| {
            Some(Cut {
                character_revision: c.clone(),
                from_projection_revision: self.from_projection_revision.clone()?,
                to_projection_revision: self.to_projection_revision.clone()?,
                projection_epoch: self.projection_epoch.clone()?,
                recovery_generation: self.recovery_generation.clone(),
            })
        });
        Scope {
            source_authority: self.source_authority.clone(),
            account_id: self.account_id.clone(),
            world_id: self.world_id.clone(),
            character_id: self.character_id.clone(),
            cut,
            family: self.family.clone(),
            recovery_generation: self.recovery_generation.clone(),
        }
    }
}
#[derive(Debug, Serialize)]
struct CursorResponse {
    contract_version: u8,
    after_source_revision: String,
    last_record_id: Option<Uuid>,
}
async fn read_cursor(p: &Projection, bytes: &[u8], peer: &str) -> Result<CursorResponse> {
    let r = CursorRequest::parse(bytes)?;
    if r.source_authority != peer {
        return Err(Error::Denied);
    }
    let scope = r.scope();
    let sk = scope.stream_key()?;
    let mut tx = p.pool.begin().await?;
    p.evidence
        .authorize(&mut tx, &scope, Access::Ingest)
        .await?;
    let row =
        sqlx::query("SELECT cursor::text,fenced FROM pd_t76_streams WHERE stream_key=$1 FOR SHARE")
            .bind(&sk)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(Error::Conflict)?;
    if row.try_get::<bool, _>("fenced")? {
        return Err(Error::Conflict);
    }
    let revision: String = row.try_get("cursor")?;
    let last = if revision == "0" {
        None
    } else {
        Some(sqlx::query_scalar("SELECT record_id FROM pd_t76_records WHERE stream_key=$1 AND source_revision=$2::text::numeric").bind(&sk).bind(&revision).fetch_optional(&mut *tx).await?.ok_or(Error::Conflict)?)
    };
    tx.commit().await?;
    Ok(CursorResponse {
        contract_version: 1,
        after_source_revision: revision,
        last_record_id: last,
    })
}
async fn cursor(
    State(state): State<StateData>,
    peer: Option<Extension<JournalPeer>>,
    request: Request,
) -> Response {
    let (authority, bytes) = match body(&state, peer, request, 1024).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match read_cursor(&state.projection, &bytes, &authority).await {
        Ok(value) => match serde_json::to_string(&value) {
            Ok(body) => {
                let mut r = (StatusCode::OK, body).into_response();
                r.headers_mut().insert(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("no-store"),
                );
                r.headers_mut().insert(
                    header::CONTENT_TYPE,
                    header::HeaderValue::from_static("application/json"),
                );
                r
            }
            Err(_) => error(Error::Unavailable),
        },
        Err(e) => error(e),
    }
}
