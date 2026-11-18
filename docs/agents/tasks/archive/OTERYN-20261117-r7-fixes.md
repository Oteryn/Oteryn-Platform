---
task_id: OTERYN-20261117-r7-fixes
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-r7-fixes

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Review round 7: seven small-slice Codex reviews of portal-rs, each finding
verified by a read-only check. The confirmed P0-P2 findings were fixed by 26
Codex Cloud packets, each with a red-then-green test. Every pushed fix was
reviewed again before integration here, and follow-ups were sent where the
review found gaps.

## Acceptance criteria

- [x] Each confirmed finding has a fix with a regression test through the real caller
- [x] Each pushed fix was reviewed read-only before integration
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - deploy/oracle/prepare-platform.sh
  - docs/agents/tasks/active/OTERYN-20261117-r7-fixes.md
  - portal-rs/bin/oteryn-portal/src/account_closure.rs
  - portal-rs/bin/oteryn-portal/src/catalog_cli.rs
  - portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs
  - portal-rs/bin/oteryn-portal/src/game_routes.rs
  - portal-rs/bin/oteryn-portal/src/internal_tls_tests.rs
  - portal-rs/bin/oteryn-portal/src/wallet_topup.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/bin/oteryn-portal/tests/oracle_prepare.rs
  - portal-rs/bin/oteryn-portal/tests/public_listener_guard.rs
  - portal-rs/crates/admin/src/analytics/store.rs
  - portal-rs/crates/admin/src/analytics/tests.rs
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/en.ftl
  - portal-rs/crates/admin/src/commerce/http.rs
  - portal-rs/crates/admin/src/commerce/page.html
  - portal-rs/crates/admin/src/commerce/pl.ftl
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/crates/admin/src/console/players.rs
  - portal-rs/crates/admin/src/console/players_tests.rs
  - portal-rs/crates/admin/src/console/stored.rs
  - portal-rs/crates/admin/src/console/stored_tests.rs
  - portal-rs/crates/admin/src/control/README.md
  - portal-rs/crates/admin/src/control/controls.rs
  - portal-rs/crates/admin/src/control/controls_tests.rs
  - portal-rs/crates/admin/src/control/http.rs
  - portal-rs/crates/admin/src/control/http_tests.rs
  - portal-rs/crates/admin/src/control/store.rs
  - portal-rs/crates/atlas/src/spawns.md
  - portal-rs/crates/atlas/src/spawns.rs
  - portal-rs/crates/atlas/src/spawns_browser.cjs
  - portal-rs/crates/atlas/src/spawns_en.ftl
  - portal-rs/crates/atlas/src/spawns_pl.ftl
  - portal-rs/crates/atlas/src/spawns_tests.rs
  - portal-rs/crates/bazaar/src/characters/tests.rs
  - portal-rs/crates/bazaar/src/game/authority_evidence.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/game_snapshot.rs
  - portal-rs/crates/bazaar/src/game/invalidation_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/browser.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/en.ftl
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold_worker.rs
  - portal-rs/crates/bazaar/src/game/runtime/invalidation.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/bazaar/src/game/runtime/pl.ftl
  - portal-rs/crates/bazaar/src/game/runtime/transport.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/bazaar/src/items/tests.rs
  - portal-rs/crates/bazaar/src/moderation_README.md
  - portal-rs/crates/bazaar/src/moderation_tests.rs
  - portal-rs/crates/bazaar/src/repository.rs
  - portal-rs/crates/bazaar/tests/postgres.rs
  - portal-rs/crates/characters/src/bootstrap/mod.rs
  - portal-rs/crates/characters/src/bootstrap/tests.rs
  - portal-rs/crates/characters/src/overview/tests.rs
  - portal-rs/crates/characters/src/profile/http.rs
  - portal-rs/crates/characters/src/profile/mod.rs
  - portal-rs/crates/characters/src/profile/tests.rs
  - portal-rs/crates/characters/src/restore/tests.rs
  - portal-rs/crates/characters/src/services/projection.rs
  - portal-rs/crates/characters/src/services/tests.rs
  - portal-rs/crates/community/src/guilds_tests.rs
  - portal-rs/crates/community/src/ingest.rs
  - portal-rs/crates/community/src/ingest/ingest_tests.rs
  - portal-rs/crates/community/src/party.rs
  - portal-rs/crates/community/src/party/last_wait_tests.rs
  - portal-rs/crates/community/src/party/party_en.ftl
  - portal-rs/crates/community/src/party/party_pl.ftl
  - portal-rs/crates/community/src/party/party_store.rs
  - portal-rs/crates/community/src/party/party_tests.rs
  - portal-rs/crates/community/src/party/party_view.rs
  - portal-rs/crates/community/src/privacy.rs
  - portal-rs/crates/community/tests/privacy_authority.rs
  - portal-rs/crates/downloads/src/domain.rs
  - portal-rs/crates/downloads/src/verification.rs
  - portal-rs/crates/downloads/tests/common/mod.rs
  - portal-rs/crates/downloads/tests/publication.rs
  - portal-rs/crates/gameauth/src/evidence.rs
  - portal-rs/crates/gameauth/src/evidence_tests.rs
  - portal-rs/crates/gameauth/src/oauth/oauth_http.rs
  - portal-rs/crates/gameauth/src/oauth/oauth_tests.rs
  - portal-rs/crates/gateway/src/golden.rs
  - portal-rs/crates/gateway/src/lib.rs
  - portal-rs/crates/gateway/tests/recover.rs
  - portal-rs/crates/hirelings/tests/flow.rs
  - portal-rs/crates/identity/src/accounts.rs
  - portal-rs/crates/identity/src/credentials.rs
  - portal-rs/crates/identity/src/credentials_tests.rs
  - portal-rs/crates/identity/src/email_change.rs
  - portal-rs/crates/identity/src/email_change_tests.rs
  - portal-rs/crates/identity/src/login_http.rs
  - portal-rs/crates/identity/src/login_http_tests.rs
  - portal-rs/crates/identity/src/mfa.rs
  - portal-rs/crates/identity/src/mfa_http.rs
  - portal-rs/crates/identity/src/mfa_http_tests.rs
  - portal-rs/crates/identity/src/passkeys_http.rs
  - portal-rs/crates/identity/src/passkeys_http_tests.rs
  - portal-rs/crates/identity/src/reset.rs
  - portal-rs/crates/identity/src/reset_http.rs
  - portal-rs/crates/identity/src/reset_http_tests.rs
  - portal-rs/crates/identity/src/reset_tests.rs
  - portal-rs/crates/identity/src/termination.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/runtime.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/identity/src/termination_tests.rs
  - portal-rs/crates/internal-api/src/canonical.rs
  - portal-rs/crates/minigames/src/league/mod.rs
  - portal-rs/crates/minigames/src/league/runtime.rs
  - portal-rs/crates/minigames/src/league/store.rs
  - portal-rs/crates/minigames/src/league/tests.rs
  - portal-rs/crates/minigames/src/puzzles/http.rs
  - portal-rs/crates/minigames/src/puzzles/tests.rs
  - portal-rs/crates/payments/src/account_tests.rs
  - portal-rs/crates/player-api/src/lib.rs
  - portal-rs/crates/player-api/src/tests.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/hunts_tests.rs
  - portal-rs/crates/playerdata/src/items/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/sessions.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
  - portal-rs/crates/store/src/catalog.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/support/src/reports/tests.rs
  - portal-rs/crates/support/src/sanctions/export_tests.rs
  - portal-rs/crates/support/src/sanctions/tests.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/crates/wallet/tests/debt.rs
  - portal-rs/crates/web/src/lib.rs
  - portal-rs/crates/web/src/routes/register.rs
  - portal-rs/crates/web/src/routes/register_authenticated_tests.rs
  - portal-rs/crates/web/src/routes/verify.rs
  - portal-rs/crates/web/src/routes/verify_budget_tests.rs
  - portal-rs/crates/web/src/session.rs
  - portal-rs/crates/web/src/session_tests.rs
  - portal-rs/crates/web/src/surface.rs
  - portal-rs/crates/wiki/src/entities/README.md
  - portal-rs/crates/wiki/src/i18n/en.ftl
  - portal-rs/crates/wiki/src/i18n/pl.ftl
  - portal-rs/crates/wiki/src/repository.rs
  - portal-rs/crates/wiki/src/web.rs
  - portal-rs/crates/wiki/src/web/tests.rs
  - portal-rs/crates/world/src/begin_guard_tests.rs
  - portal-rs/crates/world/src/catalog/mod.rs
  - portal-rs/crates/world/src/catalog/publication.rs
  - portal-rs/crates/world/src/catalog/refresh.rs
  - portal-rs/crates/world/src/catalog/tests.rs
  - portal-rs/crates/world/tests/catalog_refresh.rs
  - portal-rs/e2e/local.mjs
  - portal-rs/migrations/20261117200005_r7_chr.sql
  - portal-rs/migrations/20261117200006_r7_com_a.sql
  - portal-rs/migrations/20261117200009_r7_id_term.sql
  - portal-rs/static/atlas/viewer.js
  - portal-rs/templates/admin/wiki/edit.html
  - portal-rs/templates/oauth/en.ftl
  - portal-rs/templates/oauth/pl.ftl
  - portal-rs/templates/profile/settings.html
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
updated_at: 2026-11-17T21:07:36Z
head: 2fdacc801c9bbe42f6ecaa9d39504bc67f7b2465
branch: claude/happy-davinci-ijz3wy-r7-fixes
pr: 1943
status: completed
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - deploy/oracle/prepare-platform.sh
  - docs/agents/tasks/active/OTERYN-20261117-r7-fixes.md
  - portal-rs/bin/oteryn-portal/src/account_closure.rs
  - portal-rs/bin/oteryn-portal/src/catalog_cli.rs
  - portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs
  - portal-rs/bin/oteryn-portal/src/game_routes.rs
  - portal-rs/bin/oteryn-portal/src/internal_tls_tests.rs
  - portal-rs/bin/oteryn-portal/src/wallet_topup.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/bin/oteryn-portal/tests/oracle_prepare.rs
  - portal-rs/bin/oteryn-portal/tests/public_listener_guard.rs
  - portal-rs/crates/admin/src/analytics/store.rs
  - portal-rs/crates/admin/src/analytics/tests.rs
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/en.ftl
  - portal-rs/crates/admin/src/commerce/http.rs
  - portal-rs/crates/admin/src/commerce/page.html
  - portal-rs/crates/admin/src/commerce/pl.ftl
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/crates/admin/src/console/players.rs
  - portal-rs/crates/admin/src/console/players_tests.rs
  - portal-rs/crates/admin/src/console/stored.rs
  - portal-rs/crates/admin/src/console/stored_tests.rs
  - portal-rs/crates/admin/src/control/README.md
  - portal-rs/crates/admin/src/control/controls.rs
  - portal-rs/crates/admin/src/control/controls_tests.rs
  - portal-rs/crates/admin/src/control/http.rs
  - portal-rs/crates/admin/src/control/http_tests.rs
  - portal-rs/crates/admin/src/control/store.rs
  - portal-rs/crates/atlas/src/spawns.md
  - portal-rs/crates/atlas/src/spawns.rs
  - portal-rs/crates/atlas/src/spawns_browser.cjs
  - portal-rs/crates/atlas/src/spawns_en.ftl
  - portal-rs/crates/atlas/src/spawns_pl.ftl
  - portal-rs/crates/atlas/src/spawns_tests.rs
  - portal-rs/crates/bazaar/src/characters/tests.rs
  - portal-rs/crates/bazaar/src/game/authority_evidence.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/game_snapshot.rs
  - portal-rs/crates/bazaar/src/game/invalidation_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/browser.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/en.ftl
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold_worker.rs
  - portal-rs/crates/bazaar/src/game/runtime/invalidation.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/bazaar/src/game/runtime/pl.ftl
  - portal-rs/crates/bazaar/src/game/runtime/transport.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/bazaar/src/items/tests.rs
  - portal-rs/crates/bazaar/src/moderation_README.md
  - portal-rs/crates/bazaar/src/moderation_tests.rs
  - portal-rs/crates/bazaar/src/repository.rs
  - portal-rs/crates/bazaar/tests/postgres.rs
  - portal-rs/crates/characters/src/bootstrap/mod.rs
  - portal-rs/crates/characters/src/bootstrap/tests.rs
  - portal-rs/crates/characters/src/overview/tests.rs
  - portal-rs/crates/characters/src/profile/http.rs
  - portal-rs/crates/characters/src/profile/mod.rs
  - portal-rs/crates/characters/src/profile/tests.rs
  - portal-rs/crates/characters/src/restore/tests.rs
  - portal-rs/crates/characters/src/services/projection.rs
  - portal-rs/crates/characters/src/services/tests.rs
  - portal-rs/crates/community/src/guilds_tests.rs
  - portal-rs/crates/community/src/ingest.rs
  - portal-rs/crates/community/src/ingest/ingest_tests.rs
  - portal-rs/crates/community/src/party.rs
  - portal-rs/crates/community/src/party/last_wait_tests.rs
  - portal-rs/crates/community/src/party/party_en.ftl
  - portal-rs/crates/community/src/party/party_pl.ftl
  - portal-rs/crates/community/src/party/party_store.rs
  - portal-rs/crates/community/src/party/party_tests.rs
  - portal-rs/crates/community/src/party/party_view.rs
  - portal-rs/crates/community/src/privacy.rs
  - portal-rs/crates/community/tests/privacy_authority.rs
  - portal-rs/crates/downloads/src/domain.rs
  - portal-rs/crates/downloads/src/verification.rs
  - portal-rs/crates/downloads/tests/common/mod.rs
  - portal-rs/crates/downloads/tests/publication.rs
  - portal-rs/crates/gameauth/src/evidence.rs
  - portal-rs/crates/gameauth/src/evidence_tests.rs
  - portal-rs/crates/gameauth/src/oauth/oauth_http.rs
  - portal-rs/crates/gameauth/src/oauth/oauth_tests.rs
  - portal-rs/crates/gateway/src/golden.rs
  - portal-rs/crates/gateway/src/lib.rs
  - portal-rs/crates/gateway/tests/recover.rs
  - portal-rs/crates/hirelings/tests/flow.rs
  - portal-rs/crates/identity/src/accounts.rs
  - portal-rs/crates/identity/src/credentials.rs
  - portal-rs/crates/identity/src/credentials_tests.rs
  - portal-rs/crates/identity/src/email_change.rs
  - portal-rs/crates/identity/src/email_change_tests.rs
  - portal-rs/crates/identity/src/login_http.rs
  - portal-rs/crates/identity/src/login_http_tests.rs
  - portal-rs/crates/identity/src/mfa.rs
  - portal-rs/crates/identity/src/mfa_http.rs
  - portal-rs/crates/identity/src/mfa_http_tests.rs
  - portal-rs/crates/identity/src/passkeys_http.rs
  - portal-rs/crates/identity/src/passkeys_http_tests.rs
  - portal-rs/crates/identity/src/reset.rs
  - portal-rs/crates/identity/src/reset_http.rs
  - portal-rs/crates/identity/src/reset_http_tests.rs
  - portal-rs/crates/identity/src/reset_tests.rs
  - portal-rs/crates/identity/src/termination.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/runtime.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/identity/src/termination_tests.rs
  - portal-rs/crates/internal-api/src/canonical.rs
  - portal-rs/crates/minigames/src/league/mod.rs
  - portal-rs/crates/minigames/src/league/runtime.rs
  - portal-rs/crates/minigames/src/league/store.rs
  - portal-rs/crates/minigames/src/league/tests.rs
  - portal-rs/crates/minigames/src/puzzles/http.rs
  - portal-rs/crates/minigames/src/puzzles/tests.rs
  - portal-rs/crates/payments/src/account_tests.rs
  - portal-rs/crates/player-api/src/lib.rs
  - portal-rs/crates/player-api/src/tests.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/hunts_tests.rs
  - portal-rs/crates/playerdata/src/items/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/sessions.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
  - portal-rs/crates/store/src/catalog.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/support/src/reports/tests.rs
  - portal-rs/crates/support/src/sanctions/export_tests.rs
  - portal-rs/crates/support/src/sanctions/tests.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/crates/wallet/tests/debt.rs
  - portal-rs/crates/web/src/lib.rs
  - portal-rs/crates/web/src/routes/register.rs
  - portal-rs/crates/web/src/routes/register_authenticated_tests.rs
  - portal-rs/crates/web/src/routes/verify.rs
  - portal-rs/crates/web/src/routes/verify_budget_tests.rs
  - portal-rs/crates/web/src/session.rs
  - portal-rs/crates/web/src/session_tests.rs
  - portal-rs/crates/web/src/surface.rs
  - portal-rs/crates/wiki/src/entities/README.md
  - portal-rs/crates/wiki/src/i18n/en.ftl
  - portal-rs/crates/wiki/src/i18n/pl.ftl
  - portal-rs/crates/wiki/src/repository.rs
  - portal-rs/crates/wiki/src/web.rs
  - portal-rs/crates/wiki/src/web/tests.rs
  - portal-rs/crates/world/src/begin_guard_tests.rs
  - portal-rs/crates/world/src/catalog/mod.rs
  - portal-rs/crates/world/src/catalog/publication.rs
  - portal-rs/crates/world/src/catalog/refresh.rs
  - portal-rs/crates/world/src/catalog/tests.rs
  - portal-rs/crates/world/tests/catalog_refresh.rs
  - portal-rs/e2e/local.mjs
  - portal-rs/migrations/20261117200005_r7_chr.sql
  - portal-rs/migrations/20261117200006_r7_com_a.sql
  - portal-rs/migrations/20261117200009_r7_id_term.sql
  - portal-rs/static/atlas/viewer.js
  - portal-rs/templates/admin/wiki/edit.html
  - portal-rs/templates/oauth/en.ftl
  - portal-rs/templates/oauth/pl.ftl
  - portal-rs/templates/profile/settings.html
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
  - deploy/oracle/prepare-platform.sh
  - docs/agents/tasks/active/OTERYN-20261117-r7-fixes.md
  - portal-rs/bin/oteryn-portal/src/account_closure.rs
  - portal-rs/bin/oteryn-portal/src/catalog_cli.rs
  - portal-rs/bin/oteryn-portal/src/catalog_cli_tests.rs
  - portal-rs/bin/oteryn-portal/src/game_routes.rs
  - portal-rs/bin/oteryn-portal/src/internal_tls_tests.rs
  - portal-rs/bin/oteryn-portal/src/wallet_topup.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/bin/oteryn-portal/tests/oracle_prepare.rs
  - portal-rs/bin/oteryn-portal/tests/public_listener_guard.rs
  - portal-rs/crates/admin/src/analytics/store.rs
  - portal-rs/crates/admin/src/analytics/tests.rs
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/en.ftl
  - portal-rs/crates/admin/src/commerce/http.rs
  - portal-rs/crates/admin/src/commerce/page.html
  - portal-rs/crates/admin/src/commerce/pl.ftl
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/crates/admin/src/console/players.rs
  - portal-rs/crates/admin/src/console/players_tests.rs
  - portal-rs/crates/admin/src/console/stored.rs
  - portal-rs/crates/admin/src/console/stored_tests.rs
  - portal-rs/crates/admin/src/control/README.md
  - portal-rs/crates/admin/src/control/controls.rs
  - portal-rs/crates/admin/src/control/controls_tests.rs
  - portal-rs/crates/admin/src/control/http.rs
  - portal-rs/crates/admin/src/control/http_tests.rs
  - portal-rs/crates/admin/src/control/store.rs
  - portal-rs/crates/atlas/src/spawns.md
  - portal-rs/crates/atlas/src/spawns.rs
  - portal-rs/crates/atlas/src/spawns_browser.cjs
  - portal-rs/crates/atlas/src/spawns_en.ftl
  - portal-rs/crates/atlas/src/spawns_pl.ftl
  - portal-rs/crates/atlas/src/spawns_tests.rs
  - portal-rs/crates/bazaar/src/characters/tests.rs
  - portal-rs/crates/bazaar/src/game/authority_evidence.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/game_snapshot.rs
  - portal-rs/crates/bazaar/src/game/invalidation_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/browser.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/en.ftl
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold_worker.rs
  - portal-rs/crates/bazaar/src/game/runtime/invalidation.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/bazaar/src/game/runtime/pl.ftl
  - portal-rs/crates/bazaar/src/game/runtime/transport.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/bazaar/src/items/tests.rs
  - portal-rs/crates/bazaar/src/moderation_README.md
  - portal-rs/crates/bazaar/src/moderation_tests.rs
  - portal-rs/crates/bazaar/src/repository.rs
  - portal-rs/crates/bazaar/tests/postgres.rs
  - portal-rs/crates/characters/src/bootstrap/mod.rs
  - portal-rs/crates/characters/src/bootstrap/tests.rs
  - portal-rs/crates/characters/src/overview/tests.rs
  - portal-rs/crates/characters/src/profile/http.rs
  - portal-rs/crates/characters/src/profile/mod.rs
  - portal-rs/crates/characters/src/profile/tests.rs
  - portal-rs/crates/characters/src/restore/tests.rs
  - portal-rs/crates/characters/src/services/projection.rs
  - portal-rs/crates/characters/src/services/tests.rs
  - portal-rs/crates/community/src/guilds_tests.rs
  - portal-rs/crates/community/src/ingest.rs
  - portal-rs/crates/community/src/ingest/ingest_tests.rs
  - portal-rs/crates/community/src/party.rs
  - portal-rs/crates/community/src/party/last_wait_tests.rs
  - portal-rs/crates/community/src/party/party_en.ftl
  - portal-rs/crates/community/src/party/party_pl.ftl
  - portal-rs/crates/community/src/party/party_store.rs
  - portal-rs/crates/community/src/party/party_tests.rs
  - portal-rs/crates/community/src/party/party_view.rs
  - portal-rs/crates/community/src/privacy.rs
  - portal-rs/crates/community/tests/privacy_authority.rs
  - portal-rs/crates/downloads/src/domain.rs
  - portal-rs/crates/downloads/src/verification.rs
  - portal-rs/crates/downloads/tests/common/mod.rs
  - portal-rs/crates/downloads/tests/publication.rs
  - portal-rs/crates/gameauth/src/evidence.rs
  - portal-rs/crates/gameauth/src/evidence_tests.rs
  - portal-rs/crates/gameauth/src/oauth/oauth_http.rs
  - portal-rs/crates/gameauth/src/oauth/oauth_tests.rs
  - portal-rs/crates/gateway/src/golden.rs
  - portal-rs/crates/gateway/src/lib.rs
  - portal-rs/crates/gateway/tests/recover.rs
  - portal-rs/crates/hirelings/tests/flow.rs
  - portal-rs/crates/identity/src/accounts.rs
  - portal-rs/crates/identity/src/credentials.rs
  - portal-rs/crates/identity/src/credentials_tests.rs
  - portal-rs/crates/identity/src/email_change.rs
  - portal-rs/crates/identity/src/email_change_tests.rs
  - portal-rs/crates/identity/src/login_http.rs
  - portal-rs/crates/identity/src/login_http_tests.rs
  - portal-rs/crates/identity/src/mfa.rs
  - portal-rs/crates/identity/src/mfa_http.rs
  - portal-rs/crates/identity/src/mfa_http_tests.rs
  - portal-rs/crates/identity/src/passkeys_http.rs
  - portal-rs/crates/identity/src/passkeys_http_tests.rs
  - portal-rs/crates/identity/src/reset.rs
  - portal-rs/crates/identity/src/reset_http.rs
  - portal-rs/crates/identity/src/reset_http_tests.rs
  - portal-rs/crates/identity/src/reset_tests.rs
  - portal-rs/crates/identity/src/termination.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/runtime.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/identity/src/termination_tests.rs
  - portal-rs/crates/internal-api/src/canonical.rs
  - portal-rs/crates/minigames/src/league/mod.rs
  - portal-rs/crates/minigames/src/league/runtime.rs
  - portal-rs/crates/minigames/src/league/store.rs
  - portal-rs/crates/minigames/src/league/tests.rs
  - portal-rs/crates/minigames/src/puzzles/http.rs
  - portal-rs/crates/minigames/src/puzzles/tests.rs
  - portal-rs/crates/payments/src/account_tests.rs
  - portal-rs/crates/player-api/src/lib.rs
  - portal-rs/crates/player-api/src/tests.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/hunts_tests.rs
  - portal-rs/crates/playerdata/src/items/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/sessions.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
  - portal-rs/crates/store/src/catalog.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/support/src/reports/tests.rs
  - portal-rs/crates/support/src/sanctions/export_tests.rs
  - portal-rs/crates/support/src/sanctions/tests.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/crates/wallet/tests/debt.rs
  - portal-rs/crates/web/src/lib.rs
  - portal-rs/crates/web/src/routes/register.rs
  - portal-rs/crates/web/src/routes/register_authenticated_tests.rs
  - portal-rs/crates/web/src/routes/verify.rs
  - portal-rs/crates/web/src/routes/verify_budget_tests.rs
  - portal-rs/crates/web/src/session.rs
  - portal-rs/crates/web/src/session_tests.rs
  - portal-rs/crates/web/src/surface.rs
  - portal-rs/crates/wiki/src/entities/README.md
  - portal-rs/crates/wiki/src/i18n/en.ftl
  - portal-rs/crates/wiki/src/i18n/pl.ftl
  - portal-rs/crates/wiki/src/repository.rs
  - portal-rs/crates/wiki/src/web.rs
  - portal-rs/crates/wiki/src/web/tests.rs
  - portal-rs/crates/world/src/begin_guard_tests.rs
  - portal-rs/crates/world/src/catalog/mod.rs
  - portal-rs/crates/world/src/catalog/publication.rs
  - portal-rs/crates/world/src/catalog/refresh.rs
  - portal-rs/crates/world/src/catalog/tests.rs
  - portal-rs/crates/world/tests/catalog_refresh.rs
  - portal-rs/e2e/local.mjs
  - portal-rs/migrations/20261117200005_r7_chr.sql
  - portal-rs/migrations/20261117200006_r7_com_a.sql
  - portal-rs/migrations/20261117200009_r7_id_term.sql
  - portal-rs/static/atlas/viewer.js
  - portal-rs/templates/admin/wiki/edit.html
  - portal-rs/templates/oauth/en.ftl
  - portal-rs/templates/oauth/pl.ftl
  - portal-rs/templates/profile/settings.html
validation:
  - command: cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0 on the integrated tree
  - command: cargo test per crate with PostgreSQL 18 (gameauth, gateway, player-api, internal-api, minigames, store, admin, downloads)
    result: PASS
    evidence: gameauth 172, gateway 48, player-api 14, internal-api 14, minigames 76, store 133, admin 214, downloads 19
  - command: Portal RS CI on the PR head
    result: NOT_RUN
    evidence: full suite runs in CI shards
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: server-side fixes covered by tests through the real routers and workers
blockers:
  - none
next_action: none; delivered by PR #1943 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR #1943 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR #1943 merged as 2fdacc801c9bbe42f6ecaa9d39504bc67f7b2465
```
