use super::*;
use axum::{body::to_bytes, http::StatusCode};
use std::sync::{Arc, Mutex};
#[derive(Clone)]
struct Source(Arc<Mutex<Facts>>);
impl Evidence for Source {
    fn load<'a>(
        &'a self,
        _: Uuid,
        _: &'a mut PgConnection,
    ) -> Pin<Box<dyn Future<Output = Result<Facts, Error>> + Send + 'a>> {
        Box::pin(async move {
            self.0
                .lock()
                .map(|v| v.clone())
                .map_err(|_| Error::Unavailable)
        })
    }
}
fn facts(owner: Uuid, id: Uuid) -> Facts {
    Facts {
        badges: vec![
            ("account-age".into(), 3),
            ("loyalty".into(), 100),
            ("total-levels".into(), 500),
            ("all-professions".into(), 50),
        ],
        achievement_names: (0..6)
            .map(|i| {
                (
                    format!("oteryn:achievement/earned_{i}"),
                    (format!("Earned {i}"), format!("Zdobyte {i}")),
                )
            })
            .collect(),
        characters: vec![Character {
            id,
            owner,
            name: "<Hero>".into(),
            ownership: "generation-1".into(),
            public: true,
            earned: Some(
                (0..6)
                    .map(|i| format!("oteryn:achievement/earned_{i}"))
                    .collect(),
            ),
        }],
    }
}
#[test]
fn validation_success_refusal_and_bad_input() {
    let id = Uuid::now_v7();
    let f = facts(Uuid::now_v7(), id);
    let mut s = Selection {
        profile: Some(id),
        loyalty: Some(id),
        badges: vec!["loyalty".into()],
        achievements: (0..5)
            .map(|i| format!("oteryn:achievement/earned_{i}"))
            .collect(),
    };
    assert!(validate(&s, &f).is_ok());
    s.achievements.push("oteryn:achievement/earned_5".into());
    assert_eq!(validate(&s, &f), Err(Error::Invalid));
    s.achievements = vec!["oteryn:achievement/unearned_secret".into()];
    assert_eq!(validate(&s, &f), Err(Error::Denied));
    s.achievements = vec!["bad".into()];
    assert_eq!(validate(&s, &f), Err(Error::Invalid));
    s.achievements = vec!["oteryn:achievement/earned_0".into(); 2];
    assert_eq!(validate(&s, &f), Err(Error::Invalid));
    s.achievements.clear();
    s.loyalty = Some(Uuid::now_v7());
    assert_eq!(validate(&s, &f), Err(Error::Denied));
    s.loyalty = None;
    s.profile = None;
    s.badges = vec!["loyalty".into(); 2];
    assert_eq!(validate(&s, &f), Err(Error::Invalid));
    s.badges = vec!["loyalty".into()];
    assert_eq!(validate(&s, &Facts::default()), Err(Error::Unavailable));
}
#[tokio::test]
async fn disabled_anonymous_bad_locale() -> Result<(), Box<dyn std::error::Error>> {
    let service = Service {
        pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?,
        enabled: false,
        evidence: LocalEvidence,
    };
    assert_eq!(
        http::show(&service, None, "en", Ok(String::new()))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        service.save(Uuid::now_v7(), &Selection::default()).await,
        Err(Error::Disabled)
    );
    let service = Service {
        enabled: true,
        ..service
    };
    assert_eq!(
        http::show(&service, None, "en", Ok(String::new()))
            .await
            .status(),
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        http::show(&service, None, "de", Ok(String::new()))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert!(matches!(
        service.owner(Uuid::nil()).await,
        Err(Error::Denied)
    ));
    Ok(())
}
#[tokio::test]
async fn postgres_selections_privacy_and_http() -> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping PostgreSQL badges test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = sqlx::postgres::PgPoolOptions::new().connect(&url).await?;
    let schema = format!("badges_test_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let path = schema.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
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
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105155256_t03_accounts.sql"
    ))
    .execute(&pool)
    .await?;
    sqlx::raw_sql(include_str!(
        "../../../../migrations/20261105231203_t54_badge_selections.sql"
    ))
    .execute(&pool)
    .await?;
    let account = Uuid::now_v7();
    let other = Uuid::now_v7();
    let id = Uuid::now_v7();
    for a in [account, other] {
        sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale,created_at) VALUES($1,$2,'test-hash','active','en',now()-interval '3 years')").bind(a).bind(format!("{a}@example.invalid")).execute(&pool).await?;
    }
    let source = Source(Arc::new(Mutex::new(facts(account, id))));
    let service = Service {
        pool: pool.clone(),
        enabled: true,
        evidence: source.clone(),
    };
    assert_eq!(service.public(account, id).await?, PublicView::default());
    let selection = Selection {
        badges: vec!["account-age".into()],
        profile: Some(id),
        loyalty: Some(id),
        achievements: (0..5)
            .map(|i| format!("oteryn:achievement/earned_{i}"))
            .collect(),
    };
    service.save(account, &selection).await?;
    assert_eq!(service.owner(account).await?.selection, selection);
    assert_eq!(service.public(account, id).await?.achievements.len(), 5);
    assert_eq!(service.loyalty_character(account).await?, Some(id));
    assert!(matches!(service.owner(other).await, Err(Error::Denied)));
    let mut bad = selection.clone();
    bad.achievements.push("oteryn:achievement/earned_5".into());
    assert_eq!(service.save(account, &bad).await, Err(Error::Invalid));
    bad.achievements = vec!["oteryn:achievement/unearned_secret".into()];
    assert_eq!(service.save(account, &bad).await, Err(Error::Denied));
    assert_eq!(service.owner(account).await?.selection, selection);
    let response = http::show(&service, Some(account), "en", Ok("csrf-safe".into())).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("&lt;Hero&gt;") || html.contains("&#60;Hero&#62;"));
    assert!(!html.contains("<Hero>"));
    assert!(!html.contains("unearned_secret"));
    if let Ok(path) = std::env::var("PORTAL_BADGES_RENDER_OUTPUT") {
        std::fs::write(path, &html)?;
    }
    assert!(html.contains("maximum 5"));
    assert!(html.contains("name=\"_csrf\""));
    source.0.lock().map_err(|_| "lock")?.characters[0].public = false;
    assert_eq!(service.public(account, id).await?, PublicView::default());
    assert_eq!(service.loyalty_character(account).await?, None);
    assert_eq!(service.save(account, &selection).await, Err(Error::Denied));
    {
        let mut f = source.0.lock().map_err(|_| "lock")?;
        f.characters[0].public = true;
        f.characters[0].ownership = "generation-2".into();
    }
    assert_eq!(service.public(account, id).await?, PublicView::default());
    service.save(account, &selection).await?;
    source.0.lock().map_err(|_| "lock")?.characters[0].earned = None;
    assert_eq!(service.public(account, id).await?, PublicView::default());
    assert_eq!(
        service.save(account, &selection).await,
        Err(Error::Unavailable)
    );
    let local = Service {
        pool: pool.clone(),
        enabled: true,
        evidence: LocalEvidence,
    };
    assert_eq!(
        local.owner(account).await?.facts.badges,
        vec![("account-age".into(), 3)]
    );
    assert_eq!(
        http::save(
            &local,
            Some(account),
            "pl",
            vec![("badge".into(), "account-age".into())]
        )
        .await
        .status(),
        StatusCode::SEE_OTHER
    );
    sqlx::query("UPDATE accounts SET status='suspended' WHERE id=$1")
        .bind(account)
        .execute(&pool)
        .await?;
    assert!(matches!(service.owner(account).await, Err(Error::Denied)));
    assert_eq!(
        service.save(Uuid::now_v7(), &selection).await,
        Err(Error::Denied)
    );
    let constraint=sqlx::query("UPDATE account_badge_selections SET achievement_keys=ARRAY['a','b','c','d','e','f'] WHERE account_id=$1").bind(account).execute(&pool).await;
    assert!(constraint.is_err());
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    admin.close().await;
    Ok(())
}
