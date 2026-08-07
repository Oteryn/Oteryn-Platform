---
task_id: OTERYN-20260808-oteryn-v2-integration-baseline
required_reads:
  - AGENTS.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
search_first:
  - Oteryn-v2
  - Legacy Canary Compatibility
optional_reads: []
---

# OTERYN-20260808-oteryn-v2-integration-baseline

## Result

`completed`

- Issue: #863 — closed completed.
- Delivery PR: #866.
- Exact delivery head: `8640ffab6af19256ab634cdd45288492e3bfcc9f`.
- Squash merge: `4bbed105a66b55476698c8f6ce4075671b3a10fe`.
- Canonical ADR: `docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md`.
- Focused architecture: `docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md`.
- Architecture review evidence: `docs/agents/reports/OTERYN-20260808-platform-v2-architecture-reconciliation.md`.
- External repository writes: none.
- Runtime/schema/protocol/deployment/production changes: none.

## Accepted architecture

- Oteryn Platform remains a Laravel modular monolith.
- Native Oteryn-v2 integration is explicitly separated from `Legacy Canary Compatibility`.
- Platform owns canonical `AccountId`, Identity/OAuth+PKCE, Platform security/session lifecycle, Game Login Ticket, World Registry/routing, Gateway ticket redemption/pre-admission and Platform business/workflow state.
- The game domain owns canonical `CharacterId`, current account-character ownership, character mutation outcomes, final gameplay admission/admitted-session semantics, gameplay persistence, native `protocol-oteryn` semantics and authoritative gameplay analytics source facts.
- Native steady-state integration uses explicit commands, queries, events and projections rather than shared/direct game SQL.
- Canary numeric IDs, direct SQL, Canary session/protocol adapters and related credentials remain compatibility/migration state until separately authorized cutover proves replacement and rollback.
- ADR 0010 and ADR 0011 are superseded by ADR 0031 for target native gameplay protocol ownership/family semantics; the useful one-native-version/no-profile principle is retained in ADR 0031.
- PublicGameData and Game Analytics native consumers use explicit projections/query contracts with freshness/reconciliation semantics.

## Validation

Exact PR #866 head `8640ffab6af19256ab634cdd45288492e3bfcc9f`:

- CI: PASS.
- Agent Governance: PASS.
- Native protocol contract: PASS.
- Native protocol contract audits: PASS.
- Game Auth Ticket Concurrency: PASS.
- Platform DB Outage Validation: PASS.
- Edge Security Emulation: PASS.
- Phase 7 Production-Like Validation: PASS.
- Full 9-file exact-head diff self-review: PASS, zero material findings.
- Unresolved review threads: 0.
- E2E: `NOT_APPLICABLE` — architecture/documentation-only change.

## Deferred decisions

P1: Character command/result transport/schema, PublicGameData projection catalogue/freshness, World Registry/LiveOps runtime status, entitlements delivery saga, moderation enforcement command, native Game Catalog ownership and exact pre-admission→game-session handoff.

P2: unified correlation/security envelope, per-adapter Canary sunset criteria and mixed-version contract-drift monitoring.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T01:07:40+02:00
status: completed
branch: docs/OTERYN-20260808-oteryn-v2-integration-baseline
pr: 866
merge_sha: 4bbed105a66b55476698c8f6ce4075671b3a10fe
issue: 863
proven:
  - ADR 0031 and the focused integration architecture are merged on main through PR 866.
  - All eight exact-head workflows passed before merge.
  - Issue 863 is closed completed.
  - No external repository was mutated.
derived:
  - Future Platform native consumers must design against the Native Oteryn-v2 Integration boundary rather than Canary compatibility identifiers/schema/protocol assumptions.
unknown:
  - Deferred P1/P2 contract details remain intentionally unresolved and require focused later decisions.
conflicts: []
blockers: []
next_action: Select the next highest-risk Platform architecture question from current main; do not reopen ADR 0031 ownership boundaries unless higher-ranked authority changes.
```
