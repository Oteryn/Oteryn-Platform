#![allow(clippy::unwrap_used)]

use super::*;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use oteryn_world::{WorldOptions, WorldStore};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// SOURCE: Oteryn/Oteryn-Game@857f15f376907c592bb28d854fccff18a15cdf87,
// rulesets/character/experience/experience-table.json: levels 1,2,3 = 0,100,200.
// The rest and terminal bound are synthetic, not a copied or published game table.
fn table() -> Value {
    json!({
        "schema": "OTERYN_GAME_CHARACTER_EXPERIENCE_TABLE/v1",
        "levels": (1..=2000).map(|level| json!({"level":level,"minimum_experience": ((level-1)*100).to_string()})).collect::<Vec<_>>(),
        "terminal_exclusive_experience": "200000"
    })
}

// SOURCE: same Game commit, rulesets/training/crystal-201189f-v1.json,
// exercise_gains, percent_scale and the two dummy percentages (small excerpt).
fn rates() -> Value {
    json!({"schema_version":1, "enabled":true, "exercise_gains":[600,7,7,7,7,7,7,0],
        "percent_scale":100, "dummies":{"oteryn:item.tibia.i28558":100,"oteryn:item.tibia.i28559":110}})
}

#[test]
fn finite_experience_known_values_and_refusals() {
    assert_eq!(experience_to_level(&table(), 75, 2), Ok(25));
    assert_eq!(experience_to_level(&table(), 100, 3), Ok(100));
    assert_eq!(experience_to_level(&table(), 200, 3), Ok(0));
    for (current, target) in [(0, 0), (0, 2001), (200, 2), (200000, 2000), (u64::MAX, 1)] {
        assert_eq!(
            experience_to_level(&table(), current, target),
            Err(CalculationError::BadInput)
        );
    }
    for pointer in [
        "/levels/1/minimum_experience",
        "/terminal_exclusive_experience",
    ] {
        let mut data = table();
        *data.pointer_mut(pointer).unwrap() = json!("0");
        assert_eq!(
            experience_to_level(&data, 0, 2),
            Err(CalculationError::Unavailable)
        );
    }
    let mut data = table();
    data["levels"].as_array_mut().unwrap().pop();
    assert_eq!(
        experience_to_level(&data, 0, 2),
        Err(CalculationError::Unavailable)
    );
    assert_eq!(
        experience_to_level(&json!({}), 0, 2),
        Err(CalculationError::Unavailable)
    );
}

#[test]
fn stamina_matches_game_boundary_vectors() {
    // SOURCE: same Game commit, crates/gameplay/characters/src/stamina.rs tests.
    for (start, elapsed, expected) in [
        (0, 599, 0),
        (0, 600, 0),
        (0, 779, 0),
        (0, 780, 1),
        (2339, 780, 2340),
        (2339, 1139, 2340),
        (2339, 1140, 2341),
        (2340, 959, 2340),
        (2340, 960, 2341),
        (2519, 960, 2520),
        (0, 486600, 2520),
        (0, u64::MAX, 2520),
        (2520, 0, 2520),
    ] {
        assert_eq!(stamina_regeneration(start, elapsed), Ok(expected));
    }
    assert_eq!(
        stamina_regeneration(2521, 0),
        Err(CalculationError::BadInput)
    );
}

#[test]
fn training_uses_world_gains_and_integer_rounding() {
    let public = "oteryn:item.tibia.i28558";
    let house = "oteryn:item.tibia.i28559";
    assert_eq!(skill_training_time(&rates(), 601, 0, public), Ok((4, 2)));
    assert_eq!(skill_training_time(&rates(), 660, 0, house), Ok((2, 1)));
    // Game truncates 7.7 to 7 per charge; it does not accumulate a fractional try.
    assert_eq!(skill_training_time(&rates(), 8, 1, house), Ok((4, 2)));
    assert_eq!(skill_training_time(&rates(), 0, 1, public), Ok((0, 0)));
    let mut changed = rates();
    changed["exercise_gains"][0] = json!(300);
    assert_eq!(skill_training_time(&changed, 601, 0, public), Ok((6, 3)));
    for (family, dummy) in [(7, public), (0, "unknown")] {
        assert_eq!(
            skill_training_time(&rates(), 1, family, dummy),
            Err(CalculationError::BadInput)
        );
    }
    changed["enabled"] = json!(false);
    assert_eq!(
        skill_training_time(&changed, 1, 0, public),
        Err(CalculationError::Unavailable)
    );
    changed = rates();
    changed["percent_scale"] = json!(0);
    assert_eq!(
        skill_training_time(&changed, 1, 0, public),
        Err(CalculationError::Unavailable)
    );
    assert_eq!(
        skill_training_time(&json!({}), 1, 0, public),
        Err(CalculationError::Unavailable)
    );
}

#[test]
fn loot_conserves_indivisible_units_and_rejects_invalid_parties() {
    for (total, members) in [(0, 1), (10, 3), (i64::MAX as u64, 1000)] {
        let result = loot_split(total, members).unwrap();
        assert_eq!(result.per_member * members + result.remainder, total);
        assert!(result.remainder < members);
    }
    assert_eq!(
        loot_split(10, 3),
        Ok(LootSplit {
            per_member: 3,
            remainder: 1
        })
    );
    for (total, members) in [(10, 0), (10, 1001), (u64::MAX, 1)] {
        assert_eq!(loot_split(total, members), Err(CalculationError::BadInput));
    }
}

fn lazy_router(enabled: bool) -> Router {
    router(
        WorldStore::new(
            PgPoolOptions::new()
                .connect_lazy("postgresql://localhost/unreachable")
                .unwrap(),
            WorldOptions::default(),
        ),
        CalculatorOptions {
            enabled,
            ..Default::default()
        },
    )
}

async fn request(app: Router, path: &str) -> (StatusCode, String) {
    let response = app
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 100_000).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn forms_work_in_both_languages_without_javascript() {
    for (lang, title, invalid, unavailable) in [
        (
            "pl",
            "Kalkulatory",
            "Nieprawidłowe dane",
            "Dane świata są niedostępne",
        ),
        (
            "en",
            "Calculators",
            "Invalid input",
            "World data is unavailable",
        ),
    ] {
        let base = format!("/{lang}/wiki/calculators");
        let (status, html) = request(lazy_router(true), &base).await;
        assert_eq!(status, StatusCode::OK);
        assert!(html.contains(title));
        assert!(html.contains("width=device-width"));
        assert_eq!(html.matches("<form method=\"get\"").count(), 4);
        assert!(!html.contains("<script"));
        let (status, html) = request(
            lazy_router(true),
            &format!("{base}?calculator=loot&total=10&members=3"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(html.contains("<output>3; 1</output>"));
        let (status, html) = request(
            lazy_router(true),
            &format!("{base}?calculator=stamina&current=0&offline=780"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(html.contains("<output>1</output>"));
        for query in [
            "calculator=loot&total=10&members=0",
            "calculator=loot&total=-1&members=2",
            "calculator=stamina&current=2521&offline=10",
            "calculator=loot&total=1.5&members=2",
            "calculator=loot&total=10&members=1&members=2",
            "calculator=unknown",
            "unknown=1",
        ] {
            let (status, html) = request(lazy_router(true), &format!("{base}?{query}")).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
            assert!(html.contains(invalid));
        }
        let (status, html) = request(
            lazy_router(true),
            &format!("{base}?calculator=experience&current=0&target=2"),
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(html.contains(unavailable));
        let (status, html) = request(
            lazy_router(true),
            &format!("{base}?calculator=training&remaining=1&family=0&dummy=%22%3E%3Cscript%3E"),
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(!html.contains("<script>"));
    }
    assert_eq!(
        request(lazy_router(false), "/en/wiki/calculators").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(lazy_router(true), "/de/wiki/calculators").await.0,
        StatusCode::NOT_FOUND
    );
}

struct Database {
    admin: PgPool,
    pool: PgPool,
    schema: String,
}
impl Database {
    async fn new() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let url = match std::env::var("PORTAL_TEST_DATABASE_URL") {
            Ok(url) => url,
            Err(std::env::VarError::NotPresent) => {
                eprintln!("SKIPPED PostgreSQL calculator test: PORTAL_TEST_DATABASE_URL is unset");
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        };
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&admin)
            .await?;
        assert_eq!(version, "180006");
        let schema = format!("calculators_test_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await?;
        let path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .after_connect(move |conn, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path', $1, false)")
                        .bind(path)
                        .execute(conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        Ok(Some(Self {
            admin,
            pool,
            schema,
        }))
    }
    fn app(&self, world_enabled: bool) -> Router {
        router(
            WorldStore::new(
                self.pool.clone(),
                WorldOptions {
                    enabled: world_enabled,
                },
            ),
            CalculatorOptions {
                enabled: true,
                experience_table: Some(WorldKey {
                    kind: "test_policy".into(),
                    key: "experience".into(),
                }),
                training_rates: Some(WorldKey {
                    kind: "test_policy".into(),
                    key: "training".into(),
                }),
            },
        )
    }
    async fn seed(&self) -> TestResult {
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO world_versions (id, source_commit, status) VALUES ($1, $2, 'ready')",
        )
        .bind(id)
        .bind("a".repeat(40))
        .execute(&self.pool)
        .await?;
        for (key, data) in [("experience", table()), ("training", rates())] {
            sqlx::query("INSERT INTO world_entities (version_id, kind, key, data, publishable) VALUES ($1, 'test_policy', $2, $3, true)")
                .bind(id).bind(key).bind(data).execute(&self.pool).await?;
        }
        sqlx::query("UPDATE world_active_version SET version_id = $1 WHERE singleton")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    async fn allow(&self) -> TestResult {
        let world = WorldStore::new(self.pool.clone(), WorldOptions { enabled: true });
        world.allow("test_policy", "experience").await?;
        world.allow("test_policy", "training").await?;
        Ok(())
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
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
async fn postgres_approved_world_records_drive_results() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.seed().await?;
    db.allow().await?;
    for (query, output) in [
        (
            "calculator=experience&current=75&target=2",
            "<output>25</output>",
        ),
        (
            "calculator=training&remaining=601&family=0&dummy=oteryn:item.tibia.i28558",
            "<output>4; 2</output>",
        ),
    ] {
        let (status, html) = request(db.app(true), &format!("/en/wiki/calculators?{query}")).await;
        assert_eq!(status, StatusCode::OK);
        assert!(html.contains(output));
        assert!(html.contains("World version"));
    }
    db.close().await
}

#[tokio::test]
async fn postgres_missing_hidden_and_disabled_world_deny() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    let path = "/en/wiki/calculators?calculator=experience&current=0&target=2";
    assert_eq!(
        request(db.app(true), path).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    db.seed().await?;
    assert_eq!(
        request(db.app(true), path).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    db.allow().await?;
    assert_eq!(
        request(db.app(false), path).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let world = WorldStore::new(db.pool.clone(), WorldOptions { enabled: true });
    world.deny("test_policy", "experience").await?;
    assert_eq!(
        request(db.app(true), path).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    db.close().await
}

#[tokio::test]
async fn postgres_malformed_world_and_bad_request_are_refused() -> TestResult {
    let Some(db) = Database::new().await? else {
        return Ok(());
    };
    db.seed().await?;
    db.allow().await?;
    assert_eq!(
        request(
            db.app(true),
            "/en/wiki/calculators?calculator=experience&current=200&target=2"
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    sqlx::query("UPDATE world_entities SET data = '{}'::jsonb WHERE kind = 'test_policy'")
        .execute(&db.pool)
        .await?;
    assert_eq!(
        request(
            db.app(true),
            "/en/wiki/calculators?calculator=experience&current=0&target=2"
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    db.close().await
}
