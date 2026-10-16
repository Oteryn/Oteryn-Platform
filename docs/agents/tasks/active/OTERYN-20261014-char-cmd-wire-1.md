---
task_id: OTERYN-20261014-char-cmd-wire-1
governing_issue: 1474
required_reads:
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
search_first:
  - app/Characters/Actions/CreateCharacter.php
  - app/CharacterAuthority/CharacterCommandClient.php
optional_reads: []
---

# OTERYN-20261014 Character creation command wiring (CHAR-CMD-WIRE-1)

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Cross-repository programme: Oteryn/Oteryn-Game#1622.

Step 3 of Game contract `oteryn-game-character-authority-commands-v1` §14: the Account Center form (`account.characters.store`) records a `CreateCharacter` intent in the intent/receipt store from #1532 and shows the outcome from Game's receipt. The internal routes are mounted behind the existing default-off peer guard. Web creation stays disabled by default (`character-authority.create.enabled`); enablement (step 4) waits on U-CC7 (recovery_generation storage, PLATFORM-RESTORE-RECONCILE-P1) and U-CC9 and is out of scope.

## Acceptance criteria

- [x] Disabled by default: with `character-authority.create.enabled` false, or the command surface unusable, the form answers "temporarily unavailable" and writes nothing.
- [x] Enabled: one intent per rendered form (idempotency key `web-create:<identity id>:<form submission UUID>`); a double submit reuses the operation; the same submission with other details is refused.
- [x] `account_id` comes from the signed-in Identity; the browser cannot supply it. The target world must have a canonical WorldId; the name is canonicalized and checked against CHAR-NAME-1.
- [x] `GET /account/characters/requests/{operation}` shows PENDING until Game's receipt, then created or the rejection mapped from the result code; another Identity's or a malformed operation answers 404.
- [x] `routes/internal.php` mounts the list, read and receipt routes; they answer 503 while commands are disabled.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - app/Characters/**
  - app/Http/Controllers/Characters/CharacterCreationController.php
  - app/Http/Requests/Characters/CreateCharacterRequest.php
  - app/CharacterAuthority/CharacterCommandClient.php
  - app/CharacterAuthority/Http/CharacterCommandRoutes.php
  - app/Providers/AppServiceProvider.php
  - config/character-authority.php
  - lang/en/portal.php
  - lang/pl/portal.php
  - resources/views/characters/**
  - routes/web.php
  - routes/internal.php
  - tests/Feature/Characters/**
  - tests/Feature/CharacterAuthority/CharacterCommandClientTest.php
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/ROUTE_VIEW_NAVIGATION_INVENTORY.json
  - docs/agents/tasks/active/OTERYN-20261014-char-cmd-wire-1.md
  - docs/agents/tasks/archive/OTERYN-20261014-char-cmd-wire-1.md
modules:
  - game-auth
dependencies:
  - OTERYN-20261011-character-cmd-client
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Game#1622 CHAR-CMD-CONSUMER-1 (Game command consumer, parallel worker)
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-14T22:40:00Z
head: bc825873bfaaca4f916a3cfa2f0b5e70436d5782
branch: claude/char-cmd-wire-1
pr: 1622
status: validating
context_routes:
  - game-auth
owned_paths:
  - app/Characters/**
  - app/Http/Controllers/Characters/CharacterCreationController.php
  - app/Http/Requests/Characters/CreateCharacterRequest.php
  - app/CharacterAuthority/CharacterCommandClient.php
  - app/CharacterAuthority/Http/CharacterCommandRoutes.php
  - app/Providers/AppServiceProvider.php
  - config/character-authority.php
  - lang/en/portal.php
  - lang/pl/portal.php
  - resources/views/characters/**
  - routes/web.php
  - routes/internal.php
  - tests/Feature/Characters/**
  - tests/Feature/CharacterAuthority/CharacterCommandClientTest.php
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/ROUTE_VIEW_NAVIGATION_INVENTORY.json
  - docs/agents/tasks/active/OTERYN-20261014-char-cmd-wire-1.md
  - docs/agents/tasks/archive/OTERYN-20261014-char-cmd-wire-1.md
proven:
  - The intent wire, receipt validation and peer guard already exist from #1532; this packet only issues CreateCharacter intents and reads their receipts.
derived:
  - The CreateCharacter binding carries only account_id, target_world_id and requested_name; vocation and sex stay validated form inputs but are not part of the v1 intent.
  - Ownership of a request is proven by the intent's account_id matching the signed-in Identity, so no new column is needed.
unknown:
  - PHP suite, PHPStan and Pint results; composer cannot download packages in the authoring sandbox, so CI is the first run.
  - The Game contract text was not readable from this Platform-scoped session; the wire follows the merged Platform implementation of it.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Classifying the internal command routes as portal surfaces; they are mTLS machine endpoints, so the portal manifest excludes them by prefix like other framework/protocol routes.
  - Reusing commands.enabled as the web switch; Bazaar transfers and web creation need separate enablement.
  - Keeping outcome messages as English literals; the result page must honor the portal locale, so they live in portal.create.outcome and the store redirect carries the locale query.
changed_paths:
  - app/Characters/Actions/CreateCharacter.php
  - app/Characters/CharacterCreationOutcome.php
  - app/Characters/Data/CharacterCreationResult.php
  - app/Http/Controllers/Characters/CharacterCreationController.php
  - app/Http/Requests/Characters/CreateCharacterRequest.php
  - app/CharacterAuthority/CharacterCommandClient.php
  - app/CharacterAuthority/Http/CharacterCommandRoutes.php
  - app/Providers/AppServiceProvider.php
  - config/character-authority.php
  - lang/en/portal.php
  - lang/pl/portal.php
  - resources/views/characters/create.blade.php
  - resources/views/characters/request.blade.php
  - routes/web.php
  - routes/internal.php
  - tests/Feature/Characters/CharacterCreationCommandTest.php
  - tests/Feature/CharacterAuthority/CharacterCommandClientTest.php
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - docs/testing/ROUTE_VIEW_NAVIGATION_INVENTORY.json
  - docs/agents/tasks/active/OTERYN-20261014-char-cmd-wire-1.md
validation:
  - command: node scripts/acceptance/coverage/validate-portal-coverage.mjs --manifest-only (plus product, backend-frontend, dimension and media validators)
    result: PASS
    evidence: manifest classifies account.characters.requests.show and excludes internal.character-authority.commands.*; strict route discovery needs artisan and runs in CI
  - command: php -l on changed PHP files
    result: PASS
    evidence: no syntax errors
  - command: php artisan test tests/Feature/Characters tests/Feature/CharacterAuthority
    result: NOT_RUN
    evidence: vendor cannot be installed in the authoring sandbox (GitHub dist downloads refused); runs on the exact head in CI
blockers:
  - none
next_action: Archive this task record; PR #1622 is terminal on main.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
