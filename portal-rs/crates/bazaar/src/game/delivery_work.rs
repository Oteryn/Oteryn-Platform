//! Coalesced rechecks in the existing scheduling table. Evidence admission,
//! queue changes and resync all run under the issuer lock.
use sqlx::PgConnection;
use uuid::Uuid;

const AFTER_LAST_KEY: &str = "delivery-dirty;";
const CURSOR: &str = "delivery-evidence-scan";

pub(super) async fn mark(db: &mut PgConnection, listing: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO bazaar_work_cursors(queue,cursor_id) VALUES('delivery-dirty:' || $1::uuid,$1)
                 ON CONFLICT DO NOTHING",
    )
    .bind(listing)
    .execute(db)
    .await?;
    Ok(())
}

pub(super) async fn mark_status(db: &mut PgConnection, listing: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO bazaar_work_cursors(queue,cursor_id)
         SELECT 'delivery-dirty:' || l.id,l.id FROM bazaar_listings l WHERE l.id=$1 AND l.kind='item'
           AND (l.state='completed' OR EXISTS (SELECT 1 FROM bazaar_game_outbox o
                  WHERE o.listing_id=l.id AND o.command='DeliverBazaarItem'))
         ON CONFLICT DO NOTHING",
    )
    .bind(listing).execute(db).await?;
    Ok(())
}

pub(super) async fn mark_operation(
    db: &mut PgConnection,
    operation: Uuid,
) -> Result<(), sqlx::Error> {
    let listing =
        sqlx::query_scalar("SELECT listing_id FROM bazaar_game_outbox WHERE operation_id=$1")
            .bind(operation)
            .fetch_optional(&mut *db)
            .await?;
    if let Some(listing) = listing {
        mark_status(db, listing).await?;
    }
    Ok(())
}

pub(super) async fn batch(db: &mut PgConnection) -> Result<Vec<(Uuid, crate::State)>, sqlx::Error> {
    let start = "delivery-dirty:";
    let cursor: Option<Uuid> =
        sqlx::query_scalar("SELECT cursor_id FROM bazaar_work_cursors WHERE queue=$1")
            .bind(CURSOR)
            .fetch_optional(&mut *db)
            .await?;
    let after = cursor.map_or_else(|| start.to_owned(), |id| format!("{start}{id}"));
    let mut listings = keys(db, &after, AFTER_LAST_KEY, 24).await?;
    if listings.len() < 24 && cursor.is_some() {
        listings.extend(keys(db, start, &after, 24 - listings.len() as i64).await?);
    }
    if let Some((last, _)) = listings.last() {
        sqlx::query(
            "INSERT INTO bazaar_work_cursors(queue,cursor_id) VALUES($1,$2)
                     ON CONFLICT(queue) DO UPDATE SET cursor_id=EXCLUDED.cursor_id",
        )
        .bind(CURSOR)
        .bind(last)
        .execute(&mut *db)
        .await?;
    }
    Ok(listings)
}

async fn keys(
    db: &mut PgConnection,
    after: &str,
    through: &str,
    limit: i64,
) -> Result<Vec<(Uuid, crate::State)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT w.cursor_id,l.state FROM bazaar_work_cursors w
         JOIN bazaar_listings l ON l.id=w.cursor_id
         WHERE w.queue COLLATE \"C\">$1 AND w.queue COLLATE \"C\"<=$2 ORDER BY w.queue COLLATE \"C\" LIMIT $3",
    )
    .bind(after).bind(through).bind(limit).fetch_all(db).await
}

pub(super) async fn remove(db: &mut PgConnection, listing: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM bazaar_work_cursors WHERE queue='delivery-dirty:' || $1::uuid")
        .bind(listing)
        .execute(db)
        .await?;
    Ok(())
}

pub(super) async fn pending(
    db: &mut PgConnection,
    terminal_only: bool,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_work_cursors w JOIN bazaar_listings l ON l.id=w.cursor_id
          WHERE w.queue COLLATE \"C\">'delivery-dirty:' AND w.queue COLLATE \"C\"<$1 AND (NOT $2 OR l.state IN ('completed','cancelled')))",
    )
    .bind(AFTER_LAST_KEY).bind(terminal_only).fetch_one(db).await
}
