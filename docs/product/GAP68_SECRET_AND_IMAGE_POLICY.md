# Portal secret files and release image evidence

Owner decision adopted 2026-11-23 for GAP-68 and the [Rust portal plan §7.1](../architecture/OTERYN_PORTAL_RUST_PLAN.md#71-security-proposals-salvaged-from-1643).

## Secret files

Portal credentials can come from supported environment variables or from files supplied by the operator. Keep both outside the repository and image. A credential file must be a regular file owned by the service's effective UID, with mode exactly `0400` or `0600`. A symbolic link at the configured file path is refused. This applies to private TLS keys, service-token and token-digest files, signing keys, wrapping keys, and encrypted signing custody snapshots. Public certificates, CA bundles and verification keys follow their existing configuration rules.

1. Determine the UID that will run the relevant role inside the container or service. Provision each credential as a regular file in a restricted directory. For example, use `install -m 0400 -o "$service_uid" -- "$source" "$destination"` with operator privileges. Use `0600` when the owning process must be able to write to the file. Keep the native signing custody directory private and writable by the issuer for its lock and atomic snapshots.
2. Mount or configure the resulting file path. Verify type, UID and mode from the service's mount and user namespace before restarting. Install rotated values with the same ownership and mode before switching them into place.
3. Treat a credential file admission error as a startup refusal. Use the named configuration key to find and correct the provisioning error. Keep paths and credential contents out of shared logs and incident records.

## Release and deploy

Every Portal release image, including an image deployed in the gateway role, requires an SPDX JSON software bill of materials (SBOM) and a keyless Sigstore signature for its immutable OCI digest. Publish the SBOM for the same digest as a signed SPDX attestation. For multi-platform releases, inventory every published platform and bind the evidence to the digest selected for deployment.

1. In the approved release workflow in `Oteryn/Oteryn-Platform`, generate the SBOM from the built image, validate it with a validator for its declared SPDX version, and publish the signature and attestation using GitHub Actions OIDC. Record the immutable image digest, SPDX version, and release workflow run in the release evidence.
2. On the deploy side, configure the approved release workflow's exact certificate identity out of band. Pin the full Oteryn repository, workflow path and allowed release ref using the certificate identity emitted by that workflow. Configure the GitHub Actions issuer `https://token.actions.githubusercontent.com`. Reject unrecognized identities even when their signatures are otherwise valid. A repository-wide wildcard or an identity supplied by the image is insufficient.
3. Before starting or replacing any container, including on rollback, run `cosign verify` for the image and `cosign verify-attestation --type spdxjson` for the SBOM. Pass `--certificate-identity "$approved_identity"` and `--certificate-oidc-issuer https://token.actions.githubusercontent.com` to both. Pass the same immutable `image@sha256:…` reference to both, check that the verified attestation subject matches the selected digest, and validate the extracted SPDX predicate against its declared version. Treat missing, malformed, invalid or mismatched evidence, an unknown signer, or a verifier failure as a deploy refusal. Start only the digest that passed every check.
4. Record the digest, matched identity, SPDX validation result, and deployment decision without recording credentials.

This decision covers the Platform runtime file policy and deployment contract. Workflow wiring and qualification of the deployment gate belong to the release and deploy work.

## Acceptance checks

| Scenario | Expected result |
| --- | --- |
| Start with each supported credential file mode, `0400` and `0600`, with the effective service UID and valid contents | The credential loads and the configured service starts. |
| Start with mode `0640`, `0644` or `0666`, a symlink, a non-regular file or another UID | Startup refuses the credential and names the configuration key. Logs omit its contents. |
| Deploy the pinned digest with a valid SPDX attestation and signatures from the approved workflow identity | Verification succeeds before the container starts. |
| Supply a valid signature from another workflow, a malformed SPDX predicate, an attestation for a different digest or missing evidence | The deploy refuses before starting or replacing the container. |
