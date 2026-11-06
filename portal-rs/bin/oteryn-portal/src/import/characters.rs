use super::*;
pub(super) const TABLES: &[&str] = &["character_profile_preferences"];
pub(super) fn map(ctx: &Context, table: &str, row: &Record) -> Result<Vec<Insert>> {
    // Refuse unsupported privacy preferences instead of silently making them public.
    if row
        .get("public_comment")
        .is_some_and(|v| !v.is_null() && v != "")
        || flag(row, "show_account_association", false)?
        || flag(row, "show_status", false)?
        || !flag(row, "show_house", true)?
        || !flag(row, "show_skills", true)?
        || !flag(row, "show_deaths", true)?
        || !flag(row, "show_kills", true)?
    {
        return Err("profile preferences exceed the destination schema");
    }
    let mut record = standard(ctx, table, row, "character_profiles", &["character_id"])?;
    record.values.insert(
        "account_id".into(),
        ctx.reference(row, "identity_id", "identities")?,
    );
    record
        .values
        .insert("show_guild".into(), json!(flag(row, "show_guild", true)?));
    record.values.insert(
        "is_main_character".into(),
        json!(flag(row, "is_main_character", false)?),
    );
    Ok(vec![record])
}
