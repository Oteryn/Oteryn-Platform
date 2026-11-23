use super::staff::{Error, Kind, Settings, save};
use oteryn_admin::{
    rbac::{Authorizer, Error as AuthorityError},
    web::PortalIdentity,
};
use uuid::Uuid;

#[test]
fn menu_keys_and_position_are_validated() {
    let mut settings = Settings {
        placement: "world".into(),
        menu_key: "editor-calendar".into(),
        menu_label_en: "Calendar".into(),
        menu_label_pl: "Kalendarz".into(),
        ..Settings::default()
    };
    assert!(settings.valid());
    for key in ["javascript:open(1)", "//example.invalid", "bad/menu", ""] {
        settings.menu_key = key.into();
        assert!(!settings.valid());
    }
    settings.menu_key = "calendar".into();
    settings.menu_position = 9;
    assert!(!settings.valid());
}

#[tokio::test]
async fn editorial_discovery_lifecycle() -> crate::last_wait_tests::TestResult {
    if std::env::var_os("PORTAL_TEST_DATABASE_URL").is_none() {
        return Ok(());
    }
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = crate::last_wait_tests::TestDb::open().await?;
    let auth = Authorizer::new(db.pool.clone(), PortalIdentity::configured(db.pool.clone()));
    let published: Uuid = sqlx::query_scalar(
        "INSERT INTO news(slug,title,body,published_at,title_pl,body_pl,translation_published_at,translation_source_updated_at)
         VALUES ('calendar','Calendar','English',now(),'Kalendarz','Polski',now(),now()) RETURNING id",
    )
    .fetch_one(&db.pool)
    .await?;
    let second: Uuid = sqlx::query_scalar(
        "INSERT INTO news(slug,title,body,published_at,title_pl,body_pl,translation_published_at,translation_source_updated_at)
         VALUES ('second','Updates','English',now(),'Aktualności','Polski',now(),now()) RETURNING id",
    )
    .fetch_one(&db.pool)
    .await?;
    let draft: Uuid = sqlx::query_scalar(
        "INSERT INTO news(slug,title,body,published_at)
         VALUES ('draft','Draft','English',now()+interval '1 day') RETURNING id",
    )
    .fetch_one(&db.pool)
    .await?;
    let settings = Settings {
        placement: "community".into(),
        menu_key: "calendar".into(),
        menu_label_en: "Calendar".into(),
        menu_label_pl: "Kalendarz".into(),
        title_en: "Editorial calendar".into(),
        ..Settings::default()
    };
    let start_page: Uuid = sqlx::query_scalar(
        "INSERT INTO pages (id, slug) VALUES (uuidv7(), 'getting-started') RETURNING id",
    )
    .fetch_one(&db.pool)
    .await?;
    assert!(matches!(
        save(
            &db.pool,
            &auth,
            db.context,
            Kind::Pages,
            start_page,
            &Settings::default()
        )
        .await,
        Err(Error::Missing)
    ));
    assert!(matches!(
        save(&db.pool, &auth, db.context, Kind::News, draft, &settings).await,
        Err(Error::Unpublished)
    ));
    save(
        &db.pool,
        &auth,
        db.context,
        Kind::News,
        published,
        &settings,
    )
    .await?;
    let saved: (i64, String, i64) = sqlx::query_as(
        "SELECT revision,title_en,(SELECT count(*) FROM editorial_publication p WHERE p.document_id=d.document_id)
         FROM editorial_discovery d WHERE document_id=$1",
    )
    .bind(published)
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(saved, (1, "Editorial calendar".into(), 2));

    let duplicate_key = Settings {
        menu_position: 2,
        ..settings.clone()
    };
    assert!(matches!(
        save(
            &db.pool,
            &auth,
            db.context,
            Kind::News,
            second,
            &duplicate_key
        )
        .await,
        Err(Error::Duplicate)
    ));
    let duplicate_position = Settings {
        menu_key: "updates".into(),
        ..settings.clone()
    };
    assert!(matches!(
        save(
            &db.pool,
            &auth,
            db.context,
            Kind::News,
            second,
            &duplicate_position
        )
        .await,
        Err(Error::Duplicate)
    ));
    assert!(matches!(
        save(
            &db.pool,
            &auth,
            db.context,
            Kind::News,
            published,
            &settings
        )
        .await,
        Err(Error::Stale)
    ));

    sqlx::query("DELETE FROM account_roles WHERE account_id=$1")
        .bind(db.context.account)
        .execute(&db.pool)
        .await?;
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='security_admin'")
        .bind(db.context.account)
        .execute(&db.pool)
        .await?;
    assert!(matches!(
        save(
            &db.pool,
            &auth,
            db.context,
            Kind::News,
            second,
            &Settings::default()
        )
        .await,
        Err(Error::Authorization(AuthorityError::Denied))
    ));
    let audit_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='cms.discovery_saved'")
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(audit_count, 1);
    sqlx::query("UPDATE news SET updated_at=clock_timestamp() WHERE id=$1")
        .bind(published)
        .execute(&db.pool)
        .await?;
    let paths: Vec<String> =
        sqlx::query_scalar("SELECT path FROM editorial_publication WHERE document_id=$1")
            .bind(published)
            .fetch_all(&db.pool)
            .await?;
    assert_eq!(paths, ["/en/news/calendar"]);
    sqlx::query("DELETE FROM news WHERE id=$1")
        .bind(published)
        .execute(&db.pool)
        .await?;
    let retained: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editorial_discovery WHERE document_id=$1)")
            .bind(published)
            .fetch_one(&db.pool)
            .await?;
    assert!(!retained);
    db.close().await
}

#[tokio::test]
async fn sitemap_splits_large_catalogues() -> crate::last_wait_tests::TestResult {
    use oteryn_web::seo::EditorialSitemap;

    if std::env::var_os("PORTAL_TEST_DATABASE_URL").is_none() {
        return Ok(());
    }
    let _database_guard = oteryn_db::test_support::global_lock_guard().await?;
    let db = crate::last_wait_tests::TestDb::open().await?;
    sqlx::query(
        "INSERT INTO news(slug,title,body,published_at)
         SELECT 'discovery-' || n, 'News', 'Body', now() FROM generate_series(1,1001) n",
    )
    .execute(&db.pool)
    .await?;
    assert!(matches!(
        super::public::sitemap(&db.pool, None).await,
        Ok(EditorialSitemap::Index(2))
    ));
    let first = super::public::sitemap(&db.pool, Some(1)).await;
    let second = super::public::sitemap(&db.pool, Some(2)).await;
    match (first, second) {
        (
            Ok(EditorialSitemap::Pages {
                rows: first,
                include_static: true,
            }),
            Ok(EditorialSitemap::Pages {
                rows: second,
                include_static: false,
            }),
        ) => {
            assert_eq!(first.len(), 1_000);
            assert_eq!(second.len(), 1);
            assert!(first.iter().all(|row| row.en != second[0].en));
        }
        result => return Err(format!("unexpected sitemap documents: {result:?}").into()),
    }
    db.close().await
}
