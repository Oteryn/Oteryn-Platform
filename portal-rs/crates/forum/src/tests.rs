use crate::{
    mail::Origins,
    model::{self, Audience, Error, Paging},
    player, view,
};
use sqlx::{AssertSqlSafe, PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn validates_forum_form_values() {
    assert!(matches!(model::handle("a@b.com"), Err(Error::Invalid)));
    assert_eq!(model::handle("  Hero_123  ").ok(), Some("Hero_123"));
    assert!(matches!(
        model::text(" \n \t", 12, true),
        Err(Error::Invalid)
    ));
    let last = Paging {
        page: 10_000,
        quote: None,
        report: None,
        lang: None,
    };
    assert!(last.offset().is_ok());
    assert!(!last.has_next(21));
    assert!(matches!(model::page_number(10_001), Err(Error::Invalid)));
    assert!(matches!(
        model::text("one\ntwo", 12, false),
        Err(Error::Invalid)
    ));
    assert!(matches!(
        model::text("ę".repeat(161).as_str(), 160, false),
        Err(Error::Invalid)
    ));
    assert!(model::text("ę".repeat(160).as_str(), 160, false).is_ok());
    assert!(matches!(
        Paging {
            page: 0,
            quote: None,
            report: None,
            lang: None
        }
        .offset(),
        Err(Error::Invalid)
    ));
}

#[test]
fn localized_keys_are_complete() {
    let keys = |catalogue: &'static str| -> std::collections::BTreeSet<_> {
        catalogue
            .lines()
            .filter_map(|line| line.split_once(" = ").map(|(key, _)| key))
            .collect()
    };
    let en = keys(include_str!("../../../templates/forum/en.ftl"));
    assert!(!en.is_empty());
    assert_eq!(en, keys(include_str!("../../../templates/forum/pl.ftl")));
    for key in en {
        assert!(!view::text("en", key).is_empty(), "{key}");
        assert!(!view::text("pl", key).is_empty(), "{key}");
    }
}

#[tokio::test]
async fn forum_posting_visibility_editing_and_batch_delivery_state() -> Result {
    let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
        return Ok(());
    };
    let _guard = oteryn_db::test_support::global_lock_guard().await?;
    let root = PgPool::connect(&url).await?;
    let schema = format!("forum_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&root)
        .await?;
    let namespace = schema.clone();
    let pool = PgPoolOptions::new()
        .after_connect(move |conn, _| {
            let namespace = namespace.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false)")
                    .bind(namespace)
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await?;
    let result = exercise(&pool).await;
    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&root)
        .await?;
    result
}

async fn exercise(pool: &PgPool) -> Result {
    oteryn_db::migrate(pool).await?;
    let mut tx = pool.begin().await?;
    let owner = account(&mut tx, 2, true).await?;
    let reader = account(&mut tx, 2, true).await?;
    let new_account = account(&mut tx, 0, true).await?;
    let unverified = account(&mut tx, 2, false).await?;
    let board: Uuid = sqlx::query_scalar(
        "INSERT INTO forum_boards(name_pl,name_en,position,visibility)
         VALUES('Dyskusje','Discussions',100,'public') RETURNING id",
    )
    .fetch_one(&mut *tx)
    .await?;
    let restricted: Uuid = sqlx::query_scalar(
        "INSERT INTO forum_boards(name_pl,name_en,position,visibility)
         VALUES('Zespół','Staff',200,'staff') RETURNING id",
    )
    .fetch_one(&mut *tx)
    .await?;
    assert!(matches!(
        model::board(&mut tx, Audience::Player(owner), restricted).await,
        Err(Error::NotFound)
    ));
    assert!(
        model::board(&mut tx, Audience::Staff(owner), restricted)
            .await
            .is_ok()
    );

    for account in [new_account, unverified] {
        assert!(matches!(
            model::posting_account(&mut tx, account).await,
            Err(Error::Ineligible)
        ));
    }
    assert!(matches!(
        model::posting_account(&mut tx, owner).await,
        Err(Error::HandleRequired)
    ));
    player::save_handle(&mut tx, owner, "PartyLeader").await?;
    model::posting_account(&mut tx, owner).await?;
    let thread = model::create_thread(&mut tx, board, owner, "Quest planning").await?;
    let (first, _) = model::add_post(&mut tx, thread, owner, "First post", None).await?;
    assert!(matches!(
        model::edit_own(&mut tx, thread, first, reader, "Another member", 1).await,
        Err(Error::Forbidden)
    ));
    model::edit_own(&mut tx, thread, first, owner, "Quest starts at noon", 1).await?;
    assert!(matches!(
        model::edit_own(&mut tx, thread, first, owner, "Old draft", 1).await,
        Err(Error::Conflict)
    ));
    let posts = model::posts(&mut tx, Audience::Guest, thread, 0).await?;
    assert_eq!(posts[0].author.as_deref(), Some("PartyLeader"));
    assert!(posts[0].edited_at.is_some());

    let mail = Origins::configured(Some("https://portal.example.test"), None);
    assert!(player::subscription(&mut tx, &mail, "public", reader, thread, true).await?);
    let window: time::OffsetDateTime = sqlx::query_scalar(
        "SELECT date_trunc('minute', clock_timestamp()) + interval '1 minute 1 second'",
    )
    .fetch_one(&mut *tx)
    .await?;
    for _ in 0..4 {
        model::posting_account(&mut tx, owner).await?;
        let (post, _) = model::add_post(&mut tx, thread, owner, "A new reply", Some(first)).await?;
        mail.queue(&mut tx, thread, owner, window, "public").await?;
        assert_eq!(
            model::permalink(&mut tx, Audience::Guest, thread, post).await?,
            1
        );
    }
    assert!(matches!(
        model::posting_account(&mut tx, owner).await,
        Err(Error::Limit)
    ));
    let jobs: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM forum_mail_batches m JOIN outbox o ON o.idempotency_key=m.email_id::text
          WHERE m.thread_id=$1 AND m.account_id=$2 AND o.kind='notify.email'",
    ).bind(thread).bind(reader).fetch_one(&mut *tx).await?;
    assert_eq!(jobs, 1);
    let topics = model::topics(&mut tx, Audience::Player(reader), board, 0).await?;
    assert!(topics[0].unread);
    model::mark_read(&mut tx, reader, thread, 5).await?;
    model::mark_read(&mut tx, reader, thread, 2).await?;
    let topics = model::topics(&mut tx, Audience::Player(reader), board, 0).await?;
    assert!(!topics[0].unread);

    sqlx::query(
        "UPDATE forum_posts SET created_at=clock_timestamp()-interval '15 minutes' WHERE id=$1",
    )
    .bind(first)
    .execute(&mut *tx)
    .await?;
    assert!(matches!(
        model::edit_own(&mut tx, thread, first, owner, "Late draft", 2).await,
        Err(Error::Forbidden)
    ));
    tx.commit().await?;
    for _ in 0..30 {
        model::charge_mutation(pool, owner).await?;
    }
    assert!(matches!(
        model::charge_mutation(pool, owner).await,
        Err(Error::Limit)
    ));
    sqlx::query(
        "UPDATE forum_mutation_limits SET window_starts=clock_timestamp()-interval '1 minute'
         WHERE account_id=$1",
    )
    .bind(owner)
    .execute(pool)
    .await?;
    model::charge_mutation(pool, owner).await?;
    Ok(())
}

async fn account(db: &mut sqlx::PgConnection, days: i32, verified: bool) -> Result<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO accounts(id,email,password_hash,status,locale,created_at,email_verified_at)
         VALUES($1,$2,'test-hash',$3,'en',clock_timestamp()-make_interval(days=>$4),
                CASE WHEN $5 THEN clock_timestamp() ELSE NULL END)",
    )
    .bind(id)
    .bind(format!("{id}@example.invalid"))
    .bind(if verified {
        "active"
    } else {
        "pending_verification"
    })
    .bind(days)
    .bind(verified)
    .execute(db)
    .await?;
    Ok(id)
}
