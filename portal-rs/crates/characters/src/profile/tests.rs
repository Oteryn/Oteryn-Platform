use super::*;
#[tokio::test]
async fn postgres_preferences_ownership_privacy_and_audit() -> Result<(), Box<dyn std::error::Error>>
{
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping PostgreSQL profile test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = sqlx::postgres::PgPoolOptions::new().connect(&url).await?;
    let schema = format!("profile_test_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .after_connect(move |c, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(path)
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    for sql in [
        include_str!("../../../../migrations/20261105155256_t03_accounts.sql"),
        include_str!("../../../../migrations/20261105145653_t18_account_characters.sql"),
        include_str!("../../../../migrations/20261105231104_t45_profile_preferences.sql"),
    ] {
        sqlx::raw_sql(sql).execute(&pool).await?;
    }
    let account = Uuid::now_v7();
    let other = Uuid::now_v7();
    let one = Uuid::now_v7();
    let two = Uuid::now_v7();
    for id in [account, other] {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1,$2,'test-hash','active','en')").bind(id).bind(format!("profile-{id}@example.invalid")).execute(&pool).await?;
        sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'oteryn:character-authority:primary','1','1',1000,$2)").bind(id).bind(vec![0u8;32]).execute(&pool).await?;
    }
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='oteryn:character-authority:primary',complete_through=1000,observed_at=1000 WHERE id=1").execute(&pool).await?;
    for (id, name) in [(one, "Aldric"), (two, "Belric")] {
        sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,$4,'AVAILABLE')").bind(account).bind(id).bind(Uuid::now_v7()).bind(name).execute(&pool).await?;
    }
    let settings = Settings {
        enabled: true,
        environment: oteryn_core::Environment::Testing,
        source_authority: "oteryn:character-authority:primary".into(),
        ..Settings::default()
    };
    let service = Service::new(pool.clone(), settings.clone(), true);
    let off = Service::new(pool.clone(), settings.clone(), false);
    assert_eq!(
        off.edit(account, "Aldric", 1000).await,
        Err(Error::NotFound)
    );
    assert!(!off.guild_visible(one, true, 1000).await);
    assert_eq!(
        service.edit(other, "Aldric", 1000).await,
        Err(Error::NotFound)
    );
    assert_eq!(
        service
            .update(other, "Aldric", Preferences::default(), 1000)
            .await,
        Err(Error::NotFound)
    );
    assert_eq!(service.edit(account, "", 1000).await, Err(Error::BadInput));
    assert_eq!(
        service
            .update(account, "x", Preferences::default(), 1000)
            .await,
        Err(Error::NotFound)
    );
    assert_eq!(
        service.edit(account, "aldric", 1000).await.map(|v| v.1),
        Ok(Preferences::default())
    );
    assert!(service.guild_visible(one, true, 1000).await);
    assert!(!service.guild_visible(one, false, 1000).await);
    sqlx::query("INSERT INTO character_profiles(account_id,show_guild,is_main_character) VALUES($1,false,true)").bind(account).execute(&pool).await?;
    assert!(!service.guild_visible(one, true, 1000).await);
    assert!(
        !service
            .edit(account, "Aldric", 1000)
            .await
            .map(|v| v.1.show_guild)
            .unwrap_or(true)
    );
    let selected = Preferences {
        show_guild: true,
        is_main_character: true,
    };
    assert_eq!(
        service.update(account, "Aldric", selected, 1000).await,
        Ok(())
    );
    assert!(service.guild_visible(one, true, 1000).await);
    assert!(!service.guild_visible(two, true, 1000).await);
    assert_eq!(
        service.update(account, "Aldric", selected, 1000).await,
        Ok(())
    );
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM character_profile_events WHERE event_type='character.main_character_selected'").fetch_one(&pool).await?;
    assert_eq!(count, 1);
    let (a, b) = tokio::join!(
        service.update(account, "Aldric", selected, 1000),
        service.update(account, "Belric", selected, 1000)
    );
    assert_eq!(a, Ok(()));
    assert_eq!(b, Ok(()));
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM character_profiles WHERE account_id=$1 AND is_main_character",
    )
    .bind(account)
    .fetch_one(&pool)
    .await?;
    assert_eq!(count, 1);
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM character_profile_events WHERE event_type='character.profile_preferences_updated'").fetch_one(&pool).await?;
    assert_eq!(count, 4);
    // An audit failure rolls the preferences back, rather than publishing an unaudited allow.
    let before = service.edit(account, "Aldric", 1000).await;
    sqlx::query("ALTER TABLE character_profile_events ADD CONSTRAINT profile_test_refuse CHECK (false) NOT VALID").execute(&pool).await?;
    assert_eq!(
        service
            .update(
                account,
                "Aldric",
                Preferences {
                    show_guild: false,
                    is_main_character: false
                },
                1000
            )
            .await,
        Err(Error::Unavailable)
    );
    assert_eq!(service.edit(account, "Aldric", 1000).await, before);
    sqlx::query("ALTER TABLE character_profile_events DROP CONSTRAINT profile_test_refuse")
        .execute(&pool)
        .await?;
    // A queued publisher cannot replace ownership while this transaction reads it.
    let mut held = service.lock().await.map_err(|_| "profile lock failed")?;
    let mut publisher = pool.acquire().await?;
    sqlx::query("SET lock_timeout='100ms'")
        .execute(&mut *publisher)
        .await?;
    let publish = sqlx::query("UPDATE account_character_feed SET updated_at=now() WHERE id=1")
        .execute(&mut *publisher);
    let read = service.owned(&mut held, account, "Aldric", 1000);
    let (publish, read) = tokio::join!(publish, read);
    assert!(publish.is_err());
    assert!(read.is_ok());
    held.commit().await?;
    drop(publisher);
    assert!(!service.guild_visible(one, true, 1100).await);
    assert_eq!(
        service.update(account, "Aldric", selected, 1100).await,
        Err(Error::Unavailable)
    );
    sqlx::query("UPDATE account_character_snapshots SET invalid=true WHERE account_id=$1")
        .bind(account)
        .execute(&pool)
        .await?;
    assert!(!service.guild_visible(one, true, 1000).await);
    assert_eq!(
        service.update(account, "Aldric", selected, 1000).await,
        Err(Error::Unavailable)
    );
    sqlx::query("UPDATE account_character_snapshots SET invalid=false WHERE account_id=$1")
        .bind(account)
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Aldric','AVAILABLE')").bind(other).bind(one).bind(Uuid::now_v7()).execute(&pool).await?;
    assert!(!service.guild_visible(one, true, 1000).await);
    assert_eq!(
        service.edit(account, "Aldric", 1000).await,
        Err(Error::NotFound)
    );
    assert_eq!(
        service.update(account, "Aldric", selected, 1000).await,
        Err(Error::NotFound)
    );
    sqlx::query("DELETE FROM account_characters WHERE account_id=$1")
        .bind(other)
        .execute(&pool)
        .await?;
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(account)
        .execute(&pool)
        .await?;
    assert_eq!(
        service.update(account, "Aldric", selected, 1000).await,
        Err(Error::NotFound)
    );
    assert!(!service.guild_visible(one, true, 1000).await);
    sqlx::query("UPDATE accounts SET status='active' WHERE id=$1")
        .bind(account)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM account_characters WHERE account_id=$1 AND character_id=$2")
        .bind(account)
        .bind(one)
        .execute(&pool)
        .await?;
    assert_eq!(
        service.update(account, "Aldric", selected, 1000).await,
        Err(Error::NotFound)
    );
    assert!(service.public_preferences(one, 1000).await.is_none());
    sqlx::query("DELETE FROM accounts WHERE id=$1")
        .bind(account)
        .execute(&pool)
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM character_profiles")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    Ok(())
}
