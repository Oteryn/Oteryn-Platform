---
task_id: OTERYN-20260813-immutable-actions-pinning
mode: implementation
branch: ci/immutable-actions-pinning-1008
status: validating
project_lane: oteryn-platform-core
issue: 1008
pr: 1022
---

# Immutable GitHub Actions dependency pinning

## Goal

Close Issue #1008 by pinning external GitHub Actions dependencies to reviewed immutable commit SHAs, preserving Dependabot support, and enforcing the policy with a deterministic validator.

## Acceptance

- [x] Inventory and classify external and local `uses:` references.
- [x] Pin external actions to reviewed full 40-character SHAs without downgrades.
- [x] Preserve semantic-version comments and Dependabot github-actions support.
- [x] Add fail-closed validator and positive/negative fixtures.
- [x] Wire validation into unconditional CI pre-classification.
- [ ] Obtain terminal exact-head CI, perform final review, squash merge, close #1008, and archive this record.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T18:42:00+02:00
head: c799b88d5518e77dbd62cf388fa41cd0698db426
branch: ci/immutable-actions-pinning-1008
pr: 1022
status: validating
context_routes:
  - testing
  - ci-repair
owned_paths:
  - .github/workflows/**
  - .github/dependabot.yml
  - tools/validation/github_actions_pinning.py
  - tools/validation/test_github_actions_pinning.py
  - tests/ci/fixtures/github-actions-pinning/**
  - tests/operations/cloudflare-oteryn-endpoints/check-main-operation-workflow.py
  - tests/operations/cloudflare-oteryn-endpoints/check-marker-workflow.py
  - docs/agents/tasks/active/OTERYN-20260813-immutable-actions-pinning.md
  - docs/agents/tasks/archive/OTERYN-20260813-immutable-actions-pinning.md
proven:
  - PR #1003 and PR #1024 are terminal and merged, so shared workflow ownership is released.
  - The branch contains current main and the final Synology workflow differs from main only by six immutable uses pins.
  - Dependabot github-actions configuration remains enabled and unchanged.
  - The validator and fixtures reject mutable tags, branches, short SHAs and malformed external references while allowing local and distinct docker forms.
derived:
  - Repository-wide validation should report no mutable external uses references once exact-head CI completes.
unknown:
  - Terminal exact-head CI result on the current candidate.
  - Fresh final review result on the final candidate.
conflicts: []
first_failure:
  marker: checkpoint-validation-state
  evidence: Agent Governance run 31721463986 rejected the unsupported validation result RUNNING in the previous checkpoint.
rejected_hypotheses:
  - Local ./ references require external SHA pinning.
  - Pinning requires downgrading current action majors.
changed_paths:
  - .github/workflows/** external uses references
  - tools/validation/github_actions_pinning.py
  - tools/validation/test_github_actions_pinning.py
  - tests/ci/fixtures/github-actions-pinning/**
  - tests/operations/cloudflare-oteryn-endpoints/check-main-operation-workflow.py
  - tests/operations/cloudflare-oteryn-endpoints/check-marker-workflow.py
  - this task record
validation:
  - command: Immutable Actions Pin Migration run 31708212412
    result: PASS
    evidence: Deterministic migration pinned 177 references across 47 workflows and validated 193 uses references.
  - command: read-only deterministic migration reproduction run 31708649280
    result: PASS
    evidence: Reproduced the transformed workflow set and validator pass without repository mutation.
  - command: final exact-head repository validation
    result: NOT_RUN
    evidence: Exact-head workflows are running and have not yet reached a terminal aggregate result.
blockers: []
next_action: obtain terminal exact-head CI, perform final diff review, then squash merge PR #1022 and archive the task record.
```

## Safety

Repository CI hardening only. No deployment, protected-environment change, or secret mutation is part of this task.
