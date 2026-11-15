use oteryn_admin::rbac::{
    AccountState, AccountStateReader, AccountStateUnavailable, Authorizer, CheckContext,
};
use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions};
use uuid::Uuid;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
#[derive(Clone)]
struct Active;
impl AccountStateReader for Active {
    async fn state(&self, _: Uuid) -> Result<AccountState, AccountStateUnavailable> {
        Ok(AccountState::Active)
    }
}
struct Fixture {
    pool: PgPool,
    root: PgPool,
    schema: String,
    actor: CheckContext,
}
impl Fixture {
    async fn open(size: u32) -> TestResult<Self> {
        let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
        let root = PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;
        let schema = format!("pool_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&root)
            .await?;
        let path = format!("SET search_path TO {schema}");
        let pool = PgPoolOptions::new()
            .max_connections(size)
            .acquire_timeout(std::time::Duration::from_secs(1))
            .after_connect(move |c, _| {
                let path = path.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(path)).execute(c).await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await?;
        oteryn_db::migrate(&pool).await?;
        let actor = CheckContext {
            account: Uuid::now_v7(),
            require_staff_mfa: true,
            mfa_verified: true,
        };
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(actor.account).execute(&pool).await?;
        Ok(Self {
            pool,
            root,
            schema,
            actor,
        })
    }
    async fn close(self) -> TestResult {
        self.pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.root)
        .await?;
        self.root.close().await;
        Ok(())
    }
}
use oteryn_content::events::{Draft, Events, Translation};
#[tokio::test]
async fn event_save_and_transition_complete_with_one_connection() -> TestResult {
    let started = std::time::Instant::now();
    let db = Fixture::open(1).await?;
    let auth = Authorizer::new(db.pool.clone(), Active);
    let events = Events::new(db.pool.clone());
    let now = time::OffsetDateTime::now_utc();
    let draft = Draft {
        starts_at: now,
        ends_at: now + time::Duration::hours(1),
        featured: false,
        en: Translation {
            title: "Pool regression event".into(),
            slug: "pool-event".into(),
            summary: "Concurrent administration".into(),
            body: "Event publication regression coverage.".into(),
        },
        pl: None,
    };
    let id = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        events.save(&auth, db.actor, None, draft),
    )
    .await??;
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        events.transition(&auth, db.actor, id, 1, "published"),
    )
    .await??;
    assert_eq!(events.load(id).await?.0.status, "published");
    db.close().await?;
    eprintln!(
        "event_save_and_transition_complete_with_one_connection: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn announcement_save_completes_with_one_connection() -> TestResult {
    use oteryn_content::announcements::{Announcements, Input};
    let started = std::time::Instant::now();
    let db = Fixture::open(1).await?;
    let auth = Authorizer::new(db.pool.clone(), Active);
    let announcements = Announcements::new(db.pool.clone());
    let input = Input {
        publication_state: "published".into(),
        severity: "info".into(),
        starts_at: time::OffsetDateTime::now_utc(),
        ends_at: None,
        title: "Pool regression notice".into(),
        body: "Administration is available.".into(),
        action_label: None,
        action_url: None,
        title_pl: None,
        body_pl: None,
        action_label_pl: None,
        translation_published_at: None,
    };
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        announcements.save(&auth, db.actor, None, input),
    )
    .await??;
    assert_eq!(
        announcements
            .active("en", time::OffsetDateTime::now_utc(), 1)
            .await?
            .len(),
        1
    );
    db.close().await?;
    eprintln!(
        "announcement_save_completes_with_one_connection: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[derive(Clone)]
struct Staff(Uuid);
impl oteryn_admin::web::StaffSession for Staff {
    async fn resolve(
        &self,
        _: &oteryn_web::session::BrowserSession,
    ) -> Option<oteryn_admin::web::StaffIdentity> {
        Some(oteryn_admin::web::StaffIdentity {
            account: self.0,
            mfa_satisfied: true,
        })
    }
}
async fn blocked_by(pool: &PgPool, pid: i32) -> TestResult {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(pid)
            .fetch_one(pool)
            .await?;
            if waiting {
                return Ok::<_, sqlx::Error>(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await??;
    Ok(())
}

#[tokio::test]
async fn content_mutations_recheck_after_resource_wait() -> TestResult {
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use oteryn_admin::{
        rbac::Permission,
        web::{PanelConfig, guarded},
    };
    use sha2::{Digest, Sha256};
    use tower::ServiceExt;
    let started = std::time::Instant::now();
    let db = Fixture::open(6).await?;
    let news: Uuid=sqlx::query_scalar("INSERT INTO news(slug,title,body) VALUES('fenced-news','Original','Original body') RETURNING id").fetch_one(&db.pool).await?;
    let media = Uuid::now_v7();
    let bytes = b"original image";
    sqlx::query("INSERT INTO media(id,alt_text,mime_type,width,height,content,sha256) VALUES($1,'Original','image/png',1,1,$2,$3)").bind(media).bind(bytes.as_slice()).bind(Sha256::digest(bytes).to_vec()).execute(&db.pool).await?;
    let config = PanelConfig {
        admin_panel: true,
        host: "staff.test".into(),
    };
    let identity = Staff(db.actor.account);
    let app: Router = guarded(
        oteryn_content::news::admin_router(db.pool.clone()),
        db.pool.clone(),
        config.clone(),
        identity.clone(),
        Active,
        Permission::ManageNews,
    )
    .merge(guarded(
        oteryn_content::media::admin_router(db.pool.clone()),
        db.pool.clone(),
        config.clone(),
        identity.clone(),
        Active,
        Permission::ManageMedia,
    ))
    .merge(oteryn_content::pages::staff_router(
        db.pool.clone(),
        config,
        identity,
        Active,
    ));
    let response = app
        .clone()
        .oneshot(
            Request::get("/admin/news/create")
                .header("host", "staff.test")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get("set-cookie")
        .ok_or("cookie missing")?
        .to_str()?
        .split(';')
        .next()
        .ok_or("cookie empty")?
        .to_owned();
    let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
    let csrf = html
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .ok_or("csrf missing")?;
    for kind in [
        "news-save",
        "news-delete",
        "page-save",
        "media-upload",
        "media-delete",
    ] {
        let mut lock = db.pool.begin().await?;
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *lock)
            .await?;
        let (path, body, content_type) = match kind {
            "news-save" | "news-delete" => {
                sqlx::query("SELECT id FROM news WHERE id=$1 FOR UPDATE")
                    .bind(news)
                    .execute(&mut *lock)
                    .await?;
                (
                    if kind == "news-save" {
                        format!("/admin/news/{news}")
                    } else {
                        format!("/admin/news/{news}/delete")
                    },
                    b"slug=fenced-news&title=Changed&body=Changed&published_at=".to_vec(),
                    "application/x-www-form-urlencoded",
                )
            }
            "page-save" => {
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('fenced-page',71102))")
                    .execute(&mut *lock)
                    .await?;
                (
                    "/admin/pages/fenced-page/en".into(),
                    b"title=Changed&body=Changed&action=save".to_vec(),
                    "application/x-www-form-urlencoded",
                )
            }
            "media-upload" => {
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,12))")
                    .bind(db.actor.account.to_string())
                    .execute(&mut *lock)
                    .await?;
                let mut body = b"--audit\r\nContent-Disposition: form-data; name=\"alt_text\"\r\n\r\nNew image\r\n--audit\r\nContent-Disposition: form-data; name=\"image\"; filename=\"new.png\"\r\nContent-Type: image/png\r\n\r\n".to_vec();
                let mut image = std::io::Cursor::new(Vec::new());
                image::DynamicImage::new_rgb8(2, 2)
                    .write_to(&mut image, image::ImageFormat::Png)?;
                body.extend(image.into_inner());
                body.extend_from_slice(b"\r\n--audit--\r\n");
                (
                    "/admin/media".into(),
                    body,
                    "multipart/form-data; boundary=audit",
                )
            }
            _ => {
                sqlx::query("SELECT id FROM media WHERE id=$1 FOR UPDATE")
                    .bind(media)
                    .execute(&mut *lock)
                    .await?;
                (
                    format!("/admin/media/{media}/delete"),
                    Vec::new(),
                    "application/x-www-form-urlencoded",
                )
            }
        };
        let request = Request::post(path)
            .header("host", "staff.test")
            .header("origin", "https://staff.test")
            .header("cookie", &cookie)
            .header("x-csrf-token", csrf)
            .header("content-type", content_type)
            .body(Body::from(body))?;
        let pending = tokio::spawn(app.clone().oneshot(request));
        blocked_by(&db.pool, pid).await?;
        let mut revoke = db.pool.begin().await?;
        oteryn_admin::rbac::lock_mutation(&mut revoke).await?;
        sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
            .bind(db.actor.account)
            .execute(&mut *revoke)
            .await?;
        revoke.commit().await?;
        lock.rollback().await?;
        assert_eq!(pending.await??.status(), StatusCode::FORBIDDEN, "{kind}");
        let title: String = sqlx::query_scalar("SELECT title FROM news WHERE id=$1")
            .bind(news)
            .fetch_one(&db.pool)
            .await?;
        assert_eq!(title, "Original");
        let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM media),(SELECT count(*) FROM pages WHERE slug='fenced-page')").fetch_one(&db.pool).await?;
        assert_eq!(counts, (1, 0));
        sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(db.actor.account).execute(&db.pool).await?;
    }
    db.close().await?;
    eprintln!(
        "content_mutations_recheck_after_resource_wait duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

fn notice() -> oteryn_content::announcements::Input {
    oteryn_content::announcements::Input {
        publication_state: "published".into(),
        severity: "info".into(),
        starts_at: time::OffsetDateTime::now_utc() - time::Duration::days(1),
        ends_at: None,
        title: "Notice".into(),
        body: "Notice body".into(),
        action_label: None,
        action_url: None,
        title_pl: Some("Ogloszenie".into()),
        body_pl: Some("Tresc".into()),
        action_label_pl: None,
        translation_published_at: Some(time::OffsetDateTime::now_utc() - time::Duration::hours(1)),
    }
}
fn event_draft() -> Draft {
    Draft {
        starts_at: time::OffsetDateTime::now_utc(),
        ends_at: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
        featured: false,
        en: Translation {
            title: "Event".into(),
            slug: "bounded-event".into(),
            summary: "Summary".into(),
            body: "Event body".into(),
        },
        pl: None,
    }
}
#[tokio::test]
async fn editorial_mutations_require_access() -> TestResult {
    use oteryn_content::announcements::Announcements;
    let started = std::time::Instant::now();
    let db = Fixture::open(3).await?;
    let auth = Authorizer::new(db.pool.clone(), Active);
    let announcements = Announcements::new(db.pool.clone());
    let events = Events::new(db.pool.clone());
    let announcement = announcements.save(&auth, db.actor, None, notice()).await?;
    let event = events.save(&auth, db.actor, None, event_draft()).await?;
    sqlx::query("DELETE FROM role_permissions WHERE permission='admin.access'")
        .execute(&db.pool)
        .await?;
    assert!(matches!(
        announcements
            .save(&auth, db.actor, Some((announcement, 1)), notice())
            .await,
        Err(oteryn_content::announcements::Error::Authorization(_))
    ));
    assert!(matches!(
        events
            .save(&auth, db.actor, Some((event, 1)), event_draft())
            .await,
        Err(oteryn_content::events::Error::Authorization(_))
    ));
    assert!(matches!(
        events
            .transition(&auth, db.actor, event, 1, "published")
            .await,
        Err(oteryn_content::events::Error::Authorization(_))
    ));
    sqlx::query("INSERT INTO role_permissions(role_id,permission) SELECT id,'admin.access' FROM roles WHERE key='platform_admin'").execute(&db.pool).await?;
    db.close().await?;
    eprintln!(
        "editorial_mutations_require_access duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn editorial_mutations_bound_fence_and_resource_waits() -> TestResult {
    use oteryn_content::announcements::Announcements;
    let started = std::time::Instant::now();
    let db = Fixture::open(3).await?;
    let auth = Authorizer::new(db.pool.clone(), Active);
    let announcements = Announcements::new(db.pool.clone());
    let events = Events::new(db.pool.clone());
    let announcement = announcements.save(&auth, db.actor, None, notice()).await?;
    let event = events.save(&auth, db.actor, None, event_draft()).await?;
    for resource in [false, true] {
        for kind in ["announcement", "event-save", "event-status"] {
            let mut lock = db.pool.begin().await?;
            if resource {
                if kind == "announcement" {
                    sqlx::query("SELECT id FROM announcements WHERE id=$1 FOR UPDATE")
                        .bind(announcement)
                        .execute(&mut *lock)
                        .await?;
                } else {
                    sqlx::query("SELECT id FROM events WHERE id=$1 FOR UPDATE")
                        .bind(event)
                        .execute(&mut *lock)
                        .await?;
                }
            } else {
                sqlx::query("SELECT pg_advisory_xact_lock(71010)")
                    .execute(&mut *lock)
                    .await?;
            }
            let wait = std::time::Instant::now();
            let refused = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                match kind {
                    "announcement" => announcements
                        .save(&auth, db.actor, Some((announcement, 1)), notice())
                        .await
                        .is_err(),
                    "event-save" => events
                        .save(&auth, db.actor, Some((event, 1)), event_draft())
                        .await
                        .is_err(),
                    _ => events
                        .transition(&auth, db.actor, event, 1, "published")
                        .await
                        .is_err(),
                }
            })
            .await?;
            assert!(refused);
            assert!(wait.elapsed() >= std::time::Duration::from_secs(2));
            lock.rollback().await?;
            let mut connections = Vec::new();
            for _ in 0..3 {
                connections.push(db.pool.acquire().await?);
            }
            drop(connections);
            eprintln!("bounded {kind} resource={resource}: {:?}", wait.elapsed());
        }
    }
    db.close().await?;
    eprintln!(
        "editorial_mutations_bound_fence_and_resource_waits duration: {:?}",
        started.elapsed()
    );
    Ok(())
}

#[tokio::test]
async fn announcement_unchanged_and_metadata_saves_preserve_translation() -> TestResult {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let started = std::time::Instant::now();
    let db = Fixture::open(2).await?;
    let auth = Authorizer::new(db.pool.clone(), Active);
    let service = oteryn_content::announcements::Announcements::new(db.pool.clone());
    let mut input = notice();
    let id = service.save(&auth, db.actor, None, input.clone()).await?;
    for (version, severity) in [(1, "info"), (2, "warning")] {
        input.severity = severity.into();
        service
            .save(&auth, db.actor, Some((id, version)), input.clone())
            .await?;
        for locale in ["en", "pl"] {
            assert_eq!(
                service
                    .active(locale, time::OffsetDateTime::now_utc(), 5)
                    .await?
                    .len(),
                1
            );
            let response = oteryn_content::today::router(db.pool.clone())
                .oneshot(
                    Request::get(format!("/{locale}/announcements/ticker")).body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
            let html = String::from_utf8(to_bytes(response.into_body(), 100000).await?.to_vec())?;
            assert!(html.contains(if locale == "en" {
                "Notice"
            } else {
                "Ogloszenie"
            }));
        }
    }
    input.title = "Changed source".into();
    service
        .save(&auth, db.actor, Some((id, 3)), input.clone())
        .await?;
    assert!(
        service
            .active("pl", time::OffsetDateTime::now_utc(), 5)
            .await?
            .is_empty()
    );
    input.severity = "maintenance".into();
    service.save(&auth, db.actor, Some((id, 4)), input).await?;
    assert!(
        service
            .active("pl", time::OffsetDateTime::now_utc(), 5)
            .await?
            .is_empty()
    );
    db.close().await?;
    eprintln!(
        "announcement_unchanged_and_metadata_saves_preserve_translation duration: {:?}",
        started.elapsed()
    );
    Ok(())
}
