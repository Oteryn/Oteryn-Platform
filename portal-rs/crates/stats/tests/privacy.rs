use oteryn_characters::{
    lcfa::Settings,
    profile::{Preferences, Service, authority::Authority as ProfileAuthority},
};
use oteryn_stats::{privacy::Authority, store::Visibility};
use sqlx::PgPool;
use uuid::Uuid;
#[tokio::test]
async fn current_statistics_switches_and_missing_state_deny()
-> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping statistics privacy PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = PgPool::connect(&url).await?;
    let schema = format!("stats_privacy_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let db = PgPool::connect_with(
        admin
            .connect_options()
            .as_ref()
            .clone()
            .options([("search_path", schema.as_str())]),
    )
    .await?;
    for sql in [
        include_str!("../../../migrations/20261105155256_t03_accounts.sql"),
        include_str!("../../../migrations/20261105145653_t18_account_characters.sql"),
        include_str!("../../../migrations/20261105220226_t09c_privacy.sql"),
        include_str!("../../../migrations/20261105231104_t45_profile_preferences.sql"),
        include_str!("../../../migrations/20261107130001_t45_privacy.up.sql"),
    ] {
        sqlx::raw_sql(sql).execute(&db).await?;
    }
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105210000_t65_statistics.sql"
    ))
    .execute(&db)
    .await?;
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let owner = Uuid::now_v7();
    let character = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at,public_status_visible) VALUES($1,$2,'test','active','en',now(),true)").bind(owner).bind(format!("{owner}@example.invalid")).execute(&db).await?;
    sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'privacy-test','1','1',$2,$3)").bind(owner).bind(now).bind(vec![0u8;32]).execute(&db).await?;
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='privacy-test',complete_through=$1,observed_at=$1 WHERE id=1").bind(now).execute(&db).await?;
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Aldric','AVAILABLE')").bind(owner).bind(character).bind(Uuid::now_v7()).execute(&db).await?;
    let service = Service::new(
        db.clone(),
        Settings {
            enabled: true,
            environment: oteryn_core::Environment::Testing,
            source_authority: "privacy-test".into(),
            ..Settings::default()
        },
        true,
    );
    let privacy = Authority::new(ProfileAuthority {
        service: service.clone(),
    });
    assert!(!privacy.current_permissions(character).await.visible);
    assert!(!privacy.current_permissions(Uuid::now_v7()).await.visible);
    let end = time::OffsetDateTime::now_utc().date();
    let start = end.previous_day().ok_or("date underflow")?;
    let unknown = Uuid::now_v7();
    let mut tx = db.begin().await?;
    for day in [start, end] {
        oteryn_stats::store::partitions(&mut tx, day).await?;
    }
    for id in [character, unknown] {
        for (day, experience) in [(start, 100), (end, 200)] {
            sqlx::query("INSERT INTO stats_character_daily(character_ref,day,level,experience,skills,minutes_online,name,world,observed_at) VALUES($1,$2,100,$3,$4,60,'Aldric',$5,$6)").bind(id).bind(day).bind(experience).bind(serde_json::json!({"sword":80})).bind(Uuid::now_v7()).bind(now).execute(&mut *tx).await?;
        }
    }
    for kind in ["death", "guild_join"] {
        sqlx::query("INSERT INTO stats_character_events(character_ref,day,kind,event_key,details) VALUES($1,$2,$3,$3,$4)").bind(character).bind(end).bind(kind).bind(serde_json::json!({})).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let mut p = Preferences {
        hidden: false,
        show_skills: true,
        show_deaths: true,
        show_guild: true,
        show_online_time: true,
        ..Preferences::default()
    };
    service
        .update(owner, "Aldric", p.clone(), now)
        .await
        .map_err(|e| format!("save: {e:?}"))?;
    let value = privacy.current_permissions(character).await;
    assert!(value.visible && value.skills && value.deaths && value.guild && value.online);
    let history = oteryn_stats::store::history(&db, character, start, end, &privacy).await?;
    assert_eq!(history.daily.len(), 2);
    assert_eq!(history.events.len(), 2);
    assert!(
        history
            .daily
            .iter()
            .all(|d| d.skills.is_some() && d.minutes_online == Some(60))
    );
    let growth = oteryn_stats::store::top_growth(&db, start, end, 10, &privacy).await?;
    assert_eq!(growth.len(), 1);
    assert_eq!(growth[0].character_ref, character);
    p.show_skills = false;
    p.show_deaths = false;
    p.show_guild = false;
    p.show_online_time = false;
    service
        .update(owner, "Aldric", p.clone(), now)
        .await
        .map_err(|e| format!("save: {e:?}"))?;
    let value = privacy.current_permissions(character).await;
    assert!(value.visible && !value.skills && !value.deaths && !value.guild && !value.online);
    let history = oteryn_stats::store::history(&db, character, start, end, &privacy).await?;
    assert!(history.events.is_empty());
    assert!(
        history
            .daily
            .iter()
            .all(|d| d.skills.is_none() && d.minutes_online.is_none())
    );
    p.hidden = true;
    service
        .update(owner, "Aldric", p.clone(), now)
        .await
        .map_err(|e| format!("save: {e:?}"))?;
    assert!(!privacy.current_permissions(character).await.visible);
    assert!(
        oteryn_stats::store::history(&db, character, start, end, &privacy)
            .await?
            .daily
            .is_empty()
    );
    assert!(
        oteryn_stats::store::top_growth(&db, start, end, 10, &privacy)
            .await?
            .is_empty()
    );
    sqlx::query("DROP TABLE character_profiles")
        .execute(&db)
        .await?;
    assert!(!privacy.current_permissions(character).await.visible);
    db.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    Ok(())
}
