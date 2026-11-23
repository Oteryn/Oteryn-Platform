use super::*;
use crate::credentials_test_db::Database;

#[test]
fn family_identifiers_are_canonical() {
    let id = Uuid::now_v7();
    assert_eq!(parse_family(&id.to_string()), Ok(id));
    for invalid in [
        String::new(),
        id.simple().to_string(),
        id.to_string().to_uppercase(),
        Uuid::nil().to_string(),
    ] {
        assert!(parse_family(&invalid).is_err());
    }
}

#[tokio::test]
async fn remembered_device_review_and_revocation() -> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = Database::open().await? else {
        return Ok(());
    };
    let pool = &db.pool;
    let owner = account(pool, "owner").await?;
    let other = account(pool, "other").await?;
    let sid = [1u8; 32];
    sqlx::query("INSERT INTO web_sessions(id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES($1,$2,$3,'test',now()+interval '1 hour',now()+interval '1 day')")
        .bind(&sid[..]).bind(owner).bind("a".repeat(64)).execute(pool).await?;
    let current = family(pool, owner, true).await?;
    let expired = family(pool, owner, false).await?;
    let foreign = family(pool, other, true).await?;
    let own_ticket = ticket(pool, owner, current, 1).await?;
    let other_ticket = ticket(pool, other, foreign, 2).await?;
    let temporary = family(pool, owner, true).await?;
    sqlx::query(
        "UPDATE device_sessions SET persistence_class='session',
                 device_public_key=decode(repeat('01',32),'hex'), device_key_thumbprint=id::text
         WHERE id=$1",
    )
    .bind(temporary)
    .execute(pool)
    .await?;

    let (entries, next) = list(pool, owner, None).await?;
    assert_eq!(entries.len(), 2);
    assert!(next.is_none());
    assert_eq!(
        entries
            .iter()
            .find(|row| row.id == current)
            .map(Device::state),
        Some(DeviceState::Current)
    );
    assert_eq!(
        entries
            .iter()
            .find(|row| row.id == expired)
            .map(Device::state),
        Some(DeviceState::Expired)
    );
    assert_eq!(
        revoke(pool, owner, &sid, current).await?,
        Revocation::Confirm
    );

    sqlx::query("UPDATE web_sessions SET sudo_until=now()+interval '10 minutes' WHERE id_hash=$1")
        .bind(&sid[..])
        .execute(pool)
        .await?;
    assert_eq!(
        revoke(pool, owner, &sid, foreign).await?,
        Revocation::Missing
    );
    assert_eq!(
        revoke(pool, owner, &sid, temporary).await?,
        Revocation::Missing
    );
    assert_eq!(
        revoke(pool, other, &sid, foreign).await?,
        Revocation::Denied
    );
    assert_eq!(
        revoke(pool, owner, &sid, current).await?,
        Revocation::Revoked
    );
    assert_eq!(
        revoke(pool, owner, &sid, current).await?,
        Revocation::Revoked
    );
    let (entries, _) = list(pool, owner, None).await?;
    assert_eq!(
        entries
            .iter()
            .find(|row| row.id == current)
            .map(Device::state),
        Some(DeviceState::Revoked)
    );
    for (hash, expected) in [(own_ticket, false), (other_ticket, true)] {
        let live: bool = sqlx::query_scalar("SELECT used_at IS NULL AND expires_at>clock_timestamp() FROM game_login_tickets WHERE ticket_hash=$1")
            .bind(&hash[..]).fetch_one(pool).await?;
        assert_eq!(live, expected);
    }
    let events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE actor=$1 AND action='native_device.revoked'",
    )
    .bind(owner)
    .fetch_one(pool)
    .await?;
    assert_eq!(events, 1);

    for _ in 0..PAGE_SIZE {
        family(pool, owner, true).await?;
    }
    let (page, next) = list(pool, owner, None).await?;
    assert_eq!(page.len(), PAGE_SIZE);
    let (older, next) = list(pool, owner, next).await?;
    assert_eq!(older.len(), 2);
    assert!(next.is_none());
    assert!(
        older
            .iter()
            .all(|row| page.iter().all(|seen| seen.id != row.id))
    );
    let client = Uuid::now_v7();
    assert!(!has_native_client(pool, &[client]).await?);
    sqlx::query(
        "INSERT INTO oauth_native_clients(client_id,enabled,first_party,public_client,native_ticket)
         VALUES($1,true,true,true,true)",
    )
    .bind(client)
    .execute(pool)
    .await?;
    assert!(!has_native_client(pool, &[Uuid::now_v7()]).await?);
    assert!(has_native_client(pool, &[client]).await?);
    db.close().await
}

async fn account(pool: &PgPool, alias: &str) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,'fixture','active','en',now()) RETURNING id")
        .bind(format!("{alias}@example.test")).fetch_one(pool).await
}

async fn family(pool: &PgPool, owner: Uuid, active: bool) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar("INSERT INTO device_sessions(id,account_id,client_id,enrollment_token_id,game_generation,native_generation,absolute_expires_at,idle_expires_at) SELECT uuidv7(),id,uuidv7(),uuidv7()::text,game_auth_generation,native_security_generation,extract(epoch FROM now()+interval '1 day')::bigint,CASE WHEN $2 THEN extract(epoch FROM now()+interval '1 hour')::bigint ELSE 0 END FROM accounts WHERE id=$1 RETURNING id")
        .bind(owner).bind(active).fetch_one(pool).await
}

async fn ticket(
    pool: &PgPool,
    owner: Uuid,
    family: Uuid,
    byte: u8,
) -> Result<[u8; 32], sqlx::Error> {
    let hash = [byte; 32];
    sqlx::query("INSERT INTO game_login_tickets(ticket_hash,account_id,game_generation,native_generation,audience,expires_at,device_family_id) SELECT $1,id,game_auth_generation::text,native_security_generation::text,'oteryn-native-game-gateway',now()+interval '1 minute',$3 FROM accounts WHERE id=$2")
        .bind(&hash[..]).bind(owner).bind(family).execute(pool).await?;
    Ok(hash)
}
