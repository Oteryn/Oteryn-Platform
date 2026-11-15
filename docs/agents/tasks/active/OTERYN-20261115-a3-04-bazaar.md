---
task_id: OTERYN-20261115-a3-04-bazaar
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Audit wave 3 Bazaar repairs

Governing workstream: https://github.com/Oteryn/Oteryn-Platform/issues/536 (OPEN, live verified). The owner supplies the exact base, branch, findings, checks and push authorization. No PR or integration requested. Single agent; no subagents.

Branch: `claude/happy-davinci-ijz3wy-fix-a3-04-bazaar`.
Base: `90234d8d2d0a51a654b8ff6947d38372a740acba`.
Ownership: Bazaar authority, runtime, moderation, fixtures and additive reconciliation migration. Protected RBAC paths untouched. Jira synchronization belongs to the coordinator.

Confirmed BZ3-001 (cancelled character unlock scheduling), BZ3-002 (global restriction fences), BZ3-003 (lifetime retry/history traversal). CI-02 is a fixture clock mismatch: later intents use wall time after receipt time was captured, violating decided_at >= issued_at_source.

Implementation and PostgreSQL validation in progress. Migration timestamp uses 20261115214000: the supplied 13-digit prefix sorts before the existing 14-digit migrations and fails on absent Bazaar tables. Local test databases only, no game repository access, flags and contract bytes preserved.

Source branch disposition: retain for coordinator review after requested publication; no PR, merge or deployment.

## Qualification evidence

All four findings are confirmed and fixed. No protected RBAC path or merged migration was changed. One additive migration introduces scoped restriction evaluation, cleanup pointers, indexed disposition eligibility, publication head and admission pressure projections. The existing immutable command, receipt and audit history remains authoritative. Conflict insertion invalidates cached no-effect eligibility atomically; reconciliation verifies at most 24 legacy dispositions per committed batch.

Regression proof with the original product implementations restored and a consistent fixture clock:

- `postgres_cancelled_character_unlock_retries_both_currencies`: expected failure, 0.84 s (no replacement after cancellation).
- `postgres_item_restrictions_scope_verified_world_definition_and_value`: expected failure, 0.80 s (unrelated verified listing fenced).
- `postgres_cleanup_attempt_pointer_bounds_all_commands_and_issuer_wait`: expected failure, 2.70 s; original Return retry read 643 tuples.
- `postgres_return_publication_uses_live_dispositions_and_revokes_conflicts`: expected failure, 4.55 s; original publication read 10,274 tuples.
- `character_fixture_uses_one_source_clock_across_second_boundary`: expected failure with the old wall-clock fixture constructor.

Candidate passes both unchanged less-than-100-tuple assertions. Cleanup history covers 128 refusals for each of Return, Unlock, Release and Payout; publication covers 1,024 refused dispositions and revocation after conflict. Character return/unlock covers Coins and Gold. Restriction coverage includes verified listings in two worlds, below/above minimum, public visibility, a funded browser bid, settlement and cancellation after an audited return. Moderation fixtures now run the actual migration set rather than selected historical fragments.

Final commands (native installed Rust 1.97.0; no tool installation):

- `cargo fmt --all --check`: PASS.
- `cargo clippy --locked --all-targets -p oteryn-bazaar -- -D warnings`: PASS.
- `taskset -c 0-3 cargo test --locked -p oteryn-bazaar`, default test threads, five consecutive complete runs: PASS, 134 tests/run (116 library + 3 domain + 15 PostgreSQL integration), 670 passed, zero failures, ignores or environment skips. Real PostgreSQL used a dedicated disposable local database through `PORTAL_TEST_DATABASE_URL`.

PostgreSQL regression durations in seconds, runs 1–5:

| Test | 1 | 2 | 3 | 4 | 5 |
| --- | --- | --- | --- | --- | --- |
| postgres_cancelled_character_unlock_retries_both_currencies | 6.675 | 4.964 | 4.714 | 4.886 | 5.199 |
| postgres_item_restrictions_scope_verified_world_definition_and_value | 2.035 | 1.740 | 1.651 | 1.872 | 1.726 |
| postgres_cleanup_attempt_pointer_bounds_all_commands_and_issuer_wait | 5.781 | 5.249 | 4.957 | 5.265 | 5.267 |
| postgres_return_publication_uses_live_dispositions_and_revokes_conflicts | 5.661 | 4.965 | 5.025 | 5.002 | 4.943 |
| postgres_browser_verified_character_snapshot_facts | 3.368 | 2.998 | 2.889 | 3.039 | 3.326 |

Raw validation logs are retained locally at `/tmp/bazaar-final-{1,2,3,4,5}.log`, `/tmp/bazaar-clippy-final.log`, and `/tmp/before-correct-clock-*.log`. No credentials, environment values or connection strings are part of this evidence.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T21:26:00Z
head: 90234d8d2d0a51a654b8ff6947d38372a740acba
branch: claude/happy-davinci-ijz3wy-fix-a3-04-bazaar
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/bazaar/
  - portal-rs/migrations/20261115214000_fix_a3-04-bazaar_reconciliation.sql
proven:
  - BZ3-001, BZ3-002, BZ3-003 and CI-02 confirmed and fixed with failing-before regressions
  - Five complete four-CPU crate runs pass with real PostgreSQL and no skips
  - Formatting and strict Clippy pass
  - Immutable history, existing flags and contract wire bytes preserved
  - Single agent; protected RBAC paths untouched
  - Repository identity Oteryn/Oteryn-Platform and open governing issue 536 verified live
  - Worktree candidate validated on the specified base; commit and push readback belong to final delivery evidence
derived: []
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: All intended before-fix failures resolved in the final candidate
rejected_hypotheses:
  - CI-02 is a product deadlock; the reproducible defect is the fixture using inconsistent source clocks
changed_paths:
  - portal-rs/crates/bazaar/src/game/
  - portal-rs/crates/bazaar/src/moderation.rs
  - portal-rs/crates/bazaar/src/moderation_tests.rs
  - portal-rs/migrations/20261115214000_fix_a3-04-bazaar_reconciliation.sql
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Final candidate formatting check
  - command: cargo clippy --locked --all-targets -p oteryn-bazaar -- -D warnings
    result: PASS
    evidence: /tmp/bazaar-clippy-final.log
  - command: taskset -c 0-3 cargo test --locked -p oteryn-bazaar
    result: PASS
    evidence: Five complete runs, 670 passed and zero failures or skips; durations above
blockers: []
next_action: Publish the owner's exact task branch, retain it for coordinator review, and release worker ownership
```
