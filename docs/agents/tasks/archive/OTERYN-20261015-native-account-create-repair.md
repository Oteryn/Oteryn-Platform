---
task_id: OTERYN-20261015-native-account-create-repair
governing_issue: 1474
required_reads:
  - docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
search_first:
  - app/Accounts/ReadModels/NativeAccountOverviewReadModel.php
  - app/Characters/Actions/CreateCharacter.php
optional_reads: []
---

# Native Account Center character creation repair

## Goal

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1474.
Owner request in this chat (2026-10-15): repair character creation at the Polish Account Center.

The deployed main enables native commands and web creation, but Account Center still gates its entire roster and create action on retired numeric account binding. New native identities have no such binding. Route explicitly enabled native deployments to canonical AccountId and the existing authorized LCFA read model. Creation permits an attempt through the existing enabled command service; Game retains all game-domain validation and receipt authority. Do not infer a native slot limit from a displayed count.

## Acceptance criteria

- [x] The application HTTP regression proves Account Center -> form -> pending intent -> simulated Game receipt for an account with no retired binding; physical Game qualification remains open.
- [x] Only the authenticated account's fresh native roster is rendered; missing/stale/invalid/unavailable evidence is distinct from authoritative empty.
- [x] Disabled creation stays unavailable and PL/EN locale is preserved.
- [ ] Focused tests, formatting and static checks pass; exact-candidate CI and runtime qualification are recorded.

## Ownership

```yaml
owned_paths:
  - resources/views/characters/request.blade.php
  - public/js/character-request-status.js
  - scripts/acceptance/unit/character-request-status.test.mjs
  - scripts/acceptance/package.json
  - app/Accounts/ReadModels/NativeAccountOverviewReadModel.php
  - app/Http/Controllers/Accounts/AccountOverviewController.php
  - resources/views/identity/account/overview.blade.php
  - resources/views/identity/account/partials/native-roster.blade.php
  - lang/en/portal.php
  - lang/pl/portal.php
  - tests/Feature/Accounts/NativeAccountOverviewTest.php
  - tests/Feature/Characters/CharacterCreationCommandTest.php
  - docs/agents/tasks/active/OTERYN-20261015-native-account-create-repair.md
modules: [Accounts, Characters]
dependencies: [LCFA, CharacterAuthority]
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-15T20:58:00Z
head: 95ad4600a9f27ff3e8bf2c5ccad75939fde07bce
branch: codex/native-account-oracle-final-20261015
pr: 1642
status: completed
context_routes:
  - native-character-portfolio
  - native-characters
owned_paths:
  - resources/views/characters/request.blade.php
  - public/js/character-request-status.js
  - scripts/acceptance/unit/character-request-status.test.mjs
  - scripts/acceptance/package.json
  - app/Accounts/ReadModels/NativeAccountOverviewReadModel.php
  - app/Http/Controllers/Accounts/AccountOverviewController.php
  - resources/views/identity/account/overview.blade.php
  - resources/views/identity/account/partials/native-roster.blade.php
  - lang/en/portal.php
  - lang/pl/portal.php
  - tests/Feature/Accounts/NativeAccountOverviewTest.php
  - tests/Feature/Characters/CharacterCreationCommandTest.php
  - docs/agents/tasks/active/OTERYN-20261015-native-account-create-repair.md
proven:
  - Main 98edeee02cf2be81e0aac4d47e33f6c25630ca52 enables command and web-create deployment defaults.
  - Legacy AccountOverviewReadModel suppresses native accounts without retired numeric account binding.
derived:
  - This legacy gate prevents the native creation entry point from appearing.
unknown:
  - Browser roster and automatic result polling on the repaired deployed candidate remain to be qualified.
conflicts: []
first_failure:
  marker: legacy-account-binding-gate
  evidence: AccountOverviewReadModel::forIdentity returns none before native creation is considered.
rejected_hypotheses: []
changed_paths:
  - resources/views/characters/request.blade.php
  - public/js/character-request-status.js
  - scripts/acceptance/unit/character-request-status.test.mjs
  - scripts/acceptance/package.json
  - app/Accounts/ReadModels/NativeAccountOverviewReadModel.php
  - app/Http/Controllers/Accounts/AccountOverviewController.php
  - resources/views/identity/account/overview.blade.php
  - resources/views/identity/account/partials/native-roster.blade.php
  - lang/en/portal.php
  - lang/pl/portal.php
  - tests/Feature/Accounts/NativeAccountOverviewTest.php
  - tests/Feature/Characters/CharacterCreationCommandTest.php
validation:
  - command: php vendor/bin/phpunit tests/Feature/Accounts tests/Feature/Characters tests/Feature/CharacterProfiles/CharacterProfilePreferenceTest.php
    result: PASS
    evidence: PHP 8.5.11, 44 tests and 475 assertions; authenticated HTTP flow and simulated Game receipt.
  - command: php vendor/bin/phpunit tests/Feature/Accounts tests/Feature/Characters tests/Feature/CharacterAuthority tests/Feature/GameAuth/NativeAccountCharacters
    result: PASS
    evidence: 88 tests, 781 assertions, no failures; 4 existing database-specific skips and 1 existing assertionless wire test marked risky.
  - command: focused Pint and PHPStan; php artisan view:cache; git diff --check
    result: PASS
    evidence: Six PHP files formatted, four changed implementation/test files statically checked, Blade templates compiled.
  - command: node --check public/js/character-request-status.js; npm --prefix scripts/acceptance run test:runtime-diagnostics
    result: PASS
    evidence: 38 Node tests pass, including 6 new polling tests; same localized operation, read-only checks, terminal navigation, bounded retries and session/rate-limit failures.
  - command: authenticated deployed runtime inspection through owner-authorized Desktop Commander
    result: BLOCKED
    evidence: The existing Edge request shows CHAR_CMD_INTENT_EXPIRED. Live command flags are on. Game logs repeated transient decisions; four receipts are expired and one request remained pending. No test Character was created.
  - command: authenticated mTLS account-security read and process-local timing instrumentation
    result: PASS
    evidence: TLS 1.3 request returns observed and allowed=true in 5.034 seconds, exceeding the existing 3-second Game exchange bound. CLI observation is 2.835 to 3.868 seconds; one measured COMMIT is 1398 ms and file/directory fsync are 185 and 1207 ms. Query execution alone is 22 ms. Instrumentation forwards all original fsync calls unchanged and modifies no deployed source.
blockers: []
next_action: qualify the repaired exact candidate and deploy it with the preserved migrated Oracle state
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: retain the repair candidate for review and qualification
source_branch_evidence: pending
```

## Verification limits and resources

No Game repository mutation or manual runtime/data repair was performed. Authenticated diagnostic evidence reads advance normal observation revisions. Existing command enablement gates remain intact. Native LCFA does not supply level, vocation or native profile-management capabilities, so Account Center renders only the authorized native names and stops using numeric legacy preferences.

Task container `oteryn-account-fix-php` and `oteryn-account-fix-php-2` are ephemeral, labelled `oteryn.task=account-create-fix-20261015`; remove after validation. Keep the task checkout and candidate branch for review. PHP development dependencies were installed locally with the unrelated `ext-gd` platform requirement ignored; touched behavior does not use GD.

## Owner clarification and runtime evidence (2026-10-15)

The owner clarified that the submitted request stays pending and explicitly requested checking their computer through Remote Desktop. `Molehill-PC` Edge already displayed an expired request when inspected. The status page was a static snapshot with no automatic read, so the repair now also polls the same authorized status URL every five seconds for at most five minutes, offers a manual status link, and offers another creation only after a terminal receipt. A failed check never changes command state or resubmits creation.

Private runtime inspection is narrow and read-only except for normal security-evidence observation revisions produced by authenticated diagnostic reads. No credentials, keys, cookies, account identifiers or intent bodies are included in this record. No deployed files, certificates, source descriptors, trust floors or database rows were manually changed.

Game's account-security floor stayed at revision 21 while Platform had issued revision 33; one publication slot retained revision 21. The current node/custody registration and assigned scope match. CPU samples are low (Game runner 0.85%, Game PostgreSQL 0.10%, Platform 3.91%). Existing Game issue #2206 / PR #2208 already cover bootstrap diagnostics and a measured 5.552-second native account-security read; they remain open and were not modified. Do not discard the retained publication or loosen the 3-second exchange / 5-second freshness fences as a workaround.

Platform PR: https://github.com/Oteryn/Oteryn-Platform/pull/1637. This is a partial repair until storage/authority evidence and real Game creation are qualified.

## Storage probe and scoped next operation

Three tiny atomic file + file/directory fsync probes per existing volume: `/volume1` 2189/950/1727 ms, `/volume2` 436/455/627 ms. Each generated `oteryn-create-latency-1637-*` directory and probe file was removed and absence verified by the probe. Existing native witness and Game PostgreSQL data are on `/volume1`. Both data volumes are Btrfs on SATA HDDs; the two NVMe devices already form RAID0, so repurposing them is not part of this repair.

A protected storage relocation requires separate owner approval; the proposal is to preserve a current backup, quiesce the two application writers, relocate the witness and WAL write paths to dedicated directories on the already-existing faster `/volume2`, preserve exact current state/owners/modes, and validate real command completion against the original fences. Rollback must copy current WAL and witness state back while stopped, never resume stale retained copies. No storage relocation or process restart was performed in this invocation.

## CI repair before the second candidate

The first candidate exposed an existing profile-preference integration test that still exercises the explicitly selected compatibility deployment. Preserve that existing consumer instead of deleting its working presentation: select the native Account Center before compatibility reads whenever native projection mode or native creation is enabled; preserve the old model/view/tests for the explicitly disabled native mode. No compatibility fallback is used for unavailable, invalid or stale native data. Native creation and native portfolio tests are in NativeAccountOverviewTest. The first candidate also omitted its newly created PR number; this packet now records PR 1637.

### Reviewable relocation proposal (not executed)

| Current source | Dedicated proposed destination | Observed size |
| --- | --- | --- |
| `/volume1/@docker/volumes/oteryn-staging_platform_storage/_data/app/native-evidence-witness` | `/volume2/oteryn/character-create-1637/native-evidence-witness` | 24 KiB |
| `/volume1/@docker/volumes/oteryn-staging_postgres_data/_data/pg_wal` | `/volume2/oteryn/character-create-1637/platform-pg-wal` | 33 MiB |
| `/volume1/oteryn/game-preprod/db/pg_wal` | `/volume2/oteryn/character-create-1637/game-pg-wal` | 32 MiB |

1. Refuse any existing destination or ambiguous service/mount identity. Preserve a current restorable database backup and exact witness-store copy in root-only storage; record digests without publishing contents.
2. Quiesce Platform web/scheduler and Game through the approved operational route, then stop both PostgreSQL instances cleanly. Preserve all current intents and receipts.
3. Copy the exact three stopped write paths with current ownership, permissions and bytes. Install reviewed persistent bind mounts through the canonical deploy configuration; do not format disks, repurpose NVMe cache, edit trust floors, or change PostgreSQL durability or source freshness settings.
4. Start PostgreSQL, verify data/schema identity and recovery, start Platform and Game, then require repeated authenticated security reads inside the existing 3-second bound plus real Character creation/receipt/list/reload qualification.
5. On failure, stop writers and databases and copy the latest WAL and witness state back before selecting the original mounts. Old copies are preserved recovery evidence, never current authority after new writes.

This protected operational change needs the owner's separate task-specific production/data authority under root AGENTS.md and PLATFORM_AGENT_BOOTSTRAP.md. The current owner instruction specifically authorized remote inspection; no relocation has been started.

## Execution cleanup closeout

REMOVED: both uniquely named storage-probe directories, verified absent by the probe. The PC screenshot and both local validation containers were removed and absence verified after final checks. No temporary service, workflow, secret, or live configuration was installed.
KEPT: the task checkout, ignored PHP dependencies, PHP base image cache, and PR branch for review/continuation.
VERIFIED: exact task-labelled Docker inventory and post-removal inventory; exact PC temporary screenshot path; storage probe emitted absent=true for both directories.
REMAINING: owner decision for the protected storage relocation, existing CI baseline task-record cleanup, and real Game qualification.

The native roster is a partial in the existing Account Center view, not a new independently routed public view. This preserves the existing route/view coverage identity and shares the account security presentation. The strict portal closure gate also has pre-existing missing coverage entries; no unqualified browser evidence was marked complete.

## Oracle migration removes the observed storage blocker

The owner subsequently authorized and requested the complete Oracle migration,
governed separately by Issue 1639 / PR 1640. Source writers were frozen; existing
identity, Game data, signing history and independent custody moved coherently.
An owner-authorized browser registration and login on oteryn.com succeeded. One
real web request produced Game CHAR_CMD_OK with an accepted receipt in 202 ms
and an accepted AccountCharacters snapshot. A controlled Game restart advanced
ownership_generation 6 to 7 and returned ready; the owned Character remains in
the fresh authoritative list with availability AVAILABLE. No timeout, freshness
or trust fence was relaxed. Historical NAS latency above is retained as diagnosis,
not a current Oracle blocker.

The main Account Center still hides native portfolios, exactly the defect this
PR repairs. Deployed browser roster and automatic polling acceptance remain open
until this candidate is qualified and installed. Baseline activation packet
Issue 1623 / PR 1624 is archived consistently with the separate Oracle task.

## Replacement candidate after material upstream overlap

The source candidate in PR 1637 is retained. After deployment PR 1640 reached protected main, adding its terminal packet transition on the older repair branch created a real add/add conflict. The control plane selected a new API-native candidate on current main 95ad4600a9f27ff3e8bf2c5ccad75939fde07bce, preserving the complete reviewed application delta and the required terminal deployment transition. No history was reset, rebased, force-pushed or silently relabelled. This branch is the single active writer; the earlier candidate will close as superseded once this PR is allocated.

Replacement PR: https://github.com/Oteryn/Oteryn-Platform/pull/1642. Previous PR 1637 is superseded, not merged.

## Delivered PR closeout

PR #1642 merged through the governed exact-head Merge Queue route.
Protected main readback: 0628d229a53bf54d2d978a98b81204753ac1bf79.
Source branch disposition: auto-delete after merge; branch readback belongs to
operational closeout. Remaining deployment/native-client acceptance is owned by
active Oracle Actions task under #1639, not this delivered candidate.
Real browser acceptance passed: owner-only fresh Character roster, normal Game
receipt-backed creation and automatic duplicate-name rejection refresh.
Aggregate Issue #1474 remains open for its other programme acceptance.
