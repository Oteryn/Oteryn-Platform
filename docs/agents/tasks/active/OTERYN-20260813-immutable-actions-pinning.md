---
task_id: OTERYN-20260813-immutable-actions-pinning
mode: implementation
branch: ci/immutable-actions-pinning-1008
status: investigating
project_lane: oteryn-platform-core
issue: 1008
---

# Immutable GitHub Actions dependency pinning

## Goal

Close Issue #1008 by replacing mutable external GitHub Actions `uses:` references on current `main` with reviewed immutable 40-character commit SHAs, preserving semantic release provenance and Dependabot github-actions support, and adding a fail-closed validator with fixtures.

## Ownership

```yaml
owned_paths:
  - .github/workflows/**
  - .github/dependabot.yml
  - tools/validation/github_actions_pinning.py
  - tools/validation/test_github_actions_pinning.py
  - tests/ci/fixtures/github-actions-pinning/**
  - docs/agents/tasks/active/OTERYN-20260813-immutable-actions-pinning.md
  - docs/agents/tasks/archive/OTERYN-20260813-immutable-actions-pinning.md
modules:
  - github-actions-ci
  - supply-chain-governance
dependencies:
  - terminal PR #1003
blockers: []
cross_repository_tasks: []
```

PR #1003 is verified merged/terminal before this claim. Open PR #1013 explicitly avoids `.github/workflows/deploy-synology-staging.yml`; PR #1006 owns branch-only temporary Tibia analysis workflows not present on protected `main` and is not modified by this task.

## Acceptance

- [ ] Inventory every external `uses:` under current-main `.github/workflows/**` and relevant reusable actions/workflows.
- [ ] Preserve local `./` references without SHA requirements.
- [ ] Resolve each reviewed semantic version from authoritative upstream GitHub tag/release state and pin to its immutable 40-character commit SHA with version comments.
- [ ] Preserve recent Actions-major versions; no silent downgrade.
- [ ] Keep Dependabot `github-actions` ecosystem enabled and compatible with SHA pins.
- [ ] Add deterministic fail-closed validator for mutable tags/branches, short SHAs and malformed external references.
- [ ] Cover valid SHA, tag, branch, short SHA, docker/local/reusable-workflow forms.
- [ ] Wire validator into an unconditional governance/CI path.
- [ ] Exact-head validation, full diff self-review, independent final review, squash merge, Issue closeout and archive.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T15:51:00+02:00
head: f59227086b9d2a58a37648cd6031e9f653c51b17
branch: ci/immutable-actions-pinning-1008
pr: null
status: investigating
context_routes:
  - testing
  - ci-repair
proven:
  - PR #1003 is merged and closed; its workflow ownership is released by main commit f59227086b9d2a58a37648cd6031e9f653c51b17.
  - Current protected main at task start is f59227086b9d2a58a37648cd6031e9f653c51b17.
  - Issue #1008 remains the implementation authority.
unknown:
  - complete current-main external uses inventory and authoritative tag-to-SHA mapping
conflicts: []
validation: []
next_action: inventory current-main uses references and resolve upstream tag/SHA mappings before mutation
```

## Safety

CI/repository hardening only. No production deployment, protected-environment approval, secret mutation, or live data mutation is authorized.