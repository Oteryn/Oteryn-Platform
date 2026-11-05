#![allow(dead_code)]

use std::collections::BTreeMap;

use async_trait::async_trait;
use ed25519_dalek::{Signer, SigningKey};
use oteryn_downloads::{AccountState, Channel, PublishAccess, UpdaterPolicy, Upload};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

pub struct Access(pub AccountState, pub Option<bool>);
#[async_trait]
impl PublishAccess for Access {
    async fn account_state(&self, _: Uuid) -> AccountState {
        self.0
    }
    async fn may_publish(&self, _: Uuid, _: Channel) -> Option<bool> {
        self.1
    }
}
pub const ALLOW: Access = Access(AccountState::Active, Some(true));

fn key() -> SigningKey {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).unwrap();
    SigningKey::from_bytes(&bytes)
}
fn public(key: &SigningKey) -> Value {
    json!({"keytype":"ed25519","scheme":"ed25519","keyval":{"public":hex::encode(key.verifying_key().as_bytes())}})
}
fn key_id(key: &SigningKey) -> String {
    hex::encode(Sha256::digest(
        canonical_json::to_string(&public(key)).unwrap().as_bytes(),
    ))
}
fn signed(value: Value, keys: &[&SigningKey]) -> Vec<u8> {
    let canonical = canonical_json::to_string(&value).unwrap();
    let signatures: Vec<Value> = keys
        .iter()
        .map(|key| {
            json!({
                "keyid":key_id(key),"sig":hex::encode(key.sign(canonical.as_bytes()).to_bytes())
            })
        })
        .collect();
    // Preserve whitespace and a final newline to detect accidental reserialization by the portal.
    let mut bytes =
        serde_json::to_vec_pretty(&json!({"signed":value,"signatures":signatures})).unwrap();
    bytes.push(b'\n');
    bytes
}
fn descriptor(bytes: &[u8], version: i64) -> Value {
    json!({"version":version,"length":bytes.len(),"hashes":{"sha256":hex::encode(Sha256::digest(bytes))}})
}

pub struct SignerFixture {
    root: Vec<u8>,
    targets_key: SigningKey,
    snapshot_key: SigningKey,
    timestamp_key: SigningKey,
    channel_key: SigningKey,
}

impl SignerFixture {
    pub fn new() -> Self {
        let root_a = key();
        let root_b = key();
        let targets_key = key();
        let snapshot_key = key();
        let timestamp_key = key();
        let channel_key = key();
        let mut keys = serde_json::Map::new();
        for key in [
            &root_a,
            &root_b,
            &targets_key,
            &snapshot_key,
            &timestamp_key,
        ] {
            keys.insert(key_id(key), public(key));
        }
        let root = signed(
            json!({
                "_type":"root","spec_version":"1.0.0","version":1,
                "expires":"2099-01-01T00:00:00Z","consistent_snapshot":true,"keys":keys,
                "roles":{
                    "root":{"keyids":[key_id(&root_a),key_id(&root_b)],"threshold":2},
                    "targets":{"keyids":[key_id(&targets_key)],"threshold":1},
                    "snapshot":{"keyids":[key_id(&snapshot_key)],"threshold":1},
                    "timestamp":{"keyids":[key_id(&timestamp_key)],"threshold":1}
                }
            }),
            &[&root_a, &root_b],
        );
        Self {
            root,
            targets_key,
            snapshot_key,
            timestamp_key,
            channel_key,
        }
    }

    pub fn roots(&self) -> BTreeMap<Channel, Vec<u8>> {
        BTreeMap::from([
            (Channel::Stable, self.root.clone()),
            (Channel::Beta, self.root.clone()),
        ])
    }

    pub fn policy(&self, revision: i64, release: &str, sequence: i64) -> UpdaterPolicy {
        let mut policy: UpdaterPolicy =
            serde_json::from_slice(include_bytes!("../fixtures/policy-v1.json")).unwrap();
        policy.policy_revision = revision;
        policy.current_release_id = release.into();
        policy.current_release_sequence = sequence;
        policy.current_version_display = format!("{sequence}.0.0");
        policy.artifacts[0].target_path =
            format!("channels/stable/releases/{release}/windows/x86_64/client.zip");
        policy
    }

    pub fn upload(&self, policy: UpdaterPolicy) -> Upload {
        self.upload_expiring(policy, "2099-01-01T00:00:00Z")
    }

    pub fn upload_expiring(&self, policy: UpdaterPolicy, expires: &str) -> Upload {
        self.upload_profile(policy, expires, false)
    }

    pub fn duplicate_delegated_threshold(&self, policy: UpdaterPolicy) -> Upload {
        self.upload_profile(policy, "2099-01-01T00:00:00Z", true)
    }

    fn upload_profile(&self, policy: UpdaterPolicy, expires: &str, duplicate: bool) -> Upload {
        let revision = policy.policy_revision;
        let channel = policy.channel;
        let policy_bytes = serde_json::to_vec_pretty(&policy).unwrap();
        let mut target_entries = serde_json::Map::new();
        target_entries.insert(channel.policy_path(), json!({
            "length":policy_bytes.len(),"hashes":{"sha256":hex::encode(Sha256::digest(&policy_bytes))}
        }));
        for artifact in &policy.artifacts {
            target_entries.insert(
                artifact.target_path.clone(),
                json!({
                    "length":7,"hashes":{"sha256":hex::encode(Sha256::digest(b"package"))}
                }),
            );
        }
        let mut delegated = signed(
            json!({
                "_type":"targets","spec_version":"1.0.0","version":revision,
                "expires":expires,"targets":target_entries
            }),
            &[&self.channel_key],
        );
        let threshold = if duplicate { 2 } else { 1 };
        if duplicate {
            let mut envelope: Value = serde_json::from_slice(&delegated).unwrap();
            let signature = envelope["signatures"][0].clone();
            envelope["signatures"]
                .as_array_mut()
                .unwrap()
                .push(signature);
            delegated = serde_json::to_vec_pretty(&envelope).unwrap();
        }
        let targets = signed(
            json!({
                "_type":"targets","spec_version":"1.0.0","version":1,
                "expires":"2099-01-01T00:00:00Z","targets":{},
                "delegations":{
                    "keys":{key_id(&self.channel_key):public(&self.channel_key)},
                    "roles":[{"name":channel.as_str(),"keyids":[key_id(&self.channel_key)],
                        "threshold":threshold,"terminating":true,"paths":[format!("channels/{}/*",channel.as_str())]}]
                }
            }),
            &[&self.targets_key],
        );
        let snapshot = signed(
            json!({
                "_type":"snapshot","spec_version":"1.0.0","version":revision,
                "expires":expires,"meta":{
                    "targets.json":descriptor(&targets,1),
                    format!("{}.json", channel.as_str()):descriptor(&delegated,revision)
                }
            }),
            &[&self.snapshot_key],
        );
        let timestamp = signed(
            json!({
                "_type":"timestamp","spec_version":"1.0.0","version":revision,
                "expires":expires,"meta":{"snapshot.json":descriptor(&snapshot,revision)}
            }),
            &[&self.timestamp_key],
        );
        Upload {
            channel,
            policy: policy_bytes,
            metadata: BTreeMap::from([
                ("1.root.json".into(), self.root.clone()),
                ("1.targets.json".into(), targets),
                (format!("{revision}.{}.json", channel.as_str()), delegated),
                (format!("{revision}.snapshot.json"), snapshot),
                ("timestamp.json".into(), timestamp),
            ]),
        }
    }
}

pub struct Database {
    pub pool: PgPool,
    admin: PgPool,
    schema: String,
}
impl Database {
    pub async fn open() -> Option<Self> {
        let Ok(url) = std::env::var("PORTAL_TEST_DATABASE_URL") else {
            eprintln!("SKIP downloads DB test: PORTAL_TEST_DATABASE_URL is unset");
            return None;
        };
        let admin = PgPool::connect(&url).await.unwrap();
        // The SQL identifier is generated solely from a UUID, never from external input.
        let schema = format!("downloads_t35_{}", Uuid::now_v7().simple());
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let search_path = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |connection, _| {
                let schema = search_path.clone();
                Box::pin(async move {
                    sqlx::query("SELECT set_config('search_path', $1, false)")
                        .bind(schema)
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        oteryn_db::migrate(&pool).await.unwrap();
        Some(Self {
            pool,
            admin,
            schema,
        })
    }
    pub async fn close(self) {
        self.pool.close().await;
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP SCHEMA {} CASCADE",
            self.schema
        )))
        .execute(&self.admin)
        .await
        .unwrap();
        self.admin.close().await;
    }
}
