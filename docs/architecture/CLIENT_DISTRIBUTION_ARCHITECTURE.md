# Oteryn Client Distribution Architecture

## Status

**CURRENT — accepted through ADR 0035 / Issue #1037 on 2026-08-13.**

This document is the focused canonical Platform architecture for first-party client distribution and updater trust. It records the accepted Option A boundary without claiming runtime implementation, private signing-key custody, external-client implementation, deployment or production activation.

## Decision summary

Oteryn uses a **TUF-based, role-separated updater trust model** for first-party automatic client updates.

The accepted boundary is:

```text
Platform Downloads release policy
        |
        v
approved immutable release/artifact identity
        |
        v
protected release-signing / TUF repository boundary
        |
        v
signed versioned expiring TUF metadata
        + authenticated Oteryn channel-policy target
        |
        v
first-party updater verification and update decision
```

The Laravel web application is not a private software-signing-key custodian and browser Download Center state is not updater trust authority.

## Authority split

| Concern | Authority | Explicit non-authority |
|---|---|---|
| Browser release metadata, administrator publication workflow and release-policy intent | Platform `Downloads` | updater cryptographic trust, game admission |
| Artifact bytes | approved immutable artifact storage/reference | mutable aliases, database row identity |
| Publisher-authenticated updater metadata | accepted TUF repository/signing boundary | HTTPS alone, administrator-supplied checksum alone |
| Root trust and role delegation | TUF Root metadata bootstrapped by the first-party updater | Laravel configuration, CMS, browser session |
| Stable release authorization | stable-scoped delegated target authority | beta-scoped routine credential |
| Beta release authorization | beta-scoped delegated target authority | stable-scoped routine credential |
| Repository coherence/freshness | TUF metadata version, expiry, Snapshot/Timestamp semantics | `is_current`, page age, cache age |
| Minimum supported release, optional/recommended/required mode, revocation and explicit rollback intent | authenticated Oteryn channel-policy target | display-version string comparison, CMS prose |
| Client update/launch distribution decision | first-party updater over verified trusted state | Download Center HTML/API response |
| Gameplay/protocol admission | separately authoritative game/admission boundary | updater success or distribution state |

## Current Platform compatibility baseline

The delivered Download Center remains a valid browser-oriented foundation:

- channels are `stable` and `beta`;
- release publication maintains **one current release per channel**;
- a release may contain platform/architecture artifact variants;
- supported platform labels currently include Windows, Linux and macOS;
- supported architecture labels currently include x86_64, arm64 and x86;
- published artifact references must satisfy the immutable-reference policy delivered by Issue #948 / PR #966;
- Platform does not fetch artifacts and an administrator-supplied SHA-256 is not independent publisher-verification evidence.

ADR 0035 preserves these truths instead of reinterpreting browser `is_current` as cryptographic update authority.

## Updater policy scope

Updater-policy schema v1 keeps one current release identity and release sequence per channel.

For each signed channel-policy generation the logical policy contains at least:

```text
schema_version
policy_revision
channel
current_release_id
current_release_sequence
current_version_display
minimum_supported_release_sequence
update_mode = optional | recommended | required
artifacts[] = { platform, architecture, target_path }
revoked_release_ids[]
revoked_artifact_targets[]
rollback_authorization = none | explicit
```

This is a semantic contract, not final wire bytes.

### Release ordering

- security ordering uses positive monotonic channel-scoped release sequences;
- display `version` remains presentation metadata and is not a trust-ordering primitive;
- historical browser-only releases need not be retroactively assigned semantic ordering unless an implementation migration explicitly proves it;
- normal publication advances policy/release state monotonically;
- an older immutable release may become current only through a newer trusted policy revision carrying explicit rollback authorization.

### Exact target selection

A current release may contain multiple artifact targets, but the updater selects exactly the configured `(platform, architecture)` entry.

- missing exact target -> unavailable;
- revoked exact target -> unavailable/revoked for that target;
- no fallback to another architecture;
- no fallback to another platform;
- no fallback from stable to beta;
- no implicit selection of an older release.

A future need for independent current/minimum timelines per platform or architecture requires a new compatible architecture/schema decision.

## TUF trust profile requirements

Implementation must produce a documented TUF POUF/profile satisfying the accepted semantic boundary before activation.

At minimum:

1. Root, Targets, Snapshot and Timestamp trust roles are present.
2. Root trust is bootstrapped with the first-party updater.
3. Root private-key custody is offline and threshold-controlled rather than reduced to one web-runtime secret.
4. Stable and beta routine publication authorities are cryptographically separated by delegated target scope.
5. Consistent-snapshot behavior is used for repository metadata/targets.
6. Metadata versions and expiration are enforced.
7. Target hash and length are verified from trusted target metadata.
8. Snapshot metadata binds a coherent metadata generation rather than permitting mix-and-match state.
9. Timestamp freshness is maintained by a narrowly scoped online authority suitable to its role.
10. Trust-root/key rotation and compromise recovery are documented before production activation.

Exact maintained libraries, algorithms supported by those libraries, key backend, metadata serialization, numerical expiration intervals and operational thresholds beyond the accepted Root non-single-key invariant remain implementation/operations decisions.

## Update-state semantics

For an installed release, the updater evaluates only verified, sufficiently fresh trusted metadata and the authenticated channel policy.

Conceptual states include:

- `CURRENT`;
- `SUPPORTED_UPDATE_OPTIONAL`;
- `SUPPORTED_UPDATE_RECOMMENDED`;
- `UPDATE_REQUIRED`;
- `UNSUPPORTED`;
- `REVOKED`;
- `TARGET_UNAVAILABLE`;
- `POLICY_INCOMPATIBLE`;
- `TRUST_UNAVAILABLE`.

Rules:

- release below minimum support fails normal online launch until a valid supported update is installed;
- revoked release fails normal online launch regardless of sequence;
- `required` blocks normal online launch until the selected current release is installed and verified;
- `optional` and `recommended` never override minimum support or revocation;
- if refresh fails while the last trusted state remains valid/unexpired, only that previously trusted state may continue to be used;
- expired, signature-invalid, rollback-invalid, tuple-incompatible or internally inconsistent required metadata fails closed for normal online launch;
- recovery/settings/diagnostic UI may remain available without claiming current update authority.

## Withdrawal, revocation and rollback

### Withdrawal

Withdrawal stops a release from being selected for new current/recommended policy while preserving immutable identity and historical signed state needed by supported clients and rollback windows.

### Release revocation

A newer trusted policy may mark an exact release identity unusable for the channel. Revocation never rewrites the artifact or historical metadata.

### Artifact-target revocation

A newer trusted policy may mark one immutable platform/architecture target unusable without automatically declaring byte-distinct sibling targets compromised.

### Deliberate rollback

A legitimate rollback is expressed by a **newer** trusted policy/TUF generation selecting an already immutable older release with explicit rollback authorization. Metadata versions and policy revision still move forward.

Repointing an existing release identity or target path to different bytes is forbidden.

## Key and compromise boundary

- Laravel stores no private updater-signing keys;
- ordinary Download Center administrator authority cannot mint trusted updater metadata by itself;
- Root custody remains offline/threshold-controlled;
- routine stable and beta publishing credentials are separately scoped;
- compromise of a delegated channel role is recovered through its trusted delegating authority;
- compromise of a top-level non-Root role requires Root-authorized replacement;
- threshold Root compromise is an out-of-band security-recovery incident, never an ordinary web-admin action;
- public keys, signatures, metadata identifiers and verified public projections may be stored/presented by Platform without becoming private-key custody.

## Platform publication lifecycle

Browser publication and updater publication are distinct states.

Implementation must represent at least:

1. browser-only draft/published release metadata;
2. immutable artifact-reference validation;
3. updater-policy generation approved/pending signing;
4. signed repository generation produced;
5. exact signed generation verified/reconciled by Platform;
6. updater-publishable/active generation;
7. superseded, withdrawn, revoked or rollback-selected state with the newer policy/generation identity.

The protected signer/repository and Platform cannot be treated as one database transaction. The boundary is an idempotent reconciled saga using stable operation/generation identity. Ambiguous results must reconcile the same operation rather than create duplicate policy revisions or activate whichever state is observed first.

## Browser Download Center relationship

Existing browser pages may continue to show approved release metadata and administrator-supplied SHA-256 values.

They must not claim publisher-signature verification until an implementation proves an exact association with verified signed updater metadata. Browser `is_current` and updater-current state may coexist during migration, but any mismatch for an updater-enabled channel is explicit degraded/unavailable state rather than silent truth selection.

## Game admission separation

Updater trust answers software-distribution questions only. It does not prove:

- world/runtime readiness;
- accepted gameplay protocol;
- account authorization;
- session creation authority;
- current game admission for the selected client build.

Game admission may be stricter than updater distribution policy and remains governed by the applicable native game/admission contracts.

## Implementation handoff

Platform implementation is tracked by Issue #1039.

Platform work may implement persistence, application services, administration/public presentation, signed-generation verification/projection and reconciliation required by this boundary. It must not move private signing keys into Laravel or claim external updater conformance.

A separate authorized client-side task must later implement and prove:

- Root bootstrap and durable trusted metadata state;
- exact channel/target selection;
- metadata verification/freshness/rollback handling;
- tampered/replayed/expired/mix-and-match negative paths;
- download hash/length validation;
- hardened package installation and prior-version recovery;
- user-facing blocked/recovery states.

Production activation additionally requires exact signing infrastructure, metadata origin/TLS/availability, key rotation/recovery runbooks and real client E2E evidence.

## Evidence state

### Accepted / current

- architecture choice: TUF-based role-separated updater trust;
- stable/beta channel separation;
- one current release per channel in schema v1;
- exact platform/architecture target selection without fallback;
- private signing keys outside Laravel;
- signed minimum/update/revocation/rollback policy;
- updater and game-admission authority separation.

### Not yet implemented/proven

- Platform updater-publishing persistence/runtime;
- maintained TUF implementation and POUF/profile;
- protected signing infrastructure;
- client updater integration;
- exact metadata expiry/SLO values;
- production distribution endpoint and deployment evidence;
- real updater E2E and game-admission compatibility evidence.

Accepted architecture is not implementation or production proof.

## Related authority

- ADR 0035 — First-party client distribution and updater trust boundary
- Issue #1037 — architecture decision
- Issue #1039 — Platform implementation handoff
- Issue #948 / PR #966 — immutable artifact-reference enforcement
- `docs/architecture/ARCHITECTURE_AUTHORITY.md`
- `docs/architecture/MODULE_CATALOG.md`
- `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`
- `docs/architecture/SECURITY_ARCHITECTURE.md`
