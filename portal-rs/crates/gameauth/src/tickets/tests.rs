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
