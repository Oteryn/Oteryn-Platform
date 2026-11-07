//! One world/day only. Game aggregates are never recomputed or combined here.
use super::*;

#[derive(serde::Serialize)]
struct EconomyCell {
    key: String,
    state: String,
    values: Vec<String>,
    gap: String,
    cut: String,
    age: u64,
    epoch: String,
    revision: String,
}
#[derive(serde::Serialize)]
struct Group {
    family: String,
    columns: &'static str,
    state: &'static str,
    epoch: String,
    revision: String,
    cut: String,
    age: u64,
    cells: Vec<EconomyCell>,
}
#[derive(Template)]
#[template(path = "admin/analytics_economy.html", config = "src/web_askama.toml")]
struct EconomyPage {
    lang: &'static str,
    world: String,
    from: String,
    to: String,
    policy: String,
    groups: Vec<Group>,
}
pub(super) async fn read<I: StaffSession, A: AccountStateReader>(
    s: Arc<Context<I, A>>,
    request: Request,
    context: AuditContext,
) -> Response {
    let selection = match selection(request.uri().query(), request.uri().path()) {
        Ok(v)
            if v.end
                .zip(v.start)
                .is_some_and(|(end, start)| end - start == 86400 && start % 86400 == 0)
                && v.family.is_some_and(wire::Family::economy) =>
        {
            v
        }
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let mut tx = match s.pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    if sqlx::query("SELECT pg_advisory_xact_lock(71140001)")
        .execute(&mut *tx)
        .await
        .is_err()
    {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let now = super::super::now();
    let start = selection.start.unwrap_or_default();
    let end = selection.end.unwrap_or_default();
    let expired = end
        .checked_add(400 * 86400)
        .is_none_or(|expiry| expiry <= now);
    let mut groups = Vec::new();
    for (family, columns) in [
        (
            "economy_circulation_daily",
            "gold (end-of-day stock by location)",
        ),
        ("economy_flows_daily", "gold_in / gold_out"),
        ("economy_trades_daily", "trades / traded_gold"),
        (
            "economy_market_daily",
            "traded_units / traded_gold / offers_created / median_unit_price",
        ),
        ("economy_store_daily", "count / amount (currency units)"),
    ] {
        let mut group = Group {
            family: family.into(),
            columns,
            state: if expired { "expired" } else { "unavailable" },
            epoch: String::new(),
            revision: String::new(),
            cut: String::new(),
            age: 0,
            cells: Vec::new(),
        };
        if !expired
            && end.checked_add(86400).is_some_and(|release| release <= now)
            && let Some(world) = selection.world
        {
            let evidence = sqlx::query("SELECT b.epoch::text,b.revision::text,b.observed::text,w.observed::text AS watermark,b.generation FROM admin_analytics.subjects b JOIN admin_analytics.fence f ON f.authority=b.authority AND f.epoch=b.epoch AND f.active_generation=b.generation JOIN admin_analytics.generations g ON g.id=b.generation AND g.accepted_at IS NOT NULL AND g.epoch=f.epoch JOIN admin_analytics.watermarks w ON w.authority=b.authority AND w.family=b.family AND w.world=b.world AND w.epoch=f.epoch WHERE b.authority=$1 AND b.family=$2 AND b.world=$3 AND b.bucket_start=$4::text::numeric AND b.bucket_end=$5::text::numeric AND b.privacy=$6 AND f.privacy_floor=$6 AND w.privacy=$6 AND NOT w.conflict AND NOT b.conflict AND NOT b.erased AND w.observed >= $7::text::numeric-120 AND w.observed <= $7::text::numeric+2 AND w.cut <= w.observed AND w.cut >= b.observed AND w.cut >= g.baseline_cut AND b.bucket_end+86400 <= $7::text::numeric")
                .bind(&s.config.authority).bind(family).bind(world).bind(start.to_string()).bind(end.to_string()).bind(&s.config.privacy).bind(now.to_string()).fetch_optional(&mut *tx).await;
            let evidence = match evidence {
                Ok(v) => v,
                Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
            };
            if let Some(evidence) = evidence {
                let result = async {
                    let generation: Uuid = evidence.try_get("generation")?;
                    let epoch: String = evidence.try_get("epoch")?;
                    let revision: String = evidence.try_get("revision")?;
                    let observed: String = evidence.try_get("observed")?;
                    let watermark: String = evidence.try_get("watermark")?;
                    let age = now.saturating_sub(wire::uint(&watermark).map_err(|_|sqlx::Error::Protocol("invalid watermark".into()))?);
                    let rows = sqlx::query("SELECT key,state,values FROM admin_analytics.rows WHERE generation=$1 AND authority=$2 AND family=$3 AND world=$4 AND bucket_start=$5::text::numeric AND epoch=$6::text::numeric AND revision=$7::text::numeric ORDER BY key COLLATE \"C\" LIMIT 8193")
                        .bind(generation).bind(&s.config.authority).bind(family).bind(world).bind(start.to_string()).bind(&epoch).bind(&revision).fetch_all(&mut *tx).await?;
                    if rows.len()>8192 {return Err(sqlx::Error::Protocol("row bound".into()));}
                    group.epoch = epoch.clone();
                    group.revision = revision.clone();
                    group.cut = utc(&observed);
                    group.age = age;
                    for row in rows {
                        let key: String = row.try_get("key")?;
                        let state: String = row.try_get("state")?;
                        let values = if matches!(state.as_str(),"visible"|"low_n") {serde_json::from_value(row.try_get::<Value,_>("values")?).map_err(|e|sqlx::Error::Decode(Box::new(e)))?} else {Vec::new()};
                        group.cells.push(EconomyCell {gap: if state=="incomplete" {format!("{family}: {key} source coverage not yet delivered by Game")} else {String::new()},key,state,values,cut:utc(&observed),age,epoch:epoch.clone(),revision:revision.clone()});
                    }
                    Ok::<(), sqlx::Error>(())
                }.await;
                if result.is_err() {
                    return StatusCode::SERVICE_UNAVAILABLE.into_response();
                }
                group.state = "available";
            }
        }
        groups.push(group);
    }
    // The current wallet has no qualified purchase-credit/currency evidence.
    // A positive operator adjustment, payment intent or browser return is not a purchase.
    let purchased = Group {
        family: "purchased — Platform wallet ledger (all worlds)".into(),
        columns: "count / amount (wallet currency units)",
        state: if expired { "expired" } else { "incomplete" },
        epoch: String::new(),
        revision: String::new(),
        cut: String::new(),
        age: 0,
        cells: Vec::new(),
    };
    groups.push(purchased);
    let metadata = json!({"world":selection.world,"start":start,"end":end,"privacy_policy_revision":s.config.privacy,"sources":groups.iter().map(|g|json!({"family":g.family,"state":g.state,"epoch":g.epoch,"revision":g.revision,"cut":g.cut,"age_seconds":g.age})).collect::<Vec<_>>()});
    if audit::append(
        &mut tx,
        Entry {
            context,
            action: "admin.analytics.read",
            target: request.uri().path(),
            before: Value::Null,
            after: metadata,
        },
    )
    .await
    .is_err()
        || tx.commit().await.is_err()
    {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    if let Some(attempt) = request.extensions().get::<Attempt>() {
        attempt.recorded.store(true, Ordering::SeqCst);
    }
    if request.uri().path().ends_with("/data") {
        return axum::Json(json!({"state":if expired {"expired"} else if groups.iter().take(5).all(|g|g.state=="unavailable") {"unavailable"} else {"available"},"groups":groups})).into_response();
    }
    let english = request
        .headers()
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("en"));
    let page = EconomyPage {
        lang: if english { "en" } else { "pl" },
        world: selection.world.map(|v| v.to_string()).unwrap_or_default(),
        from: input_time(start),
        to: input_time(end),
        policy: s.config.privacy.clone(),
        groups,
    };
    match page.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
