---
task_id: OTERYN-20261013-platform-engine-flip
governing_issue: 1474
required_reads:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
search_first:
  - deploy/synology/scripts/cutover-postgres.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/release-contract.env
optional_reads:
  - docs/operations/PLATFORM_POSTGRES_BOOTSTRAP.md
---

# OTERYN-20261013-platform-engine-flip

## Goal

Governing GitHub Issue: #1474 — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

P6 cutover step 2 (ADR 0046, plan P6, `deploy/synology/scripts/cutover-postgres.sh`): the reviewed change that sets `PLATFORM_DB_ENGINE=postgres` in the release contract, with a deploy gate that keeps ordinary deploys off the stack while the cutover owns it, so the cutover rollback keeps working until native re-enable (step 4).

## Acceptance criteria

- [x] `deploy/synology/release-contract.env` declares `PLATFORM_DB_ENGINE=postgres`; the committed-engine test follows.
- [x] `lib.sh` `oteryn_require_cutover_deploy_window` refuses, read-only and before any stack change: the mariadb engine from the recorded freeze until a verified rollback; the postgres engine without a readable regular `bootstrap-complete.env` holding `BOOTSTRAPPED_AT` and nonempty OAuth and world IDs (the `cutover-postgres.sh` adoption rule), once a cutover rollback started, in the rollback window (bootstrap complete, `REENABLE_STARTED` absent) for any release other than the current one once the frozen MariaDB release (`FREEZE_PLATFORM_IMAGE`) left `current-release.env` or while a candidate of another release is pending, and after `retire-mariadb` while `compose.yml` still defines `mariadb`; and any cutover state that exists but cannot be read or inspected, or a linked cutover or bootstrap state directory (Codex review on PR 1598).
- [x] `deploy.sh` and `deploy-impact.sh` call it before their first compose command; `rollback.sh`, `health-check.sh`, `cutover-postgres.sh` and `bootstrap-postgres.sh` do not.
- [x] README engine-switch paragraph states the merge timing (after the recorded freeze), the rollback revert and the gate.
- [ ] Merged only after the cutover `freeze` step is recorded on staging (owner sequencing; preflight and freeze refuse a postgres contract).

## Ownership

```yaml
owned_paths:
  - deploy/synology/release-contract.env
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/README.md
  - tests/ci/test_synology_postgres_contract.py
  - docs/agents/tasks/active/OTERYN-20261013-platform-engine-flip.md
  - docs/agents/tasks/archive/OTERYN-20261013-platform-engine-flip.md
modules:
  - deploy/synology
dependencies:
  - PR #1591 (P6a) merged and deployed before preflight (plan P6 precondition)
  - ADR 0044 Amendment P4 accepted by the owner (plan P6 precondition; route publication stays off on postgres until then)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

`checkpoint_version` remains structural version 1. Validate with `python tools/agents/checkpoint.py <task-path> --require-checkpoint`.

```yaml
checkpoint_version: 1
updated_at: 2026-10-13T18:00:00Z
head: dbbc9180d8e30c6ad7bf05624bda2090471d0680
branch: claude/platform-engine-flip-20261013
pr: 1598
status: validating
context_routes:
  - deploy
owned_paths:
  - deploy/synology/release-contract.env
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/README.md
  - tests/ci/test_synology_postgres_contract.py
  - docs/agents/tasks/active/OTERYN-20261013-platform-engine-flip.md
proven:
  - cutover-postgres.sh preflight and freeze refuse unless the contract says mariadb; bootstrap, smoke, native-reenable, native-smoke, declare-done and retire-mariadb refuse unless it says postgres; rollback refuses unless it says mariadb again.
  - The deploy workflow's cutover action checks out main HEAD, so the flip must be on main between freeze and bootstrap.
  - A push to main that changes release-contract.env builds a Platform image and dispatches an ordinary staging deploy; without the gate it would migrate the empty PostgreSQL database before the bootstrap (which refuses a non-empty database) or, merged before the freeze, cut over without the gate.
  - The bootstrap runbook and plan step 2 expect a deploy after the bootstrap; the gate lets it pass once bootstrap-complete.env is complete (BOOTSTRAPPED_AT and nonempty OAUTH_CLIENT_ID, OAUTH_RUST_CLIENT_ID, NATIVE_WORLD_ID, last value wins as in the adoption path).
  - Review fix: the gate reads the cutover state once and refuses on a read error, an unsearchable state directory or a non-regular file, and refuses a linked cutover or bootstrap state directory before any marker is read (both cutover scripts refuse linked state directories); an unreadable state is never taken as an absent one. Refusal tests simulate the read error with failing readers on PATH (tests run as root in some sandboxes) and, when not root, with mode 000.
  - Review fix (Codex P1): a deploy of another release copies current-release.env into last-good-release.env (finalizing a pending candidate first), and cutover-postgres.sh rollback runs rollback.sh, which restores that file and refuses a PostgreSQL image. In the rollback window the gate passes only the deploy that replaces the frozen MariaDB release (current PLATFORM_IMAGE equals FREEZE_PLATFORM_IMAGE; plan step 2) and a redeploy of the current release; a missing FREEZE_PLATFORM_IMAGE, an unreadable release record or a pending candidate of another release is refused.
  - No cutover step had been recorded on staging when this branch was cut (recent staging runs were plain deploys).
derived:
  - The flip's own merge-dispatched deploy fails closed at the gate by design; the operator deploys after the bootstrap.
unknown:
  - Whether staging writes PLATFORM_IMAGE identically to the runtime .env and current-release.env (both come from the deploy's PLATFORM_IMAGE); a mismatch refuses the step 2 deploy (fail closed), never the rollback.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Refusing every postgres deploy until native re-enable; rejected because plan step 2 deploys after the bootstrap.
changed_paths:
  - deploy/synology/release-contract.env
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/deploy-impact.sh
  - deploy/synology/README.md
  - tests/ci/test_synology_postgres_contract.py
  - docs/agents/tasks/active/OTERYN-20261013-platform-engine-flip.md
validation:
  - command: python3 tests/ci/test_synology_postgres_contract.py
    result: PASS
    evidence: 73 tests OK (1 skipped as root), including CutoverDeployWindowTest; the new refusal tests fail against the previous gate
  - command: python3 tests/ci/test_synology_cutover_postgres.py
    result: PASS
    evidence: 60 tests OK
  - command: Synology contract suites (rollback, rollback recovery, deploy release identity, auto staging deploy, gateway build inputs, package routing, fresh baseline, platform mTLS, compose runtime, marketplace legacy state, native admission gate and trust publication, native route publication)
    result: PASS
    evidence: all modules OK locally
  - command: bash -n and shellcheck -x on lib.sh, deploy.sh, deploy-impact.sh
    result: PASS
    evidence: no new shellcheck findings against main
  - command: python3 -m pytest -q tests/ci/test_synology_*.py deploy/synology/tests
    result: FAIL
    evidence: 506 passed, 20 skipped (with deploy/synology/tests); test_synology_rollback_contract.py::test_marketplace_enable_and_verify_are_refused_without_transfer_contract fails on 82e15f4 too (committed postgres engine without secrets; pytest-only, the workflow runs the file's main()), outside owned paths
blockers:
  - none
next_action: CP re-requests review on PR 1598; merge only after the cutover freeze is recorded.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository squash merge of PR 1598
source_branch_evidence: pending
```

## Notes

- Merge timing: after `cutover-postgres.sh freeze` is recorded and before `bootstrap`. Merged earlier, preflight and freeze refuse and the merge-dispatched deploy fails at the gate; the fix is a revert.
- Rollback before step 4: a reviewed revert PR of the release-contract line to `mariadb`, then `cutover-postgres.sh rollback`. Its merge-dispatched deploy fails at the gate (freeze recorded, rollback not verified) by design.
- The ADR 0044 Amendment P4 sentence saying deploy.sh refuses the postgres engine is stale since P4b; it is left for the owner's acceptance of the amendment.
