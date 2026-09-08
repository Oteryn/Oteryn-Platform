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

- [ ] Prove each current dependency-PR failure class from LIVE GitHub state/logs.
- [ ] Refresh stale Dependabot branches when current protected `main` already contains the applicable harness correction.
- [ ] Keep coupled CodeQL action steps on one compatible pinned revision and configure Dependabot to update them atomically.
- [ ] Add a deterministic fail-closed regression guard for mismatched CodeQL action revisions.
- [ ] Preserve protected checks, branch protection, security assertions and fail-closed behavior.
- [ ] Pass required exact-candidate CI, normal protected integration / Merge Queue, and resulting-main verification.
- [ ] Reconcile and recheck all still-relevant open dependency PRs after the repair.

## Ownership

```yaml
owned_paths:
  - .github/dependabot.yml
  - .github/workflows/codeql.yml
  - .github/workflows/**
  - scripts/acceptance/**
  - tools/**
  - docs/agents/tasks/active/OTERYN-20260908-dependabot-ci-resilience.md
modules:
  - dependency-update CI
  - acceptance/lifecycle validation harness
  - CodeQL workflow consistency
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
updated_at: 2026-09-08T21:42:00Z
head: 3594f5ec9b3262326ba4875a68718ada3f19e666
branch: fix/dependabot-ci-resilience-1362
pr: none
status: investigating
context_routes:
  - CI/dependency validation
owned_paths:
  - .github/dependabot.yml
  - .github/workflows/codeql.yml
  - .github/workflows/**
  - scripts/acceptance/**
  - tools/**
  - docs/agents/tasks/active/OTERYN-20260908-dependabot-ci-resilience.md
proven:
  - Issue #1362 is open and governs this task.
  - Task branch fix/dependabot-ci-resilience-1362 was created from protected main 3594f5ec9b3262326ba4875a68718ada3f19e666.
  - Historical CodeQL Dependabot PRs #1280 and #1281 split init/analyze revisions and produced a version-configuration mismatch.
derived:
  - Open dependency PRs based on old main revisions may be carrying already-fixed acceptance or lifecycle harness behavior and must be refreshed before product-code changes are justified.
unknown:
  - Exact current first failing assertion for each still-red acceptance/lifecycle dependency PR.
  - Whether current main already fixes every acceptance/lifecycle failure class.
  - Current upstream-compatible CodeQL action revision to integrate.
conflicts: []
first_failure:
  marker: dependency PR validation failures
  evidence: LIVE PR workflow/job logs under Issue #1362 investigation
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260908-dependabot-ci-resilience.md
validation:
  - command: live GitHub overlap and governing-state inspection
    result: PASS
    evidence: no overlapping open repair issue/PR or same-name branch was found before task creation
blockers:
  - none
next_action: Inspect current failing job logs and compare stale dependency PR bases with current protected main before changing runtime or validation behavior.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

Keep package-specific regressions separate from shared harness/configuration failures. Do not make a red dependency PR green by skipping or weakening the failing path.