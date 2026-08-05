# Architecture Decision Records

ADRs record durable architecture decisions that should survive individual tasks and chat sessions. Architecture authority and precedence are defined by `../ARCHITECTURE_AUTHORITY.md` and ADR 0022.

## Lifecycle values

The lifecycle token begins with one of:

- `Proposed`
- `Accepted`
- `Superseded`
- `Rejected`

A record may add a bounded qualifier after the lifecycle token. Read the ADR itself for exact scope and activation limits.

## Allocation

Allocate a new ADR only after scanning every file in this directory and open architecture PRs. Use the next integer after the highest observed numeric prefix; do not reuse gaps.

Existing duplicate identifiers are historical compatibility defects. Do not rename or renumber accepted records without a separate compatibility decision that preserves inbound references.

## Inventory

Inventory reconciled on 2026-08-05. Duplicate prefixes are intentionally shown rather than hidden.

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

## Known registry debt

Historical duplicate prefixes currently exist for `0008`, `0010`, `0011`, `0015`, `0016`, `0017`, `0018` and `0021`. The inventory is navigational and collision-aware; it does not resolve those compatibility defects.

A follow-up validator should fail closed for any new duplicate identifier, missing lifecycle token, broken supersession target or inventory mismatch while preserving all existing accepted paths.

When a decision changes, add a new ADR and mark the old one `Superseded` rather than rewriting history silently.