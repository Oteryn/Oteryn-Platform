use super::*;
use crate::{
    audit06_test_db::{Database, Result},
    guilds, houses, ingest, today,
};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::Request,
};
use oteryn_db::ingest_budget::admit;
use oteryn_internal_api::guard::{Binding, Identity, IdentityMap, VerifiedClient};
use std::{sync::Arc, time::Instant};
use tower::ServiceExt;
const AUTHORITY: &str = "oteryn:game:primary";
const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
fn identities() -> Result<Arc<IdentityMap>> {
    Ok(Arc::new(IdentityMap::new(vec![Binding {
        identity: Identity::san("budget-test")?,
        group: "game-public".into(),
    }])?))
}
fn key() -> Vec<u8> {
    let mut key = vec![1, 11];
    key.extend_from_slice(b"budget-test");
    key
}
async fn send(app: Router, route: &str, body: &str) -> Result<Response> {
    Ok(app
        .oneshot(
            Request::post(format!("/internal/{route}"))
                .header("content-type", "application/json")
                .extension(VerifiedClient::from_verified_tls13_http11(Identity::san(
                    "budget-test",
                )?))
                .body(Body::from(body.to_owned()))?,
        )
        .await?)
}
async fn exhausted(response: Response) -> Result {
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers()[header::RETRY_AFTER], "60");
    assert!(to_bytes(response.into_body(), 1024).await?.is_empty());
    Ok(())
}
async fn seed(db: &PgPool, route: &str, count: i32) -> Result {
    sqlx::query("INSERT INTO community_ingest_budgets(publisher,route,admitted_at) SELECT $1,$2,array_fill(extract(epoch FROM clock_timestamp())::double precision,ARRAY[$3])")
        .bind(key()).bind(route).bind(count).execute(db).await?;
    Ok(())
}
#[tokio::test]
async fn audit06_009_all_ingest_routes_enforce_shared_budget() -> Result {
    let started = Instant::now();
    let database = Database::new().await?;
    let db = &database.db;
    let ids = identities()?;
    let config = ingest::Config {
        enabled: true,
        source_authority: AUTHORITY.into(),
    };
    let v1 = ingest::router(db.clone(), ids.clone(), config.clone());
    for (family, body) in V1_FIXTURES {
        let path = ingest::FAMILIES
            .iter()
            .find(|f| f.name == *family)
            .map_or("watermark", |f| f.path);
        let route = format!("v1/{path}");
        seed(db, &route, 5999).await?;
        let path = format!("v1/game-public/{path}");
        assert_eq!(
            send(v1.clone(), &path, body).await?.status(),
            StatusCode::OK,
            "{path}"
        );
        exhausted(send(v1.clone(), &path, body).await?).await?;
    }
    let guild = guilds::ingest_router(
        db.clone(),
        ids.clone(),
        guilds::IngestConfig {
            enabled: true,
            source_authority: AUTHORITY.into(),
        },
    );
    let house = houses::router(
        db.clone(),
        ids,
        Arc::new(houses::DenyOwners),
        houses::Config {
            ingest: config,
            serving_enabled: false,
        },
    );
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let guild_body = format!(
        r#"{{"contract_version":2,"operation":"PublishGuildV2","source_authority":"{AUTHORITY}","guild_id":"01934f10-7c06-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"1","source_observed_at":"{now}","guild":[]}}"#
    );
    let house_body = format!(
        r#"{{"contract_version":2,"operation":"PublishHouseV2","source_authority":"{AUTHORITY}","world_id":"{WORLD}","house_key":"oteryn:content.house.example_shop","projection_epoch":"1","projection_revision":"1","source_observed_at":"{now}","house":[]}}"#
    );
    for (app, family, body) in [(guild, "guild", guild_body), (house, "house", house_body)] {
        let watermark = format!(
            r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"{AUTHORITY}","family":"{family}","projection_epoch":"1","complete_through":"{now}","observed_at":"{now}"}}"#
        );
        sqlx::query("DELETE FROM community_ingest_budgets WHERE route='v2/watermark'")
            .execute(db)
            .await?;
        for (suffix, path, body) in [("", family, body), ("/watermark", "watermark", watermark)] {
            seed(db, &format!("v2/{family}{suffix}"), 5999).await?;
            let path = format!("v2/game-public/{path}");
            assert_eq!(
                send(app.clone(), &path, &body).await?.status(),
                StatusCode::OK,
                "{family}{suffix}"
            );
            exhausted(send(app.clone(), &path, &body).await?).await?;
        }
    }
    database.close().await?;
    eprintln!(
        "audit06_009_all_ingest_routes_enforce_shared_budget duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
#[tokio::test]
async fn audit06_009_daily_budget_spans_two_router_instances() -> Result {
    let started = Instant::now();
    let database = Database::new().await?;
    sqlx::query("INSERT INTO world_status(id,name) VALUES ($1,'Test World')")
        .bind(uuid::Uuid::parse_str(WORLD)?)
        .execute(&database.db)
        .await?;
    let config = today::Config {
        ingest_enabled: true,
        public_enabled: false,
        source_authority: AUTHORITY.into(),
        worlds: [uuid::Uuid::parse_str(WORLD)?].into(),
    };
    let first = today::router(database.db.clone(), identities()?, config.clone());
    let second = today::router(database.db.clone(), identities()?, config);
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let snapshot = format!(
        r#"{{"contract_version":2,"operation":"PublishWorldDailyV2","source_authority":"{AUTHORITY}","world_id":"{WORLD}","projection_epoch":"1","projection_revision":"1","source_observed_at":"{now}","daily":[]}}"#
    );
    let watermark = format!(
        r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"{AUTHORITY}","family":"world_daily","projection_epoch":"1","complete_through":"{now}","observed_at":"{now}"}}"#
    );
    for (path, body, limit) in [("world-daily", snapshot, 120), ("watermark", watermark, 60)] {
        let path = format!("v2/game-public/{path}");
        for index in 0..limit {
            let app = if index % 2 == 0 {
                first.clone()
            } else {
                second.clone()
            };
            let status = send(app, &path, &body).await?.status();
            assert!(status == StatusCode::OK, "{path}: {status}");
        }
        exhausted(send(first.clone(), &path, &body).await?).await?;
        exhausted(send(second.clone(), &path, &body).await?).await?;
    }
    database.close().await?;
    eprintln!(
        "audit06_009_daily_budget_spans_two_router_instances duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
#[tokio::test]
async fn audit06_009_atomic_budget_isolates_publishers_routes_and_expires() -> Result {
    let started = Instant::now();
    let database = Database::new().await?;
    let mut tasks = Vec::new();
    for _ in 0..24 {
        let db = database.db.clone();
        tasks.push(tokio::spawn(async move {
            admit(&db, b"publisher-a", "route-a", 12).await
        }));
    }
    let mut admitted = 0;
    for task in tasks {
        admitted += usize::from(task.await??);
    }
    assert_eq!(admitted, 12);
    assert!(admit(&database.db, b"publisher-b", "route-a", 12).await?);
    assert!(admit(&database.db, b"publisher-a", "route-b", 12).await?);
    sqlx::query("UPDATE community_ingest_budgets SET admitted_at=ARRAY[extract(epoch FROM clock_timestamp())::double precision-61]")
        .execute(&database.db).await?;
    assert!(admit(&database.db, b"publisher-a", "route-a", 1).await?);
    assert!(!admit(&database.db, b"publisher-a", "route-a", 1).await?);
    database.close().await?;
    eprintln!(
        "audit06_009_atomic_budget_isolates_publishers_routes_and_expires duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

const V1_FIXTURES: &[(&str, &str)] = &[
    (
        "highscores",
        r#"{"contract_version":1,"operation":"PublishWorldHighscoresV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","category":1,"filter":0,"projection_epoch":"1","projection_revision":"17","source_observed_at":"1790000000","computed_at":"1790000000","rows":[{"rank":"1","character_id":"01934f10-7c04-7001-805b-3b1122334401","name":"Aldric","vocation":4,"level":"412","value":"11400000000"}]}"#,
    ),
    (
        "character_profile",
        r#"{"contract_version":1,"operation":"PublishCharacterProfileV1","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"9","source_observed_at":"1790000000","profile":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Aldric","vocation":4,"level":"412","guild":[{"guild_id":"01934f10-7c06-7001-805b-3b1122334401","rank_level":3}],"guild_visible":true}]}"#,
    ),
    (
        "character_deaths",
        r#"{"contract_version":1,"operation":"PublishCharacterDeathsV1","source_authority":"oteryn:game:primary","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"3","source_observed_at":"1790000000","deaths":[{"occurred_at":"1789990000","level":"413","pvp":true,"killers":["Beren","Mira"]}]}"#,
    ),
    (
        "guild",
        r#"{"contract_version":1,"operation":"PublishGuildV1","source_authority":"oteryn:game:primary","guild_id":"01934f10-7c06-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"5","source_observed_at":"1790000000","guild":[{"world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Red Rose","state":"ACTIVE","founded_at":"1789000000","ranks":[{"level":1,"name":"Leader"},{"level":2,"name":"Vice Leader"},{"level":3,"name":"Member"}],"members":[{"name":"Aldric","rank_level":1,"joined_at":"1789000000"}]}]}"#,
    ),
    (
        "house",
        r#"{"contract_version":1,"operation":"PublishHouseV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","house_key":"oteryn:content.house.example-street-1","projection_epoch":"1","projection_revision":"12","source_observed_at":"1790000000","house":[{"name":"Example Street 1","kind":"private_house","town":"oteryn:content.area.city.example","size_sqm":"24","beds":"2","rent_gold":"5000","status":"OWNED","owner_kind":"CHARACTER","owner_name":"Aldric","paid_until":"1792000000"}]}"#,
    ),
    (
        "world_online",
        r#"{"contract_version":1,"operation":"PublishWorldOnlineV1","source_authority":"oteryn:game:primary","world_id":"01934f10-7c02-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"8812","source_observed_at":"1790000000","characters":[{"name":"Aldric","vocation":4,"level":"412"}]}"#,
    ),
    (
        "watermark",
        r#"{"contract_version":1,"operation":"PublishPublicProjectionWatermarkV1","source_authority":"oteryn:game:primary","family":"world_online","projection_epoch":"1","complete_through":"1790000000","observed_at":"1790000003"}"#,
    ),
];

#[tokio::test]
async fn audit_w2_c03_v2_families_and_deaths_share_publisher_route_budgets() -> Result {
    let started = Instant::now();
    let database = Database::new().await?;
    let ids = Arc::new(IdentityMap::new(vec![
        Binding {
            identity: Identity::san("budget-test")?,
            group: "game-public".into(),
        },
        Binding {
            identity: Identity::san("second-publisher")?,
            group: "game-public".into(),
        },
    ])?);
    let config = crate::families::IngestConfig {
        skills: true,
        achievements: true,
        equipment: true,
        outfit: true,
        source_authority: AUTHORITY.into(),
        requests_per_minute: 1,
    };
    let make = || crate::families::ingest_router(database.db.clone(), ids.clone(), config.clone());
    for (route, body) in [
        (
            "character-skills",
            include_str!(
                "../../../../docs/contracts/fixtures/public-game-families-v2/valid-character-skills.json"
            ),
        ),
        (
            "character-achievements",
            include_str!(
                "../../../../docs/contracts/fixtures/public-game-families-v2/valid-character-achievements.json"
            ),
        ),
        (
            "character-equipment",
            include_str!(
                "../../../../docs/contracts/fixtures/public-game-families-v2/valid-character-equipment.json"
            ),
        ),
        (
            "character-outfit",
            include_str!(
                "../../../../docs/contracts/fixtures/public-game-families-v2/valid-character-outfit.json"
            ),
        ),
        (
            "watermark",
            include_str!(
                "../../../../docs/contracts/fixtures/public-game-families-v2/valid-character-skills-watermark.json"
            ),
        ),
    ] {
        let body = body.trim();
        let path = format!("v2/game-public/{route}");
        assert_eq!(send(make(), &path, body).await?.status(), StatusCode::OK);
        exhausted(send(make(), &path, body).await?).await?;
        let second = make()
            .oneshot(
                Request::post(format!("/internal/{path}"))
                    .header("content-type", "application/json")
                    .extension(VerifiedClient::from_verified_tls13_http11(Identity::san(
                        "second-publisher",
                    )?))
                    .body(Body::from(body.to_owned()))?,
            )
            .await?;
        assert_eq!(second.status(), StatusCode::OK);
    }
    // The same route identity must cover the independently mounted deaths handler.
    let make_deaths = || {
        crate::deaths::ingest_router(
            database.db.clone(),
            ids.clone(),
            crate::deaths::IngestConfig {
                requests_per_minute: 1,
                enabled: true,
                source_authority: AUTHORITY.into(),
            },
        )
    };
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let snapshot = format!(
        r#"{{"contract_version":2,"operation":"PublishCharacterDeathsV2","source_authority":"{AUTHORITY}","character_id":"01934f10-7c04-7001-805b-3b1122334401","projection_epoch":"1","projection_revision":"1","source_observed_at":"{now}","deaths":[]}}"#
    );
    assert_eq!(
        send(make_deaths(), "v2/game-public/character-deaths", &snapshot)
            .await?
            .status(),
        StatusCode::OK
    );
    exhausted(send(make_deaths(), "v2/game-public/character-deaths", &snapshot).await?).await?;
    let watermark = format!(
        r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"{AUTHORITY}","family":"character_deaths","projection_epoch":"1","complete_through":"{now}","observed_at":"{now}"}}"#
    );
    exhausted(send(make_deaths(), "v2/game-public/watermark", &watermark).await?).await?;
    database.close().await?;
    eprintln!(
        "audit_w2_c03_v2_families_and_deaths_share_publisher_route_budgets {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn w3_c06_001_watermark_dispatch_shares_budget_across_families() -> Result {
    let started = Instant::now();
    let database = Database::new().await?;
    let db = &database.db;
    let ids = identities()?;
    let guild = guilds::ingest_router(
        db.clone(),
        ids.clone(),
        guilds::IngestConfig {
            enabled: true,
            source_authority: AUTHORITY.into(),
        },
    );
    let house = houses::router(
        db.clone(),
        ids.clone(),
        Arc::new(houses::DenyOwners),
        houses::Config {
            ingest: ingest::Config {
                enabled: true,
                source_authority: AUTHORITY.into(),
            },
            serving_enabled: false,
        },
    );
    let deaths = crate::deaths::ingest_router(
        db.clone(),
        ids.clone(),
        crate::deaths::IngestConfig {
            enabled: true,
            source_authority: AUTHORITY.into(),
            requests_per_minute: 6000,
        },
    );
    let daily = today::router(
        db.clone(),
        ids.clone(),
        today::Config {
            ingest_enabled: true,
            public_enabled: false,
            source_authority: AUTHORITY.into(),
            worlds: [uuid::Uuid::parse_str(WORLD)?].into(),
        },
    );
    let dispatcher = crate::v2_watermark::router(deaths, guild, house, daily);
    seed(db, "v2/watermark", 5999).await?;
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let watermark = |family: &str| {
        format!(
            r#"{{"contract_version":2,"operation":"PublishPublicProjectionWatermarkV2","source_authority":"{AUTHORITY}","family":"{family}","projection_epoch":"1","complete_through":"{now}","observed_at":"{now}"}}"#
        )
    };
    assert_eq!(
        send(
            dispatcher.clone(),
            "v2/game-public/watermark",
            &watermark("guild")
        )
        .await?
        .status(),
        StatusCode::OK
    );
    for family in ["house", "character_deaths", "world_daily"] {
        exhausted(
            send(
                dispatcher.clone(),
                "v2/game-public/watermark",
                &watermark(family),
            )
            .await?,
        )
        .await?;
    }
    let skills = crate::families::ingest_router(
        db.clone(),
        ids,
        crate::families::IngestConfig {
            skills: true,
            source_authority: AUTHORITY.into(),
            ..Default::default()
        },
    );
    exhausted(
        send(
            skills,
            "v2/game-public/watermark",
            &watermark("character_skills"),
        )
        .await?,
    )
    .await?;
    let v1 = ingest::router(
        db.clone(),
        identities()?,
        ingest::Config {
            enabled: true,
            source_authority: AUTHORITY.into(),
        },
    );
    let v1_watermark = V1_FIXTURES
        .iter()
        .find(|(family, _)| *family == "watermark")
        .ok_or("watermark fixture missing")?
        .1;
    assert_eq!(
        send(v1, "v1/game-public/watermark", v1_watermark)
            .await?
            .status(),
        StatusCode::OK,
        "a different HTTP endpoint has its own allowance"
    );
    database.close().await?;
    eprintln!(
        "w3_c06_001_watermark_dispatch_shares_budget_across_families {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn w3_c06_009_ingested_unicode_and_url_names_follow_profile_links() -> Result {
    struct Visible;
    impl crate::profiles::Privacy for Visible {
        fn visible<'a>(&'a self, _: &'a str) -> crate::profiles::DecisionFuture<'a> {
            Box::pin(async { Ok(true) })
        }
    }
    let started = Instant::now();
    let database = Database::new().await?;
    let db = &database.db;
    let ingest = ingest::router(
        db.clone(),
        identities()?,
        ingest::Config {
            enabled: true,
            source_authority: AUTHORITY.into(),
        },
    );
    let raw = V1_FIXTURES
        .iter()
        .find(|(family, _)| *family == "character_profile")
        .ok_or("profile fixture missing")?
        .1;
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let watermark = format!(
        r#"{{"contract_version":1,"operation":"PublishPublicProjectionWatermarkV1","source_authority":"{AUTHORITY}","family":"character_profile","projection_epoch":"1","complete_through":"{now}","observed_at":"{now}"}}"#
    );
    assert_eq!(
        send(ingest.clone(), "v1/game-public/watermark", &watermark)
            .await?
            .status(),
        StatusCode::OK
    );
    for (i, name) in ["Élodie", "100% Hero", "Name?Tag#One"]
        .into_iter()
        .enumerate()
    {
        let publication = raw.replace("Aldric", name).replace(
            "\"projection_revision\":\"9\"",
            &format!("\"projection_revision\":\"{}\"", i + 10),
        );
        assert_eq!(
            send(
                ingest.clone(),
                "v1/game-public/character-profile",
                &publication
            )
            .await?
            .status(),
            StatusCode::OK
        );
        let encoded =
            percent_encoding::utf8_percent_encode(name, percent_encoding::NON_ALPHANUMERIC);
        let response = crate::profiles::router(db.clone(), true, Arc::new(Visible))
            .oneshot(Request::get(format!("/en/characters/{encoded}")).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK, "{name}");
    }
    database.close().await?;
    eprintln!(
        "w3_c06_009_ingested_unicode_and_url_names_follow_profile_links {:?}",
        started.elapsed()
    );
    Ok(())
}
