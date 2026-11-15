use axum::http::StatusCode;
use oteryn_world::catalog::{self, CatalogStore, Fence, Source};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[path = "../../../crates/world/tests/support/catalog.rs"]
mod catalog_fixture;

#[tokio::test]
async fn postgres_search_uses_native_catalogue_and_readiness()
-> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!(
        "search_native_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let path = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(4)
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
    oteryn_db::migrate(&pool).await?;
    let fixture = catalog_fixture::supported();
    let fence = Fence {
        snapshot_id: fixture.text("snapshot_id"),
        source_revision: fixture.text("source_revision"),
        authority_epoch: fixture.text("authority_epoch"),
        ruleset_id: fixture.text("ruleset_id"),
        content_profile_id: fixture.text("content_profile_id"),
    };
    let candidate = catalog::verify(&fixture.bytes, &fixture.sidecar, &fence)?;
    candidate.persist_inactive(&pool).await?;
    candidate.publish(&pool).await?;
    sqlx::raw_sql("INSERT INTO world_versions(id,source_commit,status) VALUES ('00000000-0000-0000-0000-000000000051',repeat('b',40),'ready'); UPDATE world_active_version SET version_id='00000000-0000-0000-0000-000000000051'; INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES ('00000000-0000-0000-0000-000000000051','Item','legacy-sword','{\"name\":\"Legacy Sword\"}',true); INSERT INTO world_visibility(kind,key,allowed) VALUES ('Item','legacy-sword',true),('Item','oteryn:item.training_sword',true)").execute(&pool).await?;
    let ready = Arc::new(AtomicBool::new(true));
    let store = CatalogStore::new(
        pool.clone(),
        Some(Source::local(
            std::path::PathBuf::new(),
            fence,
            fixture.sha256(),
        )),
    )
    .with_readiness(ready.clone());
    let selection: oteryn_web::search::SelectionReader = Arc::new({
        let store = store.clone();
        move || {
            let store = store.clone();
            Box::pin(async move {
                store
                    .active_version_id()
                    .await
                    .map(|v| v.map(|v| v.to_string()))
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
            })
        }
    });
    let reader: oteryn_web::search::WorldReader = Arc::new(move |kind, key| {
        let store = store.clone();
        Box::pin(async move {
            if !store
                .available(&kind)
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            {
                return Ok(None);
            }
            store
                .get_entity(&kind, &key)
                .await
                .map(|v| v.map(|e| oteryn_web::search::world_title(&e.data, &e.key, "en")))
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
        })
    });
    // A projected row alone cannot grant an unsupported catalogue capability.
    sqlx::raw_sql("INSERT INTO world_entities(version_id,kind,key,data,publishable) SELECT version_id,'NPC','sword-trader','{\"name\":\"Sword Trader\"}',true FROM world_catalog_versions; INSERT INTO world_visibility(kind,key,allowed) VALUES('NPC','sword-trader',true)").execute(&pool).await?;
    let hits = oteryn_web::search::world_search_selected(
        &pool,
        "en",
        "sword",
        1,
        reader.clone(),
        selection.clone(),
    )
    .await
    .map_err(|_| "search refused")?;
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Training Sword");
    assert!(!hits[0].href.contains("legacy"));
    ready.store(false, Ordering::Release);
    for query in ["sword", "absent"] {
        assert!(matches!(
            oteryn_web::search::world_search_selected(
                &pool,
                "en",
                query,
                1,
                reader.clone(),
                selection.clone()
            )
            .await,
            Err(StatusCode::SERVICE_UNAVAILABLE)
        ));
    }
    ready.store(true, Ordering::Release);
    let flipping_reader: oteryn_web::search::WorldReader = Arc::new({
        let reader = reader.clone();
        let ready = ready.clone();
        move |kind, key| {
            let reader = reader.clone();
            let ready = ready.clone();
            Box::pin(async move {
                let result = reader(kind, key).await;
                ready.store(false, Ordering::Release);
                result
            })
        }
    });
    assert!(matches!(
        oteryn_web::search::world_search_selected(
            &pool,
            "en",
            "training",
            1,
            flipping_reader,
            selection.clone()
        )
        .await,
        Err(StatusCode::SERVICE_UNAVAILABLE)
    ));
    ready.store(true, Ordering::Release);
    sqlx::query("UPDATE world_catalog_candidates SET source_revision=repeat('c',40)")
        .execute(&pool)
        .await?;
    assert!(matches!(
        oteryn_web::search::world_search_selected(&pool, "en", "sword", 1, reader, selection).await,
        Err(StatusCode::SERVICE_UNAVAILABLE)
    ));
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    root.close().await;
    eprintln!(
        "postgres_search_uses_native_catalogue_and_readiness: {:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
