---
task_id: OTERYN-20260908-dependabot-ci-resilience
governing_issue: 1362
status: completed
project_lane: oteryn-platform-core
execution_mode: github_connector
delivery_pull_request: 1367
delivery_branch: fix/dependabot-ci-resilience-1362
risk: medium
validation_intensity: STANDARD
ownership: released
source_branch_disposition: auto_delete_after_merge
---

# OTERYN-20260908 Dependabot CI resilience — Completed

## Goal

Repair the dependency-PR validation failure classes proven from LIVE GitHub state without weakening protected checks, then leave the still-relevant dependency queue on the resulting protected-main lineage with exact-head green validation.

## Acceptance criteria

- [x] Root causes were proven from LIVE GitHub logs/state rather than inferred from package names.
- [x] Coupled CodeQL `init` and `analyze` updates are pinned to one immutable revision and Dependabot groups the coupled action update.
- [x] Protected CI contains a fail-closed contract that rejects mismatched CodeQL action revisions.
- [x] The PHPStan 2.2.13 incompatibility was repaired by narrowing the existing support-host PHPDoc instead of weakening static analysis.
- [x] Shared acceptance/lifecycle failures were proven to be stale-base validation debt by refreshed dependency candidates; no acceptance/lifecycle assertion was suppressed.
- [x] PR #1367 passed exact-head Agent Governance, CodeQL, runtime and required `platform-gate` validation and integrated through the configured Merge Queue.
- [x] Resulting protected `main` was verified at `5b272a8246ebb050a5b0522d2dce86e7ef3c57ce`.
- [x] Every still-relevant open dependency PR from the governed queue is on that resulting-main base and has a successful exact-head required `platform-gate`.
- [x] The terminal PR #1367 source branch is absent.

## Delivered scope

PR #1367 delivered the bounded repair:

- `.github/dependabot.yml` groups `github/codeql-action/*` updates;
- `.github/workflows/codeql.yml` keeps CodeQL init/analyze on the same immutable v4.37.9 commit;
- protected CI runs the dependency-update contract;
- `tests/ci/test_dependency_update_contract.py` enforces the coupled-action invariant across workflow YAML extensions;
- `config/support.php` uses the already-inferred non-falsy-string list contract required by the refreshed PHPStan candidate;
- no protected check, branch protection, security assertion, acceptance/lifecycle assertion, deployment boundary or production state was weakened.

## Protected integration evidence

```yaml
checkpoint_version: 1
updated_at: 2026-09-09T08:37:00+02:00
head: 5b272a8246ebb050a5b0522d2dce86e7ef3c57ce
branch: fix/dependabot-ci-resilience-1362
pr: 1367
status: completed
context_routes:
  - CI/dependency validation
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260908-dependabot-ci-resilience.md
proven:
  - PR 1367 final source head 6936750c5680b33631b3554665473678ca381e91 passed Agent Governance run 34288346446 and CI run 34288346056, including runtime-tests, aggregate test and required platform-gate job 102269568749.
  - The same exact head passed CodeQL 34288346088, Playwright PHP 8.5 34288346067, Support Legal 34288346072, Support Moderation 34288346096, Edge Security 34288346105, Game Auth 34288346150, DB Outage 34288346135, Phase 7 34288346054, Portal Acceptance Contract 34288346172 and Acceptance E2E 34288346061.
  - PR 1367 merged through the configured protected flow as main 5b272a8246ebb050a5b0522d2dce86e7ef3c57ce.
  - main remains protected and requires platform-gate.
  - source branch fix/dependabot-ci-resilience-1362 is absent after merge.
  - PRs 1280 and 1281 are closed unmerged after the atomic CodeQL repair superseded the unsafe split init/analyze update shape.
  - PRs 1277 and 1278 are closed and are no longer members of the still-relevant open dependency queue.
  - PR 1162 is rebased to resulting main with head 406077a1bbbecdac74093f3e3d509b30ea133840; CI 34289765499 and platform-gate 102274032921 PASS, and all triggered workflows are successful.
  - PR 1250 is rebased to resulting main with head 1823fc261dfa933a28b76ad637330d5201c85490; CI 34289777104 and platform-gate 102274057263 PASS, and all triggered workflows are successful.
  - PR 1251 is rebased to resulting main with head a53f51d1418e219362e715592d20b906246e364d; CI 34289793313 and platform-gate 102274131294 PASS, CodeQL 34289793307 PASS, and all triggered workflows are successful.
  - PR 1279 is rebased to resulting main with head 0e586ffcfaa2763716b9b09d68fd1bba3c9b641f; CI 34289802692 and platform-gate 102274090356 PASS, and all triggered workflows are successful.
derived:
  - The governed post-merge dependency reconciliation is complete for the currently relevant queue.
  - The red Agent Governance result on resulting main and later PR 1371 is lifecycle bookkeeping debt from leaving this terminal PR represented under tasks/active, not a product or dependency-validation regression.
unknown: []
conflicts: []
first_failure:
  marker: post-merge-active-task-liveness
  evidence: Agent Governance on resulting main run 34289171077 and PR 1371 run 34291752555 detect terminal PR 1367 still represented by an active task; this archive is the bounded lifecycle repair.
rejected_hypotheses:
  - Current dependency updates inherently break the shared acceptance harness.
  - CodeQL split init/analyze updates are safe to merge independently.
  - The current Agent Governance failure is caused by PR 1371 implementation changes.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260908-dependabot-ci-resilience.md
  - docs/agents/tasks/archive/OTERYN-20260908-dependabot-ci-resilience.md
validation:
  - command: PR 1367 exact-head GitHub Actions
    result: PASS
    evidence: Agent Governance 34288346446; CI 34288346056; platform-gate 102269568749; all applicable supporting workflows successful
  - command: protected Merge Queue integration
    result: PASS
    evidence: PR 1367 merged as protected main 5b272a8246ebb050a5b0522d2dce86e7ef3c57ce
  - command: post-merge dependency queue reconciliation
    result: PASS
    evidence: still-relevant open PRs 1162, 1250, 1251 and 1279 all target resulting main and have successful exact-head required platform-gate checks
  - command: terminal source branch readback
    result: PASS
    evidence: fix/dependabot-ci-resilience-1362 is absent
blockers: []
next_action: none
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1367 is terminal on protected main and the implementation branch has no continuing ownership purpose
source_branch_evidence: PR #1367 merged as protected main 5b272a8246ebb050a5b0522d2dce86e7ef3c57ce; source branch fix/dependabot-ci-resilience-1362 is absent
```

## Notes

This lifecycle archive releases the completed #1362 implementation ownership. It does not approve or merge any individual dependency PR; each dependency candidate retains its own review and Merge Queue authority.
