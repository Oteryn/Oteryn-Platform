use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use tower::ServiceExt;
type TestResult = Result<(), Box<dyn std::error::Error>>;
struct Visible(bool);
impl Privacy for Visible {
    fn visible<'a>(&'a self, _: &'a str) -> DecisionFuture<'a> {
        Box::pin(async { Ok(self.0) })
    }
}
struct DetailedPrivacy;
impl Privacy for DetailedPrivacy {
    fn visible<'a>(&'a self, _: &'a str) -> DecisionFuture<'a> {
        Box::pin(async { Ok(true) })
    }
    fn fields<'a>(
        &'a self,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<details::Visibility, Unavailable>> + Send + 'a>> {
        Box::pin(async {
            Ok(details::Visibility {
                guild: true,
                house: true,
                skills: true,
                deaths: true,
                comment: Some("<script>private text</script>".into()),
            })
        })
    }
}
struct DetailsFixture;
impl details::PublicDetails for DetailsFixture {
    fn current<'a>(&'a self, subject: &'a details::Subject) -> details::DetailsFuture<'a> {
        Box::pin(async move {
            assert_eq!(subject.epoch, "1");
            Ok(details::Details {
                guild: Some(("Fixture Guild".into(), "Leader".into())),
                house: Some("Fixture House".into()),
                skills: Some([12, 10, 10, 10, 10, 90, 50, 10]),
                deaths: Some(vec![details::Death {
                    occurred_at: time::OffsetDateTime::now_utc().unix_timestamp() - 60,
                    level: 400,
                }]),
            })
        })
    }
}
struct FenceDuringRead(PgPool);
impl details::PublicDetails for FenceDuringRead {
    fn current<'a>(&'a self, _: &'a details::Subject) -> details::DetailsFuture<'a> {
        Box::pin(async move {
            sqlx::query("UPDATE public_projection_state SET highest_epoch=2")
                .execute(&self.0)
                .await
                .map_err(|_| Unavailable)?;
            Ok(details::Details::default())
        })
    }
}
async fn send(app: Router, path: &str) -> Result<Response, Box<dyn std::error::Error>> {
    Ok(app
        .oneshot(Request::builder().uri(path).body(Body::empty())?)
        .await?)
}
#[tokio::test]
async fn search_success_refusal_bad_input() -> TestResult {
    let db = PgPool::connect_lazy("postgres://portal@127.0.0.1:1/unreachable")?;
    let app = router(db.clone(), true, Arc::new(FailClosed));
    let r = send(app.clone(), "/en/characters?name=%20Aldric%20").await?;
    assert_eq!(r.status(), StatusCode::FOUND);
    assert_eq!(r.headers()[header::LOCATION], "/en/characters/Aldric");
    for path in [
        "/en/characters?name=a&name=b",
        "/en/characters?name=%FF",
        "/en/characters?name=..",
        "/en/characters?name=a%2Fb",
        "/en/characters?name=",
        "/en/characters/Aldric?extra=1",
    ] {
        assert_eq!(
            send(app.clone(), path).await?.status(),
            StatusCode::BAD_REQUEST,
            "{path}"
        );
    }
    assert_eq!(
        send(router(db, false, Arc::new(FailClosed)), "/en/characters")
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(
            app.clone(),
            &format!("/en/characters?name={}", "x".repeat(65))
        )
        .await?
        .status(),
        StatusCode::BAD_REQUEST
    );
    for lang in ["pl", "en"] {
        let r = send(app.clone(), &format!("/{lang}/characters")).await?;
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "no-store, private");
        let b = String::from_utf8(to_bytes(r.into_body(), 20000).await?.to_vec())?;
        assert!(b.contains("label for=\"name\""));
    }
    Ok(())
}
#[tokio::test]
async fn postgres_profile_success_privacy_fences_and_name_handoff() -> TestResult {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("Skipping profile PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let admin = PgPool::connect(&url).await?;
    let schema = format!("profiles_{}", uuid::Uuid::now_v7().simple());
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
    sqlx::raw_sql(include_str!(
        "../../../migrations/20261105180000_t24_public_snapshots.sql"
    ))
    .execute(&db)
    .await?;
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1")
        .execute(&db)
        .await?;
    let payload = serde_json::json!({"profile":[{"world_id":"world-fixture","name":"Aldric","level":"412","vocation":4,"guild_visible":true,"guild":[{"guild_id":"guild-fixture","rank_level":1}]}]});
    sqlx::query("INSERT INTO public_snapshots(family,subject,epoch,version,watermark,payload) VALUES ('character_profile','subject-fixture',1,1,extract(epoch FROM now())::bigint,$1)").bind(payload).execute(&db).await?;
    sqlx::query("INSERT INTO public_snapshots(family,subject,is_watermark,epoch,version,watermark,payload) VALUES ('character_profile','',true,1,1,extract(epoch FROM now())::bigint,'{}')").execute(&db).await?;
    let app = router(db.clone(), true, Arc::new(Visible(true)));
    assert_eq!(
        send(
            router(db.clone(), true, Arc::new(FailClosed)),
            "/en/characters/Unknown"
        )
        .await?
        .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let r = send(app.clone(), "/en/characters/aldric").await?;
    assert_eq!(r.status(), StatusCode::OK);
    let body = String::from_utf8(to_bytes(r.into_body(), 20000).await?.to_vec())?;
    assert!(body.contains("412"));
    assert!(!body.contains("subject-fixture") && !body.contains("world-fixture"));
    let detailed = send(
        router_with_details(
            db.clone(),
            true,
            Arc::new(DetailedPrivacy),
            Arc::new(DetailsFixture),
        ),
        "/en/characters/Aldric",
    )
    .await?;
    assert_eq!(detailed.status(), StatusCode::OK);
    let details_body = String::from_utf8(to_bytes(detailed.into_body(), 20000).await?.to_vec())?;
    assert!(
        details_body.contains("Fixture Guild")
            && details_body.contains("Fixture House")
            && details_body.contains("Death (time and level)")
    );
    assert!(!details_body.contains("<script>"));
    assert!(!body.contains("Fixture Guild"));
    let denied_fields = send(
        router_with_details(
            db.clone(),
            true,
            Arc::new(Visible(true)),
            Arc::new(DetailsFixture),
        ),
        "/en/characters/Aldric",
    )
    .await?;
    let denied_body =
        String::from_utf8(to_bytes(denied_fields.into_body(), 20000).await?.to_vec())?;
    assert!(!denied_body.contains("Fixture Guild") && !denied_body.contains("Fixture House"));
    assert_eq!(
        send(
            router(db.clone(), true, Arc::new(Visible(false))),
            "/en/characters/Aldric"
        )
        .await?
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(
            router(db.clone(), true, Arc::new(FailClosed)),
            "/en/characters/Aldric"
        )
        .await?
        .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        send(app.clone(), "/en/characters/Unknown").await?.status(),
        StatusCode::NOT_FOUND
    );
    sqlx::query("INSERT INTO public_snapshots SELECT family,'second-fixture',false,epoch,version,watermark,payload,NULL FROM public_snapshots WHERE subject='subject-fixture'").execute(&db).await?;
    assert_eq!(
        send(app.clone(), "/en/characters/Aldric").await?.status(),
        StatusCode::NOT_FOUND
    );
    sqlx::query("DELETE FROM public_snapshots WHERE subject='second-fixture'")
        .execute(&db)
        .await?;
    let fenced = router_with_details(
        db.clone(),
        true,
        Arc::new(Visible(true)),
        Arc::new(FenceDuringRead(db.clone())),
    );
    assert_eq!(
        send(fenced, "/en/characters/Aldric").await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    sqlx::query("UPDATE public_projection_state SET highest_epoch=1")
        .execute(&db)
        .await?;
    sqlx::query("UPDATE public_snapshots SET watermark=watermark-31 WHERE is_watermark")
        .execute(&db)
        .await?;
    assert_eq!(
        send(app.clone(), "/en/characters/Aldric").await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    sqlx::query("UPDATE public_projection_state SET highest_epoch=2")
        .execute(&db)
        .await?;
    assert_eq!(
        send(app, "/en/characters/Aldric").await?.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    db.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await?;
    eprintln!("Profile PostgreSQL tests ran: 1");
    Ok(())
}
