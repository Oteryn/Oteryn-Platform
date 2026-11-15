---
task_id: OTERYN-20261115-a3-11-minigames-gateway
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261115-a3-11-minigames-gateway

Governing workstream: https://github.com/Oteryn/Oteryn-Platform/issues/536 (open).
Owner task: fix W3-01 through W3-05 and CI-01 on the specified audit integration base,
commit and push the dedicated branch; no PR or deployment.

## Ownership

Owned: gateway, minigames, required world/playerdata transaction helpers,
gateway binary/public deadline wiring and Oracle proxy configuration. RBAC-owned files excluded.
Single writer; no agents delegated. No migration required.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T21:27:43Z
head: UNKNOWN
branch: claude/happy-davinci-ijz3wy-fix-a3-11-minigames-gateway
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/gateway/
  - portal-rs/crates/minigames/
  - portal-rs/crates/world/src/store.rs
  - portal-rs/crates/playerdata/src/progress/
  - portal-rs/bin/oteryn-portal/src/gateway.rs
  - portal-rs/bin/oteryn-portal/src/main.rs
  - deploy/oracle/Caddyfile
  - deploy/oracle/public-platform.compose.yml
proven:
  - All five audit defects confirmed on base 90234d8d2d0a51a654b8ff6947d38372a740acba.
  - League advisory IDs were database-wide while their protected state is schema-local; fence and leadership locks now use the resolved league table OID.
derived: []
unknown: []
conflicts: []
first_failure:
  marker: local PG lock-table capacity
  evidence: SQLSTATE 53200 during concurrent migration and schema cleanup; local test service capacity adjusted.
rejected_hypotheses: []
changed_paths:
  - deploy/oracle/Caddyfile
  - deploy/oracle/public-platform.compose.yml
  - docs/agents/tasks/active/OTERYN-20261115-a3-11-minigames-gateway.md
  - portal-rs/bin/oteryn-portal/src/gateway.rs
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/crates/gateway/README.md
  - portal-rs/crates/gateway/src/config.rs
  - portal-rs/crates/gateway/src/golden.rs
  - portal-rs/crates/gateway/src/lib.rs
  - portal-rs/crates/gateway/src/recover.rs
  - portal-rs/crates/gateway/tests/proxies.rs
  - portal-rs/crates/gateway/tests/recover.rs
  - portal-rs/crates/minigames/src/browser.rs
  - portal-rs/crates/minigames/src/collection/README.md
  - portal-rs/crates/minigames/src/collection/http.rs
  - portal-rs/crates/minigames/src/collection/store.rs
  - portal-rs/crates/minigames/src/collection/tests.rs
  - portal-rs/crates/minigames/src/collection_pvp/http.rs
  - portal-rs/crates/minigames/src/collection_pvp/mod.rs
  - portal-rs/crates/minigames/src/league/runtime.rs
  - portal-rs/crates/minigames/src/league/store.rs
  - portal-rs/crates/minigames/src/league/tests.rs
  - portal-rs/crates/minigames/src/lib.rs
  - portal-rs/crates/minigames/src/pass/repository.rs
  - portal-rs/crates/minigames/src/pass/web.rs
  - portal-rs/crates/minigames/src/puzzles/http.rs
  - portal-rs/crates/minigames/src/puzzles/store.rs
  - portal-rs/crates/minigames/src/puzzles/tests.rs
  - portal-rs/crates/playerdata/src/progress/mod.rs
  - portal-rs/crates/playerdata/src/progress/store.rs
  - portal-rs/crates/world/src/store.rs
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Pinned Rust 1.97.0, final Rust candidate.
  - command: cargo clippy --locked --all-targets -p oteryn-gateway -p oteryn-minigames -p oteryn-world -p oteryn-playerdata -p oteryn-portal -- -D warnings
    result: PASS
    evidence: All five crates, no suppressed lints.
  - command: cargo test --locked -p oteryn-gateway -p oteryn-minigames -p oteryn-playerdata -p oteryn-world -p oteryn-portal -- --nocapture
    result: PASS
    evidence: 519 passed, zero failed; one pre-existing Chromium test ignored by default. Gateway 43, minigames 72, playerdata 156, portal 187, world 61. Real PostgreSQL enabled throughout.
  - command: taskset -c 0-3 cargo test --locked -p oteryn-minigames
    result: PASS
    evidence: Five whole-crate default-thread runs; timings below.
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261115-a3-11-minigames-gateway.md --require-checkpoint
    result: PASS
    evidence: Contract v1.
blockers: []
next_action: Coordinator reviews the pushed repair branch and integrates it; no PR requested.
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner requested a pushed repair branch for coordinator integration; no PR.
source_branch_evidence: owner task instruction
```

## Findings and regression evidence

- W3-01: trusted exact proxy peers only; Caddy overwrites the single forwarded
  address. Public login/recover/revoke tests isolate client budgets and reject
  spoofed identities from untrusted peers.
- W3-02: uncertain outcomes promote reservations into the lifetime table; cancelled
  dispatch reservations also retain the 12-hour maximum credential lifetime.
  Definitive rejections reclaim only their pending reservations. The previous
  uncertain-capacity expectation encoded the defect: it now asserts fail-closed
  admission at capacity while retaining the confirmed-budget flood checks.
- W3-03: source/version checks use transaction-held shared locks after account
  waits, including both publication APIs. Collection mutations retain adapter
  ownership fences and stream locks and reread unlock evidence on the mutation
  connection. Read-only adapters refuse mutations by default.
- W3-04: every collection/PvP/pass/puzzle transaction bounds pool admission and
  local lock/statement waits. Module deadlines plus the public composition
  deadline include shared browser middleware and cancellation rollback.
- W3-05: recovery refusal echoes accept only null or the matching string. Both
  recovery issuer paths fail closed for boolean, number, array and object echoes.
- CI-01: database-wide advisory IDs caused unrelated schema fixtures to contend
  and unrelated workers to lose leadership. Resolved-table OID scoping preserves
  synchronization for equivalent search paths while isolating different tables.

Counterchecks restored the old source resolution, 60-second budget/reconciliation
handling, echo parser, unfenced mutation checks, unbounded database waits and
unscoped advisory IDs. All corresponding new regressions failed. The PostgreSQL
countercheck reproduced SQLSTATE 55P03 for independent schemas; all four new SQL
regressions failed (66 passed, four expected failures). Fixed code was restored.

Five default-thread whole-crate runs under `taskset -c 0-3` passed:
70/70 in 51.04 s, 70/70 in 54.39 s, 70/70 in 48.77 s, 71/71 in 50.62 s,
72/72 in 78.24 s as additional fence/deadline coverage was added. Final checks
use the repository-pinned, already-installed Rust 1.97.0; the prepared environment
otherwise defaults to 1.100.0. No tools were installed and no pin changed.

Real PostgreSQL regression timings (including schema migration/cleanup), from
the passing pinned 72-test minigames run:

| Test | Duration |
| --- | ---: |
| postgres_answer_rechecks_source_after_account_wait | 4.102 s |
| postgres_answer_retains_visibility_fence_through_commit | 3.870 s |
| postgres_collection_rechecks_source_and_unlock_after_account_wait | 29.322 s |
| postgres_browser_lock_waits_release_saturated_pool | 5.551 s |
| postgres_independent_schemas_do_not_share_league_fences | 11.192 s |
| postgres_obsolete_fence_still_refuses_forged_admission_metadata | 3.630 s |
| postgres_queued_old_fences_retire_without_quarantine | 2.929 s |
| postgres_withdrawal_replay_and_new_epoch_revision_one | 5.603 s |

The outer shared-middleware deadline test also passes in 3.00 s, including an
invalid language path. No contract bytes, merged migration, activation flag or
RBAC-owned file changed. No PR, deployment, Issue comment or Jira write performed.
Jira synchronization remains coordinator-owned.

Final full pinned command completed successfully: 519 passed, zero failed, one
pre-existing ignored Chromium-only checkout test. No new ignores were added.
The separate final portal command also passed (187 passed, one existing ignored).
Source-composition checks remain intact; the clone boundary is preserved before
applying the new public minigame deadline layer.
