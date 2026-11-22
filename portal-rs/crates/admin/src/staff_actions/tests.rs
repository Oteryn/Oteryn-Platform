#![allow(clippy::unwrap_used)]

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

#[derive(Deserialize)]
struct TestForm {
    #[serde(default)]
    reason: String,
    #[serde(default)]
    conflict: bool,
}

async fn apply(
    State(fixture): State<Fixture>,
    csrf: Option<Extension<Verified>>,
    Form(form): Form<TestForm>,
) -> Response {
    let actor = fixture.actor.account;
    match run(
        &fixture.pool,
        Guard {
            actor: fixture.actor,
            authority: &fixture.authority,
            csrf: csrf.map(|Extension(proof)| proof),
        },
        Action {
            permission: Permission::GrantRoles,
            action: "staff.action.test",
            reason: &form.reason,
        },
        || Ok((vec![actor], form.conflict)),
        move |connection, conflict| {
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
                    .execute(connection)
                    .await?;
                if conflict {
                    return Err(Error::Conflict);
                }
                Ok(Some(AuditChange {
                    target: actor,
                    before: [("locale", AuditValue::Label("pl"))].into(),
                    after: [("locale", AuditValue::Label("en"))].into(),
                }))
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
    }
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
    let proof = crate::test_staff::proof(&pool, account).await?;
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
                session.csrf_token().await.unwrap()
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
        .oneshot(Request::get("/action").body(Body::empty())?)
        .await?;
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()?
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let token = String::from_utf8(to_bytes(response.into_body(), 256).await?.to_vec())?;
    let post = |token: &str, form: &str| {
        app.clone().oneshot(
            Request::post("/action")
                .header(header::COOKIE, &cookie)
                .header(header::ACCEPT_LANGUAGE, "en")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("x-csrf-token", token)
                .body(Body::from(form.to_owned()))
                .unwrap(),
        )
    };
    let good = "reason=Valid+staff+reason";
    let unchanged = || async {
        let locale: String = sqlx::query_scalar("SELECT locale FROM accounts WHERE id=$1")
            .bind(account)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(locale, "pl");
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='staff.action.test'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 0);
    };

    let bad_csrf = post("bad-token", good).await?;
    assert_eq!(bad_csrf.status(), StatusCode::FORBIDDEN);
    let page = to_bytes(bad_csrf.into_body(), 100_000).await?;
    assert!(std::str::from_utf8(&page)?.contains("Access denied"));
    assert_eq!(post(&token, good).await?.status(), StatusCode::FORBIDDEN);
    unchanged().await;
    sqlx::query("INSERT INTO role_permissions(role_id,permission) VALUES ($1,'admin.roles.grant')")
        .bind(role)
        .execute(&pool)
        .await?;

    for form in [
        "",
        "reason=%20%20short",
        &format!("reason={}", "a".repeat(MAX_REASON_CHARS + 1)),
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
    let conflict = post(&token, "reason=Valid+staff+reason&conflict=true").await?;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert!(
        String::from_utf8(to_bytes(conflict.into_body(), 100_000).await?.to_vec())?
            .contains("Change conflict")
    );
    unchanged().await;

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
            "Valid staff reason".into(),
        )]
    );
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    Ok(())
}
