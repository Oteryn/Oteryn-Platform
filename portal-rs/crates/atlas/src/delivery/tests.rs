#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use serde_json::json;
fn snapshot() -> Snapshot {
    let version = Uuid::now_v7();
    let records = vec![
        ("Item", "item:ore", json!({})),
        (
            "Creature",
            "creature:rat",
            json!({"loot":{"key":"loot:rat"},"level":5}),
        ),
        (
            "Loot",
            "loot:rat",
            json!({"algorithm":"IndependentBernoulliPpm","entries":[{"item":{"key":"item:ore"},"probability_ppm":125000,"min_count":2,"max_count":2}]}),
        ),
        (
            "Spawn.Source",
            "spawn:one",
            json!({"city":"Town","points":[{"cell":{"x":10,"y":20,"floor":7},"creature":"creature:rat","respawn_ms":60000}]}),
        ),
        (
            "Quest",
            "quest:ore",
            json!({"stages":[{"key":"s1","kind":"collect","item":{"key":"item:ore"},"count":10},{"key":"s2","kind":"item","item_resolution":"ambiguous","objective":"ore"}]}),
        ),
    ];
    Snapshot::from_visible(
        version,
        records
            .into_iter()
            .map(|(kind, key, data)| Entity {
                version_id: version,
                kind: kind.into(),
                key: key.into(),
                data,
            })
            .collect(),
    )
    .unwrap()
}
fn filters() -> Filters {
    Filters {
        item: Some("item:ore".into()),
        count: Some(10),
        ..Filters::default()
    }
}
#[test]
fn discovery_joins_and_filters() {
    let s = snapshot();
    let d = s.discover(Options { enabled: true }, &filters()).unwrap();
    assert_eq!(d.sources.len(), 1);
    let src = &d.sources[0];
    assert_eq!(src.tier, Some(1));
    assert_eq!(src.spawns[0].floor, 7);
    assert_eq!(src.estimate.as_ref().unwrap().expected_kills, 40.0);
    assert!(d.requirements[1].item.is_none());
    for f in [
        Filters {
            floor: Some(8),
            ..filters()
        },
        Filters {
            level: Some(4),
            ..filters()
        },
        Filters {
            city: Some("Other".into()),
            ..filters()
        },
        Filters {
            rarity: Some(1),
            ..filters()
        },
        Filters {
            q: Some("other".into()),
            ..filters()
        },
    ] {
        assert!(
            s.discover(Options { enabled: true }, &f)
                .unwrap()
                .sources
                .is_empty()
        );
    }
}
#[test]
fn refusal_and_bad_input() {
    let s = snapshot();
    assert_eq!(
        s.discover(Options::default(), &filters()).unwrap_err(),
        Error::Disabled
    );
    assert_eq!(
        s.discover(
            Options { enabled: true },
            &Filters {
                count: Some(0),
                ..filters()
            }
        )
        .unwrap_err(),
        Error::InvalidInput
    );
    assert_eq!(
        s.discover(
            Options { enabled: true },
            &Filters {
                item: Some("hidden".into()),
                ..filters()
            }
        )
        .unwrap_err(),
        Error::Unavailable
    );
    assert!(wiki_map_link("xx", "items", "ore").is_err());
    assert_eq!(
        wiki_map_link("en", "items", "a&b").unwrap(),
        "/en/atlas/delivery?item=a%26b"
    );
    assert!(
        Snapshot::from_visible(
            s.version,
            vec![Entity {
                version_id: Uuid::now_v7(),
                kind: "Item".into(),
                key: "x".into(),
                data: json!({})
            }]
        )
        .is_err()
    );
}
#[test]
fn uncertain_quantities_and_missing_relations_deny() {
    let mut s = snapshot();
    s.entities
        .get_mut(&("Loot".into(), "loot:rat".into()))
        .unwrap()["entries"][0]["max_count"] = json!(3);
    assert!(
        s.discover(Options { enabled: true }, &filters())
            .unwrap()
            .sources[0]
            .estimate
            .is_none()
    );
    s.entities.remove(&("Loot".into(), "loot:rat".into()));
    assert!(
        s.discover(Options { enabled: true }, &filters())
            .unwrap()
            .sources
            .is_empty()
    );
}
// SOURCE: Oteryn/Oteryn-Atlas tests/farm-intelligence.mjs, revision 0bbd188.
// Independent convolution oracle and numeric regression vectors ported to Rust.
fn oracle(required: usize, p: f64, c: f64) -> u64 {
    let mut d = vec![1.0];
    for k in 1..=1000 {
        let mut next = vec![0.0; d.len() + 1];
        for (i, m) in d.iter().enumerate() {
            next[i] += m * (1.0 - p);
            next[i + 1] += m * p;
        }
        d = next;
        if d.iter().skip(required).sum::<f64>() + 1e-12 >= c {
            return k;
        }
    }
    panic!("oracle envelope exceeded")
}
#[test]
fn bernoulli_independent_oracle() {
    for (required, ppm) in [(5, 125000), (1, 500000), (3, 750000)] {
        let e = estimate::fixed(required * 2, 2, ppm).unwrap();
        for (actual, c) in [(e.p50_kills, 0.5), (e.p80_kills, 0.8), (e.p95_kills, 0.95)] {
            assert_eq!(
                actual,
                oracle(required as usize, f64::from(ppm) / 1_000_000.0, c)
            );
        }
    }
}
#[test]
fn numeric_extremes_and_envelopes() {
    use estimate::*;
    assert_eq!(fixed(1, 1, 0), Err(EstimateError::Unreachable));
    let e = fixed(7, 3, 1_000_000).unwrap();
    assert_eq!(
        (e.expected_kills, e.p50_kills, e.p80_kills, e.p95_kills),
        (3.0, 3, 3, 3)
    );
    let e = fixed(1, 1, 100).unwrap();
    assert_eq!(
        e.p95_kills,
        ((1.0_f64 - 0.95).ln() / (-0.0001_f64).ln_1p()).ceil() as u64
    );
    let e = fixed(10_000, 1, 999_999).unwrap();
    assert!((e.expected_kills - 10_000.0 / 0.999999).abs() < 1e-9);
    assert_eq!(
        (e.p50_kills, e.p80_kills, e.p95_kills),
        (10_000, 10_000, 10_000)
    );
    for (t, q, p) in [(0, 1, 1), (100001, 1, 1), (100000, 1, 1), (1, 1, 1000001)] {
        assert!(fixed(t, q, p).is_err());
    }
    let e = exact_pmf(
        3,
        &[
            Outcome {
                quantity: 0,
                probability_ppm: 500000,
            },
            Outcome {
                quantity: 2,
                probability_ppm: 500000,
            },
        ],
    )
    .unwrap();
    assert_eq!(
        (e.expected_kills, e.p50_kills, e.p80_kills, e.p95_kills),
        (4.0, 3, 5, 8)
    );
    assert_eq!(
        exact_pmf(
            4,
            &[Outcome {
                quantity: 0,
                probability_ppm: 1_000_000
            }]
        ),
        Err(EstimateError::Unreachable)
    );
    assert!(
        exact_pmf(
            1,
            &[Outcome {
                quantity: 1,
                probability_ppm: 1
            }]
        )
        .is_err()
    );
    assert!(exact_pmf(5001, &[]).is_err());
    assert!(
        exact_pmf(
            1,
            &[Outcome {
                quantity: 1,
                probability_ppm: 500000
            }; 2]
        )
        .is_err()
    );
}
#[tokio::test]
async fn http_disabled_and_bad_filters() {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/test")
        .unwrap();
    for (enabled, path, status) in [
        (false, "/en/atlas/delivery", 404),
        (true, "/pl/atlas/delivery?count=no", 400),
        (true, "/en/atlas/delivery?unknown=1", 400),
    ] {
        let response = router(
            pool.clone(),
            Options { enabled },
            oteryn_world::WorldOptions::default(),
        )
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
        assert_eq!(response.status().as_u16(), status);
    }
}

async fn database() -> Option<(sqlx::PgPool, String)> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping delivery PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return None;
    };
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    let schema = format!("delivery_{}", Uuid::now_v7().simple());
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "CREATE SCHEMA {schema}; SET search_path TO {schema}"
    )))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105151225_t14_world_store.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let s = snapshot();
    sqlx::query("INSERT INTO world_versions(id,source_commit,status) VALUES ($1,$2,'ready')")
        .bind(s.version)
        .bind("a".repeat(40))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE world_active_version SET version_id=$1")
        .bind(s.version)
        .execute(&pool)
        .await
        .unwrap();
    for ((kind, key), data) in s.entities {
        // Match actual Game record envelopes, with synthetic content only.
        let data = if kind == "Spawn.Source" {
            json!({"declaration":data})
        } else {
            json!({"definition":data})
        };
        sqlx::query("INSERT INTO world_entities VALUES ($1,$2,$3,$4,true)")
            .bind(s.version)
            .bind(&kind)
            .bind(&key)
            .bind(data)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO world_visibility(kind,key,allowed) VALUES ($1,$2,true)")
            .bind(kind)
            .bind(key)
            .execute(&pool)
            .await
            .unwrap();
    }
    Some((pool, schema))
}
async fn cleanup(pool: sqlx::PgPool, schema: String) {
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}
#[tokio::test]
async fn postgres_http_success() {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;
    let Some((pool, schema)) = database().await else {
        return;
    };
    for lang in ["en", "pl"] {
        let response=router(pool.clone(),Options{enabled:true},oteryn_world::WorldOptions{enabled:true}).oneshot(Request::builder().uri(format!("/{lang}/atlas/delivery?item=item%3Aore&count=10&level=&floor=&city=&rarity=&q=&creature=")).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), 200);
        let body = String::from_utf8(
            to_bytes(response.into_body(), 1_000_000)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        if let Ok(dir) = std::env::var("PORTAL_DELIVERY_PREVIEW_DIR") {
            let dir = std::path::Path::new(&dir);
            std::fs::create_dir_all(dir.join("atlas")).unwrap();
            std::fs::write(dir.join(format!("{lang}.html")), &body).unwrap();
            let response = router(
                pool.clone(),
                Options { enabled: true },
                oteryn_world::WorldOptions { enabled: true },
            )
            .oneshot(
                Request::builder()
                    .uri("/atlas/delivery.css")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
            std::fs::write(
                dir.join("atlas/delivery.css"),
                to_bytes(response.into_body(), 100_000).await.unwrap(),
            )
            .unwrap();
        }
        assert!(body.contains("40.00"));
        assert!(body.contains("/atlas/7/10/20"));
        assert!(body.contains("P95:"));
        assert!(body.contains(&format!("lang=\"{lang}\"")));
    }
    cleanup(pool, schema).await;
}
#[tokio::test]
async fn postgres_hidden_source_refused() {
    let Some((pool, schema)) = database().await else {
        return;
    };
    sqlx::query("UPDATE world_visibility SET allowed=false WHERE kind='Loot'")
        .execute(&pool)
        .await
        .unwrap();
    let store = WorldStore::new(pool.clone(), oteryn_world::WorldOptions { enabled: true });
    let s = load(&store).await.unwrap();
    assert!(
        s.discover(Options { enabled: true }, &filters())
            .unwrap()
            .sources
            .is_empty()
    );
    cleanup(pool, schema).await;
}
#[tokio::test]
async fn postgres_bad_item_and_disabled_world() {
    let Some((pool, schema)) = database().await else {
        return;
    };
    let store = WorldStore::new(pool.clone(), oteryn_world::WorldOptions { enabled: true });
    let s = load(&store).await.unwrap();
    assert_eq!(
        s.discover(
            Options { enabled: true },
            &Filters {
                item: Some("missing".into()),
                ..filters()
            }
        )
        .unwrap_err(),
        Error::Unavailable
    );
    assert_eq!(
        load(&WorldStore::new(
            pool.clone(),
            oteryn_world::WorldOptions::default()
        ))
        .await
        .unwrap_err(),
        Error::Unavailable
    );
    cleanup(pool, schema).await;
}

#[test]
fn recipe_targets_preserve_resolution_without_name_matching() {
    let mut s = snapshot();
    s.entities.insert(("Quest".into(),"quest:ore".into()),json!({"oteryn_recipe":{"payload":{"recipe":{"stages":[{"kind":"collect","key":"s1","count":2,"target_items":[{"item":{"key":"item:ore"}},{"name":"ore","item_resolution":"ambiguous"},{"item":{"key":"item:ore"},"item_resolution":"unresolved"}]}]}}}}));
    let r = s.requirements();
    assert_eq!(r.len(), 3);
    assert_eq!(r[0].item.as_deref(), Some("item:ore"));
    assert!(r[1].item.is_none());
    assert!(r[2].item.is_none());
    assert!(s.creature_items("creature:rat").contains("item:ore"));
    assert!(s.creature_items("unknown").is_empty());
}
#[test]
fn original_rational_extremes() {
    let e = estimate::fixed_rational(1, 1, 7, 2_000_000_000).unwrap();
    assert_eq!(
        e.p95_kills,
        ((1.0_f64 - 0.95).ln() / (-(7.0_f64 / 2_000_000_000.0)).ln_1p()).ceil() as u64
    );
    assert_eq!(
        estimate::fixed_rational(1, 1, 1, 3).unwrap().expected_kills,
        3.0
    );
    assert!(estimate::fixed_rational(1, 1, 1, 0).is_err());
}
