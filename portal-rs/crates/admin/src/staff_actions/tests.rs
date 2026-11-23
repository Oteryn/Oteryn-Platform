use super::*;
use crate::{rbac::Authorizer, web::PortalIdentity};
use axum::{
    Extension, Form, Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, header},
    middleware,
    response::IntoResponse,
    routing::get,
};
use oteryn_web::session::{BrowserSession, SessionLayer};
use serde::Deserialize;
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use std::sync::Arc;
use tower::ServiceExt;

#[derive(Clone)]
struct Fixture {
    pool: PgPool,
    actor: CheckContext,
    authority: MutationAuthorization,
}

const OVERSIZED_REASON: &str = concat!(
    "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    "x",
);

#[derive(Deserialize)]
struct TestForm {
    #[serde(default)]
    reason: String,
    #[serde(default)]
    conflict: bool,
    #[serde(default)]
    expire_step_up: bool,
}

async fn apply(
    State(fixture): State<Fixture>,
    Extension(csrf): Extension<Verified>,
    Form(form): Form<TestForm>,
) -> Response {
    let actor = fixture.actor.account;
    let mut response = match run(
        &fixture.pool,
        Guard {
            actor: fixture.actor,
            authority: &fixture.authority,
            csrf: Some(csrf),
        },
        Action {
            permission: Permission::GrantRoles,
            reason: &form.reason,
            reason_codes: &["role_assignment", "short", OVERSIZED_REASON],
            step_up: StepUp::RecentPassword,
        },
        || Ok((vec![actor], (form.conflict, form.expire_step_up))),
        move |connection, (conflict, expire_step_up)| {
            Box::pin(async move {
                let before: String = sqlx::query_scalar("SELECT locale FROM accounts WHERE id=$1")
                    .bind(actor)
                    .fetch_one(&mut *connection)
                    .await?;
                if before == "en" {
                    return Ok(None);
                }
                sqlx::query("UPDATE accounts SET locale='en' WHERE id=$1")
                    .bind(actor)
                    .execute(&mut *connection)
                    .await?;
                if conflict {
                    return Err(Error::Conflict);
                }
                if expire_step_up {
                    sqlx::query(
                        "UPDATE web_sessions SET sudo_until=clock_timestamp() WHERE id_hash=$1",
                    )
                    .bind(
                        fixture
                            .actor
                            .session_proof
                            .ok_or(Error::Forbidden)?
                            .session_hash
                            .as_slice(),
                    )
                    .execute(&mut *connection)
                    .await?;
                }
                Ok(Some(AuditChange::TestLocale { account: actor }))
            })
        },
    )
    .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => error.response(&oteryn_ui::Registry::default().chrome(
            "en",
            "/",
            oteryn_ui::Account::Anonymous,
        )),
    };
    response.headers_mut().insert(
        "x-test-staff-handler",
        header::HeaderValue::from_static("verified"),
    );
    response
}

#[tokio::test]
async fn staff_action_guards_and_audit_share_the_mutation_transaction()
-> Result<(), Box<dyn std::error::Error>> {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let url = std::env::var("PORTAL_TEST_DATABASE_URL")?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("staff_action_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let path = format!("SET search_path TO {schema}");
    let pool = PgPoolOptions::new()
        .after_connect(move |connection, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query(AssertSqlSafe(path)).execute(connection).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    oteryn_db::migrate(&pool).await?;
    let account: Uuid = sqlx::query_scalar(
        "INSERT INTO accounts(email,password_hash,status,email_verified_at,locale)
         VALUES ('staff-action@example.test','test-only','active',now(),'pl') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let role: Uuid =
        sqlx::query_scalar("INSERT INTO roles(key) VALUES ('staff_action_fixture') RETURNING id")
            .fetch_one(&pool)
            .await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) VALUES ($1,$2)")
        .bind(account)
        .bind(role)
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'admin.access')")
        .bind(role)
        .execute(&pool)
        .await?;
    let (proof, cookie) = crate::test_staff::proof_with_cookie(&pool, account).await?;
    let context = CheckContext {
        account,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: Some(proof),
    };
    let fixture = Fixture {
        pool: pool.clone(),
        actor: context,
        authority: MutationAuthorization::new(Arc::new(Authorizer::new(
            pool.clone(),
            PortalIdentity::configured(pool.clone()),
        ))),
    };
    let app = Router::new()
        .route(
            "/action",
            get(|Extension(session): Extension<BrowserSession>| async move {
                session
                    .csrf_token()
                    .await
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
            })
            .post(apply),
        )
        .route_layer(middleware::from_fn_with_state(
            SessionLayer::new(pool.clone()),
            oteryn_web::session::middleware,
        ))
        .route_layer(Extension(oteryn_web::csrf::LocalizedRefusal))
        .with_state(fixture);
    let response = app
        .clone()
        .oneshot(
            Request::get("/action")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    let token = String::from_utf8(to_bytes(response.into_body(), 256).await?.to_vec())?;
    let post_as = |cookie: &str, token: &str, form: &str| {
        let request = Request::post("/action")
            .header(header::COOKIE, cookie)
            .header(header::ACCEPT_LANGUAGE, "en")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header("x-csrf-token", token)
            .body(Body::from(form.to_owned()));
        let app = app.clone();
        async move { Ok::<_, Box<dyn std::error::Error>>(app.oneshot(request?).await?) }
    };
    let post = |token: &str, form: &str| post_as(&cookie, token, form);
    let good = "reason=role_assignment";
    let unchanged = || async {
        let locale: String = sqlx::query_scalar("SELECT locale FROM accounts WHERE id=$1")
            .bind(account)
            .fetch_one(&pool)
            .await?;
        assert_eq!(locale, "pl");
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='staff.action.test'")
                .fetch_one(&pool)
                .await?;
        assert_eq!(count, 0);
        Ok::<_, sqlx::Error>(())
    };

    let bad_csrf = post("bad-token", good).await?;
    assert_eq!(bad_csrf.status(), StatusCode::FORBIDDEN);
    let page = to_bytes(bad_csrf.into_body(), 100_000).await?;
    assert!(std::str::from_utf8(&page)?.contains("Access denied"));
    assert_eq!(post(&token, good).await?.status(), StatusCode::FORBIDDEN);
    unchanged().await?;
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'admin.roles.grant')")
        .bind(role)
        .execute(&pool)
        .await?;

    let oversized_reason = format!("reason={OVERSIZED_REASON}");
    for form in [
        "",
        "reason=arbitrary+credential-shaped+text",
        "reason=short",
        &oversized_reason,
    ] {
        assert_eq!(
            post(&token, form).await?.status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let expiry = "UPDATE account_mfa_tokens SET expires_at=clock_timestamp()+$2::interval WHERE token_hash=$1";
    sqlx::query(expiry)
        .bind(proof.grant_hash.as_slice())
        .bind("-1 minute")
        .execute(&pool)
        .await?;
    assert_eq!(post(&token, good).await?.status(), StatusCode::FORBIDDEN);
    sqlx::query(expiry)
        .bind(proof.grant_hash.as_slice())
        .bind("1 minute")
        .execute(&pool)
        .await?;
    assert_eq!(post(&token, good).await?.status(), StatusCode::FORBIDDEN);
    let (_, other_cookie) = crate::test_staff::proof_with_cookie(&pool, account).await?;
    let response = app
        .clone()
        .oneshot(
            Request::get("/action")
                .header(header::COOKIE, &other_cookie)
                .body(Body::empty())?,
        )
        .await?;
    let other_token = String::from_utf8(to_bytes(response.into_body(), 256).await?.to_vec())?;
    crate::test_staff::recent_password(&pool, &other_cookie).await?;
    assert_eq!(post(&token, good).await?.status(), StatusCode::FORBIDDEN);
    crate::test_staff::recent_password(&pool, &cookie).await?;
    // The other session has a valid token and fresh sudo for this same actor.
    // Its request must still fail when paired with the original actor proof.
    let mismatched = post_as(&other_cookie, &other_token, good).await?;
    assert_eq!(mismatched.headers()["x-test-staff-handler"], "verified");
    assert_eq!(mismatched.status(), StatusCode::FORBIDDEN);
    unchanged().await?;
    let conflict = post(&token, "reason=role_assignment&conflict=true").await?;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    let conflict_html = String::from_utf8(to_bytes(conflict.into_body(), 100_000).await?.to_vec())?;
    assert!(conflict_html.contains("<h1>Update conflict</h1>"));
    assert!(conflict_html.contains("This action conflicts with the current state."));
    unchanged().await?;

    // The sequence is deliberately nontransactional. It attests that the
    // audit INSERT ran even though the final freshness check rolls it back.
    sqlx::raw_sql(
        "CREATE SEQUENCE staff_action_audit_attempts;
        CREATE FUNCTION count_staff_action_audit() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN PERFORM nextval('staff_action_audit_attempts'); RETURN NEW; END $$;
        CREATE TRIGGER count_staff_action_audit AFTER INSERT ON audit_log
        FOR EACH ROW WHEN (NEW.action='staff.action.test')
        EXECUTE FUNCTION count_staff_action_audit()",
    )
    .execute(&pool)
    .await?;
    let expired = post(&token, "reason=role_assignment&expire_step_up=true").await?;
    assert_eq!(expired.status(), StatusCode::FORBIDDEN);
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT is_called FROM staff_action_audit_attempts")
            .fetch_one(&pool)
            .await?
    );
    unchanged().await?;

    for _ in 0..2 {
        assert_eq!(post(&token, good).await?.status(), StatusCode::NO_CONTENT);
    }
    let rows: Vec<(serde_json::Value, serde_json::Value, String)> =
        sqlx::query_as("SELECT before,after,staff_reason FROM audit_log WHERE action='staff.action.test' AND actor=$1 AND target=$2")
            .bind(account).bind(account.to_string()).fetch_all(&pool).await?;
    assert_eq!(
        rows,
        vec![(
            serde_json::json!({"locale": "pl"}),
            serde_json::json!({"locale": "en"}),
            "role_assignment".into(),
        )]
    );
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}
