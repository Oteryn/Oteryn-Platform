use super::*;
#[test]
fn account_state_and_generations_fail_closed() {
    let account_id = Uuid::now_v7();
    for status in [AccountStatus::Suspended, AccountStatus::Unknown] {
        assert_eq!(
            validate(Principal {
                account_id,
                status,
                game_generation: 1,
                native_generation: 1
            }),
            Err(Error::Unauthorized)
        );
    }
    assert_eq!(
        validate(Principal {
            account_id,
            status: AccountStatus::Active,
            game_generation: 1,
            native_generation: 0
        }),
        Err(Error::Unauthorized)
    );
    assert!(
        validate(Principal {
            account_id,
            status: AccountStatus::Active,
            game_generation: 1,
            native_generation: 1
        })
        .is_ok()
    );
}
#[test]
fn feature_is_off_by_default() {
    assert!(!Settings::default().enabled);
}
#[test]
fn ticket_response_matches_game_golden_shape() -> Result<(), serde_json::Error> {
    // SOURCE: Oteryn/Oteryn-Game@857f15f376907c592bb28d854fccff18a15cdf87,
    // crates/platform-client/src/native_login/tests.rs, ticket response vector.
    let issued = Issued {
        protocol_version: 1,
        ticket: "ticket-1".into(),
        expires_in: 60,
    };
    assert_eq!(
        serde_json::to_string(&issued)?,
        r#"{"protocol_version":1,"ticket":"ticket-1","expires_in":60}"#
    );
    assert!(!format!("{issued:?}").contains("ticket-1"));
    Ok(())
}
#[tokio::test]
async fn budget_retention_prunes_expired_windows_in_bounded_batches()
-> crate::runtime::tests::TestResult {
    let Some(db) = crate::runtime::tests::Database::new().await? else {
        return Ok(());
    };
    // One more expired row than a batch holds.
    sqlx::query("INSERT INTO game_ticket_budgets (credential_hash, window_start, requests) SELECT sha256(convert_to('old:' || n, 'UTF8')), clock_timestamp() - interval '11 minutes', 3 FROM generate_series(1, $1) n")
        .bind(BudgetRetention::BATCH + 1)
        .execute(&db.pool)
        .await?;
    // An active window and one that lapsed recently both stay.
    sqlx::query("INSERT INTO game_ticket_budgets (credential_hash, window_start, requests) VALUES (sha256('\\x01'::bytea), clock_timestamp() - interval '30 seconds', 4), (sha256('\\x02'::bytea), clock_timestamp() - interval '5 minutes', 2)")
        .execute(&db.pool)
        .await?;
    let retention = BudgetRetention {
        pool: db.pool.clone(),
    };
    assert_eq!(retention.prune().await?, BudgetRetention::BATCH as u64);
    assert_eq!(retention.prune().await?, 1);
    assert_eq!(retention.prune().await?, 0);
    let kept: Vec<i64> =
        sqlx::query_scalar("SELECT requests FROM game_ticket_budgets ORDER BY requests DESC")
            .fetch_all(&db.pool)
            .await?;
    // The active window keeps its count: nothing was reset.
    assert_eq!(kept, vec![4, 2]);
    db.close().await
}
