use super::*;
pub(super) const TABLES: &[&str] = &[
    "character_auctions",
    "wallet_accounts",
    "wallet_ledger_entries",
    "premium_time_authority",
    "premium_time_entitlements",
    "premium_time_entitlement_events",
    "payment_orders",
    "payment_attempts",
    "payment_provider_events",
];
pub(super) fn check(data: &Dataset, report: &mut Report) {
    if data
        .get("character_auctions")
        .into_iter()
        .flatten()
        .any(|r| {
            !matches!(
                text(r, "status").as_deref(),
                Ok("completed" | "cancelled" | "expired")
            )
        })
    {
        report
            .problems
            .push("open or unresolved bazaar auctions exist".into());
    }
    for account in data.get("wallet_accounts").into_iter().flatten() {
        let result = (|| {
            if number(account, "reserved_balance")? != 0
                || number(account, "available_balance")? < 0
            {
                return Err("wallet has reserved or negative funds");
            }
            let id = text(account, "identity_id")?;
            let mut available = 0i64;
            let mut reserved = 0i64;
            for entry in data
                .get("wallet_ledger_entries")
                .into_iter()
                .flatten()
                .filter(|e| text(e, "identity_id").ok().as_ref() == Some(&id))
            {
                available = available
                    .checked_add(number(entry, "available_delta")?)
                    .ok_or("wallet sum overflow")?;
                reserved = reserved
                    .checked_add(number(entry, "reserved_delta")?)
                    .ok_or("wallet sum overflow")?;
            }
            if available != number(account, "available_balance")? || reserved != 0 {
                return Err("wallet balance does not match ledger");
            }
            Ok(())
        })();
        if let Err(code) = result {
            report.problems.push(code.into());
        }
    }
}
pub(super) fn map(ctx: &Context, data: &Dataset, table: &str, row: &Record) -> Result<Vec<Insert>> {
    let mut records = Vec::new();
    match table {
        "character_auctions" => {}, // Guarded history is intentionally not imported.
        "wallet_accounts" => {
            let mut r = insert("wallet_accounts",json!({"account_id":ctx.reference(row,"identity_id","identities")?}))?;
            copy(row,&["created_at","updated_at"],&mut r.values); records.push(r);
        }
        "wallet_ledger_entries" => {
            let id = mapped_id(ctx,table,row)?; let account = ctx.reference(row,"identity_id","identities")?;
            let delta = number(row,"available_delta")?; let reserved = number(row,"reserved_delta")?;
            if delta == 0 && reserved == 0 { return Err("empty wallet ledger entry"); }
            records.push(insert("wallet_operations",json!({"id":id,"kind":"adjustment","account_id":account,"amount":delta,"reason":"Legacy ledger import","created_at":row.get("created_at")}))?);
            records.push(insert("wallet_entries",json!({"id":id,"operation_id":id,"account_id":account,"available_delta":delta,"reserved_delta":reserved,"created_at":row.get("created_at")}))?);
        }
        "premium_time_authority" => records.push(insert("premium_authority",json!({"account_id":ctx.reference(row,"identity_id","identities")?,"authority_revision":number(row,"authority_revision")?,"updated_at":row.get("updated_at")}))?),
        "premium_time_entitlements" => {
            let id = text(row,"id")?; let revision = number(row,"lifecycle_revision")?;
            let matching = data.get("premium_time_entitlement_events").into_iter().flatten().find(|r| text(r,"entitlement_id").ok().as_ref() == Some(&id) && number(r,"lifecycle_revision").ok() == Some(revision)).ok_or("premium entitlement lacks matching final ledger event")?;
            for field in ["identity_id","effective_from","effective_until"] { if text(row,field)? != text(matching,field)? { return Err("premium entitlement disagrees with ledger"); } }
        }
        "premium_time_entitlement_events" => {
            let id = text(row,"request_id")?; let account = ctx.reference(row,"identity_id","identities")?; let actor = ctx.reference(row,"actor_identity_id","identities")?;
            let from = number(row,"effective_from")?; let until = number(row,"effective_until")?;
            let event = text(row,"event_type")?;
            if event!="GRANT" && event!="REVOKE" { return Err("unsupported premium event"); }
            let revision = number(row,"lifecycle_revision")?;
            let owner = text(row,"identity_id")?;
            let previous = if revision==1 { 0 } else {
                let previous = data.get("premium_time_entitlement_events").into_iter().flatten().find(|r| text(r,"identity_id").ok().as_ref()==Some(&owner) && number(r,"lifecycle_revision").ok()==Some(revision-1)).ok_or("premium ledger revision gap")?;
                if text(previous,"entitlement_id")? != text(row,"entitlement_id")? { return Err("premium entitlement changed identity"); }
                number(previous,"effective_until")?.checked_sub(number(previous,"effective_from")?).ok_or("premium interval overflow")?
            };
            let delta = until.checked_sub(from).and_then(|n| n.checked_sub(previous)).ok_or("premium interval overflow")?;
            records.push(insert("premium_ledger",json!({"request_id":id,"account_id":account,"actor_id":actor,"event_type":event,"duration_days":row.get("duration_days"),"reason_sha256":bytes(&hex(&text(row,"reason_sha256")?)?),"entitlement_id":text(row,"entitlement_id")?,"lifecycle_revision":number(row,"lifecycle_revision")?,"delta_seconds":delta,"effective_from":timestamp(from)?,"effective_until":timestamp(until)?,"created_at":row.get("created_at")}))?);
            records.push(insert("premium_audit",json!({"request_id":id,"actor_id":actor,"account_id":account,"event_name":if event=="GRANT" {"products.premium_granted"} else {"products.premium_revoked"},"created_at":row.get("created_at")}))?);
        }
        "payment_orders" => {
            let id = mapped_id(ctx,table,row)?;
            let state = payment_state(&text(row,"status")?)?;
            let order = text(row,"id")?;
            let attempts: Vec<_> = data.get("payment_attempts").into_iter().flatten().filter(|r| text(r,"payment_order_id").ok().as_ref() == Some(&order)).collect();
            if attempts.len() != 1 { return Err("payment order requires exactly one compatible provider attempt"); }
            let reference = text(attempts[0],"provider_checkout_reference")?;
            let mut r = insert("payment_intents",json!({"id":id,"account_id":ctx.reference(row,"identity_id","identities")?,"provider":text(row,"provider")?,"provider_reference":reference,"amount":number(row,"amount_minor")?,"currency":text(row,"currency")?,"state":state}))?;
            copy(row,&["created_at","updated_at"],&mut r.values); records.push(r);
        }
        "payment_attempts" => {}, // Folded into its order, with cardinality checked above.
        "payment_provider_events" => {
            if text(row,"processing_state")? != "processed" { return Err("unprocessed payment event requires reconciliation before cutover"); }
            let order = text(row,"payment_order_id")?;
            let parent = data.get("payment_orders").into_iter().flatten().find(|r| text(r,"id").ok().as_ref() == Some(&order)).ok_or("payment event order missing")?;
            let event = text(row,"event_type")?;
            let state = match event.as_str() { "payment.succeeded" => "succeeded", "payment.failed" => "failed", "payment.refunded" => "refunded", _ => return Err("unsupported provider event type") };
            records.push(insert("payment_events",json!({"id":mapped_id(ctx,table,row)?,"provider":text(row,"provider")?,"provider_event_id":text(row,"provider_event_id")?,"intent_id":mapped_id(ctx,"payment_orders",parent)?,"state":state,"created_at":row.get("received_at")}))?);
        }
        _ => return Err("unsupported commerce table"),
    }
    Ok(records)
}
fn payment_state(state: &str) -> Result<&'static str> {
    match state {
        "created" => Ok("created"),
        "pending" => Ok("pending"),
        "paid" | "succeeded" => Ok("succeeded"),
        "failed" | "cancelled" | "expired" => Ok("failed"),
        "refunded" => Ok("refunded"),
        _ => Err("unsupported payment state"),
    }
}
fn timestamp(seconds: i64) -> Result<String> {
    time::OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|_| "invalid Unix timestamp")?
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| "invalid timestamp")
}
