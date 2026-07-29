# Changelog

All notable architecture- and behavior-level changes to Oteryn Platform should be recorded here.

## Unreleased

### Game Catalog schema 1.2 — 2026-07-29

- Added version-pinned `canary_dynamic_threshold_v1` loot chance support.
- Preserved configured thresholds and declared roll maxima without clamping or false percentage rendering.
- Kept schema 1.0/1.1 rational rows compatible and added a rollback guard for unrepresentable threshold rows.

### Game Catalog schema 1.1 — 2026-07-29

- Added version-pinned schema `1.1.0` support for an explicitly unknown verified-content boundary.
- Preserved schema `1.0.0` activation and rollback compatibility.
- Allowed inactive import of unknown-boundary snapshots while blocking activation and public projection until a concrete verified release exists.
- Made the persisted verified-content release nullable without introducing a sentinel release.

### Architecture bootstrap — 2026-07-18

- Established Oteryn Platform as the planned first-party replacement for MyAAC.
- Defined Laravel modular monolith as the initial target architecture.
- Kept Canary in a separate repository with explicit cross-repository contracts.
- Defined module ownership for Identity, Accounts, Characters, PublicGameData, CMS, Admin, Audit, Integration, Notifications, PlatformAPI and deferred Payments.
- Added mandatory security architecture covering auth, sessions, MFA, recovery, RBAC, browser security, rate limiting, secrets, edge/origin protection and production readiness.
- Added data ownership rules for platform-owned, Canary-owned and shared-contract data.
- Added test strategy including security regression, contract and end-to-end validation.
- Added phased roadmap from Laravel bootstrap through production hardening and deferred payments.
- Added discovery contracts for Canary data integration and web-to-game authentication.
- Added ADRs for the initial application architecture, repository separation and deferred payments.
- Added durable agent project state, active-work routing, task checkpoints and handoff rules.
