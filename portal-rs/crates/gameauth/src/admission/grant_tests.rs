use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
const CHANNEL: &str = "01934f10-7c03-7001-805b-3b1122334401";
const ATTEMPT: &str = "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b";
const CHARACTER: &str = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7c";
const ACCOUNT: &str = "0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a70";
const HOST: &str = "game-eu1.example.test";
/// SHA-256 of the JCS descriptor computed outside this code (contract §7.3).
const REVISION: &str = "rt.1.5c79552f74805dff23ce2cfbb8f5c0c9";

fn request() -> Result<AdmissionRequest, Error> {
    AdmissionRequest::decode(format!(r#"{{"protocol_version":2,"game_login_ticket":"synthetic-ticket","attempt_ref":"{ATTEMPT}","character_id":"{CHARACTER}","channel_id":null,"offer":{{"client_build":"0.1.0+test","client_platform":"windows","transports":[{{"protocol_major":1,"transport_profile":1,"alpn":"oteryn-game/1"}}]}}}}"#).as_bytes())
}
fn route() -> Result<Route, Box<dyn std::error::Error>> {
    Ok(Route {
        channel: CHANNEL.parse()?,
        endpoint: Endpoint {
            host: HOST.into(),
            port: 7172,
            tls_server_name: HOST.into(),
            alpn: "oteryn-game/1".into(),
            protocol_major: 1,
            transport_profile: 1,
        },
        revision: REVISION.into(),
    })
}
fn report(generation: &str, ruleset: &str) -> Value {
    let mut members = vec![
        ("scope_ownership_generation", generation),
        ("route_revision", REVISION),
        ("runtime_observation_revision", "obs.7"),
        ("ruleset_revision", ruleset),
        ("content_revision", "content.2"),
        ("map_revision", "map.3"),
        ("world_policy_revision", "policy.4"),
        ("offer_revision", "offer.5"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), Value::String(v.into())))
    .collect::<Vec<_>>();
    members.push(("ready".into(), Value::Bool(true)));
    Value::Object(members)
}
const JTI: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8";

#[test]
fn header_is_exact_and_kid_is_bounded() -> TestResult {
    assert_eq!(
        header("fresh-2026.11_a")?,
        r#"{"alg":"Ed25519","kid":"fresh-2026.11_a","typ":"oteryn-admission+jwt"}"#
    );
    for kid in ["", "a b", "a\"b", &"k".repeat(65)] {
        assert_eq!(header(kid), Err(Error::Unavailable));
    }
    let input = signing_input("k1", b"{}")?;
    assert_eq!(header_key_id(&input).as_deref(), Some("k1"));
    // A stored input whose header is not exactly §8.1 is never re-signed.
    let foreign = format!(
        "{}.e30",
        URL_SAFE_NO_PAD.encode(r#"{"kid":"k1","alg":"Ed25519","typ":"oteryn-admission+jwt"}"#)
    );
    assert_eq!(header_key_id(&foreign), None);
    assert_eq!(header_key_id("not-base64!.e30"), None);
    Ok(())
}

#[test]
fn route_revision_matches_the_contract_digest() -> TestResult {
    let world: Uuid = WORLD.parse()?;
    let channel: Uuid = CHANNEL.parse()?;
    assert_eq!(
        route_revision(1, world, channel, HOST, 7172, HOST)?,
        REVISION
    );
    // Every descriptor field and the version move the revision; scopes never share it.
    for other in [
        route_revision(2, world, channel, HOST, 7172, HOST)?,
        route_revision(1, channel, world, HOST, 7172, HOST)?,
        route_revision(1, world, channel, "game-eu2.example.test", 7172, HOST)?,
        route_revision(1, world, channel, HOST, 7173, HOST)?,
        route_revision(1, world, channel, HOST, 7172, "game-eu2.example.test")?,
    ] {
        assert_ne!(other, REVISION);
        assert!(other.len() <= 64);
    }
    Ok(())
}

#[test]
fn claims_are_exact_in_contract_order() -> TestResult {
    let request = request()?;
    let route = route()?;
    let report = report("42", "rules.1");
    let claims = Claims {
        iat: 1_800_000_000,
        ttl: 20,
        jti: JTI.into(),
        request: &request,
        account: AuthorizedAccount {
            id: ACCOUNT.parse()?,
            security_generation: 3,
        },
        world: WORLD.parse()?,
        route: &route,
        report: &report,
    };
    let expected = format!(
        r#"{{"iss":"urn:oteryn:platform:game-admission","aud":"urn:oteryn:game:admission","iat":1800000000,"nbf":1800000000,"exp":1800000020,"jti":"{JTI}","profile":"oteryn-pre-admission-v1","purpose":"fresh_entry","attempt_ref":"{ATTEMPT}","account_id":"{ACCOUNT}","character_id":"{CHARACTER}","world_id":"{WORLD}","channel_id":"{CHANNEL}","account_security_generation":"3","route_revision":"{REVISION}","runtime_observation_revision":"obs.7","scope_ownership_generation":"42","protocol_major":1,"transport_profile":1,"ruleset_revision":"rules.1","content_revision":"content.2","map_revision":"map.3","world_policy_revision":"policy.4","offer_revision":"offer.5"}}"#
    );
    assert_eq!(String::from_utf8(payload(&claims)?)?, expected);
    let input = signing_input("k1", &payload(&claims)?)?;
    assert_eq!(claim(&input, "route_revision").as_deref(), Some(REVISION));
    assert_eq!(claim(&input, "missing"), None);
    Ok(())
}

#[test]
fn claims_fail_closed_on_bad_report_values_and_bounds() -> TestResult {
    let request = request()?;
    let route = route()?;
    let account = AuthorizedAccount {
        id: ACCOUNT.parse()?,
        security_generation: 3,
    };
    for (report, ttl, generation) in [
        (report("0", "rules.1"), 20, 3),
        (report("042", "rules.1"), 20, 3),
        (report("18446744073709551616", "rules.1"), 20, 3),
        (report("1", "rules 1"), 20, 3),
        (report("1", ""), 20, 3),
        (report("1", &"r".repeat(65)), 20, 3),
        (report("1", "rules.1"), 4, 3),
        (report("1", "rules.1"), 31, 3),
        (report("1", "rules.1"), 20, 0),
    ] {
        let claims = Claims {
            iat: 1_800_000_000,
            ttl,
            jti: JTI.into(),
            request: &request,
            account: AuthorizedAccount {
                security_generation: generation,
                ..account
            },
            world: WORLD.parse()?,
            route: &route,
            report: &report,
        };
        assert_eq!(payload(&claims), Err(Error::Unavailable));
    }
    let jti = jti()?;
    assert_eq!(jti.len(), 43);
    assert_ne!(jti, super::jti()?);
    Ok(())
}

#[test]
fn issuance_is_default_off_bounded_and_never_production() {
    fn configured(pairs: &[(&str, &str)]) -> Issuance {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Issuance::configured(move |key| {
            pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
        })
    }
    assert!(!Issuance::default().permitted());
    assert!(!configured(&[("OTERYN_ENV", "testing")]).permitted());
    assert!(!configured(&[(GRANTS_ENABLED, "1"), ("OTERYN_ENV", "testing")]).permitted());
    let on = configured(&[(GRANTS_ENABLED, "true"), ("OTERYN_ENV", "testing")]);
    assert!(on.permitted());
    assert_eq!(on.ttl_seconds, DEFAULT_TTL_SECONDS);
    // Permitted but without a composed signer still refuses.
    assert!(matches!(on.signer(), Err(Error::Unavailable)));
    let preprod = Issuance {
        environment: "preprod".into(),
        ..on.clone()
    };
    assert!(preprod.permitted());
    // `oteryn_core::Environment` names preproduction `preprod` only.
    for env in ["production", "preproduction", "local", "", "Testing"] {
        let issuance = Issuance {
            environment: env.into(),
            ..on.clone()
        };
        assert!(!issuance.permitted(), "{env}");
    }
    for (ttl, expected) in [
        ("5", Some(5)),
        ("30", Some(30)),
        ("4", None),
        ("31", None),
        ("020", None),
        ("x", None),
    ] {
        let issuance = configured(&[
            (GRANTS_ENABLED, "true"),
            ("OTERYN_ENV", "testing"),
            (GRANT_TTL_SECONDS, ttl),
        ]);
        assert_eq!(issuance.permitted(), expected.is_some(), "{ttl}");
        if let Some(expected) = expected {
            assert_eq!(issuance.ttl_seconds, expected);
        }
    }
}

#[test]
fn route_names_follow_the_gateway_rules() {
    assert!(dns_name(HOST) && server_name(HOST));
    assert!(!server_name("10.0.0.1"));
    for bad in [
        "",
        "-a.test",
        "a-.test",
        "a..test",
        "a_b.test",
        &"a".repeat(64),
    ] {
        assert!(!dns_name(bad), "{bad}");
    }
}
