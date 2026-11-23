use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
use tower::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn plain_text_limits_and_markup() {
    assert!(matches!(
        plain_reply("  Łódź & 2 < 3\nSecond line.  "),
        Ok("Łódź & 2 < 3\nSecond line.")
    ));
    assert!(plain_reply(&"ż".repeat(2000)).is_ok());
    for invalid in [
        "",
        "   ",
        "embedded\0control",
        "<strong>HTML</strong>",
        "[link](/en/)",
        "**emphasis**",
        "## Title",
        "~~~\ncode\n~~~",
        &"a".repeat(2001),
    ] {
        assert!(
            matches!(plain_reply(invalid), Err(Error::Invalid)),
            "{invalid}"
        );
    }
}

async fn account(pool: &PgPool, role: Option<&str>) -> TestResult<CheckContext> {
    let account = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO accounts(id,email,password_hash,status,locale,email_verified_at)
                 VALUES($1,$2,'test-only','active','en',now())",
    )
    .bind(account)
    .bind(format!("{account}@example.invalid"))
    .execute(pool)
    .await?;
    if let Some(role) = role {
        sqlx::query(
            "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2",
        )
        .bind(account)
        .bind(role)
        .execute(pool)
        .await?;
    }
    Ok(CheckContext {
        account,
        require_staff_mfa: true,
        mfa_verified: true,
        session_proof: Some(crate::test_staff::proof(pool, account).await?),
    })
}

#[tokio::test]
async fn postgres_article_discussion_lifecycle() -> TestResult {
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        return Ok(());
    };
    let root = PgPool::connect(&url).await?;
    let schema = format!("discussion_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let path = format!("SET search_path TO {schema}");
    let pool = PgPoolOptions::new()
        .after_connect(move |conn, _| {
            let path = path.clone();
            Box::pin(async move {
                sqlx::query(AssertSqlSafe(path)).execute(conn).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    let result = lifecycle(&pool).await;
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    result
}

async fn lifecycle(pool: &PgPool) -> TestResult {
    oteryn_db::migrate(pool).await?;
    let author = account(pool, None).await?;
    let editor = account(pool, Some("content_editor")).await?;
    let author_hash = author.session_proof.ok_or("author session")?.session_hash;
    let editor_hash = editor.session_proof.ok_or("editor session")?.session_hash;
    let slug = "published-article";
    sqlx::query(
        "INSERT INTO news(slug,title,body,published_at,title_pl,body_pl,
                      translation_published_at,translation_source_updated_at)
                 VALUES($1,'Publication','Body',now(),'Publikacja','Treść',now(),now()),
                       ('draft','Draft','Body',NULL,NULL,NULL,NULL,NULL)",
    )
    .bind(slug)
    .execute(pool)
    .await?;
    let auth =
        MutationAuthorization::new(std::sync::Arc::new(oteryn_admin::rbac::Authorizer::new(
            pool.clone(),
            oteryn_admin::web::PortalIdentity::configured(pool.clone()),
        )));
    let article_reply = Write::Reply("A reply with 2 < 3");
    assert!(matches!(
        write(
            pool,
            "en",
            "draft",
            author.account,
            &author_hash,
            Write::Reply("Draft reply")
        )
        .await,
        Err(Error::NotFound)
    ));
    assert!(matches!(
        write(
            pool,
            "en",
            slug,
            author.account,
            &author_hash,
            Write::Reply("**markup**")
        )
        .await,
        Err(Error::Invalid)
    ));
    assert!(matches!(
        write(
            pool,
            "en",
            slug,
            author.account,
            &[],
            Write::Reply("Stale session")
        )
        .await,
        Err(Error::Forbidden)
    ));
    write(
        pool,
        "en",
        slug,
        author.account,
        &author_hash,
        article_reply,
    )
    .await?;
    let id: Uuid = sqlx::query_scalar("SELECT id FROM news_discussion_replies")
        .fetch_one(pool)
        .await?;

    let app = super::super::public_router(pool.clone());
    let encoded = "A reply with 2 &#60; 3";
    for lang in ["en", "pl"] {
        let response = app
            .clone()
            .oneshot(Request::get(format!("/{lang}/news/{slug}/discussion")).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let html = axum::body::to_bytes(response.into_body(), 64 * 1024).await?;
        assert!(!String::from_utf8_lossy(&html).contains("A reply with"));
    }
    let malformed = app
        .clone()
        .oneshot(Request::get("/en/news/not_a_slug/discussion").body(Body::empty())?)
        .await?;
    assert_eq!(malformed.status(), StatusCode::NOT_FOUND);
    let edit = Edit {
        body: "A corrected reply".into(),
        revision: 1,
    };
    assert!(matches!(
        write(
            pool,
            "en",
            slug,
            editor.account,
            &editor_hash,
            Write::Edit(id, &edit)
        )
        .await,
        Err(Error::NotFound)
    ));
    let publish = Review {
        decision: Decision::Publish,
        revision: 1,
        reason: "Meets article rules".into(),
    };
    assert!(matches!(
        moderate(pool, &auth, author, id, &publish).await,
        Err(Error::Forbidden)
    ));
    moderate(pool, &auth, editor, id, &publish).await?;
    let response = app
        .clone()
        .oneshot(Request::get(format!("/pl/news/{slug}/discussion")).body(Body::empty())?)
        .await?;
    let html = axum::body::to_bytes(response.into_body(), 64 * 1024).await?;
    assert!(String::from_utf8_lossy(&html).contains(encoded));
    assert!(matches!(
        moderate(pool, &auth, editor, id, &publish).await,
        Err(Error::Conflict)
    ));

    let edit = Edit {
        revision: 2,
        ..edit
    };
    write(
        pool,
        "pl",
        slug,
        author.account,
        &author_hash,
        Write::Edit(id, &edit),
    )
    .await?;
    moderate(
        pool,
        &auth,
        editor,
        id,
        &Review {
            revision: 3,
            ..publish
        },
    )
    .await?;
    moderate(
        pool,
        &auth,
        editor,
        id,
        &Review {
            decision: Decision::Hide,
            revision: 4,
            reason: "Removed after review".into(),
        },
    )
    .await?;
    let edit = Edit {
        revision: 5,
        ..edit
    };
    assert!(matches!(
        write(
            pool,
            "en",
            slug,
            author.account,
            &author_hash,
            Write::Edit(id, &edit)
        )
        .await,
        Err(Error::Conflict)
    ));
    for _ in 1..HOURLY_LIMIT {
        write(
            pool,
            "en",
            slug,
            author.account,
            &author_hash,
            Write::Reply("Another reply"),
        )
        .await?;
    }
    assert!(matches!(
        write(
            pool,
            "en",
            slug,
            author.account,
            &author_hash,
            Write::Reply("Over quota")
        )
        .await,
        Err(Error::Limit)
    ));
    sqlx::query(
        "UPDATE news SET published_at=now()-interval '2 days',
                         translation_published_at=now()-interval '2 days',
                         expires_at=now()-interval '1 day'
         WHERE slug=$1",
    )
    .bind(slug)
    .execute(pool)
    .await?;
    let expired = app
        .clone()
        .oneshot(Request::get(format!("/en/news/{slug}/discussion")).body(Body::empty())?)
        .await?;
    assert_eq!(expired.status(), StatusCode::NOT_FOUND);
    assert!(matches!(
        write(
            pool,
            "en",
            slug,
            author.account,
            &author_hash,
            Write::Reply("Expired reply")
        )
        .await,
        Err(Error::NotFound)
    ));
    let pending: Uuid = sqlx::query_scalar(
        "SELECT id FROM news_discussion_replies WHERE status='pending' ORDER BY id LIMIT 1",
    )
    .fetch_one(pool)
    .await?;
    assert!(matches!(
        moderate(
            pool,
            &auth,
            editor,
            pending,
            &Review {
                decision: Decision::Publish,
                revision: 1,
                reason: "Expired article".into(),
            }
        )
        .await,
        Err(Error::Conflict)
    ));
    sqlx::query("UPDATE news SET expires_at=NULL WHERE slug=$1")
        .bind(slug)
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by)
                 VALUES($1,'suspension','active','Fixture suspension',now(),now()+interval '1 day',$2)")
        .bind(author.account).bind(editor.account).execute(pool).await?;
    assert!(matches!(
        write(
            pool,
            "en",
            slug,
            author.account,
            &author_hash,
            Write::Reply("Suspended reply")
        )
        .await,
        Err(Error::Forbidden)
    ));
    sqlx::query("DELETE FROM news WHERE slug=$1")
        .bind(slug)
        .execute(pool)
        .await?;
    let response = app
        .oneshot(Request::get(format!("/en/news/{slug}/discussion")).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let history: Vec<String> = sqlx::query_scalar(
        "SELECT after->>'status' FROM audit_log
        WHERE target=$1 AND action='news.discussion.moderated' ORDER BY id",
    )
    .bind(id.to_string())
    .fetch_all(pool)
    .await?;
    assert_eq!(history, ["published", "published", "hidden"]);
    Ok(())
}
