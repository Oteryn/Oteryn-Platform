use super::{Houses, houses_schema as schema};
use axum::{
    Extension,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use oteryn_internal_api::{
    canonical::{self, Value},
    guard,
};
use sqlx::{PgPool, Row};

pub(super) async fn handle(
    State(state): State<Houses>,
    Extension(v): Extension<Value>,
    Extension(watermark): Extension<bool>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    if canonical::encode(&v).ok().as_deref() != Some(body.as_ref())
        || body.contains(&b'\\')
        || headers.get("content-length").is_some_and(|h| {
            h.to_str().ok().and_then(|s| s.parse::<usize>().ok()) != Some(body.len())
        })
        || schema::validate(&v, watermark, super::now()).is_err()
    {
        return guard::failure(StatusCode::BAD_REQUEST);
    }
    if schema::text(&v, "source_authority") != Ok(state.config.ingest.source_authority.as_str()) {
        return guard::failure(StatusCode::UNAUTHORIZED);
    }
    match apply(&state.db, &v, watermark).await {
        Ok(result) => {
            axum::Json(serde_json::json!({"contract_version":2,"result":result})).into_response()
        }
        Err(status) => guard::failure(status),
    }
}
fn db_error(_: sqlx::Error) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
/// Serializes with T24's public epoch row, so either version fences the other.
pub(super) async fn apply(
    db: &PgPool,
    v: &Value,
    watermark: bool,
) -> Result<&'static str, StatusCode> {
    let bad = StatusCode::BAD_REQUEST;
    let epoch = schema::text(v, "projection_epoch").map_err(|_| bad)?;
    let authority = schema::text(v, "source_authority").map_err(|_| bad)?;
    let epoch_number = epoch.parse::<u64>().map_err(|_| bad)?;
    let mut tx = db.begin().await.map_err(db_error)?;
    let highest: Option<String> = sqlx::query_scalar(
        "SELECT highest_epoch::text FROM public_projection_state WHERE id=1 FOR UPDATE",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(db_error)?;
    if highest
        .and_then(|e| e.parse::<u64>().ok())
        .is_some_and(|e| e > epoch_number)
    {
        return Ok("superseded");
    }
    sqlx::query("UPDATE public_projection_state SET highest_epoch=$1::numeric WHERE id=1")
        .bind(epoch)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
    if watermark {
        let complete =
            schema::decimal(v, "complete_through", 0, i64::MAX as u64).map_err(|_| bad)? as i64;
        let old = sqlx::query(
            "SELECT epoch::text,complete_through FROM public_houses_v2_watermark WHERE id=1",
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_error)?;
        if let Some(old) = old
            && old.try_get::<String, _>("epoch").map_err(db_error)? == epoch
            && old
                .try_get::<i64, _>("complete_through")
                .map_err(db_error)?
                >= complete
        {
            return Ok("superseded");
        }
        sqlx::query("INSERT INTO public_houses_v2_watermark VALUES (1,$1,$2::numeric,$3) ON CONFLICT (id) DO UPDATE SET source_authority=EXCLUDED.source_authority,epoch=EXCLUDED.epoch,complete_through=EXCLUDED.complete_through").bind(authority).bind(epoch).bind(complete).execute(&mut *tx).await.map_err(db_error)?;
    } else {
        let subject = format!(
            "{}:{}",
            schema::text(v, "world_id").map_err(|_| bad)?,
            schema::text(v, "house_key").map_err(|_| bad)?
        );
        let revision = schema::text(v, "projection_revision").map_err(|_| bad)?;
        let payload = schema::json(v).map_err(|_| bad)?;
        let old = sqlx::query(
            "SELECT epoch::text,revision::text,payload FROM public_houses_v2 WHERE subject=$1",
        )
        .bind(&subject)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_error)?;
        if let Some(old) = old {
            let old_epoch: String = old.try_get("epoch").map_err(db_error)?;
            let old_revision: String = old.try_get("revision").map_err(db_error)?;
            if old_epoch == epoch {
                let order = revision
                    .len()
                    .cmp(&old_revision.len())
                    .then_with(|| revision.cmp(&old_revision));
                if order.is_lt() {
                    return Ok("superseded");
                }
                if order.is_eq() {
                    return if old
                        .try_get::<serde_json::Value, _>("payload")
                        .map_err(db_error)?
                        == payload
                    {
                        Ok("accepted")
                    } else {
                        Err(StatusCode::CONFLICT)
                    };
                }
            }
        }
        let observed =
            schema::decimal(v, "source_observed_at", 0, i64::MAX as u64).map_err(|_| bad)? as i64;
        sqlx::query("INSERT INTO public_houses_v2 VALUES ($1,$2,$3::numeric,$4::numeric,$5,$6) ON CONFLICT (subject) DO UPDATE SET source_authority=EXCLUDED.source_authority,epoch=EXCLUDED.epoch,revision=EXCLUDED.revision,observed=EXCLUDED.observed,payload=EXCLUDED.payload").bind(subject).bind(authority).bind(epoch).bind(revision).bind(observed).bind(payload).execute(&mut *tx).await.map_err(db_error)?;
    }
    tx.commit().await.map_err(db_error)?;
    Ok("accepted")
}
