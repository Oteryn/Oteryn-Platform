use super::*;
use crate::dashboard::{self, Character, Source};
use std::sync::Mutex;

#[derive(Debug)]
struct Proven {
    cut: Cut,
    stale: bool,
    commercial: bool,
    opt_in: bool,
}
impl Authority for Proven {
    fn revalidate(&self, i: &Intent, now: i64) -> Result<Option<Evidence>> {
        Ok(Some(Evidence {
            observed_at_ms: if self.stale { now - 10001 } else { now },
            clock_uncertainty_ms: 0,
            current_cut: i.character.map(|_| self.cut),
            baseline_complete: true,
            active_account: true,
            channel_opt_in: self.opt_in,
            source_due_at_ms: Some(i.due_at_ms),
            premium_commercial_agrees: self.commercial,
            stamina_minutes: Some(2520),
        }))
    }
}
impl Source for Proven {
    fn characters(&self, _: Uuid, now: i64) -> Result<Option<Vec<Character>>> {
        Ok(Some(vec![Character {
            character: Uuid::from_u128(7),
            cut: self.cut,
            level: Some(20),
            online_at_observation: None,
            stamina_minutes: Some(2520),
            premium_until_ms: None,
            available_bosses: vec![],
            last_session_balance: Some(-10),
            observed_at_ms: if self.stale { now - 10001 } else { now },
        }]))
    }
    fn current_cut(&self, _: Uuid, _: Uuid, _: i64) -> Result<Option<Cut>> {
        Ok(Some(self.cut))
    }
}
fn proven() -> Proven {
    Proven {
        cut: Cut {
            character_revision: 9,
            projection_revision: 12,
            projection_epoch: 1,
            recovery_generation: 3,
        },
        stale: false,
        commercial: true,
        opt_in: true,
    }
}
fn flags() -> Flags {
    Flags {
        dashboard: true,
        reminders: true,
        email: true,
        web_push: true,
    }
}
fn actor(owner: Uuid) -> Actor {
    Actor {
        account: owner,
        active: true,
        staff_access: false,
    }
}
fn intent() -> Intent {
    Intent {
        owner: Uuid::now_v7(),
        character: Some(Uuid::now_v7()),
        cut: Some(proven().cut),
        kind: Kind::Boss,
        entity: "boss/example".into(),
        channel: Channel::Email,
        due_at_ms: 100_000,
    }
}

#[test]
fn evidence_refusals_and_bad_input() -> Result<()> {
    let mut i = intent();
    assert!(eligible(flags(), &i, &proven(), 100_000)?);
    assert!(!eligible(Flags::default(), &i, &proven(), 100_000)?);
    assert!(!eligible(flags(), &i, &Unavailable, 100_000)?);
    let mut p = proven();
    p.stale = true;
    assert!(!eligible(flags(), &i, &p, 100_000)?);
    p.stale = false;
    p.cut.projection_epoch += 1;
    assert!(!eligible(flags(), &i, &p, 100_000)?);
    p = proven();
    p.opt_in = false;
    assert!(!eligible(flags(), &i, &p, 100_000)?);
    i.kind = Kind::Rent;
    assert!(!eligible(flags(), &i, &proven(), 100_000)?);
    i.kind = Kind::Premium;
    i.character = None;
    i.cut = None;
    p = proven();
    p.commercial = false;
    assert!(!eligible(flags(), &i, &p, 100_000)?);
    i.entity = "\ninvalid".into();
    assert!(eligible(flags(), &i, &proven(), 100_000).is_err());
    assert!(!dashboard::fresh(100_001, 100_000));
    Ok(())
}
#[test]
fn dashboard_owner_only_and_locales() -> Result<()> {
    let owner = Uuid::now_v7();
    assert_eq!(
        dashboard::load(flags(), actor(owner), owner, &proven(), 100_000)?.len(),
        1
    );
    assert!(dashboard::load(flags(), actor(Uuid::now_v7()), owner, &proven(), 100_000).is_err());
    let mut staff = actor(owner);
    staff.staff_access = true;
    assert!(dashboard::load(flags(), staff, owner, &proven(), 100_000).is_err());
    assert!(dashboard::load(Flags::default(), actor(owner), owner, &proven(), 100_000).is_err());
    assert!(
        dashboard::load(
            flags(),
            actor(owner),
            owner,
            &dashboard::Unavailable,
            100_000
        )
        .is_err()
    );
    for lang in ["pl", "en"] {
        for kind in [
            Kind::Rent,
            Kind::Premium,
            Kind::Boss,
            Kind::Stamina,
            Kind::Bazaar,
        ] {
            assert!(!message(lang, kind)?.is_empty());
        }
    }
    assert!(message("xx", Kind::Boss).is_err());
    for lang in ["pl", "en"] {
        let html = dashboard::render(flags(), actor(owner), owner, &proven(), 100_000, lang)?;
        assert!(html.contains("<dl>"));
        assert!(html.contains("2520 / 2520"));
    }
    Ok(())
}
#[derive(Debug, Default)]
struct Capture(Mutex<Vec<Uuid>>);
impl Delivery for Capture {
    fn enqueue<'a>(
        &'a self,
        _: &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        _: &'a Intent,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut ids = self.0.lock().map_err(|_| denied())?;
            if !ids.contains(&id) {
                ids.push(id);
            }
            Ok(())
        })
    }
}
async fn database() -> Result<Option<PgPool>> {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("PostgreSQL reminder test skipped: PORTAL_TEST_DATABASE_URL unset");
        return Ok(None);
    };
    let admin = PgPool::connect(&url).await?;
    let schema = format!("t79_{}", Uuid::now_v7().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .after_connect(move |c, _| {
            let schema = schema.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(schema)
                    .execute(c)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    Ok(Some(pool))
}
#[tokio::test]
async fn postgres_idempotency_delivery_refusal_and_cancellation() -> Result<()> {
    let Some(pool) = database().await? else {
        return Ok(());
    };
    let service = Reminders {
        pool: pool.clone(),
        flags: flags(),
    };
    let i = intent();
    let owner = actor(i.owner);
    let p = proven();
    let capture = Capture::default();
    let id = service.schedule(owner, &i, &p, 100_000).await?;
    assert_eq!(service.schedule(owner, &i, &p, 100_000).await?, id);
    assert!(
        service
            .schedule(actor(Uuid::now_v7()), &i, &p, 100_000)
            .await
            .is_err()
    );
    let mut bad = i.clone();
    bad.due_at_ms = -1;
    assert!(service.schedule(owner, &bad, &p, 100_000).await.is_err());
    assert!(!service.dispatch(id, &i, &p, &capture, 99_999).await?);
    assert!(service.dispatch(id, &i, &p, &capture, 100_000).await?);
    assert!(!service.dispatch(id, &i, &p, &capture, 100_000).await?);
    assert_eq!(capture.0.lock().map_err(|_| denied())?.len(), 1);
    let mut next = i.clone();
    next.channel = Channel::WebPush;
    let id2 = service.schedule(owner, &next, &p, 100_000).await?;
    let mut stale = proven();
    stale.stale = true;
    assert!(
        !service
            .dispatch(id2, &next, &stale, &capture, 100_000)
            .await?
    );
    assert!(!service.dispatch(id2, &next, &p, &capture, 100_000).await?);
    next.entity = "boss/other".into();
    let id3 = service.schedule(owner, &next, &p, 100_000).await?;
    assert!(
        service
            .cancel(actor(Uuid::now_v7()), i.owner, id3)
            .await
            .is_err()
    );
    assert!(service.cancel(owner, i.owner, id3).await?);
    sqlx::query("DELETE FROM pd_reminders WHERE account_id=$1")
        .bind(i.owner)
        .execute(&pool)
        .await?;
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(&pool)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(())
}

#[tokio::test]
async fn dashboard_http_success_disabled_anonymous_staff_and_bad_language() -> Result<()> {
    use axum::{
        Extension,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let owner = Uuid::now_v7();
    let app = dashboard::router(flags(), std::sync::Arc::new(proven()));
    let response = app
        .clone()
        .layer(Extension(actor(owner)))
        .oneshot(Request::get("/en/account/dashboard").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let html = String::from_utf8(to_bytes(response.into_body(), 65536).await?.to_vec())?;
    assert!(html.contains("Account dashboard"));
    assert!(html.contains("width=device-width"));
    let response = app
        .clone()
        .oneshot(Request::get("/en/account/dashboard").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let mut staff = actor(owner);
    staff.staff_access = true;
    let response = app
        .clone()
        .layer(Extension(staff))
        .oneshot(Request::get("/en/account/dashboard").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = app
        .layer(Extension(actor(owner)))
        .oneshot(Request::get("/xx/account/dashboard").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = dashboard::configured_router()
        .oneshot(Request::get("/en/account/dashboard").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(to_bytes(response.into_body(), 65536).await?.len(), 0);
    Ok(())
}

#[test]
fn premium_units_round_conservatively_and_unknown_stays_unknown() -> Result<()> {
    assert_eq!(premium_deadline_ms(None)?, None);
    assert_eq!(
        premium_deadline_ms(Some(1_790_086_400_000_001))?,
        Some(1_790_086_400_001)
    );
    assert_eq!(
        premium_deadline_ms(Some(1_790_086_400_000_000))?,
        Some(1_790_086_400_000)
    );
    assert!(premium_deadline_ms(Some(u64::MAX)).is_err());
    Ok(())
}
