//! G-9 revision 3 durable transport. No hunting read, sharing or aggregation is
//! enabled by ingest. Stream and catalogue admissions are authoritative inputs,
//! never inferred from incoming records or an arbitrary generation UUID.
use axum::{
    Router,
    body::{Body, Bytes},
    extract::State,
    http::{StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::post,
};
use oteryn_internal_api::{
    canonical::{self, Value},
    guard::{self, IdentityMap, JsonRule, RoutePolicy},
    playerdata::{
        self,
        sessions::{self as codec, Exchange, SessionRecord},
    },
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use uuid::Uuid;

pub const GROUP: &str = "playerdata-journal-sessions";
pub const IDENTITY: &str = "PORTAL_PLAYERDATA_JOURNAL_SESSIONS_SAN";
pub const RECORDS: &str = "/internal/v1/playerdata/journal/records";
pub const COMMANDS: [&str; 3] = [
    "/internal/v1/playerdata/journal/commands/pending",
    "/internal/v1/playerdata/journal/commands/read",
    "/internal/v1/playerdata/journal/commands/receipt",
];
#[derive(Clone, Debug, Default)]
pub struct Settings {
    pub ingest: bool,
    pub commands: bool,
    pub source: String,
}
impl Settings {
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Self {
        Self {
            ingest: get("PORTAL_PLAYERDATA_G9_INGEST_ENABLED").as_deref() == Some("true"),
            commands: get("PORTAL_PLAYERDATA_G9_REPLAY_COMMANDS_ENABLED").as_deref()
                == Some("true"),
            source: get("PORTAL_PLAYERDATA_G9_SOURCE_AUTHORITY").unwrap_or_default(),
        }
    }
}
#[derive(Clone)]
struct Surface {
    db: PgPool,
    config: Settings,
    budget: Arc<Mutex<(Instant, u32)>>,
}
fn response(status: StatusCode, bytes: Vec<u8>) -> Response {
    let mut r = Response::new(Body::from(bytes));
    *r.status_mut() = status;
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
    r
}
fn accepted() -> Vec<u8> {
    br#"{"contract_version":1,"result":"accepted"}"#.to_vec()
}
fn invalid(error: playerdata::Error) -> StatusCode {
    match error {
        playerdata::Error::Invalid => StatusCode::BAD_REQUEST,
        playerdata::Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    }
}
fn unavailable(_: sqlx::Error) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, StatusCode> {
    playerdata::get(v, k)
        .and_then(playerdata::text)
        .map_err(invalid)
}
fn id(s: &str) -> Result<Uuid, StatusCode> {
    Uuid::parse_str(s).map_err(|_| StatusCode::BAD_REQUEST)
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
async fn budget(State(s): State<Surface>, request: axum::extract::Request, next: Next) -> Response {
    let allowed = s.budget.lock().map_err(|_| ()).map(|mut w| {
        if w.0.elapsed().as_secs() >= 60 {
            *w = (Instant::now(), 0)
        };
        if w.1 >= 120 {
            false
        } else {
            w.1 += 1;
            true
        }
    });
    match allowed {
        Ok(true) => next.run(request).await,
        Ok(false) => {
            let mut r = response(StatusCode::TOO_MANY_REQUESTS, Vec::new());
            r.headers_mut()
                .insert(header::RETRY_AFTER, header::HeaderValue::from_static("60"));
            r
        }
        Err(_) => response(StatusCode::SERVICE_UNAVAILABLE, Vec::new()),
    }
}
async fn headers(request: axum::extract::Request, next: Next) -> Response {
    let mut r = next.run(request).await;
    r.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    r
}
pub fn router(db: PgPool, config: Settings, identities: Arc<IdentityMap>) -> Router {
    let valid = !config.source.is_empty()
        && config.source.len() <= 128
        && config
            .source
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b));
    let state = Surface {
        db,
        config: config.clone(),
        budget: Arc::new(Mutex::new((Instant::now(), 0))),
    };
    let mut app = Router::new();
    for (path, max, on) in [
        (RECORDS, 1048576, config.ingest),
        (COMMANDS[0], 512, config.commands),
        (COMMANDS[1], 512, config.commands),
        (COMMANDS[2], 1024, config.commands),
    ] {
        let policy = RoutePolicy {
            enabled: on && valid,
            group: GROUP.into(),
            max_body_bytes: max,
            max_json_depth: 9,
            json: JsonRule::Object,
        };
        let r = Router::new()
            .route(path, post(handle))
            .with_state(state.clone());
        let r = guard::validate_body(r, policy.clone())
            .layer(middleware::from_fn_with_state(state.clone(), budget));
        app = app.merge(guard::protect_peer(r, identities.clone(), policy));
    }
    app.layer(middleware::from_fn(headers))
}
async fn handle(
    State(s): State<Surface>,
    uri: axum::extract::OriginalUri,
    headers: axum::http::HeaderMap,
    bytes: Bytes,
) -> Response {
    let result = async {
        if headers.get(header::CONTENT_LENGTH).is_some_and(|v| {
            v.to_str().ok().and_then(|s| s.parse::<usize>().ok()) != Some(bytes.len())
        }) {
            return Err(StatusCode::BAD_REQUEST);
        }
        // Source binding follows transport/shape, preceding the closed route codec.
        let v = canonical::decode(&bytes, 1048576, 9).map_err(|_| StatusCode::BAD_REQUEST)?;
        if text(&v, "source_authority")? != s.config.source {
            return Err(StatusCode::UNAUTHORIZED);
        }
        if uri.path() == RECORDS {
            let r = codec::record(&bytes).map_err(invalid)?;
            ingest(&s.db, &r).await?;
            Ok(accepted())
        } else {
            let kind = match uri.path() {
                p if p == COMMANDS[0] => Exchange::Pending,
                p if p == COMMANDS[1] => Exchange::Read,
                _ => Exchange::Receipt,
            };
            let v = codec::exchange(&bytes, kind).map_err(invalid)?;
            exchange(&s.db, &v, &bytes, kind).await
        }
    }
    .await;
    match result {
        Ok(bytes) => response(StatusCode::OK, bytes),
        Err(status) => response(status, Vec::new()),
    }
}
fn keys(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            for (k, v) in m {
                if k.ends_with("_key") {
                    if let Value::String(s) = v {
                        out.push(s.clone())
                    }
                } else {
                    keys(v, out)
                }
            }
        }
        Value::Array(a) => {
            for v in a {
                keys(v, out)
            }
        }
        _ => {}
    }
}
async fn ingest(db: &PgPool, r: &SessionRecord) -> Result<(), StatusCode> {
    let mut tx = db.begin().await.map_err(unavailable)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,73))")
        .bind(&r.account)
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    let account = id(&r.account)?;
    let generation = id(&r.generation)?;
    let erased:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pd_deleted_scopes WHERE scope_digest=ANY($1)) OR EXISTS(SELECT 1 FROM pd_erasure_jobs WHERE account_id=$2 AND (character_id IS NULL OR character_id=$3))")
        .bind(vec![crate::store::scope_digest(&r.account,None),crate::store::scope_digest(&r.account,Some(&r.character))]).bind(&r.account).bind(&r.character).fetch_one(&mut *tx).await.map_err(unavailable)?;
    if erased {
        return Err(StatusCode::CONFLICT);
    }

    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,729))")
        .bind(&r.authority)
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    let stream=sqlx::query("SELECT admitted,blocked,conflicted,recovery_generation::text AS recovery,stored_through::text FROM pd_g9_streams WHERE source_authority=$1 AND account_id=$2 AND stream_generation=$3 FOR UPDATE")
 .bind(&r.authority).bind(account).bind(generation).fetch_optional(&mut *tx).await.map_err(unavailable)?.ok_or(StatusCode::CONFLICT)?;
    if stream.try_get::<bool, _>("blocked").map_err(unavailable)? {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    let recovery = crate::authority::session_fence(&mut tx, account)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let admitted_recovery = stream
        .try_get::<String, _>("recovery")
        .map_err(unavailable)?
        .parse::<u64>()
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let digest = Sha256::digest(&r.bytes).to_vec();
    let prior=sqlx::query("SELECT digest,canonical_bytes FROM pd_g9_records WHERE source_authority=$1 AND account_id=$2 AND stream_generation=$3 AND source_revision=$4::numeric")
 .bind(&r.authority).bind(account).bind(generation).bind(r.revision.to_string()).fetch_optional(&mut *tx).await.map_err(unavailable)?;
    // Current owner authorization applies even to retained old-generation retries.
    crate::authority::account_epoch(&mut tx, account)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let owner = crate::authority::ownership(&mut tx, account, id(&r.character)?)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let crate::authority::Ownership::Owner(cut) = owner else {
        return Err(StatusCode::CONFLICT);
    };
    if cut.character_revision != r.character_revision {
        return Err(StatusCode::CONFLICT);
    }
    if let Some(prior) = prior {
        if prior.try_get::<Vec<u8>, _>("digest").map_err(unavailable)? == digest
            && prior
                .try_get::<Option<Vec<u8>>, _>("canonical_bytes")
                .map_err(unavailable)?
                .as_deref()
                == Some(&r.bytes)
        {
            return Ok(());
        }
        sqlx::query("UPDATE pd_g9_streams SET conflicted=true WHERE source_authority=$1 AND account_id=$2 AND stream_generation=$3").bind(&r.authority).bind(account).bind(generation).execute(&mut *tx).await.map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        return Err(StatusCode::CONFLICT);
    }
    if admitted_recovery < recovery {
        return Err(StatusCode::CONFLICT);
    }
    if !stream.try_get::<bool, _>("admitted").map_err(unavailable)?
        || stream
            .try_get::<bool, _>("conflicted")
            .map_err(unavailable)?
    {
        return Err(StatusCode::CONFLICT);
    }
    let record = playerdata::get(&r.envelope, "record").map_err(invalid)?;
    let catalog = text(record, "catalog_revision")?;
    let mut references = Vec::new();
    keys(record, &mut references);
    for key in references {
        let proven:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pd_g9_catalogue_keys WHERE source_authority=$1 AND catalog_revision=$2 AND catalog_key=$3)").bind(&r.authority).bind(catalog).bind(key).fetch_one(&mut *tx).await.map_err(unavailable)?;
        if !proven {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    }
    let record_json = serde_json::to_value(record).map_err(|_| StatusCode::BAD_REQUEST)?;
    let end = if r.kind == "HUNT_SEGMENT" {
        text(record, "ended_at")?
    } else {
        text(record, "occurred_at")?
    }
    .parse::<i64>()
    .map_err(|_| StatusCode::BAD_REQUEST)?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?;
    if end < cut.decided_at as i64 || end <= now - 90 * 86400 || end > now + 1 {
        return Err(StatusCode::CONFLICT);
    }
    if r.kind == "HUNT_SEGMENT" && text(record, "world_id")? != cut.world.to_string() {
        return Err(StatusCode::CONFLICT);
    }
    let index = if r.kind == "HUNT_SEGMENT" {
        record_json["segment_index"].as_i64()
    } else {
        None
    };
    let conflicting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pd_g9_records WHERE source_authority=$1 AND ((record_kind=$2 AND business_id=$3) OR ($2='HUNT_SEGMENT' AND record_kind='HUNT_SEGMENT' AND session_id=$4 AND segment_index=$5)))")
 .bind(&r.authority).bind(&r.kind).bind(id(&r.business_id)?).bind(id(&r.session)?).bind(index).fetch_one(&mut *tx).await.map_err(unavailable)?;
    let semantic_conflict:bool=sqlx::query_scalar(r#"SELECT EXISTS(
        SELECT 1 FROM pd_g9_records old WHERE old.source_authority=$1 AND old.record IS NOT NULL AND (
          (old.session_id=$2 AND (old.account_id<>$3 OR old.stream_generation<>$4 OR old.character_id<>$5))
          OR ($6='HUNT_SEGMENT' AND old.record_kind='HUNT_SEGMENT' AND old.session_id=$2 AND
             ((old.segment_index<$7 AND ((old.record->>'ended_at')::bigint>($8->>'started_at')::bigint OR (old.record->>'final_segment')::boolean))
              OR (old.segment_index>$7 AND ((old.record->>'started_at')::bigint<($8->>'ended_at')::bigint OR ($8->>'final_segment')::boolean))))
          OR ($6='RARE_DROP' AND old.record_kind='HUNT_SEGMENT' AND old.segment_id=$9 AND
             (old.session_id<>$2 OR old.character_id<>$5 OR old.record->>'character_revision'<>$8->>'character_revision'
              OR old.record->>'catalog_revision'<>$8->>'catalog_revision'
              OR ($8->>'occurred_at')::bigint< (old.record->>'started_at')::bigint
              OR ($8->>'occurred_at')::bigint>= (old.record->>'ended_at')::bigint
              OR (SELECT count(*) FROM pd_g9_records d WHERE d.source_authority=$1 AND d.record_kind='RARE_DROP' AND d.segment_id=$9)>=(old.record->>'rare_drop_count')::bigint
              OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements(old.record->'loot') item WHERE item->>'item_key'=$8->>'item_key' AND (item->>'quantity')::numeric>=($8->>'quantity')::numeric+COALESCE((SELECT sum((d.record->>'quantity')::numeric) FROM pd_g9_records d WHERE d.source_authority=$1 AND d.record_kind='RARE_DROP' AND d.segment_id=$9 AND d.record->>'item_key'=$8->>'item_key'),0))))
          OR ($6='HUNT_SEGMENT' AND old.record_kind='RARE_DROP' AND old.segment_id=$9 AND
             (old.session_id<>$2 OR old.character_id<>$5 OR old.record->>'character_revision'<>$8->>'character_revision'
              OR old.record->>'catalog_revision'<>$8->>'catalog_revision'
              OR (old.record->>'occurred_at')::bigint<($8->>'started_at')::bigint
              OR (old.record->>'occurred_at')::bigint>=($8->>'ended_at')::bigint
              OR (SELECT count(*) FROM pd_g9_records d WHERE d.source_authority=$1 AND d.record_kind='RARE_DROP' AND d.segment_id=$9)>($8->>'rare_drop_count')::bigint
              OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements($8->'loot') item WHERE item->>'item_key'=old.record->>'item_key' AND (item->>'quantity')::numeric>=(SELECT sum((d.record->>'quantity')::numeric) FROM pd_g9_records d WHERE d.source_authority=$1 AND d.record_kind='RARE_DROP' AND d.segment_id=$9 AND d.record->>'item_key'=old.record->>'item_key'))))
        ))"#).bind(&r.authority).bind(id(&r.session)?).bind(account).bind(generation).bind(id(&r.character)?).bind(&r.kind).bind(index).bind(&record_json).bind(id(&r.segment)?).fetch_one(&mut *tx).await.map_err(unavailable)?;
    if conflicting || semantic_conflict {
        sqlx::query("UPDATE pd_g9_streams SET conflicted=true WHERE source_authority=$1 AND account_id=$2 AND stream_generation=$3").bind(&r.authority).bind(account).bind(generation).execute(&mut *tx).await.map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        return Err(StatusCode::CONFLICT);
    }
    sqlx::query("INSERT INTO pd_g9_records(source_authority,account_id,stream_generation,source_revision,record_kind,session_id,segment_id,business_id,segment_index,character_id,digest,canonical_bytes,record,expires_at) VALUES($1,$2,$3,$4::numeric,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)")
 .bind(&r.authority).bind(account).bind(generation).bind(r.revision.to_string()).bind(&r.kind).bind(id(&r.session)?).bind(id(&r.segment)?).bind(id(&r.business_id)?).bind(index).bind(id(&r.character)?).bind(digest).bind(&r.bytes).bind(record_json).bind(end+90*86400).execute(&mut *tx).await.map_err(unavailable)?;
    // The first absent revision bounds the contiguous durable prefix. Application
    // stays separately fenced until session/drop reconciliation is proven.
    sqlx::query("UPDATE pd_g9_streams s SET highest_seen=greatest(highest_seen,$4::numeric),stored_through=CASE WHEN EXISTS(SELECT 1 FROM pd_g9_records first_record WHERE first_record.source_authority=$1 AND first_record.account_id=$2 AND first_record.stream_generation=$3 AND first_record.source_revision=1) THEN COALESCE((SELECT min(r.source_revision+1)-1 FROM pd_g9_records r WHERE r.source_authority=$1 AND r.account_id=$2 AND r.stream_generation=$3 AND NOT EXISTS(SELECT 1 FROM pd_g9_records n WHERE n.source_authority=r.source_authority AND n.account_id=r.account_id AND n.stream_generation=r.stream_generation AND n.source_revision=r.source_revision+1)),0) ELSE 0 END WHERE s.source_authority=$1 AND s.account_id=$2 AND s.stream_generation=$3")
 .bind(&r.authority).bind(account).bind(generation).bind(r.revision.to_string()).execute(&mut *tx).await.map_err(unavailable)?;
    // All segment indices and sealed drop counts must reconcile before any
    // member of a completed session can advance the applied prefix.
    sqlx::query(r#"UPDATE pd_g9_records r SET resolved=true WHERE r.source_authority=$1 AND r.account_id=$2 AND r.stream_generation=$3 AND r.session_id=$4 AND EXISTS(
      SELECT 1 FROM pd_g9_records final WHERE final.source_authority=$1 AND final.account_id=$2 AND final.stream_generation=$3 AND final.session_id=$4 AND final.record_kind='HUNT_SEGMENT' AND (final.record->>'final_segment')::boolean
      AND (SELECT count(*) FROM pd_g9_records s WHERE s.source_authority=$1 AND s.session_id=$4 AND s.record_kind='HUNT_SEGMENT')=final.segment_index+1
      AND NOT EXISTS(SELECT 1 FROM pd_g9_records s WHERE s.source_authority=$1 AND s.session_id=$4 AND s.record_kind='HUNT_SEGMENT' AND (s.record->>'rare_drop_count')::bigint<>(SELECT count(*) FROM pd_g9_records d WHERE d.source_authority=$1 AND d.segment_id=s.segment_id AND d.record_kind='RARE_DROP'))
    )"#).bind(&r.authority).bind(account).bind(generation).bind(id(&r.session)?).execute(&mut *tx).await.map_err(unavailable)?;
    sqlx::query("UPDATE pd_g9_streams s SET applied_through=least(stored_through,COALESCE((SELECT min(source_revision)-1 FROM pd_g9_records r WHERE r.source_authority=$1 AND r.account_id=$2 AND r.stream_generation=$3 AND NOT resolved),stored_through)) WHERE s.source_authority=$1 AND s.account_id=$2 AND s.stream_generation=$3")
        .bind(&r.authority).bind(account).bind(generation).execute(&mut *tx).await.map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)
}
/// Durable Platform outbox admission. Callers supply the already-authorized
/// exact intent and the producer partition; no public endpoint issues commands.
pub async fn enqueue(db: &PgPool, source: &str, raw: &[u8]) -> Result<(), StatusCode> {
    let v = codec::intent(raw).map_err(invalid)?;
    let digest = hex(&Sha256::digest(raw));
    let mut tx = db.begin().await.map_err(unavailable)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,729))")
        .bind(source)
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    let admitted:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pd_g9_streams WHERE source_authority=$1 AND account_id=$2 AND stream_generation=$3 AND admitted AND NOT blocked AND NOT conflicted)")
 .bind(source).bind(id(text(&v,"account_id")?)?).bind(id(text(&v,"stream_generation")?)?).fetch_one(&mut *tx).await.map_err(unavailable)?;
    if !admitted {
        return Err(StatusCode::CONFLICT);
    }
    let existing: Option<(Vec<u8>, String)> = sqlx::query_as(
        "SELECT intent,source_authority FROM pd_g9_commands WHERE operation_id=$1 FOR UPDATE",
    )
    .bind(id(text(&v, "operation_id")?)?)
    .fetch_optional(&mut *tx)
    .await
    .map_err(unavailable)?;
    if let Some((bytes, partition)) = existing {
        return if bytes == raw && partition == source {
            Ok(())
        } else {
            Err(StatusCode::CONFLICT)
        };
    }
    sqlx::query("INSERT INTO pd_g9_commands(operation_id,source_authority,source_revision,account_id,stream_generation,intent,binding_digest) VALUES($1,$2,$3::numeric,$4,$5,$6,$7)")
 .bind(id(text(&v,"operation_id")?)?).bind(source).bind(text(&v,"source_revision")?).bind(id(text(&v,"account_id")?)?).bind(id(text(&v,"stream_generation")?)?).bind(raw).bind(digest).execute(&mut *tx).await.map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)
}
async fn exchange(
    db: &PgPool,
    v: &Value,
    raw: &[u8],
    kind: Exchange,
) -> Result<Vec<u8>, StatusCode> {
    let source = text(v, "source_authority")?;
    if matches!(kind, Exchange::Pending) {
        let max = playerdata::get(v, "max_entries")
            .and_then(|v| {
                playerdata::decimal(
                    &Value::String(match v {
                        Value::Unsigned(n) => n.to_string(),
                        Value::Signed(n) => n.to_string(),
                        _ => String::new(),
                    }),
                    1,
                    32,
                )
            })
            .map_err(invalid)? as i64;
        let rows:Vec<(Uuid,String)>=sqlx::query_as("SELECT operation_id,source_revision::text FROM pd_g9_commands WHERE source_authority=$1 AND receipt IS NULL AND (source_revision,operation_id)>($2::numeric,$3) ORDER BY source_revision,operation_id LIMIT $4")
  .bind(source).bind(text(v,"after_source_revision")?).bind(id(text(v,"after_operation_id")?)?).bind(max).fetch_all(db).await.map_err(unavailable)?;
        return Ok(format!(
            "{{\"contract_version\":1,\"entries\":[{}]}}",
            rows.iter()
                .map(|(id, rev)| format!(
                    "{{\"operation_id\":\"{id}\",\"source_revision\":\"{rev}\"}}"
                ))
                .collect::<Vec<_>>()
                .join(",")
        )
        .into_bytes());
    }
    let mut tx = db.begin().await.map_err(unavailable)?;
    let row=sqlx::query("SELECT intent,binding_digest,receipt FROM pd_g9_commands WHERE operation_id=$1 AND source_authority=$2 FOR UPDATE").bind(id(text(v,"operation_id")?)?).bind(source).fetch_optional(&mut *tx).await.map_err(unavailable)?.ok_or(StatusCode::NOT_FOUND)?;
    if matches!(kind, Exchange::Read) {
        return row.try_get("intent").map_err(unavailable);
    }
    if row
        .try_get::<String, _>("binding_digest")
        .map_err(unavailable)?
        != text(v, "binding_digest")?
    {
        return Err(StatusCode::CONFLICT);
    }
    if let Some(prior) = row
        .try_get::<Option<Vec<u8>>, _>("receipt")
        .map_err(unavailable)?
    {
        return if prior == raw {
            Ok(accepted())
        } else {
            Err(StatusCode::CONFLICT)
        };
    }
    sqlx::query("UPDATE pd_g9_commands SET receipt=$2 WHERE operation_id=$1")
        .bind(id(text(v, "operation_id")?)?)
        .bind(raw)
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)?;
    Ok(accepted())
}
