# Download Center artifact-reference immutability audit — 2026-08-09

## Scope

Audited protected `main@c1b1d26b355db26a89d983cc4abc6477bf843a26` for the delivered Download Center's client-artifact reference integrity boundary. During validation, unrelated federated-search repair PR #947 advanced protected `main` to `a82ec651f9155fc5acbfe78d6c3b792fa9b9c0b8`; this audit branch was then synchronized onto that current main without changing its semantic finding.

Primary evidence:

- `docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md`
- PR #161 and its delivered Download Center files
- `app/Downloads/Security/ArtifactUrlPolicy.php`
- `app/Downloads/Actions/PublishClientRelease.php`
- `app/Http/Requests/Downloads/SaveClientReleaseRequest.php`
- `config/downloads.php`
- `resources/views/downloads/index.blade.php`
- `lang/en/public.php`
- `tests/Unit/Downloads/ArtifactUrlPolicyTest.php`
- `tests/Feature/Downloads/DownloadCenterTest.php`
- historical Download Center lifecycle Issues #562/#622/#647/#656/#676/#679/#682

This is audit-only work. No Download Center runtime, configuration, route, view, migration, test, deployment or production path is modified.

## Ownership reconciliation

At audit selection:

- protected `main` was `c1b1d26b355db26a89d983cc4abc6477bf843a26`;
- PR #947 actively owned OPA-SEC-0005 / Issue #938 and the federated-search architecture paths;
- OPA-SEC-0006 / Issue #941 remained a separate Today/private-cache finding;
- OPA-SEC-0007 / Issue #944 remained a separate entitlement stale-authority finding;
- public-domain repair and native-auth production-verification were the two durable active task records apart from `.gitkeep`;
- PR #541 remained the public-domain external-evidence package;
- PR #338 remained the intentional Game Catalog schema 1.3 producer-compatibility hold.

PR #947 later merged, but its paths are outside this Download Center audit and its changes were incorporated into the synchronized branch base. No current owner overlapped the Download Center artifact-reference validation paths audited here.

## Accepted architecture requirement

`PUBLIC_WEBSITE_EXPANSION_PLAN.md` requires the Downloads module to own an `immutable artifact URL or approved storage reference`, says initial client releases should reference `immutable, operator-approved artifacts`, and repeats under security requirements that Download Center must `reference immutable approved artifacts` while restricting allowed artifact hosts.

The same architecture intentionally forbids arbitrary public executable upload in the first slice and defers direct binary upload until a separate storage/signing threat model exists.

That distinction matters: the requirement is not merely trusted origin. The reference itself must preserve artifact identity after publication.

## Delivered control

`ArtifactUrlPolicy::rejectionReason()` correctly rejects several classes of unsafe reference:

1. empty/non-normalized values;
2. malformed URLs/control characters;
3. schemes other than HTTPS;
4. hosts outside the exact configured allowlist;
5. URL user information;
6. URL fragments;
7. non-443 explicit ports;
8. root-only/no concrete path.

The final path condition is:

```text
path exists AND path != '/'
```

After that condition the policy returns success. There is no check for:

- content-addressed object identity;
- object version ID;
- digest binding between URL/reference and `sha256`;
- signed immutable manifest/reference;
- host-specific immutable key contract;
- a configured path grammar that itself provides an enforceable immutable object identity.

The error string says a rejected root URL `must reference a concrete immutable artifact path`, but the implementation proves only path concreteness.

## Publication does not strengthen the invariant

`PublishClientRelease` does revalidate every enabled artifact while holding the release/channel transaction lock. This is a good publication-time control, but it calls the same `ArtifactUrlPolicy::rejectionReason($artifact->artifact_url)` and therefore cannot distinguish immutable and overwriteable references on an approved host.

`SaveClientReleaseRequest` requires a lowercase 64-hex `sha256`, but the supplied digest is ordinary administrator metadata. It is not passed to `ArtifactUrlPolicy` and is not structurally bound to the URL/reference.

Published release database rows are immutable after publication. This protects metadata mutation in Platform, but it does not prevent the bytes behind an external mutable object key from changing.

## Truthful checksum boundary is not the finding

The public Download Center explicitly says:

- SHA-256 values are administrator-supplied release metadata;
- Platform does not fetch the artifact;
- Platform does not claim independent checksum verification.

Feature coverage asserts this disclosure. Historical Download Center closeout audits also explicitly preserved the no-upload/no-proxy/no-fetch and supplied-checksum-only nonclaims.

Therefore OPA-SEC-0008 does **not** claim that Platform currently promises independent checksum verification. The defect is narrower: the implementation claims to enforce the architecture's immutable-reference boundary but does not do so.

## Reproduction / falsification

With `downloads.allowed_artifact_hosts = ['downloads.example.test']`, the current policy accepts a URL such as:

```text
https://downloads.example.test/latest/oteryn-client.zip
```

because it is:

- HTTPS;
- exact-host allowlisted;
- port-compatible;
- free of userinfo/fragment;
- non-root.

Nothing in Platform proves that `latest/oteryn-client.zip` cannot be overwritten.

The same applies to a version-looking pathname such as:

```text
https://downloads.example.test/releases/3.0.0/oteryn-client.zip
```

unless the configured storage contract independently guarantees that exact object key immutable. A path containing a release number is naming convention, not proof of storage immutability.

### Negative path

1. Administrator stores approved-host artifact URL + supplied SHA-256.
2. Draft validation accepts it.
3. Publication revalidates the same URL-shape policy and publishes it.
4. Platform release row becomes immutable.
5. External storage later replaces bytes at the same allowed URL/object key.
6. Public Download Center still points the Download button to that URL and still displays the original supplied checksum.
7. No Platform publication mutation, permission bypass or host-allowlist bypass is required.

The result violates the architecture's immutable artifact-reference requirement while satisfying the current code/tests.

## Test-gap evidence

`ArtifactUrlPolicyTest` covers unsafe schemes, plain HTTP, unapproved/exact-host confusion, userinfo, fragment, nonstandard port, root-only reference and control characters. Its positive case uses a version-looking path.

It does not test rejection of:

- `latest` or `current` aliases;
- overwriteable object keys on an approved host;
- an approved path whose immutable identity is not bound to trusted object-version/content-address metadata.

`DownloadCenterTest` proves publication immutability of Platform rows, exact permissions/MFA, public filtering and approved-host fail-closed behavior, but likewise does not prove underlying artifact-reference immutability.

## Finding

### OPA-SEC-0008 — HIGH / P1

**Download Center accepts mutable artifact URLs as immutable releases.**

Issue #948 is the independent remediation owner.

A repair must make immutable-reference validity machine-testable from trusted Platform configuration/metadata. Acceptable designs may use content-addressed references, immutable object versions, or an equivalently strong host-specific storage-reference contract. A pathname that merely looks versioned is insufficient.

The repair must preserve:

- HTTPS + exact-host restrictions;
- fail-closed malformed URL handling;
- publication-time revalidation;
- truthful operator-supplied checksum semantics;
- no executable upload, proxy or artifact-fetch behavior unless separately authorized.

## Duplicate analysis

Searches for `OPA-SEC-0008`, immutable Download Center artifact URLs, mutable aliases/paths, artifact replacement and supplied checksum semantics found no owner for this root cause.

Historical Issue #562 and its re-audit chain owned the stale Download Center task lifecycle. Those audits intentionally confirmed that checksum metadata was operator-supplied and that no upload/proxy/fetch behavior was claimed. They did not validate or repair the delivered runtime policy's ability to prove an artifact URL immutable.

Issue #145 remains the broad parent public-site implementation programme, not a bounded defect owner.

## Nonclaims

This audit does not claim:

- that a malicious artifact has actually been served;
- that the configured production artifact host is currently mutable;
- that administrator-supplied SHA-256 is advertised as independently verified;
- that artifact fetching or code signing must be added by this finding;
- that PR #947, Issues #941/#944, public-domain repair, native-auth verification or Game Catalog are implicated.

The proven defect is the mismatch between the accepted immutable-reference invariant and the validation actually enforced by the delivered Download Center.
