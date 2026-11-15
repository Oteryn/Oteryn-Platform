use super::*;
use crate::audit06_test_db::{Database, Result};
use oteryn_characters::{
    lcfa::Settings,
    profile::{Service, authority::Authority},
};
#[tokio::test]
async fn audit_w2_c04_real_t45_denial_cannot_become_home_population() -> Result {
    let started = std::time::Instant::now();
    let d = Database::new().await?;
    let world = uuid::Uuid::now_v7();
    let character = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO world_status(id,name) VALUES($1,'Test')")
        .bind(world)
        .execute(&d.db)
        .await?;
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1")
        .execute(&d.db)
        .await?;
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='oteryn:game:primary',complete_through=floor(extract(epoch FROM clock_timestamp()))::bigint,observed_at=floor(extract(epoch FROM clock_timestamp()))::bigint WHERE id=1").execute(&d.db).await?;
    let authority = Authority {
        service: Service::new(
            d.db.clone(),
            Settings {
                enabled: true,
                environment: oteryn_core::Environment::Testing,
                source_authority: "oteryn:game:primary".into(),
                ..Settings::default()
            },
            true,
        ),
    };
    authority.ready().await.map_err(|_| "privacy unavailable")?;
    assert!(
        authority
            .reference(character, &world.to_string(), "Hidden", now() as i64)
            .await
            .map_err(|_| "privacy unavailable")?
            .is_none()
    );
    let roster = json!({"world_id":world.to_string(),"characters":[{"character_id":character.to_string(),"name":"Hidden","level":"100","vocation":1}]});
    sqlx::query("INSERT INTO public_snapshots(family,subject,epoch,version,watermark,payload) VALUES('world_online',$1,1,1,floor(extract(epoch FROM clock_timestamp()))::bigint,$2)").bind(world.to_string()).bind(roster).execute(&d.db).await?;
    let state = Worlds { db: d.db.clone() };
    let worlds = read(&state).await?;
    assert_eq!(worlds[0].status, "unknown");
    assert!(worlds[0].count.is_none());
    let home = home_status_for(&state).await.ok_or("home missing")?;
    assert!(home.online.is_none());
    assert_eq!(home.worlds_fresh, 0);
    d.close().await?;
    eprintln!(
        "audit_w2_c04_real_t45_denial_cannot_become_home_population {:?}",
        started.elapsed()
    );
    Ok(())
}
