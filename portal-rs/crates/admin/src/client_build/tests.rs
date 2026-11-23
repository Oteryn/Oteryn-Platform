use super::*;

fn head(fresh: bool, registered: bool) -> Head {
    Head {
        policy: Policy {
            schema_version: 1,
            policy_revision: 1,
            channel: Channel::Stable,
            current_release_id: "release-44".into(),
            current_release_sequence: 44,
            minimum_supported_release_sequence: 42,
            revoked_release_ids: vec!["release-revoked".into()],
            artifacts: vec![Artifact {
                platform: "windows".into(),
                architecture: "x86_64".into(),
            }],
        },
        fresh,
        registered,
    }
}

#[test]
fn supported_build_and_update_threshold() -> Result<(), Invalid> {
    let publication = head(true, true);
    let form = Form::parse("release=release-43&sequence=43&minimum=43")?;
    let result = preview(&form.scenario()?, Some(&publication));
    assert_eq!(result.outcome, Outcome::Eligible);
    assert_eq!(result.minimum, Some(43));
    assert!(!result.missing_update);

    let form = Form::parse("release=release-42&sequence=42&minimum=43")?;
    let result = preview(&form.scenario()?, Some(&publication));
    assert_eq!(result.outcome, Outcome::Update);
    assert_eq!(
        result.outcome.code(),
        Some("NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED")
    );
    assert_eq!(result.current, Some(44));
    assert!(!result.missing_update);

    let form = Form::parse("release=release-44&sequence=44&minimum=45")?;
    let result = preview(&form.scenario()?, Some(&publication));
    assert!(result.missing_update);

    let form = Form::parse("release=release-43&sequence=43&minimum=45&mode=open")?;
    let result = preview(&form.scenario()?, Some(&publication));
    assert_eq!(result.outcome, Outcome::Eligible);
    assert_eq!(result.minimum, Some(42));
    Ok(())
}

#[test]
fn historical_publication_and_release_identity() -> Result<(), Invalid> {
    let form = Form::parse("release=release-43&sequence=44")?;
    assert_eq!(
        preview(&form.scenario()?, Some(&head(true, false))).outcome,
        Outcome::Unverified
    );
    let form = Form::parse("release=release-revoked&sequence=43")?;
    let result = preview(&form.scenario()?, Some(&head(false, true)));
    assert_eq!(result.outcome, Outcome::Update);
    assert_eq!(result.current, None);
    assert_eq!(result.minimum, Some(42));
    assert!(result.missing_update);
    assert_eq!(
        preview(&form.scenario()?, None).outcome,
        Outcome::Unavailable
    );
    Ok(())
}

#[test]
fn legacy_and_maintenance_decisions() -> Result<(), Invalid> {
    for (query, outcome) in [
        ("mode=open&unknown=refuse", Outcome::Eligible),
        ("mode=enforcing&unknown=admit", Outcome::Eligible),
        ("mode=enforcing&unknown=refuse", Outcome::LegacyUpdate),
        ("mode=maintenance", Outcome::LegacyMaintenance),
        (
            "mode=maintenance&release=release-44&sequence=44",
            Outcome::Maintenance,
        ),
    ] {
        let form = Form::parse(query)?;
        assert_eq!(preview(&form.scenario()?, None).outcome, outcome);
    }
    let form = Form::parse("mode=maintenance&release=release-43&sequence=43")?;
    assert_eq!(
        preview(&form.scenario()?, Some(&head(true, true))).outcome,
        Outcome::Update
    );
    Ok(())
}

#[test]
fn rejects_invalid_channel_or_sequence() {
    for query in [
        "channel=nightly",
        "channel=stable&channel=beta",
        "mode=disabled",
        "unknown=maybe",
        "release=rel",
        "sequence=3",
        "release=rel&sequence=0",
        "release=rel&sequence=9007199254740992",
        "release=rel&sequence=1.5",
        "release=rel&sequence=%2B1",
        "release=%3Cx%3E&sequence=1",
        "release=..&sequence=1",
        "minimum=-1",
        "minimum=0",
        "extra=1",
    ] {
        assert!(
            Form::parse(query)
                .and_then(|form| form.scenario().map(|_| ()))
                .is_err(),
            "{query}"
        );
    }
    assert!(
        Form::parse(&format!("release={}&sequence=1", "a".repeat(97)))
            .and_then(|form| form.scenario().map(|_| ()))
            .is_err()
    );
    assert!(Form::parse(&"a".repeat(1025)).is_err());
    assert!(matches!(
        sequence("9007199254740991"),
        Ok(Some(MAX_SEQUENCE))
    ));
}
