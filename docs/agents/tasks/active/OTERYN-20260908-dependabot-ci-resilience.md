---
task_id: OTERYN-20260908-dependabot-ci-resilience
governing_issue: 1362
required_reads: []
search_first:
  - live open PRs/issues/branches touching Dependabot, acceptance, lifecycle, or CodeQL CI
optional_reads: []
---

# OTERYN-20260908-dependabot-ci-resilience

## Goal

Governing GitHub Issue: #1362 / https://github.com/Oteryn/Oteryn-Platform/issues/1362 — canonical lifecycle authority for this task.

Repair the current dependency-PR validation failure classes without weakening protected checks, and add bounded prevention for equivalent stale-base and coupled CodeQL-action failures.

## Acceptance criteria

- [x] Prove each current dependency-PR failure class from LIVE GitHub state/logs.
- [x] Refresh stale Dependabot branches when current protected `main` already contains the applicable harness correction.
- [ ] Keep coupled CodeQL action steps on one compatible pinned revision and configure Dependabot to update them atomically.
- [ ] Add a deterministic fail-closed regression guard for mismatched CodeQL action revisions.
- [ ] Preserve protected checks, branch protection, security assertions and fail-closed behavior.
- [ ] Pass required exact-candidate CI, normal protected integration / Merge Queue, and resulting-main verification.
- [ ] Reconcile and recheck all still-relevant open dependency PRs after the repair.

## Ownership

```yaml
owned_paths:
  - .github/dependabot.yml
  - .github/workflows/ci.yml
  - .github/workflows/codeql.yml
  - composer.json
  - config/support.php
  - tests/ci/test_dependency_update_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-dependabot-ci-resilience.md
modules:
  - dependency-update CI
  - CodeQL workflow consistency
  - PHPStan compatibility for support configuration
dependencies:
  - current protected main
  - open Dependabot PRs #1162 #1250 #1251 #1277 #1278 #1279 #1280 #1281
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T22:47:00Z
head: 1314450ab85d2f559ec761e64370a0e961a9aeb0
branch: fix/dependabot-ci-resilience-1362
pr: 1367
status: validating
context_routes:
  - CI/dependency validation
owned_paths:
  - .github/dependabot.yml
  - .github/workflows/ci.yml
  - .github/workflows/codeql.yml
  - composer.json
  - config/support.php
  - tests/ci/test_dependency_update_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-dependabot-ci-resilience.md
proven:
  - Issue #1362 is open and governs this task.
  - PR #1367 is the single live repair PR for this task.
  - Task branch fix/dependabot-ci-resilience-1362 was created from protected main 3594f5ec9b3262326ba4875a68718ada3f19e666; protected main has since advanced path-disjoint through portal documentation/task closeout to d86b0a43932fd4d8ef67077b39b1f06d3e0528d7.
  - Historical CodeQL Dependabot PRs #1280 and #1281 split init/analyze revisions and produced a CodeQL 4.37.9 versus 4.37.6 configuration mismatch; both remain stale and non-mergeable on old base 85eb4c41d977340e2599006d5cfef271d1e334cf.
  - Upstream github/codeql-action release inventory currently identifies v4.37.9 as the newest v4 action release, pinned by commit cdf488f595d80d6e07e03d4674febd5ab45fa938.
  - Dependabot PRs #1162, #1250, #1251, #1277, #1278 and #1279 were refreshed onto main@102d29241662d45bb810bca65f03386fcb598dad after explicit rebase requests.
  - Refreshed #1162, #1250, #1251, #1278 and #1279 have successful CI and applicable acceptance/lifecycle workflows; the prior acceptance and Complete account lifecycle failures disappeared without weakening or changing those harnesses.
  - Refreshed #1277 isolates a real dependency-specific failure: phpstan/phpstan 2.2.13 rejects config/support.php line 4 because Closure(mixed): list<string> is broader than the inferred list<non-falsy-string> return type.
  - The repair branch narrows that PHPDoc to the already-inferred runtime return type rather than suppressing PHPStan.
  - The repair branch updates CodeQL init/analyze together to cdf488f595d80d6e07e03d4674febd5ab45fa938, groups github/codeql-action/* in Dependabot, and adds a protected-CI contract that requires every CodeQL action reference to use one immutable SHA.
derived:
  - The old shared acceptance/lifecycle red state was stale-base validation debt, not a regression caused by the refreshed Composer dependency versions.
  - PRs #1280 and #1281 should be superseded after the atomic CodeQL update reaches protected main because their single-step updates are structurally unsafe by construction.
unknown:
  - Exact-head result of PR #1367 and its merge-group candidate.
  - Whether any dependency PR gains a new failure after the repair reaches protected main and all still-relevant heads are refreshed again.
conflicts: []
first_failure:
  marker: refreshed PR #1277 runtime-tests / Run static analysis
  evidence: PHPStan 2.2.13 reports varTag.nativeType at config/support.php:4; all prior install/audit/format steps pass
rejected_hypotheses:
  - Current Composer dependency versions inherently break the shared acceptance harness; refreshed #1162 #1250 #1278 #1279 all pass acceptance/lifecycle.
  - The old CodeQL SARIF error is a transient upload problem; the historical log proves incompatible action versions in one workflow execution.
changed_paths:
  - .github/dependabot.yml
  - .github/workflows/ci.yml
  - .github/workflows/codeql.yml
  - composer.json
  - config/support.php
  - tests/ci/test_dependency_update_contract.py
  - docs/agents/tasks/active/OTERYN-20260908-dependabot-ci-resilience.md
validation:
  - command: live GitHub overlap and governing-state inspection
    result: PASS
    evidence: no overlapping writer owns the exact changed paths; open PR #1365 deliberately excludes codeql.yml and changes a disjoint workflow set
  - command: refreshed Dependabot exact-head workflow inspection
    result: PASS
    evidence: #1162 #1250 #1251 #1278 #1279 are green on refreshed heads; #1277 has one isolated PHPStan failure
  - command: upstream github/codeql-action release inspection
    result: PASS
    evidence: current release list reports v4.37.9 as latest v4 release and Dependabot-provided immutable commit is cdf488f595d80d6e07e03d4674febd5ab45fa938
  - command: PR #1367 exact-head GitHub Actions
    result: NOT_RUN
    evidence: new exact head is being produced by this checkpoint reconciliation; CI must run on that resulting head
blockers:
  - none
next_action: Monitor PR #1367 exact-head CI on the reconciled task-packet head, fix any material failure, then integrate only through normal protected Merge Queue flow.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

The acceptance/lifecycle harness was intentionally left unchanged because refreshed dependency candidates prove current main already contains the needed correction. The remaining #1277 failure is handled as a strict static-analysis compatibility fix, not by relaxing the gate.
