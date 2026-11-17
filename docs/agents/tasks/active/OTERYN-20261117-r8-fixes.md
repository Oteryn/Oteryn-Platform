---
task_id: OTERYN-20261117-r8-fixes
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-r8-fixes

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Review round 8: eleven small-slice Codex reviews of portal-rs on the round 7
head. Findings were verified with red-then-green regression tests by Codex Cloud
fix packets (identity, admin, support, community, characters, wiki, store,
wallet, payments, bazaar, database, gameauth); each pushed fix was reviewed
before integration, and follow-ups were sent where the review found gaps. Also
archives the round 7 packet after PR #1943 merged.

## Acceptance criteria

- [ ] Each confirmed finding has a fix with a regression test through the real caller
- [ ] Each pushed fix was reviewed read-only before integration
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-r7-fixes.md
  - docs/agents/tasks/active/OTERYN-20261117-r8-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261117-r7-fixes.md
  - portal-rs/bin/oteryn-portal/src/code_routes.rs
  - portal-rs/bin/oteryn-portal/src/code_routes_tests.rs
  - portal-rs/bin/oteryn-portal/src/registration_delivery.rs
  - portal-rs/bin/oteryn-portal/src/wallet_topup.rs
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/bin/oteryn-portal/templates/code_issuance.html
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/crates/admin/src/console/access.rs
  - portal-rs/crates/admin/src/console/audit.rs
  - portal-rs/crates/admin/src/console/mod.rs
  - portal-rs/crates/admin/src/console/players.rs
  - portal-rs/crates/admin/src/console/stored_tests.rs
  - portal-rs/crates/admin/src/players/mod.rs
  - portal-rs/crates/admin/src/players/tests.rs
  - portal-rs/crates/admin/src/rbac/mod.rs
  - portal-rs/crates/admin/src/rbac/storage.rs
  - portal-rs/crates/admin/src/rbac/tests.rs
  - portal-rs/crates/admin/src/web.rs
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold_worker.rs
  - portal-rs/crates/bazaar/src/moderation.rs
  - portal-rs/crates/characters/src/create_web/http.rs
  - portal-rs/crates/characters/src/create_web/tests.rs
  - portal-rs/crates/characters/src/overview/tests.rs
  - portal-rs/crates/characters/src/profile/mod.rs
  - portal-rs/crates/characters/src/profile/tests.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
  - portal-rs/crates/community/src/referrals/en.ftl
  - portal-rs/crates/community/src/referrals/pl.ftl
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/community/src/referrals/web.rs
  - portal-rs/crates/content/src/events/repository.rs
  - portal-rs/crates/content/src/events/tests.rs
  - portal-rs/crates/db/src/outbox.rs
  - portal-rs/crates/gameauth/src/admission/http.rs
  - portal-rs/crates/gameauth/src/admission/store.rs
  - portal-rs/crates/gameauth/src/admission/tests.rs
  - portal-rs/crates/gameauth/src/devices.rs
  - portal-rs/crates/gameauth/src/devices_http.rs
  - portal-rs/crates/gameauth/src/devices_tests.rs
  - portal-rs/crates/gameauth/src/devices_tls_tests.rs
  - portal-rs/crates/gameauth/src/devices_wire.rs
  - portal-rs/crates/identity/src/mfa_http.rs
  - portal-rs/crates/identity/src/mfa_http_tests.rs
  - portal-rs/crates/identity/src/recovery_key_http.rs
  - portal-rs/crates/identity/src/recovery_key_tests.rs
  - portal-rs/crates/identity/src/verify.rs
  - portal-rs/crates/identity/tests/registration.rs
  - portal-rs/crates/internal-api/src/playerdata.rs
  - portal-rs/crates/payments/src/coin_store.rs
  - portal-rs/crates/payments/src/core.rs
  - portal-rs/crates/payments/tests/payments.rs
  - portal-rs/crates/payments/tests/provider_webhook.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/journal_tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/sessions.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/http.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
  - portal-rs/crates/store/src/codes.rs
  - portal-rs/crates/store/src/codes_tests.rs
  - portal-rs/crates/store/src/web.rs
  - portal-rs/crates/store/src/web_README.md
  - portal-rs/crates/store/src/web_backend.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/store/src/web_payments_tests.rs
  - portal-rs/crates/store/src/web_tests.rs
  - portal-rs/crates/support/src/reports/tests.rs
  - portal-rs/crates/support/src/reports/web.rs
  - portal-rs/crates/support/src/tickets/mod.rs
  - portal-rs/crates/support/src/tickets/tests.rs
  - portal-rs/crates/support/src/tickets/web.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/crates/web/src/lib.rs
  - portal-rs/crates/web/src/routes/verify.rs
  - portal-rs/crates/web/src/routes/verify_budget_tests.rs
  - portal-rs/crates/wiki/src/entities.rs
  - portal-rs/crates/wiki/src/entities/tests.rs
  - portal-rs/e2e/specs/staff-commerce-codes.spec.mjs
  - portal-rs/migrations/20261117210007_r8_db.sql
modules:
  - admin
  - atlas
  - bazaar
  - characters
  - community
  - downloads
  - gameauth
  - gateway
  - hirelings
  - identity
  - internal-api
  - minigames
  - oteryn-portal
  - payments
  - player-api
  - playerdata
  - store
  - support
  - wallet
  - web
  - wiki
  - world
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-17T23:25:00Z
head: pending
branch: claude/happy-davinci-ijz3wy-r8-fixes
pr: pending
status: implementing
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-r7-fixes.md
  - docs/agents/tasks/active/OTERYN-20261117-r8-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261117-r7-fixes.md
  - portal-rs/bin/oteryn-portal/src/code_routes.rs
  - portal-rs/bin/oteryn-portal/src/code_routes_tests.rs
  - portal-rs/bin/oteryn-portal/src/registration_delivery.rs
  - portal-rs/bin/oteryn-portal/src/wallet_topup.rs
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/bin/oteryn-portal/templates/code_issuance.html
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/crates/admin/src/console/access.rs
  - portal-rs/crates/admin/src/console/audit.rs
  - portal-rs/crates/admin/src/console/mod.rs
  - portal-rs/crates/admin/src/console/players.rs
  - portal-rs/crates/admin/src/console/stored_tests.rs
  - portal-rs/crates/admin/src/players/mod.rs
  - portal-rs/crates/admin/src/players/tests.rs
  - portal-rs/crates/admin/src/rbac/mod.rs
  - portal-rs/crates/admin/src/rbac/storage.rs
  - portal-rs/crates/admin/src/rbac/tests.rs
  - portal-rs/crates/admin/src/web.rs
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold_worker.rs
  - portal-rs/crates/bazaar/src/moderation.rs
  - portal-rs/crates/characters/src/create_web/http.rs
  - portal-rs/crates/characters/src/create_web/tests.rs
  - portal-rs/crates/characters/src/overview/tests.rs
  - portal-rs/crates/characters/src/profile/mod.rs
  - portal-rs/crates/characters/src/profile/tests.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
  - portal-rs/crates/community/src/referrals/en.ftl
  - portal-rs/crates/community/src/referrals/pl.ftl
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/community/src/referrals/web.rs
  - portal-rs/crates/content/src/events/repository.rs
  - portal-rs/crates/content/src/events/tests.rs
  - portal-rs/crates/db/src/outbox.rs
  - portal-rs/crates/gameauth/src/admission/http.rs
  - portal-rs/crates/gameauth/src/admission/store.rs
  - portal-rs/crates/gameauth/src/admission/tests.rs
  - portal-rs/crates/gameauth/src/devices.rs
  - portal-rs/crates/gameauth/src/devices_http.rs
  - portal-rs/crates/gameauth/src/devices_tests.rs
  - portal-rs/crates/gameauth/src/devices_tls_tests.rs
  - portal-rs/crates/gameauth/src/devices_wire.rs
  - portal-rs/crates/identity/src/mfa_http.rs
  - portal-rs/crates/identity/src/mfa_http_tests.rs
  - portal-rs/crates/identity/src/recovery_key_http.rs
  - portal-rs/crates/identity/src/recovery_key_tests.rs
  - portal-rs/crates/identity/src/verify.rs
  - portal-rs/crates/identity/tests/registration.rs
  - portal-rs/crates/internal-api/src/playerdata.rs
  - portal-rs/crates/payments/src/coin_store.rs
  - portal-rs/crates/payments/src/core.rs
  - portal-rs/crates/payments/tests/payments.rs
  - portal-rs/crates/payments/tests/provider_webhook.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/journal_tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/sessions.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/http.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
  - portal-rs/crates/store/src/codes.rs
  - portal-rs/crates/store/src/codes_tests.rs
  - portal-rs/crates/store/src/web.rs
  - portal-rs/crates/store/src/web_README.md
  - portal-rs/crates/store/src/web_backend.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/store/src/web_payments_tests.rs
  - portal-rs/crates/store/src/web_tests.rs
  - portal-rs/crates/support/src/reports/tests.rs
  - portal-rs/crates/support/src/reports/web.rs
  - portal-rs/crates/support/src/tickets/mod.rs
  - portal-rs/crates/support/src/tickets/tests.rs
  - portal-rs/crates/support/src/tickets/web.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/crates/web/src/lib.rs
  - portal-rs/crates/web/src/routes/verify.rs
  - portal-rs/crates/web/src/routes/verify_budget_tests.rs
  - portal-rs/crates/wiki/src/entities.rs
  - portal-rs/crates/wiki/src/entities/tests.rs
  - portal-rs/e2e/specs/staff-commerce-codes.spec.mjs
  - portal-rs/migrations/20261117210007_r8_db.sql
proven:
  - Each round-7 finding was confirmed by a read-only check with file:line evidence
  - Each pushed fix was reviewed read-only against its packet before integration
derived:
  - none
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-r7-fixes.md
  - docs/agents/tasks/active/OTERYN-20261117-r8-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261117-r7-fixes.md
  - portal-rs/bin/oteryn-portal/src/code_routes.rs
  - portal-rs/bin/oteryn-portal/src/code_routes_tests.rs
  - portal-rs/bin/oteryn-portal/src/registration_delivery.rs
  - portal-rs/bin/oteryn-portal/src/wallet_topup.rs
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/bin/oteryn-portal/templates/code_issuance.html
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/crates/admin/src/console/access.rs
  - portal-rs/crates/admin/src/console/audit.rs
  - portal-rs/crates/admin/src/console/mod.rs
  - portal-rs/crates/admin/src/console/players.rs
  - portal-rs/crates/admin/src/console/stored_tests.rs
  - portal-rs/crates/admin/src/players/mod.rs
  - portal-rs/crates/admin/src/players/tests.rs
  - portal-rs/crates/admin/src/rbac/mod.rs
  - portal-rs/crates/admin/src/rbac/storage.rs
  - portal-rs/crates/admin/src/rbac/tests.rs
  - portal-rs/crates/admin/src/web.rs
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold_worker.rs
  - portal-rs/crates/bazaar/src/moderation.rs
  - portal-rs/crates/characters/src/create_web/http.rs
  - portal-rs/crates/characters/src/create_web/tests.rs
  - portal-rs/crates/characters/src/overview/tests.rs
  - portal-rs/crates/characters/src/profile/mod.rs
  - portal-rs/crates/characters/src/profile/tests.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
  - portal-rs/crates/community/src/referrals/en.ftl
  - portal-rs/crates/community/src/referrals/pl.ftl
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/community/src/referrals/web.rs
  - portal-rs/crates/content/src/events/repository.rs
  - portal-rs/crates/content/src/events/tests.rs
  - portal-rs/crates/db/src/outbox.rs
  - portal-rs/crates/gameauth/src/admission/http.rs
  - portal-rs/crates/gameauth/src/admission/store.rs
  - portal-rs/crates/gameauth/src/admission/tests.rs
  - portal-rs/crates/gameauth/src/devices.rs
  - portal-rs/crates/gameauth/src/devices_http.rs
  - portal-rs/crates/gameauth/src/devices_tests.rs
  - portal-rs/crates/gameauth/src/devices_tls_tests.rs
  - portal-rs/crates/gameauth/src/devices_wire.rs
  - portal-rs/crates/identity/src/mfa_http.rs
  - portal-rs/crates/identity/src/mfa_http_tests.rs
  - portal-rs/crates/identity/src/recovery_key_http.rs
  - portal-rs/crates/identity/src/recovery_key_tests.rs
  - portal-rs/crates/identity/src/verify.rs
  - portal-rs/crates/identity/tests/registration.rs
  - portal-rs/crates/internal-api/src/playerdata.rs
  - portal-rs/crates/payments/src/coin_store.rs
  - portal-rs/crates/payments/src/core.rs
  - portal-rs/crates/payments/tests/payments.rs
  - portal-rs/crates/payments/tests/provider_webhook.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/journal_tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/sessions.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/http.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
  - portal-rs/crates/store/src/codes.rs
  - portal-rs/crates/store/src/codes_tests.rs
  - portal-rs/crates/store/src/web.rs
  - portal-rs/crates/store/src/web_README.md
  - portal-rs/crates/store/src/web_backend.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/store/src/web_payments_tests.rs
  - portal-rs/crates/store/src/web_tests.rs
  - portal-rs/crates/support/src/reports/tests.rs
  - portal-rs/crates/support/src/reports/web.rs
  - portal-rs/crates/support/src/tickets/mod.rs
  - portal-rs/crates/support/src/tickets/tests.rs
  - portal-rs/crates/support/src/tickets/web.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/crates/web/src/lib.rs
  - portal-rs/crates/web/src/routes/verify.rs
  - portal-rs/crates/web/src/routes/verify_budget_tests.rs
  - portal-rs/crates/wiki/src/entities.rs
  - portal-rs/crates/wiki/src/entities/tests.rs
  - portal-rs/e2e/specs/staff-commerce-codes.spec.mjs
  - portal-rs/migrations/20261117210007_r8_db.sql
validation:
  - command: cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0 on the integrated tree
  - command: cargo test per crate with PostgreSQL 18
    result: PASS
    evidence: payments 50, admin 221, wallet 40, playerdata 181, internal-api 14, identity 193, web 68, support 103, store 136, wiki 47, oteryn-portal 238, community 194, db 12
  - command: Portal RS CI on the PR head
    result: NOT_RUN
    evidence: full suite runs in CI shards
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: server-side fixes covered by tests through the real routers and workers
blockers:
  - none
next_action: Portal RS CI on the PR head, then squash auto-merge
```
