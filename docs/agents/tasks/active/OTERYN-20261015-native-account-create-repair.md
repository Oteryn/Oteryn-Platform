---
task_id: OTERYN-20261015-native-account-create-repair
governing_issue: 1474
required_reads:
  - docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
search_first:
  - app/Accounts/ReadModels/AccountOverviewReadModel.php
  - app/Characters/Actions/CreateCharacter.php
optional_reads: []
---

# Native Account Center character creation repair

## Goal

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1474.
Owner request in this chat (2026-10-15): repair character creation at the Polish Account Center.

The deployed main enables native commands and web creation, but Account Center still gates its entire roster and create action on IdentityCanaryAccount. New native identities have no such binding. Replace that compatibility composition with canonical AccountId and the existing authorized LCFA read model. Creation permits an attempt through the existing enabled command service; Game retains all game-domain validation and receipt authority. Do not infer a native slot limit from a displayed count.

## Acceptance criteria

- [x] A native account without a legacy binding can follow Account Center -> creation -> pending intent -> Game receipt.
- [x] Only the authenticated account's fresh native roster is rendered; missing/stale/invalid/unavailable evidence is distinct from authoritative empty.
- [x] Disabled creation stays unavailable and PL/EN locale is preserved.
- [ ] Focused tests, formatting and static checks pass; exact-candidate CI and runtime qualification are recorded.

## Ownership

```yaml
owned_paths:
  - app/Accounts/ReadModels/AccountOverviewReadModel.php
  - app/Http/Controllers/Accounts/AccountOverviewController.php
  - resources/views/identity/account/overview.blade.php
  - lang/en/portal.php
  - lang/pl/portal.php
  - tests/Feature/Accounts/AccountOverviewTest.php
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
updated_at: 2026-10-15T16:04:36Z
head: UNKNOWN
branch: codex/fix-native-account-character-creation-20261015
pr: none
status: validating
context_routes:
  - native-character-portfolio
  - native-characters
owned_paths:
  - app/Accounts/ReadModels/AccountOverviewReadModel.php
  - app/Http/Controllers/Accounts/AccountOverviewController.php
  - resources/views/identity/account/overview.blade.php
  - lang/en/portal.php
  - lang/pl/portal.php
  - tests/Feature/Accounts/AccountOverviewTest.php
  - tests/Feature/Characters/CharacterCreationCommandTest.php
  - docs/agents/tasks/active/OTERYN-20261015-native-account-create-repair.md
proven:
  - Main 98edeee02cf2be81e0aac4d47e33f6c25630ca52 enables command and web-create deployment defaults.
  - Legacy AccountOverviewReadModel suppresses native accounts without IdentityCanaryAccount.
derived:
  - This legacy gate prevents the native creation entry point from appearing.
unknown:
  - Authenticated live browser outcome; the public URL redirects to login.
conflicts: []
first_failure:
  marker: legacy-account-binding-gate
  evidence: AccountOverviewReadModel::forIdentity returns none before native creation is considered.
rejected_hypotheses: []
changed_paths:
  - app/Accounts/ReadModels/AccountOverviewReadModel.php
  - app/Http/Controllers/Accounts/AccountOverviewController.php
  - resources/views/identity/account/overview.blade.php
  - lang/en/portal.php
  - lang/pl/portal.php
  - tests/Feature/Accounts/AccountOverviewTest.php
  - tests/Feature/Characters/CharacterCreationCommandTest.php
validation:
  - command: php vendor/bin/phpunit AccountOverviewTest CharacterCreationTest CharacterCreationCommandTest
    result: PASS
    evidence: PHP 8.5.11, 26 tests and 266 assertions; authenticated HTTP flow and simulated Game receipt.
  - command: php vendor/bin/phpunit tests/Feature/Accounts tests/Feature/Characters tests/Feature/CharacterAuthority tests/Feature/GameAuth/NativeAccountCharacters
    result: PASS
    evidence: 88 tests, 781 assertions, no failures; 4 existing database-specific skips and 1 existing assertionless wire test marked risky.
  - command: focused Pint and PHPStan; php artisan view:cache; git diff --check
    result: PASS
    evidence: Six PHP files formatted, four changed implementation/test files statically checked, Blade templates compiled.
  - command: authenticated deployed runtime browser qualification
    result: BLOCKED
    evidence: Live account URL redirects this session to login; no authenticated browser or test account is supplied. NAS read connector returns HTTP 429.
blockers: []
next_action: Publish the single repair candidate for exact-head CI and authenticated runtime qualification.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: retain the repair candidate for review and qualification
source_branch_evidence: pending
```

## Verification limits and resources

No Game repository mutation, credentials, live data or runtime changes. Existing command enablement gates remain intact. Native LCFA does not supply level, vocation or native profile-management capabilities, so Account Center renders only the authorized native names and stops using numeric legacy preferences.

Task container `oteryn-account-fix-php` is ephemeral, labelled `oteryn.task=account-create-fix-20261015`; remove after validation. Keep the task checkout and candidate branch for review. PHP development dependencies were installed locally with the unrelated `ext-gd` platform requirement ignored; touched behavior does not use GD.
