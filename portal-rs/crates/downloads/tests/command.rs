#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;
use common::{Database, SignerFixture};
use oteryn_downloads::{Channel, Error, FeatureFlags, command::Command};
use uuid::Uuid;

#[tokio::test]
async fn db_command_success_refusals_and_bad_input() {
    let Some(db) = Database::open().await else {
        return;
    };
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1,'publisher@example.invalid','unused','active','en')").bind(actor).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,role_id FROM role_permissions WHERE permission='downloads.manage' LIMIT 1").bind(actor).execute(&db.pool).await.unwrap();
    let dir = std::env::temp_dir().join(format!("update-origin-{}", Uuid::now_v7()));
    std::fs::create_dir_all(dir.join("metadata")).unwrap();
    let signer = SignerFixture::new();
    let upload = signer.upload(signer.policy(1, "rel-a", 1));
    for (name, bytes) in &upload.metadata {
        std::fs::write(dir.join("metadata").join(name), bytes).unwrap();
    }
    std::fs::write(dir.join("policy-v1.json"), &upload.policy).unwrap();
    let root = dir.join("root.json");
    std::fs::write(&root, &signer.roots()[&Channel::Stable]).unwrap();
    let command = Command::parse([
        "--channel".into(),
        "stable".into(),
        "--actor".into(),
        actor.to_string(),
        "--dir".into(),
        dir.to_str().unwrap().into(),
    ])
    .unwrap();
    let on = FeatureFlags { downloads: true };
    assert!(matches!(
        command
            .execute(db.pool.clone(), FeatureFlags::default(), &root)
            .await,
        Err(Error::Disabled)
    ));
    std::fs::write(&root, &SignerFixture::new().roots()[&Channel::Stable]).unwrap();
    assert!(matches!(
        command.execute(db.pool.clone(), on, &root).await,
        Err(Error::Verification)
    ));
    std::fs::write(&root, &signer.roots()[&Channel::Stable]).unwrap();
    let timestamp = dir.join("metadata/timestamp.json");
    let mut bad: serde_json::Value =
        serde_json::from_slice(&upload.metadata["timestamp.json"]).unwrap();
    bad["signatures"][0]["sig"] = serde_json::json!("00".repeat(64));
    std::fs::write(&timestamp, serde_json::to_vec(&bad).unwrap()).unwrap();
    assert!(matches!(
        command.execute(db.pool.clone(), on, &root).await,
        Err(Error::Verification)
    ));
    std::fs::write(&timestamp, &upload.metadata["timestamp.json"]).unwrap();
    std::fs::write(dir.join("policy-v1.json"), b"{broken").unwrap();
    assert!(matches!(
        command.execute(db.pool.clone(), on, &root).await,
        Err(Error::Invalid(_))
    ));
    std::fs::write(dir.join("policy-v1.json"), &upload.policy).unwrap();
    let denied = Command {
        actor: Uuid::now_v7(),
        channel: Channel::Stable,
        dir: dir.clone(),
    };
    assert!(matches!(
        denied.execute(db.pool.clone(), on, &root).await,
        Err(Error::Denied)
    ));
    let id = command.execute(db.pool.clone(), on, &root).await.unwrap();
    let recorded: Uuid = sqlx::query_scalar("SELECT published_by FROM client_releases WHERE id=$1")
        .bind(id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(recorded, actor);
    std::fs::remove_dir_all(dir).unwrap();
    db.close().await;
}

#[test]
fn command_arguments_and_contract_fixture_bytes() {
    for args in [
        vec![],
        vec!["--channel", "preview"],
        vec!["--actor", "broken"],
        vec!["--unknown", "x"],
        vec!["--dir"],
    ] {
        assert!(Command::parse(args.into_iter().map(str::to_owned)).is_err());
    }
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/contracts/fixtures/client-update-origin-v1");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(base.join("manifest.json")).unwrap()).unwrap();
    use sha2::{Digest, Sha256};
    for file in manifest["files"].as_array().unwrap() {
        let name = file["path"].as_str().unwrap();
        let bytes = std::fs::read(base.join(name)).unwrap();
        assert_eq!(bytes.len() as u64, file["bytes"].as_u64().unwrap());
        assert_eq!(
            hex::encode(Sha256::digest(&bytes)),
            file["sha256"].as_str().unwrap()
        );
        assert_eq!(
            bytes,
            std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures")
                    .join(name)
            )
            .unwrap()
        );
    }
}

#[tokio::test]
async fn db_signed_contract_bundles_publish_for_both_channels() {
    let Some(db) = Database::open().await else {
        return;
    };
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO accounts(id,email,password_hash,status,locale) VALUES($1,'fixture-publisher@example.invalid','unused','active','en')").bind(actor).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO account_roles(account_id,role_id) SELECT $1,role_id FROM role_permissions WHERE permission='downloads.manage' LIMIT 1").bind(actor).execute(&db.pool).await.unwrap();
    for channel in [Channel::Stable, Channel::Beta] {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/origin-v1")
            .join(channel.as_str());
        let root = dir.join("metadata/1.root.json");
        let command = Command {
            channel,
            actor,
            dir,
        };
        command
            .execute(db.pool.clone(), FeatureFlags { downloads: true }, &root)
            .await
            .unwrap();
        let downloads = oteryn_downloads::Downloads::new(
            db.pool.clone(),
            FeatureFlags { downloads: true },
            Default::default(),
        );
        assert_eq!(
            downloads.metadata(channel, "1.root.json").await.unwrap(),
            std::fs::read(root).unwrap()
        );
    }
    db.close().await;
}
