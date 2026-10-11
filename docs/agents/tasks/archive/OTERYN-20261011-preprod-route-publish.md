---
task_id: OTERYN-20261011-preprod-route-publish
governing_issue: 1474
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
search_first:
  - app/GameAuth/Worlds/DisposableNativeStore.php
  - app/GameAuth/Worlds/NativeTopologyRegistry.php
  - app/Console/Commands/PublishNativeRoute.php
  - app/Console/Commands/IssueNativeTopology.php
optional_reads: []
---

# OTERYN-20261011 preproduction-store native route publication

## Goal

Owner decision 2026-10-11, option a of ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 §2. The deploy route-publication step that PR #1550 adds (default OFF behind `OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED`) runs `game-auth:native-topology:issue` and `game-auth:native-route:publish`. Both refuse the Synology MariaDB store through `DisposableNativeStore`.

Add a strictly fenced, separate preproduction-store mode inside the two existing commands and keep `DisposableNativeStore` unchanged for every other path. Record the decision as ADR 0044 and in contract §17.

## Acceptance criteria

- [x] The mode is entered only by an explicit switch (`GAME_AUTH_NATIVE_ROUTE_PREPRODUCTION_STORE_ENABLED`, default off). A malformed value refuses. Compose maps it from `OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED`, so the deploy invocation of PR #1550 needs no change.
- [x] It runs only with `APP_ENV=preproduction`. Every other environment, `testing` included, refuses.
- [x] It runs only on the code-allowlisted store: driver `mysql`, host `mariadb`, port 3306, database `oteryn_platform`, no unix socket, no read/write split. Any other store, including the disposable SQLite store, refuses.
- [x] It publishes exactly one route record: the one for the World row whose WorldId equals the configured native World (`OTERYN_STAGING_NATIVE_WORLD_ID`). A foreign World, a foreign route, a missing World row or an unissued Channel refuses.
- [x] It is idempotent. An unchanged route gives `unchanged` with no write, and a retained topology gives `retained`.
- [x] Every successful run writes one `admin_audit_events` row in the same transaction.
- [x] Output holds identifiers and route fields only, with no credential or host.
- [x] Every refusal is a named `NATIVE_PREPROD_STORE_*` reason with nothing written.
- [x] With the switch off, both commands behave exactly as before (default-path test plus the unchanged `NativeOperatorCommandsTest`).
- [x] ADR 0044 (Accepted 2026-10-11, owner option a) and a contract §17 status paragraph record the amendment.
- [x] The legacy token ratchet passes with no group raised.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - app/GameAuth/Worlds/PreproductionNativeRouteStore.php
  - app/GameAuth/Worlds/NativePreproductionStoreRefused.php
  - app/GameAuth/Worlds/NativeTopologyRegistry.php
  - app/Console/Commands/IssueNativeTopology.php
  - app/Console/Commands/PublishNativeRoute.php
  - config/game-auth.php
  - deploy/synology/compose.yml
  - tests/Feature/GameAuth/NativeTopology/PreproductionStoreRouteCommandsTest.php
  - docs/architecture/adr/0044-preproduction-store-native-route-publication.md
  - docs/architecture/adr/README.md
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/archive/OTERYN-20261011-preprod-route-publish.md
modules:
  - game-auth
  - deploy-synology
dependencies:
  - PR #1550 adds the deploy step and the OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED deploy variable. This PR is based on main with files disjoint from #1550 and merges in either order.
blockers:
  - none
cross_repository_tasks:
  - Game ARCH-PREPROD-ROUTE-PUBLISH-AUTH-V1 review record may note the accepted amendment (Game-owned, reported to the lead)
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T22:00:00Z
head: see PR head
branch: claude/preprod-route-publish-20261011
pr: none
status: completed
context_routes:
  - auth-identity
  - security
  - deploy
owned_paths:
  - app/GameAuth/Worlds/PreproductionNativeRouteStore.php
  - app/GameAuth/Worlds/NativePreproductionStoreRefused.php
  - app/GameAuth/Worlds/NativeTopologyRegistry.php
  - app/Console/Commands/IssueNativeTopology.php
  - app/Console/Commands/PublishNativeRoute.php
  - config/game-auth.php
  - deploy/synology/compose.yml
  - tests/Feature/GameAuth/NativeTopology/PreproductionStoreRouteCommandsTest.php
  - docs/architecture/adr/0044-preproduction-store-native-route-publication.md
  - docs/architecture/adr/README.md
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/archive/OTERYN-20261011-preprod-route-publish.md
proven:
  - With every condition met, issue mints then retains the WorldId and publish writes one route record. A repeat is unchanged with no write; an endpoint change advances the version; login off keeps the revision; one audit row per run; no credential or host in output (feature test).
  - Twenty-three fence violations each refuse both commands with their named reason and nothing written. They cover switch, four environments, three world-id forms, nine store profiles including the disposable SQLite store, an open transaction, missing row, mismatch, unissued Channel, foreign World and foreign route.
  - With the switch unset, false or "false", both commands keep the disposable path and refuse the deployed store unchanged.
derived:
  - The test store is a double carrying the deployed connection profile on the in-memory test database. Real MariaDB locking is the same query builder path as the disposable mode.
unknown:
  - none
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Widen DisposableNativeStore. It also guards the trust-key command and test readback; the owner chose a separate fence.
  - A command flag. It would require a change to PR #1550's lib.sh invocation; the environment switch follows the deploy variable instead.
  - Allowlist from configuration. An environment variable could then widen it; the allowlist is code.
changed_paths:
  - app/GameAuth/Worlds/PreproductionNativeRouteStore.php
  - app/GameAuth/Worlds/NativePreproductionStoreRefused.php
  - app/GameAuth/Worlds/NativeTopologyRegistry.php
  - app/Console/Commands/IssueNativeTopology.php
  - app/Console/Commands/PublishNativeRoute.php
  - config/game-auth.php
  - deploy/synology/compose.yml
  - tests/Feature/GameAuth/NativeTopology/PreproductionStoreRouteCommandsTest.php
  - docs/architecture/adr/0044-preproduction-store-native-route-publication.md
  - docs/architecture/adr/README.md
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/archive/OTERYN-20261011-preprod-route-publish.md
validation:
  - command: php8.5 vendor/bin/pint --test (changed PHP files)
    result: PASS
    evidence: passed
  - command: php8.5 vendor/bin/phpstan analyse (changed PHP files, level 10)
    result: PASS
    evidence: no errors
  - command: php8.5 vendor/bin/phpunit tests/Feature/GameAuth/NativeTopology tests/Feature/GameAuth/NativeLogin/NativeAdmissionIssuerTest.php
    result: PASS
    evidence: 47 tests, 3 skipped (MariaDB concurrency, pre-existing)
  - command: python3 -m pytest tests/ci (Synology compose and native admission tests) tools/validation
    result: PASS
    evidence: 154 and 139 passed
  - command: python3 tools/validation/adr_registry.py; python3 tools/validation/test_adr_registry.py
    result: PASS
    evidence: 49 ADRs; 13 tests
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group above baseline; baseline not raised
blockers:
  - none
next_action: none in Platform after merge. docs/operations/NATIVE_LOGIN_OWNER_STEPS.md from PR #1550 can drop its route-publication known limit once both merge.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository feature branch with no retention purpose after merge
source_branch_evidence: same-repository PR from this branch; the repository setting delete_branch_on_merge is true, so a merge removes the ref; confirm it is absent after merge
```
