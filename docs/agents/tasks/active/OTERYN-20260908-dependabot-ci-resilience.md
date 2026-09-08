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
- [x] Keep coupled CodeQL action steps on one compatible pinned revision and configure Dependabot to update them atomically.
- [x] Add a deterministic fail-closed regression guard for mismatched CodeQL action revisions.
- [x] Preserve protected checks, branch protection, security assertions and fail-closed behavior.
- [ ] Pass required exact-candidate CI, normal protected integration, and resulting-main verification.
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
updated_at: 2026-09-08T22:51:00Z
head: b53e7a0a42bb90598718a0f420c18662963f9144
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
  - Historical CodeQL Dependabot PRs #1280 and #1281 split init/analyze revisions and produced a CodeQL 4.37.9 versus 4.37.6 configuration mismatch; both remain stale single-step updates and are superseded by this atomic repair once it reaches protected main.
  - Upstream github/codeql-action release inventory identifies v4.37.9 as the newest v4 action release, pinned by commit cdf488f595d80d6e07e03d4674febd5ab45fa938.
  - Dependabot PRs #1162, #1250, #1251, #1277, #1278 and #1279 were refreshed onto current-main lineage after explicit rebase requests.
  - Refreshed #1162, #1250, #1251, #1278 and #1279 pass their applicable CI/acceptance/lifecycle workflows; the historical shared acceptance and Complete account lifecycle failures disappeared without weakening or changing those harnesses.
  - Refreshed #1277 isolates a real dependency-specific failure: phpstan/phpstan 2.2.13 rejects config/support.php line 4 because Closure(mixed): list<string> is broader than the inferred list<non-falsy-string> return type.
  - The repair narrows that PHPDoc to the already-inferred runtime return type rather than suppressing PHPStan.
  - The repair updates CodeQL init/analyze together to cdf488f595d80d6e07e03d4674febd5ab45fa938, groups github/codeql-action/* in Dependabot, and adds a protected-CI contract requiring every CodeQL action reference to use one immutable SHA.
  - The first PR #1367 exact-head candidate passed CI, CodeQL, Playwright PHP 8.5, Support Legal, Support Moderation, Edge Security, Game Auth Concurrency, DB Outage and Phase 7 before protected main moved.
  - Protected main advanced path-disjoint through merged PR #1365 to ba5f8ca3c9f99b7947b8aee24883aa304314076b.
  - The task branch was explicitly reconciled with protected main in merge commit b53e7a0a42bb90598718a0f420c18662963f9144; compare state is ahead with behind_by=0 and merge-base exactly ba5f8ca3c9f99b7947b8aee24883aa304314076b.
derived:
  - The old shared acceptance/lifecycle red state was stale-base validation debt, not a regression caused by the refreshed Composer dependency versions.
  - PRs #1280 and #1281 should be closed as superseded after the atomic CodeQL update reaches protected main.
unknown:
  - Exact-head result of the final PR #1367 candidate after current-main reconciliation.
  - Whether any dependency PR gains a new failure after the repair reaches protected main and all still-relevant heads are refreshed again.
conflicts: []
first_failure:
  marker: refreshed PR #1277 runtime-tests / Run static analysis
  evidence: PHPStan 2.2.13 reports varTag.nativeType at config/support.php:4; all prior install/audit/format steps pass
rejected_hypotheses:
  - Current Composer dependency versions inherently break the shared acceptance harness; refreshed #1162 #1250 #1278 #1279 pass acceptance/lifecycle.
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
    evidence: no overlapping live writer owns the exact repair paths; merged PR #1365 changed a disjoint workflow set and was reconciled before final validation
  - command: refreshed Dependabot exact-head workflow inspection
    result: PASS
    evidence: #1162 #1250 #1251 #1278 #1279 are green on refreshed heads; #1277 has one isolated PHPStan failure addressed by this repair
  - command: upstream github/codeql-action release inspection
    result: PASS
    evidence: current release list reports v4.37.9 as latest v4 release and immutable commit cdf488f595d80d6e07e03d4674febd5ab45fa938
  - command: PR #1367 first exact-head GitHub Actions candidate
    result: SUPERSEDED
    evidence: core affected workflows passed, but protected main moved before every broad workflow was terminal; final candidate must rerun after reconciliation
  - command: compare protected main ba5f8ca3c9f99b7947b8aee24883aa304314076b...b53e7a0a42bb90598718a0f420c18662963f9144
    result: PASS
    evidence: behind_by=0 and merge-base equals current protected main
blockers:
  - none
next_action: Monitor the final PR #1367 exact-head workflows after current-main reconciliation, fix any material failure, then use normal protected integration and verify resulting main before reconciling the dependency queue.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

The acceptance/lifecycle harness is intentionally unchanged because refreshed dependency candidates prove current main already contains the needed correction. The remaining #1277 failure is handled as a strict static-analysis compatibility fix, not by relaxing the gate.
