use super::*;
pub(super) const TABLES: &[&str] = &[
    "support_tickets",
    "support_ticket_messages",
    "player_reports",
    "enforcement_records",
];
pub(super) fn map(ctx: &Context, table: &str, row: &Record) -> Result<Vec<Insert>> {
    let mut record = match table {
        "support_tickets" => {
            let mut r = standard(
                ctx,
                table,
                row,
                "tickets",
                &[
                    "category",
                    "subject",
                    "status",
                    "lock_version",
                    "last_message_at",
                    "closed_at",
                ],
            )?;
            r.values.insert(
                "account_id".into(),
                ctx.reference(row, "identity_id", "identities")?,
            );
            // Submission replay tokens are obsolete across the cutover, not account identifiers.
            r.values
                .insert("request_key".into(), mapped_id(ctx, table, row)?);
            r
        }
        "support_ticket_messages" => {
            let mut r = standard(ctx, table, row, "ticket_messages", &["author_kind", "body"])?;
            r.values.insert(
                "ticket_id".into(),
                ctx.reference(row, "support_ticket_id", "support_tickets")?,
            );
            let author = ctx.reference(row, "author_identity_id", "identities")?;
            if author.is_null() {
                return Err("deleted message author has no compatible target");
            }
            r.values.insert("author_account_id".into(), author);
            let visibility = text(row, "visibility")?;
            if !["public", "internal"].contains(&visibility.as_str()) {
                return Err("unsupported message visibility");
            }
            r.values
                .insert("internal".into(), json!(visibility == "internal"));
            r
        }
        "player_reports" => {
            let mut r = standard(
                ctx,
                table,
                row,
                "player_reports",
                &[
                    "report_type",
                    "category",
                    "target_reference",
                    "evidence_summary",
                    "status",
                    "public_outcome",
                    "moderator_notes",
                    "lock_version",
                    "processed_at",
                ],
            )?;
            r.values.insert(
                "reporter_account_id".into(),
                ctx.reference(row, "reporter_identity_id", "identities")?,
            );
            r.values.insert(
                "assigned_to".into(),
                ctx.reference(row, "assigned_to", "identities")?,
            );
            r.values
                .insert("request_key".into(), mapped_id(ctx, table, row)?);
            r
        }
        "enforcement_records" => {
            if row.get("expires_at").is_none_or(Value::is_null) && text(row, "status")? == "active"
            {
                return Err(
                    "permanent sanction requires approval evidence absent from the source schema",
                );
            }
            let mut r = standard(
                ctx,
                table,
                row,
                "sanctions",
                &[
                    "category",
                    "status",
                    "public_reason",
                    "moderator_notes",
                    "effective_at",
                    "expires_at",
                    "acknowledged_at",
                    "lock_version",
                ],
            )?;
            r.values.insert(
                "account_id".into(),
                ctx.reference(row, "identity_id", "identities")?,
            );
            r.values.insert(
                "created_by".into(),
                ctx.reference(row, "created_by", "identities")?,
            );
            r
        }
        _ => return Err("unsupported support table"),
    };
    // Retain source times for all history, including appeal workflows.
    copy(row, &["created_at", "updated_at"], &mut record.values);
    let mut records = vec![record];
    if table == "enforcement_records" {
        let mut r = insert(
            "support_appeal_workflows",
            json!({"sanction_id":mapped_id(ctx,table,row)?,"owner_id":ctx.reference(row,"identity_id","identities")?,"status":text(row,"appeal_status")?,"message":row.get("appeal_message"),"outcome":row.get("appeal_outcome"),"acknowledged_at":row.get("acknowledged_at")}),
        )?;
        copy(row, &["created_at", "updated_at"], &mut r.values);
        records.push(r);
    }
    Ok(records)
}
