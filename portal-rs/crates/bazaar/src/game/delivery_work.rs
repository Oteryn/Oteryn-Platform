//! Coalesced rechecks in the existing scheduling table. Evidence admission,
//! queue changes and resync all run under the issuer lock.
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(super) struct Queue {
    prefix: &'static str,
    end: &'static str,
    cursor: &'static str,
}

pub(super) const DELIVERY: Queue = Queue {
    prefix: "delivery-dirty:",
    end: "delivery-dirty;",
    cursor: "delivery-evidence-scan",
};
pub(super) const LISTING: Queue = Queue {
    prefix: "resync-listing-dirty:",
    end: "resync-listing-dirty;",
    cursor: "resync-listing-scan",
};

pub(super) async fn mark_status(db: &mut PgConnection, listing: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO bazaar_work_cursors(queue,cursor_id)
         SELECT 'delivery-dirty:' || l.id,l.id FROM bazaar_listings l WHERE l.id=$1 AND l.kind='item'
           AND (l.state='completed' OR EXISTS (SELECT 1 FROM bazaar_authority_compensations c
                  WHERE c.listing_id=l.id) OR EXISTS (SELECT 1 FROM bazaar_game_outbox o
                  WHERE o.listing_id=l.id AND (o.command='DeliverBazaarItem'
                      OR (l.state='awaiting_delivery' AND o.command='ReturnBazaarItem'))))
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

impl Queue {
    pub(super) async fn mark(
        self,
        db: &mut PgConnection,
        listing: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO bazaar_work_cursors(queue,cursor_id) VALUES($2 || $1::uuid,$1)
                 ON CONFLICT DO NOTHING",
        )
        .bind(listing)
        .bind(self.prefix)
        .execute(db)
        .await?;
        Ok(())
    }

    pub(super) async fn batch(
        self,
        db: &mut PgConnection,
    ) -> Result<Vec<(Uuid, crate::State)>, sqlx::Error> {
        let start = self.prefix;
        let cursor: Option<Uuid> =
            sqlx::query_scalar("SELECT cursor_id FROM bazaar_work_cursors WHERE queue=$1")
                .bind(self.cursor)
                .fetch_optional(&mut *db)
                .await?;
        let after = cursor.map_or_else(|| start.to_owned(), |id| format!("{start}{id}"));
        let mut listings = keys(db, &after, self.end, 24).await?;
        if listings.len() < 24 && cursor.is_some() {
            listings.extend(keys(db, start, &after, 24 - listings.len() as i64).await?);
        }
        if let Some((last, _)) = listings.last() {
            sqlx::query(
                "INSERT INTO bazaar_work_cursors(queue,cursor_id) VALUES($1,$2)
                     ON CONFLICT(queue) DO UPDATE SET cursor_id=EXCLUDED.cursor_id",
            )
            .bind(self.cursor)
            .bind(last)
            .execute(&mut *db)
            .await?;
        }
        Ok(listings)
    }

    pub(super) async fn remove(
        self,
        db: &mut PgConnection,
        listing: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM bazaar_work_cursors WHERE queue=$2 || $1::uuid")
            .bind(listing)
            .bind(self.prefix)
            .execute(db)
            .await?;
        Ok(())
    }

    pub(super) async fn pending(
        self,
        db: &mut PgConnection,
        terminal_only: bool,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bazaar_work_cursors w JOIN bazaar_listings l ON l.id=w.cursor_id
          WHERE w.queue COLLATE \"C\">$3 AND w.queue COLLATE \"C\"<$1 AND (NOT $2 OR l.state IN ('completed','cancelled')))",
    )
    .bind(self.end).bind(terminal_only).bind(self.prefix).fetch_one(db).await
    }
}
