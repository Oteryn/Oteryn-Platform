//! LCFA owner resolution shared by public profiles and staff enforcement.
use sqlx::{PgConnection, types::Uuid};

#[derive(Debug)]
pub enum Owner {
    Missing,
    Ambiguous,
    Unreliable,
    Current {
        account: Uuid,
        name: String,
        available: bool,
    },
}

/// Resolve claims against the current feed. The caller holds the feed row
/// shared and, for a mutation, locks and revalidates the candidate account.
pub async fn resolve(
    db: &mut PgConnection,
    character: Uuid,
    epoch: &str,
    authority: &str,
) -> Result<Owner, sqlx::Error> {
    let claims: Vec<(Uuid, String, bool, bool)> = sqlx::query_as(
        "SELECT c.account_id,c.name,c.availability='AVAILABLE',
            COALESCE(NOT s.invalid AND s.projection_epoch=$2 AND s.source_authority=$3,false)
         FROM account_characters c
         JOIN accounts a ON a.id=c.account_id AND a.status='active'
         LEFT JOIN account_character_snapshots s ON s.account_id=c.account_id
         WHERE c.character_id=$1
         ORDER BY c.account_id LIMIT 2",
    )
    .bind(character)
    .bind(epoch)
    .bind(authority)
    .fetch_all(db)
    .await?;
    Ok(match claims.as_slice() {
        [] => Owner::Missing,
        [(account, name, available, true)] => Owner::Current {
            account: *account,
            name: name.clone(),
            available: *available,
        },
        rows if rows.iter().any(|row| !row.3) => Owner::Unreliable,
        _ => Owner::Ambiguous,
    })
}
