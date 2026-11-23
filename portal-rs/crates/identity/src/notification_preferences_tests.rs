use super::*;
use crate::privacy::test_support;

#[test]
fn languages_are_explicit() -> Result<(), serde_urlencoded::de::Error> {
    for (value, expected) in [("pl", MailLanguage::Pl), ("en", MailLanguage::En)] {
        assert_eq!(
            serde_urlencoded::from_str::<LanguageInput>(&format!("language={value}"))?.language,
            expected
        );
        assert_eq!(expected.code(), value);
    }
    assert!(serde_urlencoded::from_str::<LanguageInput>("language=de").is_err());
    Ok(())
}

#[derive(serde::Deserialize)]
struct LanguageInput {
    language: MailLanguage,
}

#[tokio::test]
async fn saves_optional_choices_and_checks_account_and_session()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(_guard) = oteryn_db::test_support::global_lock_guard().await? else {
        eprintln!("SKIPPED PostgreSQL test: PORTAL_TEST_DATABASE_URL is unset");
        return Ok(());
    };
    let (pool, admin, schema) =
        test_support::database(&std::env::var("PORTAL_TEST_DATABASE_URL")?).await?;
    let result = exercise(&pool).await;
    test_support::close(pool, admin, schema).await?;
    result
}

async fn exercise(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let account: AccountId = sqlx::query_scalar(
        "INSERT INTO accounts (email, password_hash, status, locale, email_verified_at)
         VALUES ('preferences@example.invalid', '!fixture', 'active', 'en', now()) RETURNING id",
    )
    .fetch_one(pool)
    .await?;
    let initial = current(pool, account).await?.ok_or("missing preferences")?;
    assert_eq!(initial.language, "en");
    assert_eq!(initial.choices.news, (false, true));
    assert!(!initial.choices.events);
    assert_eq!(initial.choices.operational, None);
    let choices = Choices {
        news: (true, false),
        support: (true, true),
        events: true,
        forum_digest: true,
        operational: Some(true),
    };
    let inactive_session = MutationActor::browser(account, "absent session".into());
    assert!(
        !save(
            pool,
            &inactive_session,
            Some(MailLanguage::Pl),
            Choices::default()
        )
        .await?
    );
    assert!(save(pool, &account.into(), Some(MailLanguage::Pl), choices).await?);
    let saved = current(pool, account).await?.ok_or("missing preferences")?;
    assert_eq!(saved.language, "pl");
    assert_eq!(saved.choices.news, (true, false));
    assert!(saved.choices.events && saved.choices.forum_digest);
    assert_eq!(saved.choices.operational, Some(true));
    assert!(save(pool, &account.into(), None, Choices::default()).await?);
    let saved = current(pool, account).await?.ok_or("missing preferences")?;
    assert_eq!(saved.language, "pl");
    assert_eq!(saved.choices.news, (false, false));
    assert!(saved.choices.events && saved.choices.forum_digest);
    assert_eq!(saved.choices.operational, Some(true));
    sqlx::query("UPDATE accounts SET status = 'suspended' WHERE id = $1")
        .bind(account)
        .execute(pool)
        .await?;
    assert!(current(pool, account).await?.is_none());
    assert!(
        !save(
            pool,
            &account.into(),
            Some(MailLanguage::En),
            Choices::default()
        )
        .await?
    );
    Ok(())
}
