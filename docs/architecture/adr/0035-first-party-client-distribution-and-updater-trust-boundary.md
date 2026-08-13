# ADR 0035 — First-party client distribution and updater trust boundary

## Status

Proposed — 2026-08-13

- Decision owner: repository owner
- Decision Issue: #1037
- Recommendation: Option A — TUF-based signed update repository with channel-scoped target authority and a protected signing boundary outside the web application
- Applies to: first-party Oteryn client release distribution, updater trust, stable/beta channel policy, minimum-supported-release policy and release withdrawal/revocation/rollback semantics
- Does not authorize: runtime implementation, database migration, route/API implementation, external client changes, private signing-key operations, deployment, production activation or protected-environment access

## Current state

### PROVEN

- `Downloads` already stores `stable` and `beta` release records plus platform/architecture artifact variants for Windows, Linux and macOS across the repository-defined architectures.
- Published artifacts require an approved machine-testable immutable-reference contract after Issue #948 / PR #966; the web application still does not fetch artifacts and administrator-supplied SHA-256 is not represented as independent publisher verification.
- `MODULE_CATALOG.md` and `PORTAL_COMPLETENESS_ARCHITECTURE.md` leave minimum-version, mandatory-update, updater-manifest and publisher-provenance policy unresolved.
- The current Download Center is browser presentation and administration metadata. It is not a cryptographic updater trust root and has no signing-key authority.
- No open architecture PR or active architecture task owned this exact decision when Issue #1037 was created.

### DERIVED

- Making a launcher consume mutable `is_current`, a `latest` alias, browser metadata or an administrator-supplied digest directly would create an updater authority that the Download Center was not designed to provide.
- A secure automatic updater needs freshness, rollback and mix-and-match protection in addition to artifact integrity because authentic old release metadata can still be unsafe when replayed as current policy.

### UNKNOWN

- The exact first-party updater implementation, serialization/library choice, bootstrap packaging, secure local state and release-signing infrastructure are outside this Platform-only review and were not inspected.
- Exact numerical metadata expiry intervals, release operational staffing and key-custody ceremony are deployment decisions requiring implementation evidence.

## Problem

The first-party updater needs one machine-verifiable answer to all of these questions without trusting a mutable web response:

- which channel and target tuple it is evaluating;
- which release is current;
- whether the installed release is still supported;
- whether an update is optional, recommended or required;
- whether a previously valid release has been withdrawn or revoked;
- whether a deliberate rollback to older application bytes is authorized;
- whether metadata is fresh and internally consistent;
- whether the publisher, rather than only the transport endpoint or an administrator, authorized the release state.

If these semantics are invented ad hoc inside the web application or launcher, compromise of one mutable endpoint/key can become compromise of the update channel and stale signed data can be replayed without a durable freshness model.

## Non-negotiable invariants

1. The Platform web application is not a private software-signing-key custodian.
2. Artifact bytes remain immutable; a published release identity is never repointed to different bytes.
3. Browser Download Center state is not updater trust authority.
4. Updater metadata is authenticated, versioned, freshness-bounded and rollback/mix-and-match resistant.
5. `stable` and `beta` are distinct authorization scopes; neither silently falls back to or promotes into the other.
6. Target identity includes channel, platform and architecture. A missing target fails unavailable rather than falling back to another target.
7. Minimum-supported-release and mandatory-update policy are signed machine policy, not inferred from version strings, CMS text or `is_current` alone.
8. A client-side updater decision never grants game admission. Authoritative gameplay/protocol admission remains separately owned and may reject a client that distribution policy still considers installable.
9. Withdrawal and revocation preserve history. Rollback selects an already immutable artifact through newer trusted policy; it never rewrites old metadata or artifact bytes.
10. The existing no-upload/no-proxy/no-fetch boundary and truthful supplied-checksum wording remain valid until separately changed by an authorized implementation task.

## Options

### Option A — TUF-based repository with protected role separation — RECOMMENDED

Adopt The Update Framework security model for the first-party updater repository:

- bootstrap the updater with trusted Root metadata / root trust anchors;
- use Root, Targets, Snapshot and Timestamp roles with versioned expiring metadata;
- require consistent-snapshot semantics for updater metadata/targets;
- delegate non-overlapping `stable` and `beta` target scopes so a routine channel credential cannot authorize the other channel;
- keep Root private keys offline with a threshold that does not reduce root trust to one private key;
- keep release/Targets/Snapshot signing inside a protected release-publishing boundary and Timestamp signing in a narrowly scoped online freshness service; the Laravel web process receives no private signing key;
- represent Oteryn application policy as a versioned policy target authenticated by TUF metadata rather than inventing a second unsigned manifest protocol;
- let TUF target metadata authenticate target length/hash while the Oteryn policy target carries the application semantics defined in the companion contract.

Security properties: reuses a mature role/freshness/version/delegation model for rollback, freeze and mix-and-match resistance; separates root recovery from frequently used release metadata; supports key rotation without treating HTTPS as publisher identity.

Costs: more metadata roles, key operations, bootstrap/rotation procedures and client integration work than a single signature envelope. The project must document an implementation POUF/profile and prove the selected client library/format before activation.

### Option B — Oteryn-specific single signed manifest envelope

Define one canonical signed JSON/CBOR envelope containing artifact digests, current/minimum release, mandatory-update, revocation, generation and expiry fields. Pin one or more publisher trust anchors in the client and keep private keys outside Laravel.

Benefits: smaller implementation surface and simpler release operations.

Costs: Oteryn becomes responsible for designing and validating its own replay, rollback, freeze, mix-and-match, key-rotation, threshold/recovery and metadata-consistency protocol. A future move to role-separated trust would require migration of both release operations and clients. This option is acceptable only if TUF implementation feasibility is disproven by concrete client/release constraints.

### Option C — HTTPS + checksum + mutable current/latest policy

Continue using the browser-oriented Download Center as the updater source and rely on HTTPS, immutable artifact URLs and the displayed SHA-256.

Benefits: least implementation effort.

Costs: does not establish publisher origin from the administrator-supplied digest, does not provide a signed freshness/replay model and makes mutable Platform state an updater security authority. Rejected as the target architecture.

### Option D — Give the Laravel application online signing authority

Store or make a release-signing key available to the web application so it can sign manifests dynamically.

Benefits: operationally simple publication path.

Costs: expands a web-application compromise into software-update signing compromise, couples ordinary administration to key custody and conflicts with least privilege. Rejected.

### Trade-off matrix

| Criterion | Option A — TUF | Option B — custom signed envelope | Option C — HTTPS/checksum | Option D — web signing |
|---|---|---|---|---|
| Publisher-authenticated metadata | Strong, role-separated | Strong only if the custom protocol is designed correctly | Insufficient | Strong cryptographically, weak trust placement |
| Rollback/freeze/mix-and-match resistance | Native framework semantics | Must be designed and proven by Oteryn | Missing | Must be designed and proven by Oteryn |
| Channel isolation | Delegated non-overlapping authority | Custom key/scope policy required | Application convention only | Application convention/key policy |
| Web-compromise blast radius | Web runtime has no signing authority | Web runtime can remain outside signing | Mutable web policy remains authoritative | Web compromise reaches updater signing authority |
| Key rotation/recovery | Role/threshold model | Custom protocol/runbooks | Not an updater signing model | Custom protocol/runbooks |
| Initial implementation complexity | Highest | Medium | Lowest | Medium |
| Long-term protocol maintenance | Reuse maintained security model | Oteryn owns security-protocol evolution | Low code cost but unacceptable security gap | Oteryn owns protocol plus larger compromise surface |
| Reversibility/migration | Requires client TUF bootstrap but supports later role/key evolution | Easier first bootstrap; later migration may be disruptive | Easy to start, expensive to secure later | Key-custody migration required to reduce web authority |
| Recommendation | **Preferred** | Fallback only if TUF feasibility is disproven | Reject | Reject |

## Recommendation

Choose **Option A**.

The Update Framework separates Root, Targets, Snapshot and Timestamp authority and uses signed versioned/expiring metadata. Oteryn should reuse those security semantics rather than create a custom updater trust protocol. The Platform-specific decision remains narrow: `Downloads` owns release policy/activation presentation, a protected release-publishing boundary materializes signed updater metadata, and the updater trusts only verified TUF repository state plus the authenticated Oteryn policy target.

This recommendation does not select an external client library or authorize implementation. Acceptance of this ADR is required before canonical module/portal architecture is rewritten from “future decision” to “accepted implementation handoff.”

## Proposed Oteryn updater policy semantics

For every exact tuple `(channel, platform, architecture)`, the authenticated policy target has a versioned application schema and carries at least:

```text
schema_version
policy_revision
channel
platform
architecture
current_release_id
current_release_sequence
current_version_display
minimum_supported_release_sequence
update_mode = optional | recommended | required
revoked_release_ids[]
rollback_authorization = none | explicit
```

Security-relevant ordering uses monotonically assigned release/policy sequences, not lexical comparison of the display version string. Updater-enabled releases therefore do not require retroactively interpreting historical Download Center version strings as a total order.

The policy references a TUF target path/release identity; target length and cryptographic hashes come from trusted TUF target metadata rather than being redefined as a second authority inside the application policy.

## Proposed decision semantics

- `minimum_supported_release_sequence` defines the oldest still-supported installed release for the exact channel/target.
- A revoked release is unsupported regardless of its sequence.
- `optional` permits a supported older release to continue without updating.
- `recommended` permits continued use but requests an update.
- `required` makes the current signed policy require installation of the selected current release before the updater authorizes normal online launch.
- An installed release below minimum or explicitly revoked is blocked by updater policy even if `update_mode` would otherwise be optional/recommended.
- If trusted metadata refresh fails while the locally trusted TUF metadata is still unexpired, the updater may continue using only that previously trusted state; it must not accept any new release/policy data.
- Once required trusted metadata is expired or invalid, the updater cannot assert current update-policy authority and fails closed for normal online launch. Recovery/settings/diagnostic UI may remain available.
- A channel change is an explicit user/operator action and starts a separately trusted channel evaluation; it is not fallback.

## Withdrawal, revocation and rollback

- **Withdrawal** removes a release from new recommendation/current selection but preserves signed historical metadata and immutable artifact identity.
- **Revocation** is a newer signed policy decision that marks an exact release identity unusable. It never mutates the old artifact or old metadata.
- **Rollback** is a newer signed policy revision selecting an older immutable release and must carry explicit rollback authorization. Clients must reject an application-version downgrade that is not authorized by newer trusted policy.
- A beta artifact may be promoted to stable only through new stable-authorized metadata that references the exact immutable bytes/hash; promotion never mutates the beta record.
- A release may not be removed in a way that breaks already-issued consistent metadata before the retention/rollback window is satisfied.

## Signing and compromise boundary

- Root trust is bootstrapped into the first-party updater and Root private-key custody is offline/threshold-controlled.
- Stable and beta delegated target authority is cryptographically separable. A beta channel publishing credential must not authorize stable targets, while compromise of a higher delegating Targets authority remains a correspondingly larger security incident.
- The Platform web runtime stores no private updater-signing key and cannot mint trusted updater metadata merely because an administrator can publish Download Center records.
- The protected release-publishing system may consume an approved Platform release-policy snapshot, but generated metadata becomes updater-authoritative only after the required signatures and repository consistency checks exist.
- Platform may store/serve public keys, signatures, metadata references and verified projections; those are not private-key custody.
- A compromised top-level non-Root role is recovered by Root-authorized role-key rotation. A compromised delegated channel role is recovered by newer metadata from its trusted delegating Targets authority. Threshold Root compromise is an out-of-band trust-recovery/security-incident condition, not an ordinary web-admin action.

## Publication/activation model

Proposed effective state:

```text
operator-approved Platform release policy
        + exact immutable artifact identities
        + protected signer / TUF repository generation
        + Platform verification of expected signed metadata identity
        -> updater-publishable release generation
```

A release may remain visible as browser-only metadata without becoming updater-publishable. Conversely, an updater generation cannot be claimed production-active solely because signed files exist; deployment, origin, availability and real client evidence remain separate gates.

## Implementation handoff after acceptance

A separately implementation-authorized task should own, in order:

1. a TUF adoption POUF/profile, selected maintained client/repository implementation and threat-model validation;
2. persistence for opaque release identity/sequence and signed updater-policy generation without reinterpreting historical browser-only versions;
3. protected release-signing/repository publication integration outside the Laravel private-key boundary;
4. Platform verification/import/activation of the exact signed generation;
5. updater-facing immutable metadata distribution and browser/admin presentation of signed/unsigned state;
6. first-party client bootstrap, persistent trusted metadata, failure UX and channel switching in its separately authorized repository;
7. game-admission compatibility enforcement under the game-owned contract rather than trusting updater state as admission authority;
8. key rotation, compromise, rollback, disaster-recovery and release-withdrawal runbooks;
9. exact test/E2E evidence before any production activation claim.

## Acceptance evidence required after decision

- canonical architecture documents are reconciled only after Option A (or another named option) is explicitly accepted;
- repository validators prove ADR numbering/lifecycle and decision-backlog consistency;
- full-diff review checks channel isolation, rollback/revocation, expiry/failure, key compromise and browser/updater/admission separation;
- runtime/browser E2E remains NOT_APPLICABLE for this architecture-only decision package;
- implementation later requires real updater negative-path tests for tampered/expired/replayed/mixed metadata, wrong channel/target, revoked releases, unauthorized downgrade, key rotation and unavailable metadata.

## Decision required

Repository owner: accept **Option A (TUF-based role-separated update repository)** as the target first-party updater architecture, or select Option B. Options C and D are rejected by the security invariants above.

## Related records

- Issue #1037
- Issue #948 / PR #966
- `docs/architecture/MODULE_CATALOG.md`
- `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`
- `docs/architecture/SECURITY_ARCHITECTURE.md`
- `docs/contracts/CLIENT_DISTRIBUTION_UPDATE_CONTRACT.md`
