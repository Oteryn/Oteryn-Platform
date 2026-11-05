use super::*;
type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn fixture(collect: bool) -> TestResult<(Uuid, CommandRequest, Allocation, String)> {
    let bytes = if collect {
        include_str!("../tests/fixtures/collect.json")
    } else {
        include_str!("../tests/fixtures/start.json")
    };
    let value: serde_json::Value = serde_json::from_str(bytes)?;
    let id = |key: &str| -> TestResult<Uuid> {
        Ok(value[key]
            .as_str()
            .ok_or("missing fixture field")?
            .parse()?)
    };
    let revision = |key: &str| -> TestResult<Positive> {
        Ok(Positive::new(
            value[key]
                .as_str()
                .ok_or("missing fixture field")?
                .parse()?,
        )?)
    };
    let action = if collect {
        Action::CollectHirelingExpedition(CollectBinding {
            expedition_id: id("expedition_id")?,
            expected_expedition_revision: revision("expected_expedition_revision")?,
        })
    } else {
        Action::StartHirelingExpedition(StartBinding {
            reward_table_key: value["reward_table_key"]
                .as_str()
                .ok_or("missing fixture field")?
                .into(),
            reward_table_revision: revision("reward_table_revision")?,
            reward_row_key: value["reward_row_key"]
                .as_str()
                .ok_or("missing fixture field")?
                .into(),
        })
    };
    Ok((
        id("account_id")?,
        CommandRequest {
            idempotency_key: Uuid::now_v7(),
            character_id: id("character_id")?,
            world_id: id("world_id")?,
            hireling_id: id("hireling_id")?,
            expected_character_revision: revision("expected_character_revision")?,
            expected_hireling_revision: revision("expected_hireling_revision")?,
            action,
        },
        Allocation {
            operation: id("operation_id")?,
            decision: id("issuer_decision_id")?,
            revision: revision("source_revision")?,
            now: revision("issued_at_source")?.get() as i64,
        },
        bytes.into(),
    ))
}
fn proof(account: Uuid, r: &CommandRequest, now: i64) -> Preflight {
    Preflight {
        source_authority: "oteryn:game:primary".into(),
        account_id: account,
        character_id: r.character_id,
        world_id: r.world_id,
        hireling_id: r.hireling_id,
        character_revision: r.expected_character_revision,
        hireling_revision: r.expected_hireling_revision,
        complete_through: now,
        baseline_and_recovery: Evidence::Confirmed,
        active_account_and_security: Evidence::Confirmed,
        current_ownership_and_privacy: Evidence::Confirmed,
        placed_in_owned_house: Evidence::Confirmed,
        character_offline: Evidence::Confirmed,
        approved_non_currency_quote: Evidence::Confirmed,
        chest_capacity_available: Evidence::Confirmed,
        weekly_budget_available: Evidence::Confirmed,
        unfinished_account_expeditions: Some(0),
        state: match &r.action {
            Action::StartHirelingExpedition(b) => ObservedState::Ready(b.clone()),
            Action::CollectHirelingExpedition(b) => ObservedState::ReadyToCollect(b.clone()),
        },
    }
}
#[test]
fn exact_contract_envelopes_and_digests() -> TestResult {
    for collect in [false, true] {
        let (account, r, allocation, bytes) = fixture(collect)?;
        let encoded = match &r.action {
            Action::StartHirelingExpedition(b) => encode(account, &r, b, allocation),
            Action::CollectHirelingExpedition(b) => encode(account, &r, b, allocation),
        }?;
        assert_eq!(encoded, bytes);
        assert_eq!(
            digest(&encoded),
            if collect {
                "f3a54b33b50f12101556feeeb78358efe3044da465cabaaa47b3c0de794f0a18"
            } else {
                "089c2b002d357da3743fcae5b0703a4d9efb630a6ee28fec5db041e7812b8538"
            }
        );
        assert!(!encoded.contains("idempotency_key"));
    }
    Ok(())
}
#[test]
fn strict_input_types_and_bounds() -> TestResult {
    let (account, mut r, _, _) = fixture(false)?;
    assert!(r.validate(account).is_ok());
    for invalid in [
        "0",
        "01",
        "+1",
        "-1",
        "1.0",
        "1e1",
        "18446744073709551616",
        "",
    ] {
        assert!(serde_json::from_value::<Positive>(serde_json::json!(invalid)).is_err());
    }
    for invalid in [
        serde_json::json!(1),
        serde_json::Value::Null,
        serde_json::json!(true),
    ] {
        assert!(serde_json::from_value::<Positive>(invalid).is_err());
    }
    assert_eq!(
        serde_json::from_str::<Positive>("\"18446744073709551615\"")?.get(),
        u64::MAX
    );
    assert!(Positive::new(0).is_err());
    for invalid in ["", "Upper", "a/b", "a\\b", "a\n", &"a".repeat(65)] {
        if let Action::StartHirelingExpedition(b) = &mut r.action {
            b.reward_row_key = invalid.into();
        }
        assert_eq!(r.validate(account), Err(Reject::InvalidInput));
    }
    let (_, r, _, _) = fixture(false)?;
    let mut value = serde_json::to_value(&r)?;
    value["account_id"] = serde_json::json!(account);
    assert!(serde_json::from_value::<CommandRequest>(value).is_err());
    let mut value = serde_json::to_value(&r)?;
    value["action"]["binding"]["reward_quantity"] = serde_json::json!(9999);
    assert!(serde_json::from_value::<CommandRequest>(value).is_err());
    let mut value = serde_json::to_value(&r)?;
    value["action"]["command"] = serde_json::json!("CancelHirelingExpedition");
    assert!(serde_json::from_value::<CommandRequest>(value).is_err());
    let mut invalid = r;
    invalid.idempotency_key = Uuid::nil();
    assert_eq!(invalid.validate(account), Err(Reject::InvalidInput));
    invalid.idempotency_key = "0192a000-0000-4000-8000-000000000001".parse()?;
    assert_eq!(invalid.validate(account), Err(Reject::InvalidInput));
    let (_, mut r, _, _) = fixture(true)?;
    if let Action::CollectHirelingExpedition(b) = &mut r.action {
        b.expedition_id = Uuid::nil();
    }
    assert_eq!(r.validate(account), Err(Reject::InvalidInput));
    Ok(())
}
#[test]
fn fresh_authorized_preflight_and_typed_refusals() -> TestResult {
    for collect in [false, true] {
        let (account, r, a, _) = fixture(collect)?;
        let p = proof(account, &r, a.now);
        let check = |p: &Preflight| p.authorize("oteryn:game:primary", account, &r, a.now, false);
        assert_eq!(check(&p), Ok(()));
        for state in [ObservedState::Unknown, ObservedState::Ambiguous] {
            let mut bad = p.clone();
            bad.state = state;
            assert_eq!(check(&bad), Err(Reject::Unavailable));
        }
        for evidence in [Evidence::Unknown, Evidence::Ambiguous, Evidence::Denied] {
            let mut bad = p.clone();
            bad.current_ownership_and_privacy = evidence;
            assert!(check(&bad).is_err());
            let mut bad = p.clone();
            bad.baseline_and_recovery = evidence;
            assert!(check(&bad).is_err());
            let mut bad = p.clone();
            bad.active_account_and_security = evidence;
            assert!(check(&bad).is_err());
            let mut bad = p.clone();
            bad.placed_in_owned_house = evidence;
            assert!(check(&bad).is_err());
        }
        let mut bad = p.clone();
        bad.character_offline = Evidence::Denied;
        assert_eq!(check(&bad), Err(Reject::CharacterInSession));
        bad = p.clone();
        bad.character_revision = Positive::new(99)?;
        assert_eq!(check(&bad), Err(Reject::StalePrecondition));
        bad = p.clone();
        bad.hireling_revision = Positive::new(99)?;
        assert_eq!(check(&bad), Err(Reject::StalePrecondition));
        for skew in [-30, 2] {
            bad = p.clone();
            bad.complete_through += skew;
            assert_eq!(check(&bad), Err(Reject::Unavailable));
        }
        bad = p.clone();
        bad.account_id = Uuid::now_v7();
        assert_eq!(check(&bad), Err(Reject::NotEligible));
        bad = p.clone();
        bad.source_authority = "other".into();
        assert_eq!(check(&bad), Err(Reject::Unavailable));
        bad = p.clone();
        bad.state = ObservedState::Running;
        assert_eq!(
            check(&bad),
            Err(if collect {
                Reject::NotReturned
            } else {
                Reject::Busy
            })
        );
        if collect {
            for (evidence, reason) in [
                (Evidence::Denied, Reject::ChestFull),
                (Evidence::Unknown, Reject::Unavailable),
                (Evidence::Ambiguous, Reject::Unavailable),
            ] {
                let mut full = p.clone();
                full.chest_capacity_available = evidence;
                assert_eq!(check(&full), Err(reason));
            }
            let mut missing = p.clone();
            missing.approved_non_currency_quote = Evidence::Unknown;
            assert_eq!(check(&missing), Err(Reject::Unavailable));
            bad.state = ObservedState::Collected;
            assert_eq!(check(&bad), Err(Reject::AlreadyCollected));
        } else {
            bad = p.clone();
            bad.weekly_budget_available = Evidence::Denied;
            assert_eq!(check(&bad), Err(Reject::WeeklyLimit));
            bad = p.clone();
            bad.unfinished_account_expeditions = Some(3);
            assert_eq!(check(&bad), Err(Reject::Busy));
            bad.unfinished_account_expeditions = None;
            assert_eq!(check(&bad), Err(Reject::Unavailable));
            bad = p.clone();
            bad.approved_non_currency_quote = Evidence::Unknown;
            assert_eq!(check(&bad), Err(Reject::Unavailable));
        }
        // Retry after a mutable state/revision change keeps the original result.
        bad = p.clone();
        bad.state = ObservedState::Collected;
        bad.hireling_revision = Positive::new(99)?;
        assert_eq!(
            bad.authorize("oteryn:game:primary", account, &r, a.now, true),
            Ok(())
        );
        bad.current_ownership_and_privacy = Evidence::Denied;
        assert_eq!(
            bad.authorize("oteryn:game:primary", account, &r, a.now, true),
            Err(Reject::NotEligible)
        );
    }
    let flags = CommandFlags::default();
    assert!(
        !flags.projection_enabled
            && !flags.commands_enabled
            && !flags.start_enabled
            && !flags.collect_enabled
    );
    Ok(())
}
