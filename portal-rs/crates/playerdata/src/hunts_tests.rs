use super::*;

fn fixture() -> Session {
    Session {
        session_id: Uuid::now_v7(),
        character_id: Uuid::now_v7(),
        started_at: 100,
        ended_at: 160,
        duration_seconds: 60,
        experience_gained: 100,
        experience_lost: 5,
        places: BTreeMap::from([("place:cave".into(), 60)]),
        loot: BTreeMap::from([("item:ruby".into(), 3)]),
        supplies: BTreeMap::new(),
        damage_dealt: vec![Damage {
            damage_type_key: "damage:physical".into(),
            source_kind: Source::SelfActor,
            source_key: None,
            amount: 10,
        }],
        damage_taken: vec![],
        healing_done: 1,
        healing_received: 2,
        monsters_killed: BTreeMap::from([("monster:rat".into(), 1)]),
        party: None,
    }
}
struct Prices;
impl Pricing for Prices {
    fn quote(&self, _: &str, _: Uuid) -> Option<Quote> {
        Some(Quote {
            unit_gold: 101,
            provenance: "fixture:npc-v1".into(),
        })
    }
}
#[test]
fn rates_valuation_and_unknown_prices() -> Result<(), Error> {
    let mut s = fixture();
    s.validate()?;
    assert_eq!(s.experience_per_hour(), Some(6000));
    assert_eq!(valuation(&s, &Prices).map(|v| v.balance), Some(303));
    assert!(valuation(&s, &Unpriced).is_none());
    s.duration_seconds = 0;
    s.places.clear();
    assert_eq!(s.experience_per_hour(), None);
    assert_eq!(valuation(&s, &Prices).and_then(|v| v.profit_per_hour), None);
    Ok(())
}
#[test]
fn bad_facts_fail_closed() {
    let mut s = fixture();
    s.experience_gained = -1;
    assert_eq!(s.validate(), Err(Error::Invalid));
    let mut s = fixture();
    s.places.clear();
    assert_eq!(s.validate(), Err(Error::Invalid));
    let mut s = fixture();
    s.damage_dealt[0].source_key = Some("player:private".into());
    assert_eq!(s.validate(), Err(Error::Invalid));
    assert!(serde_json::from_value::<Session>(serde_json::json!({"unexpected":true})).is_err());
}
#[test]
fn split_preserves_gold_and_never_duplicates_subject_loot() -> Result<(), Error> {
    let mut s = fixture();
    s.party = Some(Party {
        party_id: Uuid::now_v7(),
        subject_slot: 0,
        members: BTreeMap::from([(0, s.loot.clone()), (1, BTreeMap::new())]),
    });
    let enabled = Flags {
        read: true,
        party: true,
    };
    assert_eq!(
        split_loot(&s, enabled, &Prices)?,
        vec![Transfer {
            from_slot: 0,
            to_slot: 1,
            gold: 151
        }]
    );
    assert_eq!(
        split_loot(&s, Flags::default(), &Prices),
        Err(Error::Unavailable)
    );
    assert_eq!(split_loot(&s, enabled, &Unpriced), Err(Error::Unavailable));
    if let Some(party) = s.party.as_mut() {
        party.members.insert(0, BTreeMap::new());
    }
    assert_eq!(s.validate(), Err(Error::Invalid));
    Ok(())
}
#[test]
fn comparisons_reject_duplicates_and_filters_reject_bad_input() -> Result<(), Error> {
    let a = fixture();
    let mut b = fixture();
    b.duration_seconds = 30;
    b.places.insert("place:cave".into(), 30);
    assert_eq!(
        compare(&[a.clone(), b], &Prices)?.experience_per_hour,
        vec![Some(6000), Some(12000)]
    );
    assert_eq!(compare(&[a.clone(), a], &Prices), Err(Error::Invalid));
    assert_eq!(
        Filter {
            from: Some(10),
            until: Some(9),
            ..Filter::default()
        }
        .validate(),
        Err(Error::Invalid)
    );
    assert_eq!(
        Filter {
            place: Some("<script>".into()),
            ..Filter::default()
        }
        .validate(),
        Err(Error::Invalid)
    );
    Ok(())
}

async fn database() -> Result<Option<(sqlx::PgPool, String)>, Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("PostgreSQL hunt test skipped: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(None);
    };
    // SQL identifiers use only this generated prefix and UUID hexadecimal digits.
    let schema = format!("t73_{}", Uuid::now_v7().simple());
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&pool)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
        .execute(&pool)
        .await?;
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105230109_t73_hunts.sql"
    ))
    .execute(&pool)
    .await?;
    Ok(Some((pool, schema)))
}
async fn cleanup(pool: &sqlx::PgPool, schema: &str) -> Result<(), sqlx::Error> {
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(pool)
        .await?;
    pool.close().await;
    Ok(())
}
#[tokio::test]
async fn postgres_owner_list_detail_filters_and_refusal() -> Result<(), Box<dyn std::error::Error>>
{
    let Some((pool, schema)) = database().await? else {
        return Ok(());
    };
    let repo = store::Repository::new(
        pool.clone(),
        Flags {
            read: true,
            party: false,
        },
    );
    let owner = Uuid::now_v7();
    let other = Uuid::now_v7();
    let permit = Permit::from_verified_journal(owner, Uuid::now_v7(), 1, 200)?;
    let s = fixture();
    repo.project(&permit, &s, 300).await?;
    repo.project(&permit, &s, 300).await?;
    assert_eq!(repo.detail(owner, 200, s.session_id).await?, s);
    assert_eq!(
        repo.detail(other, 200, s.session_id).await,
        Err(Error::NotFound)
    );
    assert!(repo.list(other, 200, &Filter::default()).await?.is_empty());
    assert_eq!(repo.list(owner, 200, &Filter::default()).await?.len(), 1);
    assert!(
        repo.list(
            owner,
            200,
            &Filter {
                place: Some("place:other".into()),
                ..Filter::default()
            }
        )
        .await?
        .is_empty()
    );
    assert!(
        repo.list(
            owner,
            200,
            &Filter {
                party: Some(true),
                ..Filter::default()
            }
        )
        .await?
        .is_empty()
    );
    assert!(
        repo.list(
            owner,
            200,
            &Filter {
                from: Some(101),
                ..Filter::default()
            }
        )
        .await?
        .is_empty()
    );
    assert!(
        repo.list(
            owner,
            200,
            &Filter {
                character: Some(Uuid::now_v7()),
                ..Filter::default()
            }
        )
        .await?
        .is_empty()
    );
    assert_eq!(
        repo.detail(owner, 300, s.session_id).await,
        Err(Error::NotFound)
    );
    cleanup(&pool, &schema).await?;
    Ok(())
}
#[tokio::test]
async fn postgres_conflicts_revocation_and_missing_evidence_deny()
-> Result<(), Box<dyn std::error::Error>> {
    let Some((pool, schema)) = database().await? else {
        return Ok(());
    };
    let repo = store::Repository::new(
        pool.clone(),
        Flags {
            read: true,
            party: false,
        },
    );
    let owner = Uuid::now_v7();
    let permit = Permit::from_verified_journal(owner, Uuid::now_v7(), 1, 200)?;
    let s = fixture();
    repo.project(&permit, &s, 300).await?;
    let mut changed = s.clone();
    changed.experience_gained += 1;
    assert_eq!(
        repo.project(&permit, &changed, 300).await,
        Err(Error::Invalid)
    );
    assert_eq!(
        repo.detail(owner, 200, s.session_id).await,
        Err(Error::NotFound)
    );
    // A separate session isolates the completeness and revision fences.
    let s = fixture();
    repo.project(&permit, &s, 300).await?;
    sqlx::query("UPDATE pd_hunt_access SET complete=false")
        .execute(&pool)
        .await?;
    assert_eq!(
        repo.detail(owner, 200, s.session_id).await,
        Err(Error::NotFound)
    );
    sqlx::query("UPDATE pd_hunt_access SET complete=true, character_revision=2")
        .execute(&pool)
        .await?;
    assert_eq!(
        repo.detail(owner, 200, s.session_id).await,
        Err(Error::NotFound)
    );
    sqlx::query("UPDATE pd_hunt_access SET character_revision=1")
        .execute(&pool)
        .await?;
    repo.revoke(owner, s.session_id).await?;
    repo.project(&permit, &s, 300).await?;
    assert_eq!(
        repo.detail(owner, 200, s.session_id).await,
        Err(Error::NotFound)
    );
    assert!(repo.list(owner, 200, &Filter::default()).await?.is_empty());
    cleanup(&pool, &schema).await?;
    Ok(())
}
#[tokio::test]
async fn postgres_disabled_and_bad_input_do_not_persist() -> Result<(), Box<dyn std::error::Error>>
{
    let Some((pool, schema)) = database().await? else {
        return Ok(());
    };
    let permit = Permit::from_verified_journal(Uuid::now_v7(), Uuid::now_v7(), 1, 200)?;
    let mut s = fixture();
    let disabled = store::Repository::new(pool.clone(), Flags::default());
    assert_eq!(
        disabled.project(&permit, &s, 300).await,
        Err(Error::Unavailable)
    );
    assert_eq!(
        disabled.list(permit.account, 200, &Filter::default()).await,
        Err(Error::Unavailable)
    );
    let repo = store::Repository::new(
        pool.clone(),
        Flags {
            read: true,
            party: false,
        },
    );
    s.duration_seconds = -1;
    assert_eq!(repo.project(&permit, &s, 300).await, Err(Error::Invalid));
    assert!(
        repo.list(permit.account, 200, &Filter::default())
            .await?
            .is_empty()
    );
    cleanup(&pool, &schema).await?;
    Ok(())
}

#[tokio::test]
async fn default_off_http_denies_without_database() -> Result<(), Box<dyn std::error::Error>> {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let pool =
        sqlx::postgres::PgPoolOptions::new().connect_lazy("postgres://localhost/unreachable")?;
    let router = web::router(pool, Flags::default(), std::sync::Arc::new(Unpriced));
    for path in [
        "/en/account/hunts",
        "/en/account/hunts/compare?ids=bad",
        "/en/account/hunts/bad/share",
    ] {
        let response = router
            .clone()
            .oneshot(Request::get(path).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.headers()["cache-control"], "private, no-store");
    }
    Ok(())
}

#[tokio::test]
async fn postgres_http_owner_details_comparison_and_bad_query()
-> Result<(), Box<dyn std::error::Error>> {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use sha2::{Digest, Sha256};
    use tower::ServiceExt;
    let Some((pool, schema)) = database().await? else {
        return Ok(());
    };
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105144852_t02_web_sessions.sql"
    ))
    .execute(&pool)
    .await?;
    let owner = Uuid::now_v7();
    let outsider = Uuid::now_v7();
    let flags = Flags {
        read: true,
        party: false,
    };
    let repo = store::Repository::new(pool.clone(), flags);
    let current = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    )?;
    let permit = Permit::from_verified_journal(owner, Uuid::now_v7(), 1, current)?;
    let mut a = fixture();
    a.started_at = current - 120;
    a.ended_at = current - 60;
    let mut b = a.clone();
    b.session_id = Uuid::now_v7();
    repo.project(&permit, &a, current + 60).await?;
    repo.project(&permit, &b, current + 60).await?;
    for (token, account) in [("a".repeat(64), owner), ("b".repeat(64), outsider)] {
        sqlx::query("INSERT INTO web_sessions (id_hash,account_id,csrf_token,user_agent_summary,idle_expires_at,absolute_expires_at) VALUES ($1,$2,$3,'test',now()+interval '1 hour',now()+interval '1 hour')")
            .bind(Sha256::digest(token.as_bytes()).to_vec()).bind(account).bind("c".repeat(64)).execute(&pool).await?;
    }
    let router = web::router(pool.clone(), flags, std::sync::Arc::new(Prices));
    let cookie = format!("{}={}", oteryn_web::session::COOKIE_NAME, "a".repeat(64));
    let other_cookie = format!("{}={}", oteryn_web::session::COOKIE_NAME, "b".repeat(64));
    let response = router
        .clone()
        .oneshot(Request::get("/en/account/hunts").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    for path in [
        "/en/account/hunts?character=&place=&from=&until=&party=".to_owned(),
        format!("/en/account/hunts/{}", a.session_id),
        format!(
            "/pl/account/hunts/compare?ids={},{}",
            a.session_id, b.session_id
        ),
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::get(path)
                    .header("cookie", &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        let html = String::from_utf8(to_bytes(response.into_body(), 1024 * 1024).await?.to_vec())?;
        assert!(html.contains("viewport"));
        assert!(html.contains("item:ruby"));
        assert!(!html.contains("<script"));
        assert!(
            html.contains("Hunt session analyzer") || html.contains("Analizator sesji polowania")
        );
    }
    for path in [
        format!("/en/account/hunts/{}", a.session_id),
        format!(
            "/en/account/hunts/compare?ids={},{}",
            a.session_id, b.session_id
        ),
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::get(path)
                    .header("cookie", &other_cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    for path in [
        "/en/account/hunts?unknown=1",
        "/en/account/hunts?from=bad",
        "/en/account/hunts/compare?ids=bad",
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::get(path)
                    .header("cookie", &cookie)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let response = router
        .oneshot(
            Request::post(format!("/en/account/hunts/{}/share", a.session_id))
                .header("cookie", &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    cleanup(&pool, &schema).await?;
    Ok(())
}
