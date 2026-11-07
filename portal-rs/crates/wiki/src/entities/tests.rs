#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use axum::{
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::Request,
};
use oteryn_admin::rbac::{AccountState, AccountStateReader, AccountStateUnavailable, CheckContext};
use serde_json::json;
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct Active;
impl AccountStateReader for Active {
    async fn state(&self, _: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        Ok(AccountState::Active)
    }
}
struct Database {
    pool: PgPool,
    root: PgPool,
    schema: String,
    version: Uuid,
}
impl Database {
    async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("Skipping PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let root = PgPool::connect(&url).await.unwrap();
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&root)
            .await
            .unwrap();
        assert_eq!(version, "180006");
        let schema = format!("wiki_entities_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await
            .unwrap();
        let path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .after_connect(move |c, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(path)).execute(c).await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        let version = Uuid::now_v7();
        sqlx::query("INSERT INTO world_versions(id,source_commit,status) VALUES($1,$2,'ready')")
            .bind(version)
            .bind("a".repeat(40))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE world_active_version SET version_id=$1")
            .bind(version)
            .execute(&pool)
            .await
            .unwrap();
        Some(Self {
            pool,
            root,
            schema,
            version,
        })
    }
    fn world(&self) -> WorldStore {
        WorldStore::new(self.pool.clone(), WorldOptions { enabled: true })
    }
    async fn entity(
        &self,
        kind: &str,
        key: &str,
        data: serde_json::Value,
        publishable: bool,
        allowed: bool,
    ) {
        sqlx::query("INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES($1,$2,$3,$4,$5)").bind(self.version).bind(world_kind(kind)).bind(key).bind(data).bind(publishable).execute(&self.pool).await.unwrap();
        if allowed {
            self.world().allow(world_kind(kind), key).await.unwrap();
        }
    }
    fn app(&self, enabled: bool) -> Router {
        mount(
            crate::web::public_router(self.pool.clone()),
            EntityState::new(self.pool.clone(), enabled),
        )
    }
    async fn get(&self, url: &str, enabled: bool) -> (StatusCode, String) {
        let mut request = Request::builder().uri(url).body(Body::empty()).unwrap();
        request.extensions_mut().insert(ConnectInfo(
            "127.0.0.1:12345".parse::<std::net::SocketAddr>().unwrap(),
        ));
        let response = self.app(enabled).oneshot(request).await.unwrap();
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert!(response.headers().contains_key("content-security-policy"));
        let status = response.status();
        let body = String::from_utf8(
            to_bytes(response.into_body(), 1_000_000)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        (status, body)
    }
    async fn close(self) {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await
        .unwrap();
        self.root.close().await;
    }
}
#[tokio::test]
async fn postgres_entity_editorial_publication_and_locales() {
    let Some(db) = Database::open().await else {
        return;
    };
    db.entity("quests", "oteryn:quest/test", json!({"definition":{"display_name":"Quest <test>","classification":"OTS_HYPOTHESIS_ONLY","health":20,"source_refs":{"private":"DO NOT PUBLISH"}}}), true,true).await;
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&db.pool).await.unwrap();
    let context = CheckContext {
        account: actor,
        require_staff_mfa: true,
        mfa_verified: true,
    };
    let repo = crate::repository::Repository::new(db.pool.clone(), Active);
    let translations = ["en", "pl"].map(|locale| domain::Translation {
        locale: locale.into(),
        title: "Description".into(),
        slug: "quest-description".into(),
        summary: "Summary".into(),
        markdown: "**Published prose**".into(),
    });
    let id = repo
        .create(context, &translations, Some("quests:oteryn:quest/test"), "")
        .await
        .unwrap();
    let url = "/pl/wiki/quests/oteryn%3Aquest%2Ftest";
    let (status, body) = db.get(url, true).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body.contains("Published prose"));
    repo.transition(context, id, 1, domain::Action::Submit)
        .await
        .unwrap();
    repo.transition(context, id, 2, domain::Action::Publish)
        .await
        .unwrap();
    let (status, body) = db.get(url, true).await;
    assert_eq!(status, StatusCode::OK);
    for text in [
        "lang=\"pl\"",
        "Opis redakcyjny",
        "<strong>Published prose</strong>",
        "OTS_HYPOTHESIS_ONLY",
        &db.version.to_string(),
    ] {
        assert!(body.contains(text), "missing {text}");
    }
    assert!(!body.contains("Quest <test>"));
    assert!(!body.contains("DO NOT PUBLISH"));
    let (_, english) = db.get("/en/wiki/quests/oteryn%3Aquest%2Ftest", true).await;
    assert!(english.contains("Editorial description"));
    assert!(
        db.get("/en/wiki/search?q=Description", true)
            .await
            .1
            .contains("quest-description")
    );
    repo.transition(context, id, 3, domain::Action::Unpublish)
        .await
        .unwrap();
    assert!(!db.get(url, true).await.1.contains("Published prose"));
    db.close().await;
}
#[tokio::test]
async fn postgres_refuses_hidden_inactive_missing_and_disabled() {
    let Some(db) = Database::open().await else {
        return;
    };
    db.entity(
        "items",
        "denied",
        json!({"name":"Hidden item"}),
        true,
        false,
    )
    .await;
    db.entity(
        "achievements",
        "secret",
        json!({"name":"Secret achievement"}),
        false,
        true,
    )
    .await;
    db.entity(
        "items",
        "visible",
        json!({"name":"Visible item"}),
        true,
        true,
    )
    .await;
    for url in [
        "/en/wiki/items/denied",
        "/en/wiki/achievements/secret",
        "/en/wiki/items/missing",
        "/fr/wiki/items/visible",
        "/en/wiki/unknown/visible",
    ] {
        assert_eq!(db.get(url, true).await.0, StatusCode::NOT_FOUND);
    }
    assert_eq!(
        db.get("/en/wiki/items/visible", false).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        db.get("/en/wiki/items", false).await.0,
        StatusCode::NOT_FOUND
    );
    let (_, body) = db.get("/en/wiki", true).await;
    assert!(body.contains("Visible item"));
    assert!(!body.contains("Hidden item"));
    assert!(!body.contains("Secret achievement"));
    db.world().deny("Item", "visible").await.unwrap();
    assert_eq!(
        db.get("/en/wiki/items/visible", true).await.0,
        StatusCode::NOT_FOUND
    );
    sqlx::query("UPDATE world_active_version SET version_id=NULL")
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        db.get("/en/wiki/items", true).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        db.get("/en/wiki/items/visible", true).await.0,
        StatusCode::NOT_FOUND
    );
    db.close().await;
}
#[tokio::test]
async fn postgres_filters_search_pagination_and_invalid_input() {
    let Some(db) = Database::open().await else {
        return;
    };
    for n in 0..25 {
        db.entity(
            "items",
            &format!("item-{n:02}"),
            json!({"name":format!("Sword {n:02}"),"category":"weapon"}),
            true,
            true,
        )
        .await;
    }
    db.entity("quests","quest",json!({"definition":{"display_name":"Sword quest","classification":"SCOPED_EVIDENCE_ONLY"}}),true,true).await;
    let (status, body) = db.get("/en/wiki/items?category=weapon&q=Sword", true).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Sword 23"));
    assert!(!body.contains("Sword 24"));
    assert!(body.contains("Next page"));
    let (_, body) = db
        .get("/en/wiki/items?category=weapon&q=Sword&page=2", true)
        .await;
    assert!(body.contains("Sword 24"));
    assert!(!body.contains("Next page"));
    assert!(
        !db.get("/en/wiki/items?category=food", true)
            .await
            .1
            .contains("Sword 00")
    );
    let (status, body) = db
        .get(
            "/en/wiki/search?q=quest&classification=SCOPED_EVIDENCE_ONLY",
            true,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Sword quest"));
    assert!(!body.contains("Sword 00"));
    for query in [
        "q=x",
        "page=0",
        "page=1001",
        "page=bad",
        "kind=unknown",
        "q[]=sword",
        "unknown=x",
        "category=%00",
    ] {
        assert_eq!(
            db.get(&format!("/en/wiki/search?{query}"), true).await.0,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{query}"
        );
    }
    assert_eq!(
        db.get("/en/wiki/items?kind=quests", true).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    db.close().await;
}
#[test]
fn projection_and_input_bounds() {
    let entity = Entity {
        version_id: Uuid::now_v7(),
        kind: "items".into(),
        key: "key".into(),
        data: json!({"definition":{"semantics":{"presentation":{"state":"KNOWN","value":{"name":{"state":"KNOWN","value":"Dry grass"}}}}}}),
    };
    assert_eq!(name(&entity, "en"), "Dry grass");
    let mut hidden = entity.clone();
    hidden.data["definition"]["semantics"]["presentation"]["state"] = json!("UNKNOWN");
    assert_eq!(name(&hidden, "en"), "key");
    assert_eq!(segment("oteryn:item/a+b"), "oteryn%3Aitem%2Fa%2Bb");
    for lang in ["pl", "en"] {
        for key in KINDS.into_iter().chain(["editorial", "unknown", "invalid"]) {
            assert!(!t(lang, key).is_empty());
        }
    }
    let mut filters = Filters {
        q: "x".repeat(81),
        ..Filters::default()
    };
    assert!(filters.validate().is_err());
}

#[tokio::test]
async fn postgres_ambiguous_editorial_binding_denies() {
    let Some(db) = Database::open().await else {
        return;
    };
    db.entity("items", "test", json!({"name":"Test item"}), true, true)
        .await;
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor).execute(&db.pool).await.unwrap();
    let context = CheckContext {
        account: actor,
        require_staff_mfa: true,
        mfa_verified: true,
    };
    let repo = crate::repository::Repository::new(db.pool.clone(), Active);
    for n in 0..2 {
        let translations = ["en", "pl"].map(|locale| domain::Translation {
            locale: locale.into(),
            title: "Ambiguous description".into(),
            slug: format!("description-{n}"),
            summary: "Summary".into(),
            markdown: "Uncertain prose".into(),
        });
        let id = repo
            .create(context, &translations, Some("items:test"), "")
            .await
            .unwrap();
        repo.transition(context, id, 1, domain::Action::Submit)
            .await
            .unwrap();
        repo.transition(context, id, 2, domain::Action::Publish)
            .await
            .unwrap();
    }
    let (status, body) = db.get("/en/wiki/items/test", true).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(!body.contains("Uncertain prose"));
    db.close().await;
}

#[tokio::test]
async fn entity_refusals_are_localized_documents_with_unchanged_status() {
    for lang in ["pl", "en"] {
        for status in [
            StatusCode::NOT_FOUND,
            StatusCode::UNPROCESSABLE_ENTITY,
            StatusCode::SERVICE_UNAVAILABLE,
        ] {
            let response = error(status, lang);
            assert_eq!(response.status(), status);
            assert!(
                response.headers()["content-type"]
                    .to_str()
                    .unwrap()
                    .starts_with("text/html")
            );
            let body = String::from_utf8(
                axum::body::to_bytes(response.into_body(), 100_000)
                    .await
                    .unwrap()
                    .to_vec(),
            )
            .unwrap();
            assert!(body.starts_with("<!doctype html>"));
            assert!(body.contains(&format!("<html lang=\"{lang}\">")));
            assert!(body.contains("<title>Wiki · Oteryn</title>"));
            assert!(body.contains("<main"));
            let key = if status == StatusCode::NOT_FOUND {
                "missing"
            } else if status == StatusCode::UNPROCESSABLE_ENTITY {
                "invalid"
            } else {
                "unavailable"
            };
            assert!(body.contains(&t(lang, key)));
        }
    }
}

// Test-only publication seam. It models an accepted Item payload/publication
// profile for the Game producer's synthetic vector. It is never built into serve.
async fn publish_synthetic_item_for_test(
    db: &Database,
    candidate: &oteryn_world::catalog::Candidate,
) -> Result<(), oteryn_world::catalog::Error> {
    let entities = candidate.cached_entities(&db.pool, "item").await?;
    for entity in entities {
        assert_eq!(entity.entity_type, "item");
        assert_eq!(entity.capability_id, "item");
        db.entity("items", &entity.content_key, entity.data, true, false)
            .await;
    }
    sqlx::query("UPDATE world_active_version SET version_id=$1 WHERE singleton")
        .bind(db.version)
        .execute(&db.pool)
        .await
        .unwrap();
    Ok(())
}
#[tokio::test]
async fn postgres_native_export_cache_refuses_unknown_and_proves_synthetic_200() {
    use oteryn_world::catalog::{Error, Fence};
    let Some(db) = Database::open().await else {
        return;
    };
    sqlx::query("UPDATE world_active_version SET version_id=NULL WHERE singleton")
        .execute(&db.pool)
        .await
        .unwrap();
    let source = |fixture: &catalog_fixture::Fixture| {
        oteryn_world::catalog::verify(
            &fixture.bytes,
            &fixture.sidecar,
            &Fence {
                source_revision: fixture.text("source_revision"),
                authority_epoch: fixture.text("authority_epoch"),
                ruleset_id: fixture.text("ruleset_id"),
                content_profile_id: fixture.text("content_profile_id"),
                snapshot_id: fixture.text("snapshot_id"),
            },
        )
        .unwrap()
    };
    let unavailable = source(&catalog_fixture::negative());
    unavailable.persist_inactive(&db.pool).await.unwrap();
    assert_eq!(
        publish_synthetic_item_for_test(&db, &unavailable).await,
        Err(Error::UnsupportedCapability)
    );
    for lang in ["pl", "en"] {
        for path in ["wiki", "wiki/search", "wiki/items"] {
            assert_eq!(
                db.get(&format!("/{lang}/{path}"), true).await.0,
                StatusCode::SERVICE_UNAVAILABLE
            );
        }
    }
    let supported = source(&catalog_fixture::supported());
    supported.persist_inactive(&db.pool).await.unwrap();
    // Supported carrier caching alone still cannot activate public native data.
    assert_eq!(db.world().active_version_id().await.unwrap(), None);
    publish_synthetic_item_for_test(&db, &supported)
        .await
        .unwrap();
    let key = "oteryn:item.training_sword";
    let detail = format!("/en/wiki/items/{}", segment(key));
    assert_eq!(db.get(&detail, true).await.0, StatusCode::NOT_FOUND);
    db.world().allow("Item", key).await.unwrap();
    for lang in ["pl", "en"] {
        for path in [
            "wiki".to_owned(),
            "wiki/items".into(),
            "wiki/search?q=Training".into(),
            format!("wiki/items/{}", segment(key)),
        ] {
            let (status, body) = db.get(&format!("/{lang}/{path}"), true).await;
            assert_eq!(status, StatusCode::OK);
            assert!(body.contains("Training Sword"));
        }
    }
    db.world().deny("Item", key).await.unwrap();
    assert_eq!(db.get(&detail, true).await.0, StatusCode::NOT_FOUND);
    db.close().await;
}

#[path = "../../../world/tests/support/catalog.rs"]
mod catalog_fixture;
