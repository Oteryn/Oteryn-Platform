use super::*;
use crate::reset_http::tests::db;
#[tokio::test]
async fn encrypted_delivery_renders_and_refuses_tampering_and_consumed_tokens()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(db) = db::Database::open().await? else {
        return Ok(());
    };
    let key = DeliveryKey::parse(&"11".repeat(32)).ok_or("key")?;
    sqlx::query("INSERT INTO accounts(email,password_hash,locale,status) VALUES ('mail@example.test','unused','pl','active')").execute(&db.pool).await?;
    oteryn_identity::reset::request(&db.pool, "mail@example.test", &key).await?;
    let json: String =
        sqlx::query_scalar("SELECT payload::text FROM outbox WHERE kind='email.password-reset'")
            .fetch_one(&db.pool)
            .await?;
    let mut payload: Payload = serde_json::from_str(&json)?;
    let token = key.open(&payload.ciphertext).ok_or("ciphertext")?;
    assert!(!json.contains(&token));
    let mailer = Mailer {
        pool: db.pool.clone(),
        key,
        smtp: AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1").build(),
        from: "portal@example.test".parse()?,
        origin: "https://portal.example.test".into(),
    };
    let message = mailer.prepare(&payload).await?.ok_or("message")?;
    let formatted = String::from_utf8(message.formatted())?.replace("=\r\n", "");
    assert!(formatted.contains(&format!(
        "https://portal.example.test/pl/reset-password/{token}"
    )));
    payload.ciphertext[15] ^= 1;
    assert!(mailer.prepare(&payload).await.is_err());
    payload.ciphertext[15] ^= 1;
    sqlx::query("UPDATE account_password_resets SET expires_at=now()-interval '1 second'")
        .execute(&db.pool)
        .await?;
    assert!(mailer.prepare(&payload).await?.is_none());
    sqlx::query(
        "UPDATE account_password_resets SET expires_at=now()+interval '1 hour',used_at=now()",
    )
    .execute(&db.pool)
    .await?;
    assert!(mailer.prepare(&payload).await?.is_none());
    db.close().await
}
