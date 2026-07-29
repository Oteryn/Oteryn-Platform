---
task_id: OTERYN-20260728-account-security-lifecycle
required_reads:
  - AGENTS.md
  - docs/agents/PROJECT_STATE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/testing/PRODUCT_COMPLETENESS_BENCHMARK.md
---

# OTERYN-20260728-account-security-lifecycle

## Goal

Close Issue #276 with a secure Platform-owned account lifecycle: confirmed primary-email change, registered active-session inventory and targeted revocation, privacy/status controls, bounded termination, a verifier-only high-assurance recovery artifact, and durable deny-by-default decisions for exceptional Canary binding changes and email-code MFA.

## Result

- PR #283 merged as `28faad47f95df10d1a9b437a16a1be91556671c6`.
- Issue #276 closed automatically as completed.
- Platform Identity now provides confirmed email change with old-address recovery, registered session inventory/revocation, private-by-default status controls, verifier-only one-time recovery keys and bounded account termination with grace/cancellation/finalization.
- English and Polish account-security UI, validation, token errors and notification links are covered across desktop, tablet and mobile.
- Self-service Canary import, unlink, rebind or transfer remains intentionally unavailable without a separately reviewed operation contract; Canary remained read-only throughout delivery.
- Email-code MFA remains intentionally excluded because email is already the recovery channel.
- Repository and isolated acceptance evidence do not establish production deployment or `PRODUCTION_PROVEN` status.

## Final checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-29T08:00:00+02:00
head: 9b740ebfb61422bb9ecc51a114e74712075d5eb3
branch: feat/OTERYN-20260728-account-security-lifecycle
pr: 283
merge_sha: 28faad47f95df10d1a9b437a16a1be91556671c6
status: completed
context_routes:
  - agent-governance
  - architecture
  - auth-identity
  - accounts-characters
  - canary-integration
  - security
  - web-cms
  - testing
proven:
  - Primary-email change, registered-session inventory/revocation, privacy controls, verifier-only recovery keys and bounded Platform termination are implemented with owner-scoped authorization, replay denial and bounded audit metadata.
  - Stale or revoked registered sessions are invalidated before protected controllers execute.
  - Guest redirects preserve an allowlisted English or Polish account-security locale before login rendering.
  - The 43-capability product ledger now classifies 9 implemented, 8 partial, 25 missing and 1 not applicable capability.
  - All 12 required workflows passed on exact final PR head 9b740ebfb61422bb9ecc51a114e74712075d5eb3 before squash merge.
  - PR #283 merged as 28faad47f95df10d1a9b437a16a1be91556671c6 and automatically closed Issue #276.
  - Canary and login-server schema or session compatibility were unchanged; blakinio/canary remained read-only.
derived:
  - Issue #276 is complete for the approved Platform-owned boundary.
  - Trust derives from authenticated Platform Identity, registered session state and the server-resolved immutable binding; browser identifiers do not establish ownership.
unknown:
  - Direct production behavior until Issue #91 is separately authorized and executed against an exact deployed release.
conflicts: []
first_failure:
  marker: phpstan-test-helper-method-not-found
  evidence: CI initially rejected a test-only assertion call; after correction, exact-head static analysis and the complete workflow set passed.
rejected_hypotheses:
  - Hardcoded Polish copy was sufficient instead of complete localization dictionaries and localized domain outcomes.
  - Route middleware ordering alone always preserved locale through guest authentication redirects.
  - First-slice termination should mutate, unlink or delete Canary-owned accounts or characters.
  - Email-code MFA should be treated as a delivered second factor while email remains the recovery channel.
validation:
  - command: CI 30397675722, Agent Governance 30397675719, Account Security Format Diagnostics 30397675673 and Static Diagnostics 30397675704
    result: PASS
    evidence: Composer validation/audit, Pint, PHPStan, full PHPUnit and checkpoint governance passed on exact final head 9b740ebfb61422bb9ecc51a114e74712075d5eb3.
  - command: Portal Acceptance Contract 30397675723 and Acceptance E2E and Visual UX 30397675886
    result: PASS
    evidence: strict route/product ledgers, all nine zero-retry account lifecycle scenarios and complete E2E/visual UX passed.
  - command: Phase 7 30397675693, Synology preflight 30397675741, image build 30397675676, DB outage 30397675884, edge security 30397675672 and game-auth concurrency 30397675649
    result: PASS
    evidence: every required production-like, infrastructure, failure-path and concurrency workflow passed on the exact final head.
blockers:
  - none
next_action: None. This task lifecycle is complete; remaining benchmark and production work belongs to separate issues #277-#281 and #91.
```

## Boundary

This archive closes the Platform-owned account-security lifecycle. It does not authorize Canary mutation or claim direct production verification.
