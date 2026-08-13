# Architecture Decision Records

ADRs record durable architecture decisions that should survive individual tasks and chat sessions. Architecture authority and precedence are defined by `../ARCHITECTURE_AUTHORITY.md` and ADR 0022.

## Lifecycle values

Every ADR must contain exactly one lifecycle declaration using one of the repository's established forms:

```text
- Status: Accepted
Status: Accepted
## Status

Accepted
```

The lifecycle token must begin with one of:

- `Proposed`
- `Accepted`
- `Superseded`
- `Rejected`

A declaration may add a bounded qualifier or date after the lifecycle token. Multiple lifecycle declarations in one ADR are rejected. A `Superseded` record must identify a resolvable replacement. Read the ADR itself for exact scope and activation limits.

## Allocation

Allocate a new ADR only after scanning every file in this directory and open architecture PRs. Use the next integer after the highest observed numeric prefix; do not reuse gaps.

Existing duplicate identifiers are historical compatibility defects. Do not rename or renumber accepted records without a separate compatibility decision that preserves inbound references.

## Machine validation

Run:

```bash
python3 tools/validation/adr_registry.py
python3 tools/validation/test_adr_registry.py
```

The repository test suite executes both through `tools/validation/phpunit/AdrRegistryValidationTest.php`, registered in `phpunit.xml` without changing workflow files.

The validator fails closed for:

- invalid ADR filenames;
- missing or ambiguous lifecycle declarations;
- README inventory drift;
- a new duplicate numeric prefix;
- any change to the exact historical duplicate-path allowlist;
- a missing or ambiguous supersession target.

The historical exception is a closed exact-path allowlist, not permission to create more collisions. Existing paths remain the stable inbound-reference identity for this bounded compatibility repair.

## Inventory

Inventory reconciled on 2026-08-09. Duplicate prefixes are intentionally shown rather than hidden.

- `0001-laravel-modular-monolith.md`
- `0002-separate-platform-and-canary-repositories.md`
- `0003-defer-payments-module.md`
- `0004-authoritative-platform-account-ownership.md`
- `0005-character-creation-product-policy.md`
- `0006-admin-rbac-and-audit-policy.md`
- `0007-phase7-engineering-and-production-go-live-gate.md`
- `0008-oteryn-frontend-information-and-shell-architecture.md`
- `0008-risk-based-continuous-e2e-validation.md`
- `0009-oteryn-game-authentication-architecture.md`
- `0010-native-gameplay-protocol-selection.md`
- `0010-wiki-module-and-persistence-foundation.md`
- `0011-safe-editorial-media-boundary.md`
- `0011-single-native-protocol-version.md`
- `0012-public-wiki-read-search.md`
- `0013-wiki-administration.md`
- `0014-wiki-editorial-media-integration.md`
- `0015-machine-enforced-portal-acceptance-ledger.md`
- `0015-wiki-launch-content-provisioning.md`
- `0016-character-bazaar-wallet-and-escrow.md`
- `0016-versioned-game-catalog-snapshots.md`
- `0017-account-security-lifecycle.md`
- `0017-platform-support-moderation-boundary.md`
- `0018-game-catalog-unknown-verified-boundary.md`
- `0018-read-only-community-data-boundary.md`
- `0019-game-catalog-runtime-loot-thresholds.md`
- `0020-use-single-level-gateway-public-hostname.md`
- `0021-provider-neutral-payment-security-core.md`
- `0021-require-canary-owned-character-deletion-lifecycle.md`
- `0022-architecture-authority-index-and-focused-canonical-documents.md`
- `0023-machine-readable-architecture-decision-backlog.md`
- `0024-merged-source-branch-lifecycle-policy.md`
- `0025-player-companion-and-portal-tools-boundary.md`
- `0026-proprietary-repository-licensing-policy.md`
- `0027-confidential-vulnerability-disclosure-policy.md`
- `0028-platform-accountid-cross-boundary-identity.md`
- `0029-platform-world-channel-identity-and-topology.md`
- `0030-native-character-portfolio-account-center-v2.md`
- `0031-native-oteryn-v2-integration-boundary.md`
- `0032-portal-composition-tracking-and-server-system-ownership.md`
- `0033-federated-content-search-and-discoverability.md`
- `0034-native-game-catalog-content-ownership.md`

## Preserved legacy duplicate paths

The validator permits exactly these historical sets:

- `0008`: `0008-oteryn-frontend-information-and-shell-architecture.md`, `0008-risk-based-continuous-e2e-validation.md`
- `0010`: `0010-native-gameplay-protocol-selection.md`, `0010-wiki-module-and-persistence-foundation.md`
- `0011`: `0011-safe-editorial-media-boundary.md`, `0011-single-native-protocol-version.md`
- `0015`: `0015-machine-enforced-portal-acceptance-ledger.md`, `0015-wiki-launch-content-provisioning.md`
- `0016`: `0016-character-bazaar-wallet-and-escrow.md`, `0016-versioned-game-catalog-snapshots.md`
- `0017`: `0017-account-security-lifecycle.md`, `0017-platform-support-moderation-boundary.md`
- `0018`: `0018-game-catalog-unknown-verified-boundary.md`, `0018-read-only-community-data-boundary.md`
- `0021`: `0021-provider-neutral-payment-security-core.md`, `0021-require-canary-owned-character-deletion-lifecycle.md`

Changing one of these exact sets requires an explicit compatibility review. Every other duplicate prefix is rejected.

When a decision changes, add a new ADR and mark the old one `Superseded` rather than rewriting history silently.
