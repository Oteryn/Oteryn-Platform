use std::{
    collections::{BTreeMap, BTreeSet},
    pin::Pin,
    sync::Arc,
};

use async_trait::async_trait;
use bytes::Bytes;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use tough::{
    IntoVec, RepositoryLoader, TargetName, Transport, TransportError, TransportErrorKind,
    schema::{PathSet, Role, Root, Signed},
};
use url::Url;

use crate::{Error, UpdaterPolicy, Upload, Versions};

type TransportStream = Pin<Box<dyn futures::Stream<Item = Result<Bytes, TransportError>> + Send>>;

const MAX_METADATA_BYTES: usize = 1024 * 1024;
const MAX_BUNDLE_BYTES: usize = 8 * MAX_METADATA_BYTES;
pub(crate) const MAX_POLICY_BYTES: usize = 256 * 1024;

#[derive(Debug)]
pub(crate) struct Verified {
    pub upload: Upload,
    pub policy: UpdaterPolicy,
    pub versions: Versions,
    pub expires_at: OffsetDateTime,
    pub policy_file: String,
    pub artifacts: BTreeMap<String, serde_json::Value>,
}

/// No HTTP or filesystem fallback: verification can read only the bounded uploaded bundle.
#[derive(Debug, Clone)]
struct BundleTransport(Arc<BTreeMap<String, Vec<u8>>>);

#[async_trait]
impl Transport for BundleTransport {
    async fn fetch(&self, url: Url) -> Result<TransportStream, TransportError> {
        let data = self
            .0
            .get(url.path())
            .ok_or_else(|| TransportError::new(TransportErrorKind::FileNotFound, &url))?;
        Ok(Box::pin(futures::stream::once(std::future::ready(Ok(
            Bytes::copy_from_slice(data),
        )))))
    }
}

pub(crate) async fn verify(upload: Upload, trusted_root: &[u8]) -> Result<Verified, Error> {
    if upload.metadata.len() > 32
        || upload
            .metadata
            .values()
            .any(|b| b.is_empty() || b.len() > MAX_METADATA_BYTES)
        || upload.metadata.values().map(Vec::len).sum::<usize>() > MAX_BUNDLE_BYTES
        || upload.policy.is_empty()
        || upload.policy.len() > MAX_POLICY_BYTES
    {
        return Err(Error::Invalid("upload size"));
    }
    // Reject duplicate decoded key IDs before threshold verification, including delegated roles.
    // This also rejects alternate hex spellings of one signing identity.
    for metadata in upload.metadata.values() {
        let envelope: serde_json::Value =
            serde_json::from_slice(metadata).map_err(|_| Error::Verification)?;
        let signatures = envelope
            .get("signatures")
            .and_then(serde_json::Value::as_array)
            .ok_or(Error::Verification)?;
        let mut key_ids = BTreeSet::new();
        for signature in signatures {
            let id = signature
                .get("keyid")
                .and_then(serde_json::Value::as_str)
                .ok_or(Error::Verification)?;
            let decoded = hex::decode(id).map_err(|_| Error::Verification)?;
            if decoded.len() != 32 || !key_ids.insert(decoded) {
                return Err(Error::Verification);
            }
        }
    }
    let policy: UpdaterPolicy =
        serde_json::from_slice(&upload.policy).map_err(|_| Error::Invalid("policy JSON"))?;
    policy.validate(upload.channel)?;
    let anchor: Signed<Root> =
        serde_json::from_slice(trusted_root).map_err(|_| Error::Verification)?;
    let anchor_version = anchor.signed.version.get();
    if upload
        .metadata
        .get(&format!("{anchor_version}.root.json"))
        .map(Vec::as_slice)
        != Some(trusted_root)
    {
        return Err(Error::Verification);
    }
    let policy_file = format!(
        "{}.{}",
        hex::encode(Sha256::digest(&upload.policy)),
        upload.channel.policy_path()
    );
    let mut files: BTreeMap<String, Vec<u8>> = upload
        .metadata
        .iter()
        .map(|(name, bytes)| (format!("/metadata/{name}"), bytes.clone()))
        .collect();
    files.insert(format!("/targets/{policy_file}"), upload.policy.clone());
    let repository = RepositoryLoader::new(
        &trusted_root,
        Url::parse("https://bundle.invalid/metadata/").map_err(|_| Error::Verification)?,
        Url::parse("https://bundle.invalid/targets/").map_err(|_| Error::Verification)?,
    )
    .transport(BundleTransport(Arc::new(files)))
    .limits(tough::Limits {
        max_root_updates: 16,
        ..Default::default()
    })
    .load()
    .await
    .map_err(|_| Error::Verification)?;
    if !repository.root().signed.consistent_snapshot {
        return Err(Error::Verification);
    }
    // This bounded profile has one channel delegation and no top-level or nested targets.
    let top = &repository.targets().signed;
    let delegations = top.delegations.as_ref().ok_or(Error::Verification)?;
    let role = repository
        .delegated_role(upload.channel.as_str())
        .ok_or(Error::Verification)?;
    let expected_scope = format!("channels/{}/*", upload.channel.as_str());
    let scope_matches = match &role.paths {
        PathSet::Paths(paths) => paths.len() == 1 && paths[0].value() == expected_scope,
        _ => false,
    };
    if !top.targets.is_empty()
        || delegations.roles.len() != 1
        || !scope_matches
        || !role.terminating
    {
        return Err(Error::Verification);
    }
    let delegated = &role.targets.as_ref().ok_or(Error::Verification)?.signed;
    if delegated.delegations.is_some() || delegated.targets.len() != policy.artifacts.len() + 1 {
        return Err(Error::Verification);
    }
    let target_name =
        TargetName::new(upload.channel.policy_path()).map_err(|_| Error::Verification)?;
    let bytes = repository
        .read_target(&target_name)
        .await
        .map_err(|_| Error::Verification)?
        .ok_or(Error::Verification)?
        .into_vec()
        .await
        .map_err(|_| Error::Verification)?;
    if bytes != upload.policy {
        return Err(Error::Verification);
    }
    let mut artifacts = BTreeMap::new();
    for artifact in &policy.artifacts {
        let name = TargetName::new(&artifact.target_path).map_err(|_| Error::Verification)?;
        let target = delegated.targets.get(&name).ok_or(Error::Verification)?;
        if target.length == 0 || target.hashes.sha256.as_ref().len() != 32 {
            return Err(Error::Verification);
        }
        artifacts.insert(
            artifact.target_path.clone(),
            serde_json::to_value(target).map_err(|_| Error::Verification)?,
        );
    }
    let versions = Versions {
        root: repository.root().signed.version.get(),
        targets: top.version.get(),
        delegated: delegated.version.get(),
        snapshot: repository.snapshot().signed.version.get(),
        timestamp: repository.timestamp().signed.version.get(),
    };
    let mut expected = BTreeMap::new();
    for version in anchor_version..=versions.root {
        expected.insert(format!("{version}.root.json"), ());
    }
    for name in [
        format!("{}.targets.json", versions.targets),
        format!("{}.{}.json", versions.delegated, upload.channel.as_str()),
        format!("{}.snapshot.json", versions.snapshot),
        "timestamp.json".to_owned(),
    ] {
        expected.insert(name, ());
    }
    if !expected.keys().eq(upload.metadata.keys()) {
        return Err(Error::Invalid("metadata filenames"));
    }
    let expires = [
        repository.root().signed.expires(),
        top.expires(),
        delegated.expires(),
        repository.snapshot().signed.expires(),
        repository.timestamp().signed.expires(),
    ]
    .into_iter()
    .min()
    .ok_or(Error::Verification)?;
    let expires_at = OffsetDateTime::from_unix_timestamp(expires.timestamp())
        .map_err(|_| Error::Verification)?;
    Ok(Verified {
        upload,
        policy,
        versions,
        expires_at,
        policy_file,
        artifacts,
    })
}
