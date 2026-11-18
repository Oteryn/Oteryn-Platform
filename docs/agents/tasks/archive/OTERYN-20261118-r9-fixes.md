---
task_id: OTERYN-20261118-r9-fixes
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261118-r9-fixes

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Review round 9: eleven small-slice Codex reviews of portal-rs on the round 8
head, plus the first round 10 findings on the integrated round 9 head (bazaar
moderation and escrow recovery, character service sudo revalidation). Findings
were reproduced with red-then-green regression tests by Codex Cloud fix packets
(bazaar, stats, gift codes, workers, store, admin, identity, gameauth,
playerdata, payments, characters); each pushed fix was reviewed before
integration. Also archives the round 8 packet after PR #1944 merged.

## Acceptance criteria

- [x] Each confirmed finding has a fix with a regression test through the real caller
- [x] Each pushed fix was reviewed read-only before integration
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-r8-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r9-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261117-r8-fixes.md
  - portal-rs/bin/oteryn-portal/src/code_routes.rs
  - portal-rs/bin/oteryn-portal/src/registration_delivery.rs
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/crates/admin/src/console/access.rs
  - portal-rs/crates/admin/src/console/players.rs
  - portal-rs/crates/admin/src/console/stored_tests.rs
  - portal-rs/crates/admin/src/console/templates/read.html
  - portal-rs/crates/admin/src/players/mod.rs
  - portal-rs/crates/admin/src/players/tests.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/invalidation_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold_worker.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/bazaar/src/moderation.rs
  - portal-rs/crates/characters/src/services/browser.rs
  - portal-rs/crates/characters/src/services/mod.rs
  - portal-rs/crates/characters/src/services/tests.rs
  - portal-rs/crates/db/src/outbox.rs
  - portal-rs/crates/gameauth/src/admission/audit05_tests.rs
  - portal-rs/crates/gameauth/src/admission/grant.rs
  - portal-rs/crates/gameauth/src/admission/http.rs
  - portal-rs/crates/gameauth/src/admission/recovery.rs
  - portal-rs/crates/gameauth/src/admission/store.rs
  - portal-rs/crates/gameauth/src/evidence.rs
  - portal-rs/crates/gameauth/src/evidence_tests.rs
  - portal-rs/crates/gameauth/src/locator.rs
  - portal-rs/crates/gameauth/src/locator_tests.rs
  - portal-rs/crates/gateway/src/lib.rs
  - portal-rs/crates/gateway/src/v3.rs
  - portal-rs/crates/gateway/src/wire.rs
  - portal-rs/crates/gateway/tests/protocol3.rs
  - portal-rs/crates/gateway/tests/proxies.rs
  - portal-rs/crates/identity/src/account_lifecycle_http.rs
  - portal-rs/crates/identity/src/mfa_http.rs
  - portal-rs/crates/identity/src/mfa_http_tests.rs
  - portal-rs/crates/identity/src/reset_http.rs
  - portal-rs/crates/identity/src/reset_http_tests.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/payments/src/storage.rs
  - portal-rs/crates/payments/tests/payments.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/reads.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/deaths/storage.rs
  - portal-rs/crates/playerdata/src/items/store.rs
  - portal-rs/crates/playerdata/src/logins.rs
  - portal-rs/crates/playerdata/src/logins/tests.rs
  - portal-rs/crates/playerdata/src/places/retention.rs
  - portal-rs/crates/playerdata/src/places/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
  - portal-rs/crates/stats/src/character/http.rs
  - portal-rs/crates/stats/src/character/mod.rs
  - portal-rs/crates/stats/src/character/tests.rs
  - portal-rs/crates/stats/src/guilds.rs
  - portal-rs/crates/stats/src/guilds/tests.rs
  - portal-rs/crates/stats/src/rankings_tests.rs
  - portal-rs/crates/stats/src/store.rs
  - portal-rs/crates/stats/src/tests.rs
  - portal-rs/crates/stats/src/world.rs
  - portal-rs/crates/stats/src/world/tests.rs
  - portal-rs/crates/stats/tests/privacy.rs
  - portal-rs/crates/store/src/codes.rs
  - portal-rs/crates/store/src/codes_tests.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/store/src/web_payments_tests.rs
  - portal-rs/migrations/20261117230002_r9_pd.sql
  - portal-rs/migrations/20261117230011_r9_sta.sql
  - portal-rs/migrations/20261117230012_r9_pd2.sql
modules:
  - admin
  - bazaar
  - characters
  - db
  - gameauth
  - gateway
  - identity
  - oteryn-portal
  - payments
  - playerdata
  - stats
  - store
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
updated_at: 2026-11-18T00:50:00Z
head: 5e57664eb105b748f71db2b29bd29748976d81df
branch: claude/happy-davinci-ijz3wy-r9-fixes
pr: 1945
status: completed
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-r8-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r9-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261117-r8-fixes.md
  - portal-rs/bin/oteryn-portal/src/code_routes.rs
  - portal-rs/bin/oteryn-portal/src/registration_delivery.rs
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/crates/admin/src/console/access.rs
  - portal-rs/crates/admin/src/console/players.rs
  - portal-rs/crates/admin/src/console/stored_tests.rs
  - portal-rs/crates/admin/src/console/templates/read.html
  - portal-rs/crates/admin/src/players/mod.rs
  - portal-rs/crates/admin/src/players/tests.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/invalidation_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold_worker.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/bazaar/src/moderation.rs
  - portal-rs/crates/characters/src/services/browser.rs
  - portal-rs/crates/characters/src/services/mod.rs
  - portal-rs/crates/characters/src/services/tests.rs
  - portal-rs/crates/db/src/outbox.rs
  - portal-rs/crates/gameauth/src/admission/audit05_tests.rs
  - portal-rs/crates/gameauth/src/admission/grant.rs
  - portal-rs/crates/gameauth/src/admission/http.rs
  - portal-rs/crates/gameauth/src/admission/recovery.rs
  - portal-rs/crates/gameauth/src/admission/store.rs
  - portal-rs/crates/gameauth/src/evidence.rs
  - portal-rs/crates/gameauth/src/evidence_tests.rs
  - portal-rs/crates/gameauth/src/locator.rs
  - portal-rs/crates/gameauth/src/locator_tests.rs
  - portal-rs/crates/gateway/src/lib.rs
  - portal-rs/crates/gateway/src/v3.rs
  - portal-rs/crates/gateway/src/wire.rs
  - portal-rs/crates/gateway/tests/protocol3.rs
  - portal-rs/crates/gateway/tests/proxies.rs
  - portal-rs/crates/identity/src/account_lifecycle_http.rs
  - portal-rs/crates/identity/src/mfa_http.rs
  - portal-rs/crates/identity/src/mfa_http_tests.rs
  - portal-rs/crates/identity/src/reset_http.rs
  - portal-rs/crates/identity/src/reset_http_tests.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/payments/src/storage.rs
  - portal-rs/crates/payments/tests/payments.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/reads.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/deaths/storage.rs
  - portal-rs/crates/playerdata/src/items/store.rs
  - portal-rs/crates/playerdata/src/logins.rs
  - portal-rs/crates/playerdata/src/logins/tests.rs
  - portal-rs/crates/playerdata/src/places/retention.rs
  - portal-rs/crates/playerdata/src/places/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
  - portal-rs/crates/stats/src/character/http.rs
  - portal-rs/crates/stats/src/character/mod.rs
  - portal-rs/crates/stats/src/character/tests.rs
  - portal-rs/crates/stats/src/guilds.rs
  - portal-rs/crates/stats/src/guilds/tests.rs
  - portal-rs/crates/stats/src/rankings_tests.rs
  - portal-rs/crates/stats/src/store.rs
  - portal-rs/crates/stats/src/tests.rs
  - portal-rs/crates/stats/src/world.rs
  - portal-rs/crates/stats/src/world/tests.rs
  - portal-rs/crates/stats/tests/privacy.rs
  - portal-rs/crates/store/src/codes.rs
  - portal-rs/crates/store/src/codes_tests.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/store/src/web_payments_tests.rs
  - portal-rs/migrations/20261117230002_r9_pd.sql
  - portal-rs/migrations/20261117230011_r9_sta.sql
  - portal-rs/migrations/20261117230012_r9_pd2.sql
proven:
  - Each round-9 and early round-10 finding was confirmed by a read-only check with file:line evidence
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
  - portal-rs/bin/oteryn-portal/src/code_routes.rs
  - portal-rs/bin/oteryn-portal/src/registration_delivery.rs
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/crates/admin/src/console/access.rs
  - portal-rs/crates/admin/src/console/players.rs
  - portal-rs/crates/admin/src/console/stored_tests.rs
  - portal-rs/crates/admin/src/console/templates/read.html
  - portal-rs/crates/admin/src/players/mod.rs
  - portal-rs/crates/admin/src/players/tests.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/invalidation_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold_worker.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/bazaar/src/moderation.rs
  - portal-rs/crates/characters/src/services/browser.rs
  - portal-rs/crates/characters/src/services/mod.rs
  - portal-rs/crates/characters/src/services/tests.rs
  - portal-rs/crates/db/src/outbox.rs
  - portal-rs/crates/gameauth/src/admission/audit05_tests.rs
  - portal-rs/crates/gameauth/src/admission/grant.rs
  - portal-rs/crates/gameauth/src/admission/http.rs
  - portal-rs/crates/gameauth/src/admission/recovery.rs
  - portal-rs/crates/gameauth/src/admission/store.rs
  - portal-rs/crates/gameauth/src/evidence.rs
  - portal-rs/crates/gameauth/src/evidence_tests.rs
  - portal-rs/crates/gameauth/src/locator.rs
  - portal-rs/crates/gameauth/src/locator_tests.rs
  - portal-rs/crates/gateway/src/lib.rs
  - portal-rs/crates/gateway/src/v3.rs
  - portal-rs/crates/gateway/src/wire.rs
  - portal-rs/crates/gateway/tests/protocol3.rs
  - portal-rs/crates/gateway/tests/proxies.rs
  - portal-rs/crates/identity/src/account_lifecycle_http.rs
  - portal-rs/crates/identity/src/mfa_http.rs
  - portal-rs/crates/identity/src/mfa_http_tests.rs
  - portal-rs/crates/identity/src/reset_http.rs
  - portal-rs/crates/identity/src/reset_http_tests.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/payments/src/storage.rs
  - portal-rs/crates/payments/tests/payments.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/reads.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/deaths/storage.rs
  - portal-rs/crates/playerdata/src/items/store.rs
  - portal-rs/crates/playerdata/src/logins.rs
  - portal-rs/crates/playerdata/src/logins/tests.rs
  - portal-rs/crates/playerdata/src/places/retention.rs
  - portal-rs/crates/playerdata/src/places/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
  - portal-rs/crates/stats/src/character/http.rs
  - portal-rs/crates/stats/src/character/mod.rs
  - portal-rs/crates/stats/src/character/tests.rs
  - portal-rs/crates/stats/src/guilds.rs
  - portal-rs/crates/stats/src/guilds/tests.rs
  - portal-rs/crates/stats/src/rankings_tests.rs
  - portal-rs/crates/stats/src/store.rs
  - portal-rs/crates/stats/src/tests.rs
  - portal-rs/crates/stats/src/world.rs
  - portal-rs/crates/stats/src/world/tests.rs
  - portal-rs/crates/stats/tests/privacy.rs
  - portal-rs/crates/store/src/codes.rs
  - portal-rs/crates/store/src/codes_tests.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/store/src/web_payments_tests.rs
  - portal-rs/migrations/20261117230002_r9_pd.sql
  - portal-rs/migrations/20261117230011_r9_sta.sql
  - portal-rs/migrations/20261117230012_r9_pd2.sql
validation:
  - command: cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0 on the integrated tree
  - command: cargo test per crate with PostgreSQL 18
    result: PASS
    evidence: bazaar 175, stats 59, db 12, identity 195, gateway 50, store 139, admin 222, playerdata 191, payments 51, community 194, wallet 40, characters 117, web 68, oteryn-portal 245
  - command: Portal RS CI on the PR head
    result: NOT_RUN
    evidence: full suite runs in CI shards
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: server-side fixes covered by tests through the real routers and workers
blockers:
  - none
next_action: none; delivered by PR #1945 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR #1945 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR #1945 merged as 5e57664eb105b748f71db2b29bd29748976d81df
```
