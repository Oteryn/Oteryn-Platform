#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! §19.1-§19.3 issuer wire, byte for byte against
//! `docs/contracts/fixtures/native-gateway-protocol3-v1/`.
use super::*;
use uuid::Uuid;

const REQUEST: &str = include_str!(
    "../../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/login-request.json"
);
const SUCCESS: &str = include_str!(
    "../../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/login-success.json"
);
const CHANNEL_REFUSAL: &str = include_str!(
    "../../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/login-channel-refusal.json"
);
const ROUTE_UNAVAILABLE: &str = include_str!(
    "../../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/login-route-unavailable.json"
);
const UNSUPPORTED: &str = include_str!(
    "../../../../../docs/contracts/fixtures/native-gateway-protocol3-v1/login-unsupported-version.json"
);
const ATTEMPT: &str = "0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b";

fn channel(suffix: &str, key: &str) -> Result<wire::Channel, uuid::Error> {
    Ok(wire::Channel {
        channel_id: format!("0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a7{suffix}").parse::<Uuid>()?,
        key: key.into(),
    })
}

#[test]
fn protocol3_decodes_only_behind_its_switch_and_binds_the_version() {
    assert_eq!(
        AdmissionRequest::decode(REQUEST.as_bytes()).err(),
        Some(Error::UnsupportedVersion)
    );
    assert_eq!(
        AdmissionRequest::decode_with(REQUEST.as_bytes(), false).err(),
        Some(Error::UnsupportedVersion)
    );
    let v3 = AdmissionRequest::decode_with(REQUEST.as_bytes(), true).expect("protocol 3");
    assert_eq!(v3.protocol, 3);
    let v2 = AdmissionRequest::decode_with(
        REQUEST
            .replace("\"protocol_version\":3", "\"protocol_version\":2")
            .as_bytes(),
        true,
    )
    .expect("protocol 2");
    assert_eq!(v2.protocol, 2);
    // A retry cannot change protocol under one attempt_ref.
    assert_ne!(v2.digest(), v3.digest());
    assert_eq!(
        AdmissionRequest::decode_with(
            REQUEST
                .replace("\"protocol_version\":3", "\"protocol_version\":4")
                .as_bytes(),
            true
        )
        .err(),
        Some(Error::UnsupportedVersion)
    );
}

#[test]
fn protocol3_bodies_match_the_fixtures() -> Result<(), Box<dyn std::error::Error>> {
    let channels = vec![
        channel("1", "antica")?,
        channel("2", "obscura")?,
        channel("3", "pythera")?,
    ];
    // The stored attempt keeps the protocol 2 shaped core (§6.1).
    let core = format!(
        r#"{{"protocol_version":2,"attempt_ref":"{ATTEMPT}","world_id":"0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a70","channel_id":"0192b3c4-5d6e-7f80-8a1b-2c3d4e5f6a71","endpoint":{{"host":"antica.game.example.test","port":7172,"tls_server_name":"antica.game.example.test","alpn":"oteryn-game/1","protocol_major":1,"transport_profile":1}},"grant":{{"profile":"oteryn-pre-admission-v1","token":"eyJhbGciOiJFZDI1NTE5In0.eyJzdWIiOiJzeW50aGV0aWMifQ.c3ludGhldGlj","valid_for_seconds":20}}}}"#
    );
    let recovery = wire::Recovery {
        handle: "otr1.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8".into(),
        expires_in: 43200,
    };
    let body = wire::success3(core.as_bytes(), true, &channels, Some(&recovery))?;
    assert_eq!(String::from_utf8(body)?, SUCCESS);
    let body = wire::success3(core.as_bytes(), false, &channels, None)?;
    assert!(String::from_utf8(body)?.contains(r#""channel_selection":"requested""#));
    // 1..=16 entries.
    assert_eq!(
        wire::success3(core.as_bytes(), true, &[], None).err(),
        Some(Error::Unavailable)
    );
    assert_eq!(
        String::from_utf8(Error::ChannelUnavailable.body_for(
            Some(ATTEMPT),
            3,
            &[channel("2", "obscura")?]
        ))?,
        CHANNEL_REFUSAL
    );
    assert_eq!(
        String::from_utf8(Error::RouteUnavailable.body_for(Some(ATTEMPT), 3, &[]))?,
        ROUTE_UNAVAILABLE
    );
    assert_eq!(
        String::from_utf8(Error::UnsupportedVersion.body(Some(ATTEMPT)))?,
        UNSUPPORTED
    );
    // Protocol 2 bodies never carry a protocol 3 member.
    assert!(
        !String::from_utf8(Error::ChannelUnavailable.body_for(
            Some(ATTEMPT),
            2,
            &[channel("2", "obscura")?]
        ))?
        .contains("channels")
    );
    Ok(())
}

#[test]
fn channel_keys_labels_and_handles() {
    assert!(wire::channel_key_valid("antica"));
    assert!(wire::channel_key_valid("oteryn-staging-1"));
    for bad in ["", "-a", "Antica", "a_b", &"a".repeat(65)] {
        assert!(!wire::channel_key_valid(bad), "{bad}");
    }
    assert_eq!(
        wire::Channel {
            channel_id: Uuid::nil(),
            key: "pythera".into()
        }
        .label(),
        "Pythera"
    );
    let handle = "otr1.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8";
    assert!(protocol3::handle_hash(handle).is_some());
    for bad in [
        "otr1.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh9",
        "otr2.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
        "otr1.AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=",
        "otr1.",
    ] {
        assert!(protocol3::handle_hash(bad).is_none(), "{bad}");
    }
}

#[test]
fn protocol3_and_recovery_configuration_fail_closed() {
    use protocol3::{Handles, Protocol3};
    let config = |pairs: &[(&str, &str)]| {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Protocol3::configured(
            move |key| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()),
            crate::runtime::Settings::default(),
        )
    };
    let on = [
        (protocol3::PROTOCOL3_ENABLED, "true"),
        ("OTERYN_ENV", "testing"),
    ];
    assert!(config(&[]).is_none());
    assert!(config(&[(protocol3::PROTOCOL3_ENABLED, "true")]).is_none());
    assert!(
        config(&[
            (protocol3::PROTOCOL3_ENABLED, "true"),
            ("OTERYN_ENV", "production")
        ])
        .is_none()
    );
    assert_eq!(config(&on).map(|c| c.handles), Some(Handles::Off));
    let recovery = |extra: &[(&str, &str)], env: &str| {
        let mut pairs = vec![
            (protocol3::PROTOCOL3_ENABLED, "true"),
            (protocol3::RECOVERY_ENABLED, "true"),
            ("OTERYN_ENV", env),
        ];
        pairs.extend_from_slice(extra);
        config(&pairs).map(|c| c.handles)
    };
    let devices = (protocol3::DEVICES_ENABLED, "true");
    assert_eq!(
        recovery(&[devices], "testing"),
        Some(Handles::On {
            lifetime_seconds: 43200,
            require_device_binding: true,
            devices_enabled: true
        })
    );
    // §19.15.1: binding on with the device-session switch off keeps protocol 3
    // logins working; only handles and recovery are refused (fail closed).
    let unbound = Some(Handles::On {
        lifetime_seconds: 43200,
        require_device_binding: true,
        devices_enabled: false,
    });
    assert_eq!(recovery(&[], "testing"), unbound);
    for off in ["false", "TRUE", "1"] {
        assert_eq!(
            recovery(&[(protocol3::DEVICES_ENABLED, off)], "testing"),
            unbound,
            "{off}"
        );
        assert_eq!(
            recovery(
                &[
                    (protocol3::DEVICES_ENABLED, off),
                    (protocol3::REQUIRE_DEVICE_BINDING, "true")
                ],
                "testing"
            ),
            unbound,
            "{off}"
        );
    }
    // U21: never beyond testing.
    assert_eq!(recovery(&[devices], "preprod"), Some(Handles::Off));
    assert_eq!(recovery(&[], "preprod"), Some(Handles::Off));
    assert_eq!(
        recovery(&[(protocol3::REQUIRE_DEVICE_BINDING, "false")], "testing"),
        Some(Handles::On {
            lifetime_seconds: 43200,
            require_device_binding: false,
            devices_enabled: false
        })
    );
    assert_eq!(
        recovery(
            &[devices, (protocol3::HANDLE_LIFETIME_SECONDS, "3600")],
            "testing"
        ),
        Some(Handles::On {
            lifetime_seconds: 3600,
            require_device_binding: true,
            devices_enabled: true
        })
    );
    for bad in ["3599", "43201", "0", "+3600", "x"] {
        assert_eq!(
            recovery(
                &[devices, (protocol3::HANDLE_LIFETIME_SECONDS, bad)],
                "testing"
            ),
            Some(Handles::Invalid),
            "{bad}"
        );
    }
    assert_eq!(
        recovery(
            &[devices, (protocol3::REQUIRE_DEVICE_BINDING, "no")],
            "testing"
        ),
        Some(Handles::Invalid)
    );
}
