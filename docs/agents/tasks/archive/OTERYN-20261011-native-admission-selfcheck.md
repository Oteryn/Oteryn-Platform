---
task_id: OTERYN-20261011-native-admission-selfcheck
governing_issue: 1419
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
search_first:
  - app/GameAuth/NativeAdmission/NativeAdmissionGrantIssuer.php
  - app/GameAuth/NativeAdmission/NativeAdmissionKeyring.php
optional_reads: []
---

# OTERYN-20261011 native admission signing self-check

## Goal

Governing GitHub Issue: #1419 (N4-P native Gateway and FND-04 grant issuance). This task was requested from coordination issue Oteryn/Oteryn-Game#1622 after the owner authorized enabling native login on Synology.

Add a Platform command, `game-auth:native-admission:self-check`, that proves the configured native admission key can sign a grant that verifies against the trust registry. Platform PR #1540 makes deploy refuse native admission until such a self-check exists. Wiring the command into the deploy scripts is a follow-up after #1540 merges.

## Acceptance criteria

- [x] The command runs `NativeAdmissionGrantIssuer::assertSignable()`, signs a throwaway grant through the real `issue()`, verifies it with the registry's trusted public key for the kid and checks that `resign()` is byte-identical.
- [x] Exit 0 prints one JSON line with the key id. Exit 1 prints one reason line. No key, seed, signature or token bytes are printed or logged.
- [x] Feature tests cover the success path, a loaded retiring key and 11 refusal cases.
- [x] The legacy token ratchet passes with no group rising.
- [ ] Exact-head CI passes. PHP 8.5 dependencies could not be installed in the authoring sandbox (disk), so CI is the first PHP run.

## Ownership

```yaml
owned_paths:
  - app/Console/Commands/VerifyNativeAdmissionSigning.php
  - tests/Feature/GameAuth/NativeAdmission/NativeAdmissionSelfCheckCommandTest.php
  - docs/agents/tasks/archive/OTERYN-20261011-native-admission-selfcheck.md
modules:
  - game-auth
dependencies:
  - PR #1540 (deploy gate) consumes this command in a follow-up
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T13:10:00Z
head: 7bd49dc30c99db42a211b72ee87ba5ed8b48dd74
branch: claude/native-admission-selfcheck-20261011
pr: 1542
status: completed
context_routes:
  - auth-identity
  - security
owned_paths:
  - app/Console/Commands/VerifyNativeAdmissionSigning.php
  - tests/Feature/GameAuth/NativeAdmission/NativeAdmissionSelfCheckCommandTest.php
  - docs/agents/tasks/archive/OTERYN-20261011-native-admission-selfcheck.md
proven:
  - The issuer self-check (contract §9.2) is `NativeAdmissionGrantIssuer::assertSignable()`. It requires the switch on, APP_ENV testing or preproduction, a loaded key and a kid that is trusted for the loaded public key.
  - Keyring and issuer failures throw `NativeAdmissionUnavailable`, whose messages carry no key material.
  - Commands under app/Console/Commands are auto-discovered; no registration is needed.
derived:
  - The command is read-only. It writes no rows, files or logs, and the throwaway grant stays in memory.
unknown:
  - PHPUnit, PHPStan and Pint results on the exact head; CI is the first run.
  - Follow-up: after #1540 merges, deploy/synology scripts run the command in the Platform container and gate GAME_AUTH_NATIVE_ADMISSION_ENABLED=true on exit 0.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Verify only with the public key derived from the loaded seed. This would not catch a key-to-trust mismatch independently of the issuer.
changed_paths:
  - app/Console/Commands/VerifyNativeAdmissionSigning.php
  - tests/Feature/GameAuth/NativeAdmission/NativeAdmissionSelfCheckCommandTest.php
  - docs/agents/tasks/archive/OTERYN-20261011-native-admission-selfcheck.md
validation:
  - command: php8.5 -l on changed PHP files
    result: PASS
    evidence: no syntax errors
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group above baseline
  - command: PHPUnit focused test
    result: NOT_RUN
    evidence: composer dependencies do not fit the sandbox disk; CI runs the suite on the PR head
blockers:
  - none
next_action: none; the deploy wiring follow-up belongs to the #1540 owner after merge.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository feature branch with no retention purpose after merge
source_branch_evidence: same-repository PR #1542 from this branch; the repository setting delete_branch_on_merge is true (REST repos/Oteryn/Oteryn-Platform), so a merge removes the ref; confirm it is absent after merge
```
