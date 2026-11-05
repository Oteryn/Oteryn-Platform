#![allow(clippy::unwrap_used)]
use super::*;
use axum::extract::Request;
#[tokio::test]
async fn disabled_router_refuses_before_database() {
    use tower::ServiceExt;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let app = router(
        pool,
        PanelConfig::default(),
        WorldOptions::default(),
        PendingAuthentication,
        PendingAuthentication,
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/admin/world")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
#[test]
fn translations_and_bad_names() {
    for lang in ["pl", "en"] {
        assert!(!text(lang, "title").is_empty());
    }
    assert!(crate::types::validate_label("\0", "name").is_err());
}

#[derive(Clone)]
struct Identity {
    account: Uuid,
    mfa: bool,
    state: oteryn_admin::rbac::AccountState,
}
impl StaffSession for Identity {
    async fn resolve(&self, _: &BrowserSession) -> Option<oteryn_admin::web::StaffIdentity> {
        Some(oteryn_admin::web::StaffIdentity {
            account: self.account,
            mfa_satisfied: self.mfa,
        })
    }
}
impl AccountStateReader for Identity {
    async fn state(
        &self,
        _: Uuid,
    ) -> Result<oteryn_admin::rbac::AccountState, oteryn_admin::rbac::AccountStateUnavailable> {
        Ok(self.state)
    }
}
fn app(pool: PgPool, identity: Identity) -> Router {
    router(
        pool,
        PanelConfig {
            admin_panel: true,
            host: "staff.test".into(),
        },
        WorldOptions { enabled: true },
        identity.clone(),
        identity,
    )
}
async fn get(app: Router, uri: &str) -> Response {
    use tower::ServiceExt;
    app.oneshot(
        Request::get(uri)
            .header("host", "staff.test")
            .header("accept-language", "en")
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}
async fn post(app: Router, cookie: &str, csrf: &str, body: String) -> Response {
    use tower::ServiceExt;
    app.oneshot(
        Request::post("/admin/world")
            .header("host", "staff.test")
            .header("origin", "https://staff.test")
            .header("cookie", cookie)
            .header("x-csrf-token", csrf)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(axum::body::Body::from(body))
            .unwrap(),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn postgres_staff_publication_visibility_names_diff_and_refusals() {
    use axum::body::to_bytes;
    use oteryn_admin::rbac::AccountState;
    use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED PostgreSQL T14C test: PORTAL_TEST_DATABASE_URL is unset");
        return;
    };
    let root = PgPool::connect(&url).await.unwrap();
    let schema = format!("world_admin_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await
        .unwrap();
    let path = schema.clone();
    let pool = PgPoolOptions::new()
        .max_connections(5)
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
        .await
        .unwrap();
    oteryn_db::migrate(&pool).await.unwrap();
    let identity = Identity {
        account: Uuid::now_v7(),
        mfa: true,
        state: AccountState::Active,
    };
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'").bind(identity.account).execute(&pool).await.unwrap();
    let one = Uuid::now_v7();
    let two = Uuid::now_v7();
    let failed = Uuid::now_v7();
    for (id, commit, status) in [
        (one, "a".repeat(40), "ready"),
        (two, "b".repeat(40), "ready"),
        (failed, "c".repeat(40), "failed"),
    ] {
        sqlx::query("INSERT INTO world_versions(id,source_commit,status) VALUES($1,$2,$3)")
            .bind(id)
            .bind(commit)
            .bind(status)
            .execute(&pool)
            .await
            .unwrap();
    }
    for (id, key, data, publishable) in [
        (one, "entry", json!({"value":1}), true),
        (two, "entry", json!({"value":2}), true),
        (two, "hidden", json!({"secret":true}), false),
        (two, "<script>", json!({}), true),
    ] {
        sqlx::query("INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES($1,'test',$2,$3,$4)").bind(id).bind(key).bind(data).bind(publishable).execute(&pool).await.unwrap();
    }
    sqlx::query("UPDATE world_active_version SET version_id=$1 WHERE singleton")
        .bind(one)
        .execute(&pool)
        .await
        .unwrap();
    let response = get(
        app(pool.clone(), identity.clone()),
        &format!("/admin/world?version={two}&compare={one}"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let body = String::from_utf8(
        to_bytes(response.into_body(), 200000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    let csrf = body
        .split("name=\"_csrf\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    assert!(body.contains("Validation results"));
    assert!(body.contains("script"));
    assert!(!body.contains("<script>"));
    assert!(body.contains("value"));
    std::fs::write("/tmp/t14c-en.html", &body).unwrap();
    assert_eq!(
        get(app(pool.clone(), identity.clone()), "/admin/world?compare=")
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        get(
            app(pool.clone(), identity.clone()),
            &format!("/admin/world?version={one}&compare={one}")
        )
        .await
        .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        get(
            app(pool.clone(), identity.clone()),
            "/admin/world?version=bad"
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        get(
            app(pool.clone(), identity.clone()),
            &format!("/admin/world?version={}", Uuid::now_v7())
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    let publish = format!("action=publish&version={two}");
    assert_eq!(
        post(
            app(pool.clone(), identity.clone()),
            &cookie,
            "wrong",
            publish.clone()
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(
            app(pool.clone(), identity.clone()),
            &cookie,
            csrf,
            format!("action=publish&version={failed}")
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        post(app(pool.clone(), identity.clone()), &cookie, csrf, publish)
            .await
            .status(),
        StatusCode::SEE_OTHER
    );
    let store = WorldStore::new(pool.clone(), WorldOptions { enabled: true });
    assert_eq!(store.active_version_id().await.unwrap(), Some(two));
    let visible = format!("action=visibility&version={two}&kind=test&key=entry&allowed=true");
    assert_eq!(
        post(
            app(pool.clone(), identity.clone()),
            &cookie,
            csrf,
            visible.clone()
        )
        .await
        .status(),
        StatusCode::SEE_OTHER
    );
    assert!(store.get_entity("test", "entry").await.unwrap().is_some());
    assert_eq!(
        post(
            app(pool.clone(), identity.clone()),
            &cookie,
            csrf,
            format!("action=visibility&version={two}&kind=test&key=hidden&allowed=true")
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert!(store.get_entity("test", "hidden").await.unwrap().is_none());
    assert_eq!(
        post(
            app(pool.clone(), identity.clone()),
            &cookie,
            csrf,
            format!("action=names&version={two}&kind=test&key=entry&pl=Test&en=Example")
        )
        .await
        .status(),
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        post(
            app(pool.clone(), identity.clone()),
            &cookie,
            csrf,
            format!("action=names&version={two}&kind=test&key=entry&pl=&en=Example")
        )
        .await
        .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let names: (String, String) =
        sqlx::query_as("SELECT pl,en FROM world_admin_names WHERE kind='test' AND key='entry'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(names, ("Test".into(), "Example".into()));
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action LIKE 'world.%'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 3);
    for (mfa, state) in [
        (false, AccountState::Active),
        (true, AccountState::Unknown),
        (true, AccountState::Ambiguous),
        (true, AccountState::Disabled),
    ] {
        assert_eq!(
            get(
                app(
                    pool.clone(),
                    Identity {
                        mfa,
                        state,
                        ..identity.clone()
                    }
                ),
                "/admin/world"
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
    }
    // An audit failure must roll back the pointer and visibility change.
    sqlx::query("CREATE FUNCTION fail_world_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test audit failure'; END; $$").execute(&pool).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_world_audit BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION fail_world_audit()").execute(&pool).await.unwrap();
    assert_eq!(
        post(
            app(pool.clone(), identity.clone()),
            &cookie,
            csrf,
            format!("action=publish&version={one}")
        )
        .await
        .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(store.active_version_id().await.unwrap(), Some(two));
    sqlx::query("DROP TRIGGER fail_world_audit ON audit_log")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM role_permissions WHERE permission='game_catalog.snapshots.activate'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        post(
            app(pool.clone(), identity.clone()),
            &cookie,
            csrf,
            format!("action=publish&version={one}")
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(identity.account)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        get(app(pool.clone(), identity), "/admin/world")
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await
        .unwrap();
    root.close().await;
}
