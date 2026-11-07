//! Synthetic PostgreSQL regression: real adapters and existing authenticated Game push ingest.
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_characters::{
    lcfa::Settings,
    profile::{Preferences, Service, authority::Authority as ProfileAuthority},
};
use oteryn_community::{ingest, online, privacy::Authority};
use oteryn_internal_api::guard::{Binding, Identity, IdentityMap, VerifiedClient};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
async fn push(app: Router, path: &str, body: String) -> Result {
    let response = app
        .oneshot(
            Request::post(format!("/internal/v1/game-public/{path}"))
                .header("content-type", "application/json")
                .extension(VerifiedClient::from_verified_tls13_http11(Identity::san(
                    "privacy-test",
                )?))
                .body(Body::from(body))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(())
}
async fn page(app: Router) -> Result<String> {
    let response = app
        .oneshot(Request::get("/en/online").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()["cache-control"]
            .to_str()?
            .contains("no-store")
    );
    Ok(String::from_utf8(
        to_bytes(response.into_body(), 1_000_000).await?.to_vec(),
    )?)
}
#[tokio::test]
async fn online_real_privacy_owner_mutations_and_missing_state() -> Result {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping real privacy PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = PgPool::connect(&url).await?;
    let schema = format!("privacy_{}", Uuid::now_v7().simple());
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
        include_str!("../../../migrations/20261105180000_t24_public_snapshots.sql"),
    ] {
        sqlx::raw_sql(sql).execute(&db).await?;
    }
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let owner = Uuid::now_v7();
    let foreign = Uuid::now_v7();
    let character = Uuid::now_v7();
    let world = Uuid::now_v7();
    for id in [owner, foreign] {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at,public_status_visible) VALUES($1,$2,'test','active','en',now(),true)").bind(id).bind(format!("{id}@example.invalid")).execute(&db).await?;
        sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'privacy-test','1','1',$2,$3)").bind(id).bind(now).bind(vec![0u8;32]).execute(&db).await?;
    }
    sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='privacy-test',complete_through=$1,observed_at=$1 WHERE id=1").bind(now).execute(&db).await?;
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Aldric','AVAILABLE')").bind(owner).bind(character).bind(world).execute(&db).await?;
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
    let authority = Authority::new(
        db.clone(),
        ProfileAuthority {
            service: service.clone(),
        },
    );
    let app = online::router(
        db.clone(),
        Arc::new(authority.clone()),
        online::Config {
            enabled: true,
            ..online::Config::default()
        },
    );
    let identities = IdentityMap::new(vec![Binding {
        identity: Identity::san("privacy-test")?,
        group: "game-public".into(),
    }])?;
    let ingest = ingest::router(
        db.clone(),
        Arc::new(identities),
        ingest::Config {
            enabled: true,
            source_authority: "privacy-test".into(),
        },
    );
    push(ingest.clone(),"character-profile",format!(r#"{{"contract_version":1,"operation":"PublishCharacterProfileV1","source_authority":"privacy-test","character_id":"{character}","projection_epoch":"1","projection_revision":"1","source_observed_at":"{now}","profile":[{{"world_id":"{world}","name":"Aldric","vocation":4,"level":"412","guild":[],"guild_visible":false}}]}}"#)).await?;
    push(ingest.clone(),"watermark",format!(r#"{{"contract_version":1,"operation":"PublishPublicProjectionWatermarkV1","source_authority":"privacy-test","family":"character_profile","projection_epoch":"1","complete_through":"{now}","observed_at":"{now}"}}"#)).await?;
    push(ingest,"world-online",format!(r#"{{"contract_version":1,"operation":"PublishWorldOnlineV1","source_authority":"privacy-test","world_id":"{world}","projection_epoch":"1","projection_revision":"1","source_observed_at":"{now}","characters":[{{"name":"Aldric","vocation":4,"level":"412"}},{{"name":"Unknown","vocation":4,"level":"500"}}]}}"#)).await?;
    assert!(!page(app.clone()).await?.contains("Aldric")); // no saved privacy state
    let mut p = Preferences {
        hidden: false,
        show_status: true,
        ..Preferences::default()
    };
    assert_eq!(
        service.update(foreign, "Aldric", p.clone(), now).await,
        Err(oteryn_characters::profile::Error::NotFound)
    );
    service
        .update(owner, "Aldric", p.clone(), now)
        .await
        .map_err(|e| format!("save: {e:?}"))?;
    let html = page(app.clone()).await?;
    assert!(html.contains("Aldric"));
    assert!(!html.contains("Unknown"));
    assert!(!html.contains(&owner.to_string()));
    p.show_status = false;
    service
        .update(owner, "Aldric", p.clone(), now)
        .await
        .map_err(|e| format!("save: {e:?}"))?;
    assert!(!page(app.clone()).await?.contains("Aldric"));
    p.show_status = true;
    p.hidden = true;
    service
        .update(owner, "Aldric", p.clone(), now)
        .await
        .map_err(|e| format!("save: {e:?}"))?;
    assert!(!page(app.clone()).await?.contains("Aldric"));
    p.hidden = false;
    service
        .update(owner, "Aldric", p.clone(), now)
        .await
        .map_err(|e| format!("save: {e:?}"))?;
    sqlx::query("UPDATE accounts SET public_status_visible=false WHERE id=$1")
        .bind(owner)
        .execute(&db)
        .await?;
    assert!(!page(app.clone()).await?.contains("Aldric"));
    sqlx::query("UPDATE accounts SET public_status_visible=true WHERE id=$1")
        .bind(owner)
        .execute(&db)
        .await?;
    sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,'Aldric','AVAILABLE')").bind(foreign).bind(character).bind(world).execute(&db).await?;
    assert!(!page(app.clone()).await?.contains("Aldric"));
    sqlx::query("DELETE FROM account_characters WHERE account_id=$1")
        .bind(foreign)
        .execute(&db)
        .await?;
    sqlx::query("DROP TABLE character_profiles")
        .execute(&db)
        .await?;
    let response = app
        .oneshot(Request::get("/en/online").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    db.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    Ok(())
}
