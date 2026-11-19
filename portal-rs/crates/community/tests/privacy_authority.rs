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
        include_str!("../../../migrations/20261105160001_t22_character_commands.sql"),
        include_str!("../../../migrations/20261105145653_t18_account_characters.sql"),
        include_str!("../../../migrations/20261105220226_t09c_privacy.sql"),
        include_str!("../../../migrations/20261105231104_t45_profile_preferences.sql"),
        include_str!("../../../migrations/20261107130001_t45_privacy.up.sql"),
        concat!(
            include_str!("../../../migrations/20261105180000_t24_public_snapshots.sql"),
            include_str!("../../../migrations/20261115120019_fix_a2-l8_projection_conflicts.sql")
        ),
        include_str!("../../../migrations/20261115110016_fix_w2a_ingest_budget.sql"),
    ] {
        sqlx::raw_sql(sql).execute(&db).await?;
    }
    sqlx::raw_sql(
        include_str!("../../../migrations/20261115216000_fix_a3-06-community_serving.sql")
            .split("-- Indexed parent")
            .next()
            .ok_or("missing privacy migration")?,
    )
    .execute(&db)
    .await?;
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
    push(ingest.clone(),"world-online",format!(r#"{{"contract_version":1,"operation":"PublishWorldOnlineV1","source_authority":"privacy-test","world_id":"{world}","projection_epoch":"1","projection_revision":"1","source_observed_at":"{now}","characters":[{{"name":"Aldric","vocation":4,"level":"412"}},{{"name":"Unknown","vocation":4,"level":"500"}}]}}"#)).await?;
    push(ingest,"watermark",format!(r#"{{"contract_version":1,"operation":"PublishPublicProjectionWatermarkV1","source_authority":"privacy-test","family":"world_online","projection_epoch":"1","complete_through":"{now}","observed_at":"{now}"}}"#)).await?;
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

#[tokio::test]
async fn audit06_configured_rankings_uses_authority() -> Result {
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    if let Ok(schema) = std::env::var("AUDIT06_RANKINGS_SCHEMA") {
        let db = PgPool::connect_with(
            url.parse::<sqlx::postgres::PgConnectOptions>()?
                .options([("search_path", schema.as_str())]),
        )
        .await?;
        let response = oteryn_community::rankings::configured_router(db.clone())
            .oneshot(Request::get("/en/rankings/level").body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
        assert!(html.contains("Aldric"));
        assert!(!html.contains("HiddenSecret"));
        db.close().await;
        return Ok(());
    }
    let fixture = RankingFixture::new().await?;
    let schema = &fixture.schema;
    // Configure only the child test process; other tests never observe these flags.
    let status = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "audit06_configured_rankings_uses_authority",
            "--nocapture",
        ])
        .env("AUDIT06_RANKINGS_SCHEMA", schema)
        .env("PORTAL_PUBLIC_RANKINGS_ENABLED", "true")
        .env("OTERYN_CHARACTER_PROFILES_ENABLED", "true")
        .env("OTERYN_LCFA_ENABLED", "true")
        .env("OTERYN_ENV", "testing")
        .env("OTERYN_LCFA_SOURCE_AUTHORITY", "privacy-test")
        .status()?;
    assert!(status.success());
    fixture.finish().await
}

struct RankingFixture {
    admin: PgPool,
    db: PgPool,
    schema: String,
    service: Service,
    owner: Uuid,
    character: Uuid,
    hidden: Uuid,
    now: i64,
}
impl RankingFixture {
    async fn new() -> Result<Self> {
        let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
        let admin = PgPool::connect(&url).await?;
        let schema = format!("ranking_config06_{}", Uuid::now_v7().simple());
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
        oteryn_db::migrate(&db).await?;
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let owner = Uuid::now_v7();
        let character = Uuid::now_v7();
        let hidden = Uuid::now_v7();
        let world = Uuid::now_v7();
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at,public_status_visible) VALUES($1,'rank@example.invalid','test','active','en',now(),true)").bind(owner).execute(&db).await?;
        sqlx::query("INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,projection_revision,source_observed_at,content_digest) VALUES($1,'privacy-test','1','1',$2,$3)").bind(owner).bind(now).bind(vec![0u8;32]).execute(&db).await?;
        sqlx::query("UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',source_authority='privacy-test',complete_through=$1,observed_at=$1 WHERE id=1").bind(now).execute(&db).await?;
        sqlx::query("UPDATE public_projection_state SET highest_epoch=1")
            .execute(&db)
            .await?;
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
        let mut rows = vec![];
        for (i, id, name, hidden) in [
            (1, character, "Aldric", false),
            (2, hidden, "HiddenSecret", true),
        ] {
            sqlx::query("INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES($1,$2,$3,$4,'AVAILABLE')").bind(owner).bind(id).bind(world).bind(name).execute(&db).await?;
            service
                .update(
                    owner,
                    name,
                    Preferences {
                        hidden,
                        ..Preferences::default()
                    },
                    now,
                )
                .await
                .map_err(|_| "privacy save failed")?;
            sqlx::query("INSERT INTO public_snapshots(family,subject,epoch,version,watermark,payload) VALUES('character_profile',$1,1,1,$2,$3)").bind(id.to_string()).bind(now).bind(serde_json::json!({"profile":[{"world_id":world,"name":name}]})).execute(&db).await?;
            rows.push(serde_json::json!({"rank":i.to_string(),"character_id":id,"name":name,"vocation":1,"level":"100","value":"1000"}));
        }
        sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('character_profile','',true,1,1,$1,'null')").bind(now).execute(&db).await?;
        sqlx::query("INSERT INTO public_snapshots(family,subject,epoch,version,watermark,payload) VALUES('highscores',$1,1,1,$2,$3)").bind(format!("{world}:1:0")).bind(now).bind(serde_json::json!({"world_id":world,"category":1,"filter":0,"rows":rows})).execute(&db).await?;

        Ok(Self {
            admin,
            db,
            schema,
            service,
            owner,
            character,
            hidden,
            now,
        })
    }
    async fn finish(self) -> Result {
        self.db.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await?;
        self.admin.close().await;
        Ok(())
    }
}

#[tokio::test]
async fn w3_c06_007_real_preference_update_during_rankings() -> Result {
    let pg_started = std::time::Instant::now();
    let fixture = RankingFixture::new().await?;
    let db = fixture.db.clone();
    let service = fixture.service.clone();
    let owner = fixture.owner;
    let character = fixture.character;
    let now = fixture.now;
    // The first allow must not survive a restrictive save during the next lookup.
    #[derive(Debug)]
    struct RestrictSecond {
        authority: Authority,
        service: Service,
        owner: Uuid,
        first: String,
        changed: std::sync::atomic::AtomicBool,
    }
    impl oteryn_community::rankings::PublicVisibility for RestrictSecond {
        fn is_hidden<'a>(
            &'a self,
            id: &'a str,
        ) -> oteryn_community::rankings::VisibilityFuture<'a, bool> {
            Box::pin(async move {
                if id != self.first && !self.changed.swap(true, std::sync::atomic::Ordering::SeqCst)
                {
                    self.service
                        .update(
                            self.owner,
                            "Aldric",
                            Preferences {
                                hidden: true,
                                ..Preferences::default()
                            },
                            time::OffsetDateTime::now_utc().unix_timestamp(),
                        )
                        .await
                        .map_err(|_| oteryn_community::rankings::VisibilityUnavailable)?;
                }
                oteryn_community::rankings::PublicVisibility::is_hidden(&self.authority, id).await
            })
        }
    }
    let authority = Authority::new(
        db.clone(),
        ProfileAuthority {
            service: service.clone(),
        },
    );
    let (before, _) =
        oteryn_community::rankings::home_top(&db, &authority, oteryn_web::Language::En, 10, true)
            .await
            .map_err(|_| "initial ranking")?;
    assert!(before.iter().any(|row| row.name == "Aldric"));
    for home in [false, true] {
        service
            .update(
                owner,
                "Aldric",
                Preferences {
                    hidden: false,
                    ..Preferences::default()
                },
                now,
            )
            .await
            .map_err(|_| "reset privacy")?;
        let visibility = Arc::new(RestrictSecond {
            authority: authority.clone(),
            service: service.clone(),
            owner,
            first: character.to_string(),
            changed: std::sync::atomic::AtomicBool::new(false),
        });
        if home {
            assert!(
                oteryn_community::rankings::home_top(
                    &db,
                    visibility.as_ref(),
                    oteryn_web::Language::En,
                    10,
                    true
                )
                .await
                .is_err()
            );
        } else {
            let response = oteryn_community::rankings::router(db.clone(), true, visibility)
                .oneshot(Request::get("/en/rankings/level").body(Body::empty())?)
                .await?;
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert!(
                !String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?
                    .contains("Aldric")
            );
        }
    }
    eprintln!(
        "w3_c06_007_real_preference_update_during_rankings {:?}",
        pg_started.elapsed()
    );
    fixture.finish().await
}

#[tokio::test]
async fn w3_c06_006_ranking_name_handoff() -> Result {
    let pg_started = std::time::Instant::now();
    let fixture = RankingFixture::new().await?;
    let db = fixture.db.clone();
    let service = fixture.service.clone();
    let owner = fixture.owner;
    let character = fixture.character;
    let now = fixture.now;
    let hidden = fixture.hidden;
    let authority = Authority::new(
        db.clone(),
        ProfileAuthority {
            service: service.clone(),
        },
    );
    service
        .update(
            owner,
            "Aldric",
            Preferences {
                hidden: false,
                ..Preferences::default()
            },
            now,
        )
        .await
        .map_err(|_| "reset privacy")?;
    sqlx::query("UPDATE account_characters SET name='Newname' WHERE character_id=$1")
        .bind(character)
        .execute(&db)
        .await?;
    sqlx::query("UPDATE account_characters SET name='Aldric' WHERE character_id=$1")
        .bind(hidden)
        .execute(&db)
        .await?;
    service
        .update(
            owner,
            "Aldric",
            Preferences {
                hidden: false,
                ..Preferences::default()
            },
            now,
        )
        .await
        .map_err(|_| "new holder privacy")?;
    for (id, name) in [(character, "Newname"), (hidden, "Aldric")] {
        sqlx::query("UPDATE public_snapshots SET payload=jsonb_set(payload,'{profile,0,name}',to_jsonb($2::text)),version=version+1 WHERE family='character_profile' AND subject=$1").bind(id.to_string()).bind(name).execute(&db).await?;
    }
    assert!(
        !oteryn_community::rankings::PublicVisibility::is_hidden(
            &authority,
            &character.to_string()
        )
        .await
        .map_err(|_| "current visibility")?
    );
    let (ranks, _) =
        oteryn_community::rankings::home_top(&db, &authority, oteryn_web::Language::En, 10, true)
            .await
            .map_err(|_| "ranking read")?;
    assert!(
        ranks.is_empty(),
        "retained statistics must not link to a new name holder"
    );
    let response = oteryn_community::rankings::router(db.clone(), true, Arc::new(authority))
        .oneshot(Request::get("/en/rankings/level").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        !String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?
            .contains("Aldric")
    );
    eprintln!("w3_c06_006_ranking_name_handoff {:?}", pg_started.elapsed());

    fixture.finish().await
}

async fn seed_online(db: &PgPool) -> Result {
    sqlx::query("INSERT INTO public_snapshots(family,subject,epoch,version,watermark,payload) SELECT 'world_online',payload->>'world_id',1,1,watermark,jsonb_build_object('world_id',payload->>'world_id','characters',jsonb_build_array(jsonb_build_object('name','Aldric','level','100','vocation',1))) FROM public_snapshots WHERE family='highscores'")
        .execute(db).await?;
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES('world_online','',true,1,1,floor(extract(epoch FROM clock_timestamp()))::bigint,'null')")
        .execute(db).await?;
    Ok(())
}

#[tokio::test]
async fn stale_online_watermark_withholds_presence_with_fresh_profiles() -> Result {
    let fixture = RankingFixture::new().await?;
    fixture
        .service
        .update(
            fixture.owner,
            "Aldric",
            Preferences {
                hidden: false,
                show_status: true,
                ..Preferences::default()
            },
            fixture.now,
        )
        .await
        .map_err(|_| "privacy save failed")?;
    seed_online(&fixture.db).await?;
    let app = online::router(
        fixture.db.clone(),
        Arc::new(Authority::new(
            fixture.db.clone(),
            ProfileAuthority {
                service: fixture.service.clone(),
            },
        )),
        online::Config {
            enabled: true,
            ..Default::default()
        },
    );
    assert!(page(app.clone()).await?.contains("Aldric"));

    let world = sqlx::query_scalar::<_, String>(
        "SELECT subject FROM public_snapshots WHERE family='world_online' AND NOT is_watermark",
    )
    .fetch_one(&fixture.db)
    .await?;
    for age in [Some(31_i64), Some(-10), None] {
        if let Some(age) = age {
            sqlx::query("UPDATE public_snapshots SET watermark=floor(extract(epoch FROM clock_timestamp()))::bigint-$1 WHERE family='world_online' AND is_watermark")
                .bind(age).execute(&fixture.db).await?;
        } else {
            sqlx::query(
                "DELETE FROM public_snapshots WHERE family='world_online' AND is_watermark",
            )
            .execute(&fixture.db)
            .await?;
        }
        let html = page(app.clone()).await?;
        assert!(!html.contains("Aldric"), "watermark age: {age:?}");
        assert!(!html.contains(&world));
        assert!(!html.contains("Visible characters online:"));
        assert!(html.contains("Data is stale"));
    }
    fixture.finish().await
}

async fn stale_profile_page(online_page: bool) -> Result {
    let fixture = RankingFixture::new().await?;
    let db = &fixture.db;
    let authority = Arc::new(Authority::new(
        db.clone(),
        ProfileAuthority {
            service: fixture.service.clone(),
        },
    ));
    let (app, path) = if online_page {
        seed_online(db).await?;
        (
            online::router(
                db.clone(),
                authority,
                online::Config {
                    enabled: true,
                    ..Default::default()
                },
            ),
            "/en/online",
        )
    } else {
        (
            oteryn_community::rankings::router(db.clone(), true, authority),
            "/en/rankings/level",
        )
    };
    let request = || Request::get(path).body(Body::empty());
    assert_eq!(
        app.clone().oneshot(request()?).await?.status(),
        StatusCode::OK
    );
    sqlx::query("UPDATE public_snapshots SET watermark=watermark-60 WHERE family='character_profile' AND is_watermark").execute(db).await?;
    let status = app.oneshot(request()?).await?.status();
    fixture.finish().await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    Ok(())
}

#[tokio::test]
async fn stale_profile_online_returns_503() -> Result {
    stale_profile_page(true).await
}

#[tokio::test]
async fn stale_profile_rankings_returns_503() -> Result {
    stale_profile_page(false).await
}
