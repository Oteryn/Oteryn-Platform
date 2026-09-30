---
task_id: OTERYN-20260930-n4p-2b-redemption-concurrency
governing_issue: 1419
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
search_first:
  - app/GameAuth/NativeLogin
  - tests/Feature/GameAuth/Concurrency
optional_reads: []
---

# OTERYN-20260930-n4p-2b-redemption-concurrency

## Goal

Governing GitHub Issue: #1419 (https://github.com/Oteryn/Oteryn-Platform/issues/1419). Coordination: Oteryn/Oteryn-Game#162, owner authority Q14. Pre-activation follow-ups from the N4P-2 security review (#1423 comments 5902866882 and 5902971240).

N4P-2b: prove contract §16 "concurrent redeem, one winner" on real InnoDB for two concurrent redeems of one ticket and two concurrent requests with one `attempt_ref`; when a first request loses by deadlock (1213) or lock wait timeout (1205), roll back and re-read the attempt once instead of answering `NATIVE_LOGIN_UNAVAILABLE`. Add `#[SensitiveParameter]` to `GameLoginTicketSecrets::hash()`.

Out of scope: U13 retired attempt rows, route selection (N4P-3), enabling anything, workflow edits, production and secrets.

## Acceptance criteria

- [x] Independent-process InnoDB proof: one ticket, two attempt_refs - one grant, the other `NATIVE_LOGIN_TICKET_REJECTED`.
- [x] Same attempt_ref, identical request - both receive the byte-identical grant; one attempt row.
- [x] Same attempt_ref, changed request member - one grant, the other `NATIVE_LOGIN_ATTEMPT_CONFLICT`; never a second grant.
- [x] 1213/1205 on a first request (before commit, outermost transaction) is re-read once as a lost race.
- [x] A lock conflict that repeats on the one re-read is the retryable `NATIVE_LOGIN_UNAVAILABLE`, grants nothing and leaves the ticket unused (control-plane decision on author question 1; the contract has no specific rule).
- [x] `#[SensitiveParameter]` on `GameLoginTicketSecrets::hash()`.
- [x] Registered in the existing `concurrency-proof` job without a workflow edit.

## Ownership

```yaml
owned_paths:
  - app/GameAuth/NativeLogin/NativeAdmissionAttempts.php
  - app/GameAuth/Tickets/GameLoginTicketSecrets.php
  - tests/Feature/GameAuth/Concurrency/NativeGameTicketConcurrencyTest.php
  - docs/agents/tasks/active/OTERYN-20260930-n4p-2b-redemption-concurrency.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-2b-redemption-concurrency.md
  - docs/agents/tasks/active/OTERYN-20260930-n4p-2-native-ticket-redemption.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-2-native-ticket-redemption.md
modules:
  - GameAuth/NativeLogin
dependencies:
  - "#1423 N4P-2 native ticket redemption (merged ecfcc573)"
blockers:
  - none
cross_repository_tasks:
  - none
```

Overlap: the integrated N4P-2 packet owned `app/GameAuth/NativeLogin/**`; it is archived by this task. No open PR touches the owned paths (checked 2026-09-30).

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-30T23:30:00Z
head: 1fee84a
branch: claude/n4p-2b-redemption-concurrency
pr: 1424
status: completed
terminal_pr_policy: archive_pending
context_routes:
  - auth-identity
  - security
owned_paths:
  - app/GameAuth/NativeLogin/NativeAdmissionAttempts.php
  - app/GameAuth/Tickets/GameLoginTicketSecrets.php
  - tests/Feature/GameAuth/Concurrency/NativeGameTicketConcurrencyTest.php
  - docs/agents/tasks/active/OTERYN-20260930-n4p-2b-redemption-concurrency.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-2b-redemption-concurrency.md
  - docs/agents/tasks/active/OTERYN-20260930-n4p-2-native-ticket-redemption.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-2-native-ticket-redemption.md
proven:
  - Before the fix, all three races deadlocked on InnoDB (Innodb_deadlocks +1 each) and the rolled-back request answered NATIVE_LOGIN_UNAVAILABLE; still exactly one grant and one attempt row
  - The deadlock is SELECT ... FOR UPDATE gap lock on the missing attempt_ref (both requests) against the winner's insert intention while the loser waits on the ticket row; InnoDB rolled back the inserting winner, so the waiting request then redeems - either side can be the rolled-back one
  - With the fix the same races pass on 3 consecutive runs while deadlocks still occur each run, so the re-read path is exercised, not avoided
  - The retry applies only when the transaction body had not finished and no outer transaction remains (a deadlock rolls back the whole outer InnoDB transaction)
  - The workflow step `php artisan test --filter=GameTicketConcurrencyTest` is a PHPUnit regex and selects NativeGameTicketConcurrencyTest; app/GameAuth and tests/Feature/GameAuth paths classify to all gates, so no workflow edit is needed
  - CI run 36662949877 job runtime-tests failed at PHPStan (level 10) only: pcntl_wifexited/pcntl_wexitstatus received the by-reference waitpid status typed mixed in the new concurrency test; fixed with the repository's is_int guard pattern
derived:
  - Contract §6.2/§11.2/§13 do not name a result for a repeated lock conflict; control-plane decision (repair of 1fee84a) - a second 1205/1213 on the re-read rolled back with nothing committed, so it is NATIVE_LOGIN_UNAVAILABLE (retryable, same request with backoff), never a grant. Other second lost races (unique violation, ticket consumed by the same attempt_ref without a visible record) stay ADMISSION_ATTEMPT_RECONCILIATION_REQUIRED
unknown:
  - U13 attempt audit retention
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Laravel DB::transaction attempts > 1 rejected; it would silently re-run the whole issuance instead of the documented single re-read
  - Avoiding the gap lock (insert-first or READ COMMITTED) rejected; a larger change to reviewed N4P-2 locking than the review asked for
changed_paths:
  - app/GameAuth/NativeLogin/NativeAdmissionAttempts.php
  - app/GameAuth/Tickets/GameLoginTicketSecrets.php
  - tests/Feature/GameAuth/Concurrency/NativeGameTicketConcurrencyTest.php
  - docs/agents/tasks/active/OTERYN-20260930-n4p-2b-redemption-concurrency.md
  - docs/agents/tasks/archive/OTERYN-20260930-n4p-2-native-ticket-redemption.md (moved from active)
validation:
  - command: php artisan test --filter=GameTicketConcurrencyTest (MariaDB 10.11 InnoDB, GAME_AUTH_CONCURRENCY_TEST=1, pcntl)
    result: PASS
    evidence: local PHP 8.5.11; 8 tests (5 existing + 3 new); new tests fail without the fix (3 failed) and pass with it on 3 consecutive runs; CI runs MariaDB 11.8
  - command: php artisan test (full suite, sqlite), vendor/bin/pint --test, composer audit
    result: PASS
    evidence: local PHP 8.5.11; 715 tests, 0 failures (638 carry the pre-existing missing-.env warning); Pint passed; no advisories
  - command: composer analyse (PHPStan)
    result: BLOCKED
    evidence: phpstan/phpstan is dist-only and its api.github.com zipball is refused by the session proxy (403); exact-head CI is authoritative
  - command: repair session - php -l on changed PHP files, git diff --check, checkpoint.py --require-checkpoint
    result: PASS
    evidence: local PHP 8.4.19; composer install refused (repository requires php ^8.5, and with only the php requirement ignored GitHub dist downloads fail "Could not authenticate against github.com"), so PHPStan, Pint and PHPUnit did not run locally; exact-head CI (runtime-tests and concurrency-proof on MariaDB 11.8) is authoritative for the new repeated-lock-conflict test
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: no route or actor path; the issuer stays behind the default-off switch until N4P-3
blockers:
  - none
next_action: none; PR 1424 integrated as e289fc773a430461a7e6c5a3412a4267a90c81f8 and this packet was archived by the next #1419 delivery task (N4P-ING-RS)
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR integration completed
source_branch_evidence: source ref claude/n4p-2b-redemption-concurrency absent (ls-remote 2026-09-30) after PR 1424 integrated as e289fc773a430461a7e6c5a3412a4267a90c81f8
```
