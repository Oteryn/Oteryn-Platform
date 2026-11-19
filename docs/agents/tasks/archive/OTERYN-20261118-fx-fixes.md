---
task_id: OTERYN-20261118-fx-fixes
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261118-fx-fixes

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Flow and class review of portal-rs on the round 15 head (21 flow/class
reviews and 11 slice reviews, read-only). Every finding confirmed P1/P2 by a
second read-only check is fixed with a red-then-green regression test through
the real caller: Bazaar recovery and projection-epoch handling (second epoch
site, tombstone regression, page errors), player-data and character custody,
store/wallet payer holds and closure obligations, referral and league workers,
security revocations for future suspensions, login-history and closure audit
retention, Game account-closure wire aligned byte for byte with the Game
producer, and copied death details purged on schedule even while statistics
is disabled. Integrated with two further reviewed fix lanes: acct2 (referral
reward payer and retry ordering, league feed generations; migrations
20261118140000, 20261118140100, 20261118140200) and fb (feature-flag
boundaries always on, referral activity fenced at deployment transitions
with migration 20261118130101, Game closure proof publication and
authorized cancellation, payer guard and money/ownership property tests,
every route classified on its production listener). Integration fixes
align the store loyalty and refund-closure test fixtures with the fenced
referral deployment and real Game closure boundary. Also archives the
round 15 packet after PR #1948 merged.

## Acceptance criteria

- [x] Each confirmed finding has a fix with a regression test through the real caller
- [x] Each pushed fix was reviewed read-only before integration
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261118-fx-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r15-fixes.md
  - portal-rs/bin/oteryn-portal/src/account_closure.rs
  - portal-rs/bin/oteryn-portal/src/code_routes.rs
  - portal-rs/bin/oteryn-portal/src/code_routes_tests.rs
  - portal-rs/bin/oteryn-portal/src/coin_ceiling_tests.rs
  - portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs
  - portal-rs/bin/oteryn-portal/src/flag_boundary_tests.rs
  - portal-rs/bin/oteryn-portal/src/game_login.rs
  - portal-rs/bin/oteryn-portal/src/game_routes.rs
  - portal-rs/bin/oteryn-portal/src/game_routes_store_tests.rs
  - portal-rs/bin/oteryn-portal/src/game_routes_tests.rs
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/src/seed.rs
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/crates/admin/src/begin_guard_tests.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/crates/admin/src/console/access.rs
  - portal-rs/crates/admin/src/console/mod.rs
  - portal-rs/crates/admin/src/players/http.rs
  - portal-rs/crates/admin/src/rbac/panel_tests.rs
  - portal-rs/crates/admin/src/rbac/storage.rs
  - portal-rs/crates/admin/src/support_web.rs
  - portal-rs/crates/admin/src/web.rs
  - portal-rs/crates/admin/src/web_tests.rs
  - portal-rs/crates/atlas/src/delivery/http.rs
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_evidence.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/authority_wire.rs
  - portal-rs/crates/bazaar/src/game/coin_ports.rs
  - portal-rs/crates/bazaar/src/game/invalidation_tests.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/invalidation.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/bazaar/src/game/runtime/transport.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/characters/src/create_web/http.rs
  - portal-rs/crates/characters/src/restore/mod.rs
  - portal-rs/crates/characters/src/restore/storage.rs
  - portal-rs/crates/characters/src/restore/tests.rs
  - portal-rs/crates/characters/src/services/mod.rs
  - portal-rs/crates/characters/src/services/tests.rs
  - portal-rs/crates/characters/src/services/transport.rs
  - portal-rs/crates/community/src/begin_guard_tests.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/db/src/lib.rs
  - portal-rs/crates/db/src/test_support.rs
  - portal-rs/crates/db/tests/migrations.rs
  - portal-rs/crates/gameauth/src/admission/http.rs
  - portal-rs/crates/gameauth/src/admission/mod.rs
  - portal-rs/crates/identity/src/account_lifecycle_http.rs
  - portal-rs/crates/identity/src/email_change_mail.rs
  - portal-rs/crates/identity/src/mfa_http.rs
  - portal-rs/crates/identity/src/security_revocations_tests.rs
  - portal-rs/crates/identity/src/termination.rs
  - portal-rs/crates/identity/src/termination/README.md
  - portal-rs/crates/identity/src/termination/fixtures/SOURCE.md
  - portal-rs/crates/identity/src/termination/fixtures/pending.json
  - portal-rs/crates/identity/src/termination/fixtures/read.json
  - portal-rs/crates/identity/src/termination/fixtures/watermark.json
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/http.rs
  - portal-rs/crates/identity/src/termination/game_closure/runtime.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/identity/src/termination/game_closure/wire.rs
  - portal-rs/crates/identity/src/termination_tests.rs
  - portal-rs/crates/minigames/src/collection_pvp/tests.rs
  - portal-rs/crates/minigames/src/league/mod.rs
  - portal-rs/crates/minigames/src/league/runtime.rs
  - portal-rs/crates/minigames/src/league/store.rs
  - portal-rs/crates/minigames/src/league/tests.rs
  - portal-rs/crates/minigames/src/league/transport.rs
  - portal-rs/crates/minigames/src/pass/tests.rs
  - portal-rs/crates/payments/Cargo.toml
  - portal-rs/crates/payments/src/lib.rs
  - portal-rs/crates/payments/src/property_tests.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/reads.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/deaths/storage.rs
  - portal-rs/crates/playerdata/src/hunts_web.rs
  - portal-rs/crates/playerdata/src/logins.rs
  - portal-rs/crates/playerdata/src/logins/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/stats/src/begin_guard_tests.rs
  - portal-rs/crates/stats/src/tests.rs
  - portal-rs/crates/stats/src/worker.rs
  - portal-rs/crates/store/src/delivery.rs
  - portal-rs/crates/store/src/delivery_tests.rs
  - portal-rs/crates/store/src/lib.rs
  - portal-rs/crates/store/src/loyalty_stream_tests.rs
  - portal-rs/crates/store/src/property_tests.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/support/src/tickets/mod.rs
  - portal-rs/crates/support/src/tickets/tests.rs
  - portal-rs/crates/wallet/src/debt.rs
  - portal-rs/crates/wallet/src/domain.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/crates/wallet/src/lib.rs
  - portal-rs/crates/wallet/src/payer_guard_tests.rs
  - portal-rs/crates/wallet/src/property_tests.rs
  - portal-rs/crates/wallet/tests/coin_http/mod.rs
  - portal-rs/crates/wallet/tests/debt.rs
  - portal-rs/crates/web/src/lib.rs
  - portal-rs/crates/web/src/session.rs
  - portal-rs/crates/world/src/admin_pool_tests.rs
  - portal-rs/docs/FLAG_BOUNDARY.md
  - portal-rs/docs/FLAG_BOUNDARY_ALWAYS_ON.txt
  - portal-rs/migrations/20261118120000_security_revocations_future_suspension.sql
  - portal-rs/migrations/20261118130101_fb_referral_activity_flag.sql
  - portal-rs/migrations/20261118140000_fx_acct2_referral_reward_retry.sql
  - portal-rs/migrations/20261118140100_fx_acct2_referral_reward_due_index.sql
  - portal-rs/migrations/20261118140200_fx_acct2_league_feed_generation.sql
modules:
  - admin
  - atlas
  - bazaar
  - characters
  - community
  - db
  - gameauth
  - identity
  - minigames
  - oteryn-portal
  - payments
  - playerdata
  - stats
  - store
  - support
  - wallet
  - web
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
updated_at: 2026-11-18T17:10:00Z
head: bc50ac53
branch: claude/happy-davinci-ijz3wy-fx
pr: 1951
status: completed
terminal_pr_policy: archived
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261118-fx-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r15-fixes.md
  - portal-rs/bin/oteryn-portal/src/account_closure.rs
  - portal-rs/bin/oteryn-portal/src/code_routes.rs
  - portal-rs/bin/oteryn-portal/src/code_routes_tests.rs
  - portal-rs/bin/oteryn-portal/src/coin_ceiling_tests.rs
  - portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs
  - portal-rs/bin/oteryn-portal/src/flag_boundary_tests.rs
  - portal-rs/bin/oteryn-portal/src/game_login.rs
  - portal-rs/bin/oteryn-portal/src/game_routes.rs
  - portal-rs/bin/oteryn-portal/src/game_routes_store_tests.rs
  - portal-rs/bin/oteryn-portal/src/game_routes_tests.rs
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/src/seed.rs
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/crates/admin/src/begin_guard_tests.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/crates/admin/src/console/access.rs
  - portal-rs/crates/admin/src/console/mod.rs
  - portal-rs/crates/admin/src/players/http.rs
  - portal-rs/crates/admin/src/rbac/panel_tests.rs
  - portal-rs/crates/admin/src/rbac/storage.rs
  - portal-rs/crates/admin/src/support_web.rs
  - portal-rs/crates/admin/src/web.rs
  - portal-rs/crates/admin/src/web_tests.rs
  - portal-rs/crates/atlas/src/delivery/http.rs
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_evidence.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/authority_wire.rs
  - portal-rs/crates/bazaar/src/game/coin_ports.rs
  - portal-rs/crates/bazaar/src/game/invalidation_tests.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/invalidation.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/bazaar/src/game/runtime/transport.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/characters/src/create_web/http.rs
  - portal-rs/crates/characters/src/restore/mod.rs
  - portal-rs/crates/characters/src/restore/storage.rs
  - portal-rs/crates/characters/src/restore/tests.rs
  - portal-rs/crates/characters/src/services/mod.rs
  - portal-rs/crates/characters/src/services/tests.rs
  - portal-rs/crates/characters/src/services/transport.rs
  - portal-rs/crates/community/src/begin_guard_tests.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/db/src/lib.rs
  - portal-rs/crates/db/src/test_support.rs
  - portal-rs/crates/db/tests/migrations.rs
  - portal-rs/crates/gameauth/src/admission/http.rs
  - portal-rs/crates/gameauth/src/admission/mod.rs
  - portal-rs/crates/identity/src/account_lifecycle_http.rs
  - portal-rs/crates/identity/src/email_change_mail.rs
  - portal-rs/crates/identity/src/mfa_http.rs
  - portal-rs/crates/identity/src/security_revocations_tests.rs
  - portal-rs/crates/identity/src/termination.rs
  - portal-rs/crates/identity/src/termination/README.md
  - portal-rs/crates/identity/src/termination/fixtures/SOURCE.md
  - portal-rs/crates/identity/src/termination/fixtures/pending.json
  - portal-rs/crates/identity/src/termination/fixtures/read.json
  - portal-rs/crates/identity/src/termination/fixtures/watermark.json
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/http.rs
  - portal-rs/crates/identity/src/termination/game_closure/runtime.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/identity/src/termination/game_closure/wire.rs
  - portal-rs/crates/identity/src/termination_tests.rs
  - portal-rs/crates/minigames/src/collection_pvp/tests.rs
  - portal-rs/crates/minigames/src/league/mod.rs
  - portal-rs/crates/minigames/src/league/runtime.rs
  - portal-rs/crates/minigames/src/league/store.rs
  - portal-rs/crates/minigames/src/league/tests.rs
  - portal-rs/crates/minigames/src/league/transport.rs
  - portal-rs/crates/minigames/src/pass/tests.rs
  - portal-rs/crates/payments/Cargo.toml
  - portal-rs/crates/payments/src/lib.rs
  - portal-rs/crates/payments/src/property_tests.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/reads.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/deaths/storage.rs
  - portal-rs/crates/playerdata/src/hunts_web.rs
  - portal-rs/crates/playerdata/src/logins.rs
  - portal-rs/crates/playerdata/src/logins/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/stats/src/begin_guard_tests.rs
  - portal-rs/crates/stats/src/tests.rs
  - portal-rs/crates/stats/src/worker.rs
  - portal-rs/crates/store/src/delivery.rs
  - portal-rs/crates/store/src/delivery_tests.rs
  - portal-rs/crates/store/src/lib.rs
  - portal-rs/crates/store/src/loyalty_stream_tests.rs
  - portal-rs/crates/store/src/property_tests.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/support/src/tickets/mod.rs
  - portal-rs/crates/support/src/tickets/tests.rs
  - portal-rs/crates/wallet/src/debt.rs
  - portal-rs/crates/wallet/src/domain.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/crates/wallet/src/lib.rs
  - portal-rs/crates/wallet/src/payer_guard_tests.rs
  - portal-rs/crates/wallet/src/property_tests.rs
  - portal-rs/crates/wallet/tests/coin_http/mod.rs
  - portal-rs/crates/wallet/tests/debt.rs
  - portal-rs/crates/web/src/lib.rs
  - portal-rs/crates/web/src/session.rs
  - portal-rs/crates/world/src/admin_pool_tests.rs
  - portal-rs/docs/FLAG_BOUNDARY.md
  - portal-rs/docs/FLAG_BOUNDARY_ALWAYS_ON.txt
  - portal-rs/migrations/20261118120000_security_revocations_future_suspension.sql
  - portal-rs/migrations/20261118130101_fb_referral_activity_flag.sql
  - portal-rs/migrations/20261118140000_fx_acct2_referral_reward_retry.sql
  - portal-rs/migrations/20261118140100_fx_acct2_referral_reward_due_index.sql
  - portal-rs/migrations/20261118140200_fx_acct2_league_feed_generation.sql
proven:
  - Each finding was confirmed by a read-only check with file:line evidence
  - Each pushed fix was reviewed read-only against its packet before integration
  - The closure wire matches the Game producer fixtures pinned at Game 553802fa
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
  - docs/agents/tasks/active/OTERYN-20261118-fx-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r15-fixes.md
  - portal-rs/bin/oteryn-portal/src/account_closure.rs
  - portal-rs/bin/oteryn-portal/src/code_routes.rs
  - portal-rs/bin/oteryn-portal/src/code_routes_tests.rs
  - portal-rs/bin/oteryn-portal/src/coin_ceiling_tests.rs
  - portal-rs/bin/oteryn-portal/src/coin_purchases_tests.rs
  - portal-rs/bin/oteryn-portal/src/flag_boundary_tests.rs
  - portal-rs/bin/oteryn-portal/src/game_login.rs
  - portal-rs/bin/oteryn-portal/src/game_routes.rs
  - portal-rs/bin/oteryn-portal/src/game_routes_store_tests.rs
  - portal-rs/bin/oteryn-portal/src/game_routes_tests.rs
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/bin/oteryn-portal/src/seed.rs
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/crates/admin/src/begin_guard_tests.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/crates/admin/src/console/access.rs
  - portal-rs/crates/admin/src/console/mod.rs
  - portal-rs/crates/admin/src/players/http.rs
  - portal-rs/crates/admin/src/rbac/panel_tests.rs
  - portal-rs/crates/admin/src/rbac/storage.rs
  - portal-rs/crates/admin/src/support_web.rs
  - portal-rs/crates/admin/src/web.rs
  - portal-rs/crates/admin/src/web_tests.rs
  - portal-rs/crates/atlas/src/delivery/http.rs
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_evidence.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/authority_wire.rs
  - portal-rs/crates/bazaar/src/game/coin_ports.rs
  - portal-rs/crates/bazaar/src/game/invalidation_tests.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/invalidation.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/bazaar/src/game/runtime/transport.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/characters/src/create_web/http.rs
  - portal-rs/crates/characters/src/restore/mod.rs
  - portal-rs/crates/characters/src/restore/storage.rs
  - portal-rs/crates/characters/src/restore/tests.rs
  - portal-rs/crates/characters/src/services/mod.rs
  - portal-rs/crates/characters/src/services/tests.rs
  - portal-rs/crates/characters/src/services/transport.rs
  - portal-rs/crates/community/src/begin_guard_tests.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/db/src/lib.rs
  - portal-rs/crates/db/src/test_support.rs
  - portal-rs/crates/db/tests/migrations.rs
  - portal-rs/crates/gameauth/src/admission/http.rs
  - portal-rs/crates/gameauth/src/admission/mod.rs
  - portal-rs/crates/identity/src/account_lifecycle_http.rs
  - portal-rs/crates/identity/src/email_change_mail.rs
  - portal-rs/crates/identity/src/mfa_http.rs
  - portal-rs/crates/identity/src/security_revocations_tests.rs
  - portal-rs/crates/identity/src/termination.rs
  - portal-rs/crates/identity/src/termination/README.md
  - portal-rs/crates/identity/src/termination/fixtures/SOURCE.md
  - portal-rs/crates/identity/src/termination/fixtures/pending.json
  - portal-rs/crates/identity/src/termination/fixtures/read.json
  - portal-rs/crates/identity/src/termination/fixtures/watermark.json
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/http.rs
  - portal-rs/crates/identity/src/termination/game_closure/runtime.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/identity/src/termination/game_closure/wire.rs
  - portal-rs/crates/identity/src/termination_tests.rs
  - portal-rs/crates/minigames/src/collection_pvp/tests.rs
  - portal-rs/crates/minigames/src/league/mod.rs
  - portal-rs/crates/minigames/src/league/runtime.rs
  - portal-rs/crates/minigames/src/league/store.rs
  - portal-rs/crates/minigames/src/league/tests.rs
  - portal-rs/crates/minigames/src/league/transport.rs
  - portal-rs/crates/minigames/src/pass/tests.rs
  - portal-rs/crates/payments/Cargo.toml
  - portal-rs/crates/payments/src/lib.rs
  - portal-rs/crates/payments/src/property_tests.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/reads.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/deaths/storage.rs
  - portal-rs/crates/playerdata/src/hunts_web.rs
  - portal-rs/crates/playerdata/src/logins.rs
  - portal-rs/crates/playerdata/src/logins/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/stats/src/begin_guard_tests.rs
  - portal-rs/crates/stats/src/tests.rs
  - portal-rs/crates/stats/src/worker.rs
  - portal-rs/crates/store/src/delivery.rs
  - portal-rs/crates/store/src/delivery_tests.rs
  - portal-rs/crates/store/src/lib.rs
  - portal-rs/crates/store/src/loyalty_stream_tests.rs
  - portal-rs/crates/store/src/property_tests.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/store/src/web_payments.rs
  - portal-rs/crates/support/src/tickets/mod.rs
  - portal-rs/crates/support/src/tickets/tests.rs
  - portal-rs/crates/wallet/src/debt.rs
  - portal-rs/crates/wallet/src/domain.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/crates/wallet/src/lib.rs
  - portal-rs/crates/wallet/src/payer_guard_tests.rs
  - portal-rs/crates/wallet/src/property_tests.rs
  - portal-rs/crates/wallet/tests/coin_http/mod.rs
  - portal-rs/crates/wallet/tests/debt.rs
  - portal-rs/crates/web/src/lib.rs
  - portal-rs/crates/web/src/session.rs
  - portal-rs/crates/world/src/admin_pool_tests.rs
  - portal-rs/docs/FLAG_BOUNDARY.md
  - portal-rs/docs/FLAG_BOUNDARY_ALWAYS_ON.txt
  - portal-rs/migrations/20261118120000_security_revocations_future_suspension.sql
  - portal-rs/migrations/20261118130101_fb_referral_activity_flag.sql
  - portal-rs/migrations/20261118140000_fx_acct2_referral_reward_retry.sql
  - portal-rs/migrations/20261118140100_fx_acct2_referral_reward_due_index.sql
  - portal-rs/migrations/20261118140200_fx_acct2_league_feed_generation.sql
validation:
  - command: cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings
    result: PASS
    evidence: integrated tree
  - command: cargo test per touched crate with PostgreSQL 18
    result: PASS
    evidence: local PostgreSQL 18.6 on the integrated fx+acct2+fb tree (20b67f33 plus this packet) - db 15, identity 208, wallet 43, store 146, payments 60, minigames 80, characters 121, admin 225, community 200, stats 61, bazaar 214, web 69, support 103, gameauth 184, playerdata 205, world 70, atlas 34, oteryn-portal 264; 0 failed
  - command: Portal RS CI on the PR head
    result: PASS
    evidence: PR #1951 merged as bc50ac53 with required checks green
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: server-side fixes covered by tests through the real routers and workers; portal-rs E2E dispatched after merge
blockers:
  - none
next_action: none; delivered by PR #1951 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR; head branch deleted after merge
source_branch_evidence: PR 1951 merged as bc50ac53f9654d43fb9f858441ba86420c3e5a77
```
