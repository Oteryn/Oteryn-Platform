# Oteryn Client Distribution Update Contract

## Status

**PROPOSED — non-authoritative until ADR 0035 is accepted.**

This document defines the Platform-side semantic contract that would follow acceptance of ADR 0035 Option A. It does not grant runtime, external-repository, signing-key, deployment or production authority.

## Purpose

Provide one fail-closed contract for first-party updater publication while preserving the existing browser Download Center and immutable artifact-reference controls.

The target design uses The Update Framework (TUF) as the cryptographic repository/freshness layer and a small Oteryn policy target as the application-policy layer.

## Authority split

| Concern | Proposed authority | Explicit non-authority |
|---|---|---|
| Release approval, channel policy, minimum supported release and mandatory-update intent | Platform `Downloads` application/domain policy | Browser templates, client-local inference, CMS prose |
| Artifact bytes | Approved immutable artifact storage | Platform database row, mutable alias |
| Artifact hash/length trusted by updater | Verified TUF targets metadata | Administrator-supplied Download Center checksum by itself |
| Updater repository trust/key delegation | TUF Root metadata bootstrapped in first-party updater | HTTPS certificate or Laravel configuration |
| Stable target publication | Stable delegated target authority under TUF | Beta target key/role |
| Beta target publication | Beta delegated target authority under TUF | Stable target key/role |
| Metadata consistency/freshness | TUF Snapshot/Timestamp metadata and expiry/version semantics | `is_current`, cache age or page publication time |
| Private updater-signing keys | Protected release-signing boundary appropriate to each TUF role | Laravel web runtime, database, ordinary administrator session |
| Updater launch/update decision | First-party updater over verified TUF + verified Oteryn policy target | Download Center HTML/API response |
| Gameplay/protocol admission | Separately authoritative game/admission contract | Updater decision or Download Center state |

## Core identifiers

### Distribution tuple

Every updater policy decision is scoped to exactly:

```text
channel       = stable | beta
platform      = windows | linux | macos
architecture  = x86_64 | arm64 | x86
```

The initial values match the current `DownloadCatalog` implementation. Extension requires an explicit compatible contract/schema change. Unknown values fail unsupported; no nearest-target fallback exists.

### Release identity

Updater-enabled releases require an opaque immutable `release_id` plus a positive integer `release_sequence` scoped to the exact distribution tuple.

Invariants:

- `release_id` never identifies two different byte sets;
- `release_sequence` is assigned monotonically for normal forward publication within one tuple;
- display `version` is presentation metadata and is not the security ordering primitive;
- historical browser-only Download Center rows need not be retroactively interpreted as ordered updater releases;
- a deliberate application rollback may select an older `release_sequence` only through a newer trusted policy revision carrying explicit rollback authorization.

The exact representation of `release_id` is an implementation detail. It must be stable, non-secret and unambiguous across Platform persistence, signed policy and release evidence.

### Policy identity

Every Oteryn updater policy target carries a positive, monotonically increasing `policy_revision` for the exact distribution tuple.

A client persists the highest trusted policy revision it has accepted and does not replace it with a lower revision even if an attacker can serve old but otherwise valid bytes. TUF metadata version/expiry checks remain independently required.

## TUF repository profile — proposed semantic requirements

Acceptance of ADR 0035 Option A means the implementation profile must satisfy all of these requirements before activation:

1. Root, Targets, Snapshot and Timestamp roles are present.
2. Root trust is bootstrapped with the first-party updater.
3. Root private-key custody is offline and threshold-controlled; no single web-runtime credential is root authority.
4. Stable and beta publication use cryptographically distinct delegated target authority with non-overlapping target scopes.
5. Consistent snapshots are enabled for the updater repository.
6. Metadata versions and expiration are enforced; clients retain previously trusted metadata and reject rollback to lower trusted metadata versions.
7. Target hash and length are verified before an artifact/policy target becomes usable.
8. Snapshot metadata binds a coherent metadata set; the updater does not combine channel metadata from incompatible repository generations.
9. Timestamp freshness is maintained by a narrowly scoped online signer/service; expiry values and exact operational intervals are defined in the implementation POUF/runbook rather than guessed in this architecture contract.
10. All historical Root metadata required for supported clients to rotate trust remains available under the selected TUF implementation rules.

The exact serialization, algorithms, library, key backend, thresholds beyond the root non-single-key invariant and metadata expiry intervals are implementation choices that must be documented in the adoption POUF/profile and security-reviewed before activation.

## Oteryn policy target

For each exact distribution tuple, TUF authenticates one current Oteryn policy target with an application schema beginning at `oteryn.update-policy/1`.

Conceptual payload:

```json
{
  "schema_version": "oteryn.update-policy/1",
  "policy_revision": 42,
  "channel": "stable",
  "platform": "windows",
  "architecture": "x86_64",
  "current_release_id": "opaque-stable-id",
  "current_release_sequence": 108,
  "current_version_display": "2.8.4",
  "current_target_path": "channels/stable/windows/x86_64/releases/opaque-stable-id/package",
  "minimum_supported_release_sequence": 105,
  "update_mode": "required",
  "revoked_release_ids": [],
  "rollback_authorization": null
}
```

This is a semantic example, not final wire bytes. The implementation POUF may choose canonical JSON or another TUF-compatible metadata/profile format.

### Required fields and rules

- `schema_version`: exact supported application-policy schema; unknown major schema fails incompatible.
- `policy_revision`: positive monotonic tuple-scoped revision.
- `channel`, `platform`, `architecture`: must exactly equal the configured/requested distribution tuple.
- `current_release_id`: immutable release selected by this policy.
- `current_release_sequence`: sequence assigned to that exact release.
- `current_version_display`: presentation only; does not determine trust ordering.
- `current_target_path`: exact TUF target path for the selected immutable artifact/package.
- `minimum_supported_release_sequence`: must be positive and cannot exceed `current_release_sequence` unless an explicit rollback policy schema later defines and validates that exceptional state.
- `update_mode`: exactly `optional`, `recommended` or `required`.
- `revoked_release_ids`: bounded unique list of immutable release identities; the current release cannot also be revoked.
- `rollback_authorization`: absent/null for normal forward policy; required when selecting a `current_release_sequence` below the highest current release sequence previously trusted by that client for the tuple.

The policy target does **not** redefine artifact hashes or lengths. Those values are authoritative from the verified TUF target metadata. Platform may display an independently stored administrator-supplied checksum, but equality with TUF target metadata is not assumed unless an implementation explicitly verifies it.

## Effective update state

For an installed release with known tuple and identity:

1. refresh and verify TUF metadata;
2. verify/fetch the exact Oteryn policy target through trusted TUF target metadata;
3. verify policy schema, tuple and monotonic policy revision;
4. resolve the selected artifact target from the same trusted repository view;
5. evaluate release support/revocation;
6. choose update/launch state.

Conceptual states:

| State | Condition | Updater behavior |
|---|---|---|
| `CURRENT` | installed identity equals current and is not revoked | normal launch allowed while trusted policy remains valid |
| `SUPPORTED_UPDATE_OPTIONAL` | installed sequence >= minimum, not current/revoked, mode optional | launch allowed; optional update offered |
| `SUPPORTED_UPDATE_RECOMMENDED` | installed sequence >= minimum, not current/revoked, mode recommended | launch allowed; prominent update recommendation |
| `UPDATE_REQUIRED` | installed supported but mode required | normal online launch blocked until selected current release is installed and verified |
| `UNSUPPORTED` | installed sequence below minimum | normal online launch blocked; update required if a valid target is available |
| `REVOKED` | installed release id is revoked | normal online launch blocked regardless of sequence |
| `TARGET_UNAVAILABLE` | exact target tuple/current target absent | no cross-target fallback; explicit unavailable state |
| `POLICY_INCOMPATIBLE` | unknown required schema/tuple semantics | fail closed |
| `TRUST_UNAVAILABLE` | required TUF/policy metadata invalid or expired without a still-valid trusted state | fail closed for normal online launch |

Recovery/settings/diagnostic/log-export UI may remain available in blocked states. This contract does not require the process to exit immediately.

## Refresh and failure semantics

- A network failure never converts an untrusted response into trusted state.
- If refresh fails but the locally persisted TUF metadata and Oteryn policy are still valid and unexpired, the updater may continue using only that trusted state.
- If the trusted metadata needed for current policy is expired, signature-invalid, version-invalid, tuple-incompatible or internally inconsistent, updater trust is unavailable; normal online launch fails closed.
- A missing `stable` artifact never falls back to `beta`, another architecture or another operating system.
- A missing current artifact is not interpreted as withdrawal or successful rollback; it is inconsistent/unavailable state.
- Local deletion/corruption of trusted metadata triggers a bootstrap/recovery path from the embedded Root trust anchor; it does not disable verification.
- Cached web/API Download Center responses never repair or override TUF trust failures.

## Channel semantics

### Stable

- intended production channel;
- targets require stable-delegated authority;
- stable policy references only stable-authorized target paths;
- beta role/key compromise must not authorize stable target metadata.

### Beta

- explicit opt-in/non-default channel unless later product policy changes;
- targets require beta-delegated authority;
- beta policy cannot alter stable minimum-version, revocation or current-release state.

### Channel switching

A channel switch is explicit and creates a new tuple. The updater verifies the destination channel independently from its trusted Root. It does not carry minimum-version/current/revocation policy across channels by inference.

A release artifact proven byte-identical may be authorized in both channels, but each channel requires its own trusted metadata reference/authorization. Promotion is new stable publication, not mutation of beta authority.

## Minimum-supported and mandatory-update semantics

- Minimum support is expressed by `minimum_supported_release_sequence`, not lexical/string comparison of `version`.
- `optional` and `recommended` never override minimum support or revocation.
- `required` means the signed distribution policy instructs the first-party updater to block normal online launch until the exact selected current release is installed and verified.
- Distribution policy must not claim that a release is game-admissible. The game/admission authority may apply stricter protocol/security compatibility policy independently.
- A future scheduled `required_after`/deadline field is not part of schema v1. Adding clock-dependent enforcement requires a separate compatible schema decision that defines trustworthy time/failure semantics.

## Artifact installation invariants

Before installation/use as current:

- the target is resolved through trusted metadata for the exact tuple;
- download length is bounded by trusted target metadata;
- hash verification succeeds against trusted TUF target metadata;
- archive/package extraction is separately hardened against traversal/link/device/path attacks by the client implementation;
- installation is staged and atomically switched or otherwise rollback-safe for the platform;
- failure leaves the prior installed version recoverable where technically possible;
- no downloaded executable is trusted merely because it came from an allowlisted HTTPS host.

Package extraction/install mechanics are client implementation scope, but these acceptance properties are required for the cross-boundary contract.

## Withdrawal

Withdrawal means “do not select this release for new current/recommended policy.”

Rules:

- preserve release identity, prior signed metadata and artifact retention needed by supported metadata/rollback windows;
- issue newer signed policy selecting another non-revoked release;
- do not delete an artifact still referenced by trusted unexpired metadata required for supported clients;
- withdrawal alone does not imply compromise/revocation and need not hard-block an already installed supported release unless minimum/update-mode policy also changes.

## Revocation

Revocation is a security/compatibility decision for an exact immutable release identity.

Rules:

- only newer trusted channel policy may introduce/remove a revocation;
- revocation does not rewrite old metadata or bytes;
- an installed revoked release is blocked for normal online launch;
- the current selected release cannot be revoked in the same effective policy;
- revocation removal/recovery is a newer signed policy decision and must not resurrect a missing or incompatible artifact implicitly;
- security/audit records should capture release id, policy revision and bounded reason classification, not private key material.

## Deliberate rollback

A legitimate rollback differs from a metadata rollback attack.

- TUF metadata versions and Oteryn `policy_revision` still move forward.
- The new policy may select a previously published lower `release_sequence` only with `rollback_authorization`.
- The selected older release must still have exact immutable bytes/target metadata available and must not be revoked.
- `minimum_supported_release_sequence` must be reconciled so the selected rollback target is supportable.
- Clients persist enough trusted application state to reject an application downgrade that lacks this newer explicit authorization.
- Repointing an existing `release_id`, target path or historical signed metadata to different bytes is forbidden.

The exact rollback-authorization object shape is an implementation-schema decision, but it must bind at least the new policy revision, selected rollback release id/sequence and the prior selected sequence being superseded.

## Key lifecycle

### Root

- offline custody;
- threshold greater than one private key for production trust;
- backups/custody separation and recovery procedure required before activation;
- normal role key rotation is authorized through newer Root metadata;
- threshold Root compromise is an out-of-band trust-recovery incident and cannot be repaired by Laravel admin state alone.

### Stable/Beta targets

- separate credentials/roles;
- least-privilege path/channel scope;
- rotation/revocation authorized by the role that delegates them;
- protected release environment, never ordinary web-runtime secret storage.

### Snapshot/Timestamp

- use narrowly scoped keys appropriate to automated repository freshness;
- compromise does not grant Root authority;
- rotation and expiry runbooks are required;
- exact online/offline placement and thresholds follow the selected maintained TUF implementation and POUF, not ad-hoc Laravel signing.

## Platform persistence/projection boundary

The current `client_releases`/`client_release_artifacts` model remains valid browser-oriented evidence but is insufficient by itself for updater trust.

An implementation after ADR acceptance should add an explicit updater publication model rather than overload `is_current` with cryptographic meaning. It must be possible to distinguish at least:

- browser-only draft/published release metadata;
- artifact immutable-reference validation state;
- updater-policy generation pending signing/publication;
- exact verified signed generation identity;
- updater-publishable/active generation;
- withdrawn/revoked state and superseding policy revision.

No migration shape is prescribed here.

## Publication transaction semantics

The protected signing system and Platform may not share a database transaction, so publication is a reconciled saga rather than a false distributed transaction.

Required observable states:

1. Platform release policy approved but not signed;
2. signed repository generation produced but not yet Platform-verified/activated;
3. exact generation verified against expected tuple/releases/policy;
4. active updater generation;
5. superseded/withdrawn/revoked with newer generation identity.

Retries are idempotent on a stable generation/request identity. Ambiguous publication does not create a second policy revision or silently activate whichever metadata is observed first.

## Browser Download Center relationship

- Existing pages may continue to present approved releases and supplied SHA-256 facts.
- Until updater-signature integration exists, they must not claim “publisher verified” or equivalent cryptographic provenance.
- After implementation, any signed/verified badge must be derived from a verified exact TUF generation/release association, not from the presence of a checksum string.
- Browser “current” and updater current must not silently disagree. If both are exposed for the same updater-enabled channel/tuple, the Platform must reconcile them or render the discrepancy unavailable/degraded rather than inventing one source as truth.

## Game admission separation

The updater answers distribution questions only. It cannot prove:

- a world is ready;
- a protocol version is accepted;
- an account is authorized;
- a game session may be created;
- the selected build is currently admitted by game authority.

A game-side compatibility/admission gate may require the same or a stricter release/protocol boundary. The Platform must not treat successful updater verification as authoritative game admission.

## Observability and audit

Implementation must emit bounded identifiers for:

- policy/generation publication request and result;
- exact channel/target tuple;
- release id/sequence;
- TUF metadata/generation identities without private keys;
- signing/publication/verification failure class;
- activation/supersession/revocation/rollback decision;
- key id/role identifiers when safe, never private key material.

Metrics/logs must not use complete artifact URLs with sensitive query material as unbounded labels. No private signing material enters application logs, audit metadata or support output.

## Validation handoff

### Platform-side focused tests

- tuple/channel uniqueness and no fallback;
- release/policy sequence monotonicity;
- signed-generation state transitions/idempotency;
- exact association between Platform policy and verified TUF generation;
- browser/current reconciliation;
- withdrawal/revocation/rollback invariants;
- no private signing key required by web runtime;
- no claim that existing supplied SHA-256 is publisher verification.

### TUF/repository tests

- tampered metadata/signature;
- metadata rollback;
- expired/frozen metadata;
- mix-and-match snapshot/targets;
- wrong delegated channel role;
- missing target and target hash/length mismatch;
- root/targets/timestamp key rotation and compromised-key removal;
- consistent-snapshot retrieval.

### Client E2E

- first bootstrap from embedded Root;
- normal forward stable update;
- explicit beta opt-in and stable/beta isolation;
- supported optional/recommended behavior;
- mandatory update and below-minimum block;
- revoked installed release;
- unavailable/expired metadata;
- deliberate authorized rollback versus replayed old policy;
- interrupted download/install with prior-version recovery;
- cross-platform/architecture mismatch rejection.

Production activation also requires exact distribution-origin/TLS/availability evidence and release-operation runbooks; repository tests alone are not production proof.

## Open implementation details after architecture acceptance

The following are intentionally not fixed by this semantic contract:

- maintained TUF implementation/library and exact POUF/metaformat;
- exact signing algorithms supported by that implementation;
- key backend/HSM/offline media and numerical thresholds beyond the Root non-single-key production invariant;
- exact metadata expiration intervals and service SLOs;
- concrete `release_id` encoding;
- concrete persistence/migration schema;
- concrete metadata distribution hostname/path;
- client UI copy and package installer mechanics.

Each must be proven before activation without weakening the semantic/security invariants above.

## Related authority

- Proposed ADR 0035 — first-party client distribution and updater trust boundary
- Issue #1037
- Issue #948 / PR #966 — immutable artifact-reference enforcement
- `docs/architecture/MODULE_CATALOG.md`
- `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`
- `docs/architecture/SECURITY_ARCHITECTURE.md`
