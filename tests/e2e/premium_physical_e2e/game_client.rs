//! Ephemeral physical E2E driver copied into the pinned Oteryn-Game checkout by Platform CI.
//! It exercises the real Game PremiumSnapshotClient and snapshot validator; it is not a mock producer.

use oteryn_game_server::premium::client::{PremiumClientConfig, PremiumSnapshotClient, PullFailure, fresh_nonce};
use oteryn_game_server::premium::snapshot::validate;
use std::{env, fs};

fn required(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("missing {name}"))
}

fn account_bytes(value: &str) -> [u8; 16] {
    let compact: String = value.chars().filter(|c| *c != '-').collect();
    assert_eq!(compact.len(), 32, "account id must be a UUID");
    let mut out = [0u8; 16];
    for (index, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&compact[index * 2..index * 2 + 2], 16)
            .expect("account id must be lowercase hexadecimal");
    }
    out
}

fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(future)
}

#[test]
fn physical_platform_premium_endpoint() {
    let account = account_bytes(&required("PREMIUM_E2E_ACCOUNT"));
    let expected = required("PREMIUM_E2E_EXPECT");
    let platform_revision = required("PREMIUM_E2E_PLATFORM_REVISION");
    let config = PremiumClientConfig {
        origin: required("PREMIUM_E2E_ORIGIN"),
        identity_pem: fs::read(required("PREMIUM_E2E_IDENTITY_PEM")).expect("client identity"),
        platform_ca_pem: fs::read(required("PREMIUM_E2E_PLATFORM_CA_PEM")).expect("Platform CA"),
    };
    let client = PremiumSnapshotClient::new(&config).expect("real Game Premium client config");
    let nonce = fresh_nonce().expect("fresh request nonce");
    let result = block_on(client.pull(account, &nonce));

    match expected.as_str() {
        "503" | "404" => match result {
            Err(PullFailure::Status { status, .. }) => {
                assert_eq!(status.to_string(), expected, "unexpected HTTP refusal");
            }
            other => panic!("expected HTTP {expected}, got {other:?}"),
        },
        state => {
            let body = result.expect("snapshot body");
            let evidence = validate(&body, account, &nonce).expect("real Game snapshot validation");
            assert_eq!(format!("{:?}", evidence.state), state);
            assert_eq!(evidence.producer_revision, platform_revision);
        }
    }
}
