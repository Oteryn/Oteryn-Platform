#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use common::{ALLOW, Access, Database, SignerFixture};
use oteryn_downloads::{
    AccountState, Channel, Downloads, Error, FeatureFlags, RollbackAuthorization, router,
};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use uuid::Uuid;

const ON: FeatureFlags = FeatureFlags { downloads: true };

#[tokio::test]
async fn db_upload_is_served_byte_identical_and_history_survives_publication() {
    let Some(db) = Database::open().await else {
        return;
    };
    let signer = SignerFixture::new();
    let downloads = Downloads::new(db.pool.clone(), ON, signer.roots());
    let upload = signer.upload(signer.policy(1, "rel-a", 1));
    let publication = downloads
        .publish(&ALLOW, Uuid::now_v7(), upload.clone())
        .await
        .unwrap();
    let app = router(db.pool.clone(), ON);
    for (filename, expected) in &upload.metadata {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/updates/stable/metadata/{filename}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "application/json");
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        assert_eq!(
            response.headers()["cache-control"],
            if filename == "timestamp.json" {
                "no-store, no-transform"
            } else {
                "public, max-age=31536000, immutable, no-transform"
            }
        );
        assert_eq!(
            to_bytes(response.into_body(), 2_000_000)
                .await
                .unwrap()
                .as_ref(),
            expected
        );
    }
    for alias in ["root.json", "targets.json", "snapshot.json", "stable.json"] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/updates/stable/metadata/{alias}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["cache-control"],
            "no-store, no-transform"
        );
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    }
    let hashed_path = format!(
        "{}.channels/stable/policy-v1.json",
        hex::encode(Sha256::digest(&upload.policy))
    );
    for path in [
        "/updates/stable/policy-v1.json".to_owned(),
        format!("/updates/stable/targets/{hashed_path}"),
    ] {
        let response = app
            .clone()
            .oneshot(Request::get(&path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["cache-control"],
            if path.contains("/targets/") {
                "public, max-age=31536000, immutable, no-transform"
            } else {
                "no-store, no-transform"
            }
        );
        assert_eq!(
            to_bytes(response.into_body(), 262144)
                .await
                .unwrap()
                .as_ref(),
            upload.policy
        );
    }
    downloads
        .publish(
            &ALLOW,
            Uuid::now_v7(),
            signer.upload(signer.policy(2, "rel-b", 2)),
        )
        .await
        .unwrap();
    assert_eq!(
        downloads
            .metadata(Channel::Stable, "1.snapshot.json")
            .await
            .unwrap(),
        upload.metadata["1.snapshot.json"]
    );
    assert_eq!(
        downloads
            .policy(Channel::Stable, Some(&hashed_path))
            .await
            .unwrap(),
        upload.policy
    );
    let previous: Option<Uuid> = sqlx::query_scalar(
        "SELECT previous_publication_id FROM client_releases WHERE policy_revision = 2",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(previous, Some(publication));
    assert!(
        sqlx::query("UPDATE client_releases SET version_display = 'changed'")
            .execute(&db.pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM client_releases")
            .execute(&db.pool)
            .await
            .is_err()
    );
    db.close().await;
}

#[tokio::test]
async fn db_bad_signatures_tampering_expiry_and_wrong_channel_are_rejected() {
    let Some(db) = Database::open().await else {
        return;
    };
    let signer = SignerFixture::new();
    let downloads = Downloads::new(db.pool.clone(), ON, signer.roots());
    let good = signer.upload(signer.policy(1, "rel-a", 1));
    let mut bad = good.clone();
    let mut timestamp: serde_json::Value =
        serde_json::from_slice(&bad.metadata["timestamp.json"]).unwrap();
    timestamp["signatures"][0]["sig"] = serde_json::Value::String("00".repeat(64));
    bad.metadata.insert(
        "timestamp.json".into(),
        serde_json::to_vec(&timestamp).unwrap(),
    );
    assert!(matches!(
        downloads.publish(&ALLOW, Uuid::now_v7(), bad).await,
        Err(Error::Verification)
    ));
    for filename in [
        "1.root.json",
        "1.targets.json",
        "1.stable.json",
        "1.snapshot.json",
        "timestamp.json",
    ] {
        let mut repeated = good.clone();
        let mut envelope: serde_json::Value =
            serde_json::from_slice(&repeated.metadata[filename]).unwrap();
        let mut duplicate = envelope["signatures"][0].clone();
        duplicate["keyid"] = serde_json::json!(duplicate["keyid"].as_str().unwrap().to_uppercase());
        envelope["signatures"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        repeated
            .metadata
            .insert(filename.into(), serde_json::to_vec(&envelope).unwrap());
        assert!(matches!(
            downloads.publish(&ALLOW, Uuid::now_v7(), repeated).await,
            Err(Error::Verification)
        ));
    }
    // Hashes and parent signatures are valid; one delegated key still cannot satisfy a two-key threshold.
    let repeated = signer.duplicate_delegated_threshold(signer.policy(1, "rel-a", 1));
    assert!(matches!(
        downloads.publish(&ALLOW, Uuid::now_v7(), repeated).await,
        Err(Error::Verification)
    ));
    let mut tampered = good.clone();
    tampered.policy.push(b' ');
    assert!(matches!(
        downloads.publish(&ALLOW, Uuid::now_v7(), tampered).await,
        Err(Error::Verification)
    ));
    let expired = signer.upload_expiring(signer.policy(1, "rel-a", 1), "2000-01-01T00:00:00Z");
    assert!(matches!(
        downloads.publish(&ALLOW, Uuid::now_v7(), expired).await,
        Err(Error::Verification)
    ));
    let mut wrong_channel = good.clone();
    wrong_channel.channel = Channel::Beta;
    assert!(matches!(
        downloads
            .publish(&ALLOW, Uuid::now_v7(), wrong_channel)
            .await,
        Err(Error::Invalid(_))
    ));
    let other_signer = SignerFixture::new();
    assert!(matches!(
        downloads
            .publish(
                &ALLOW,
                Uuid::now_v7(),
                other_signer.upload(other_signer.policy(1, "rel-a", 1))
            )
            .await,
        Err(Error::Verification)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM client_releases")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        0
    );
    db.close().await;
}

#[tokio::test]
async fn db_rollback_needs_new_signed_explicit_policy_and_known_immutable_release() {
    let Some(db) = Database::open().await else {
        return;
    };
    let signer = SignerFixture::new();
    let downloads = Downloads::new(db.pool.clone(), ON, signer.roots());
    let first = signer.upload(signer.policy(1, "rel-a", 1));
    downloads
        .publish(&ALLOW, Uuid::now_v7(), first.clone())
        .await
        .unwrap();
    downloads
        .publish(
            &ALLOW,
            Uuid::now_v7(),
            signer.upload(signer.policy(2, "rel-b", 2)),
        )
        .await
        .unwrap();
    assert!(matches!(
        downloads.publish(&ALLOW, Uuid::now_v7(), first).await,
        Err(Error::Conflict)
    ));
    let mut rollback = signer.policy(3, "rel-a", 1);
    assert!(matches!(
        downloads
            .publish(&ALLOW, Uuid::now_v7(), signer.upload(rollback.clone()))
            .await,
        Err(Error::Conflict)
    ));
    rollback.rollback_authorization = RollbackAuthorization::Explicit;
    downloads
        .publish(&ALLOW, Uuid::now_v7(), signer.upload(rollback))
        .await
        .unwrap();
    let policy: oteryn_downloads::UpdaterPolicy =
        serde_json::from_slice(&downloads.policy(Channel::Stable, None).await.unwrap()).unwrap();
    assert_eq!(policy.current_release_id, "rel-a");
    assert_eq!(policy.policy_revision, 3);
    // A fresh version may not reuse a sequence that was already assigned before rollback.
    assert!(matches!(
        downloads
            .publish(
                &ALLOW,
                Uuid::now_v7(),
                signer.upload(signer.policy(4, "rel-c", 2))
            )
            .await,
        Err(Error::Conflict)
    ));
    assert!(matches!(
        downloads
            .publish(
                &ALLOW,
                Uuid::now_v7(),
                signer.upload(signer.policy(4, "rel-a", 3))
            )
            .await,
        Err(Error::Conflict)
    ));
    downloads
        .publish(
            &ALLOW,
            Uuid::now_v7(),
            signer.upload(signer.policy(4, "rel-c", 3)),
        )
        .await
        .unwrap();
    db.close().await;
}

#[tokio::test]
async fn db_account_state_permissions_flags_and_bad_input_fail_closed() {
    let Some(db) = Database::open().await else {
        return;
    };
    let signer = SignerFixture::new();
    let downloads = Downloads::new(db.pool.clone(), ON, signer.roots());
    let upload = signer.upload(signer.policy(1, "rel-a", 1));
    for access in [
        Access(AccountState::Inactive, Some(true)),
        Access(AccountState::Unknown, Some(true)),
        Access(AccountState::Ambiguous, Some(true)),
        Access(AccountState::Active, None),
        Access(AccountState::Active, Some(false)),
    ] {
        assert!(matches!(
            downloads
                .publish(&access, Uuid::now_v7(), upload.clone())
                .await,
            Err(Error::Denied)
        ));
    }
    let disabled = Downloads::new(db.pool.clone(), FeatureFlags::default(), signer.roots());
    assert!(matches!(
        disabled
            .publish(&ALLOW, Uuid::now_v7(), upload.clone())
            .await,
        Err(Error::Disabled)
    ));
    let mut bad = upload;
    bad.policy = b"{broken".to_vec();
    assert!(matches!(
        downloads.publish(&ALLOW, Uuid::now_v7(), bad).await,
        Err(Error::Invalid(_))
    ));
    let app = router(db.pool.clone(), ON);
    for (path, status) in [
        ("/updates/stable/policy-v1.json", StatusCode::NOT_FOUND),
        ("/updates/unknown/policy-v1.json", StatusCode::BAD_REQUEST),
        (
            "/updates/stable/targets/channels/beta/policy-v1.json",
            StatusCode::NOT_FOUND,
        ),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(Request::get(path).body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            status
        );
    }
    assert_eq!(
        router(db.pool.clone(), FeatureFlags::default())
            .oneshot(
                Request::get("/updates/stable/metadata/timestamp.json")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert!(matches!(
        downloads.metadata(Channel::Stable, "../root.json").await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM client_channels")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        0
    );
    db.close().await;
}

#[tokio::test]
async fn db_concurrent_publication_cannot_fork_channel_history() {
    let Some(db) = Database::open().await else {
        return;
    };
    let signer = SignerFixture::new();
    let downloads = Downloads::new(db.pool.clone(), ON, signer.roots());
    downloads
        .publish(
            &ALLOW,
            Uuid::now_v7(),
            signer.upload(signer.policy(1, "rel-a", 1)),
        )
        .await
        .unwrap();
    let (left, right) = tokio::join!(
        downloads.publish(
            &ALLOW,
            Uuid::now_v7(),
            signer.upload(signer.policy(2, "rel-b", 2))
        ),
        downloads.publish(
            &ALLOW,
            Uuid::now_v7(),
            signer.upload(signer.policy(2, "rel-c", 3))
        )
    );
    assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
    assert!(matches!(left, Err(Error::Conflict)) || matches!(right, Err(Error::Conflict)));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM client_releases")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        2
    );
    db.close().await;
}

#[tokio::test]
async fn disabled_surface_does_not_connect_to_database() {
    let pool = PgPoolForTest::lazy();
    let response = router(pool, FeatureFlags::default())
        .oneshot(
            Request::get("/updates/stable/policy-v1.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.headers()["cache-control"], "no-store");
}
struct PgPoolForTest;
impl PgPoolForTest {
    fn lazy() -> sqlx::PgPool {
        sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://portal@127.0.0.1:1/unreachable")
            .unwrap()
    }
}

#[test]
fn policy_schema_fixture_and_negative_inputs() {
    let policy: oteryn_downloads::UpdaterPolicy =
        serde_json::from_slice(include_bytes!("fixtures/policy-v1.json")).unwrap();
    policy.validate(Channel::Stable).unwrap();
    let actual: serde_json::Value = serde_json::to_value(&policy).unwrap();
    let expected: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/policy-v1.json")).unwrap();
    assert_eq!(actual, expected);
    for field in [
        "schema_version",
        "policy_revision",
        "current_release_sequence",
        "minimum_supported_release_sequence",
    ] {
        let mut bad = expected.clone();
        bad[field] = serde_json::json!("1");
        assert!(serde_json::from_value::<oteryn_downloads::UpdaterPolicy>(bad).is_err());
    }
    let mut bad = expected.clone();
    bad["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<oteryn_downloads::UpdaterPolicy>(bad).is_err());
    let mut bad = policy.clone();
    bad.minimum_supported_release_sequence = 2;
    assert!(bad.validate(Channel::Stable).is_err());
    let mut bad = policy.clone();
    bad.artifacts[0].target_path = "channels/stable/releases/../client.zip".into();
    assert!(bad.validate(Channel::Stable).is_err());
    let mut bad = policy;
    bad.artifacts.push(bad.artifacts[0].clone());
    assert!(bad.validate(Channel::Stable).is_err());
    assert!(Channel::parse("preview").is_err());
}

#[tokio::test]
async fn db_release_identity_support_floor_and_revocations_cannot_be_rewritten() {
    let Some(db) = Database::open().await else {
        return;
    };
    let signer = SignerFixture::new();
    let downloads = Downloads::new(db.pool.clone(), ON, signer.roots());
    downloads
        .publish(
            &ALLOW,
            Uuid::now_v7(),
            signer.upload(signer.policy(1, "rel-a", 1)),
        )
        .await
        .unwrap();
    let mut support = signer.policy(2, "rel-b", 2);
    support.minimum_supported_release_sequence = 2;
    support.revoked_release_ids.push("rel-a".into());
    support
        .revoked_artifact_targets
        .push("channels/stable/releases/rel-a/windows/x86_64/client.zip".into());
    downloads
        .publish(&ALLOW, Uuid::now_v7(), signer.upload(support.clone()))
        .await
        .unwrap();
    let head = downloads.policy(Channel::Stable, None).await.unwrap();
    // Clearing revocations and lowering the support floor both fail, including when signed.
    assert!(matches!(
        downloads
            .publish(
                &ALLOW,
                Uuid::now_v7(),
                signer.upload(signer.policy(3, "rel-c", 3))
            )
            .await,
        Err(Error::Conflict)
    ));
    support.policy_revision = 3;
    support.artifacts[0].platform = "linux".into();
    assert!(matches!(
        downloads
            .publish(&ALLOW, Uuid::now_v7(), signer.upload(support))
            .await,
        Err(Error::Conflict)
    ));
    assert_eq!(downloads.policy(Channel::Stable, None).await.unwrap(), head);
    assert!(matches!(
        downloads.policy(Channel::Beta, None).await,
        Err(Error::NotFound)
    ));
    let mut beta_policy = signer.policy(1, "beta-a", 1);
    beta_policy.channel = Channel::Beta;
    beta_policy.artifacts[0].target_path =
        "channels/beta/releases/beta-a/windows/x86_64/client.zip".into();
    let beta = signer.upload(beta_policy);
    downloads
        .publish(&ALLOW, Uuid::now_v7(), beta.clone())
        .await
        .unwrap();
    assert_eq!(
        downloads.policy(Channel::Beta, None).await.unwrap(),
        beta.policy
    );
    assert_eq!(downloads.policy(Channel::Stable, None).await.unwrap(), head);
    assert!(matches!(
        downloads.metadata(Channel::Stable, "1.beta.json").await,
        Err(Error::NotFound)
    ));

    db.close().await;
}
