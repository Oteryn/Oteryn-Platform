---
task_id: OTERYN-20260813-immutable-actions-pinning
mode: implementation
branch: ci/immutable-actions-pinning-1008
status: implementing
project_lane: oteryn-platform-core
issue: 1008
pr: 1022
---

# Immutable GitHub Actions dependency pinning

## Goal

Close Issue #1008 by replacing mutable external GitHub Actions `uses:` references with reviewed immutable 40-character commit SHAs, preserving semantic release provenance and Dependabot github-actions support, and adding a fail-closed validator with fixtures.

## Ownership

```yaml
owned_paths:
  - .github/workflows/**
  - .github/dependabot.yml
  - tools/validation/github_actions_pinning.py
  - tools/validation/test_github_actions_pinning.py
  - tests/ci/fixtures/github-actions-pinning/**
  - tests/operations/cloudflare-oteryn-endpoints/check-main-operation-workflow.py
  - docs/agents/tasks/active/OTERYN-20260813-immutable-actions-pinning.md
  - docs/agents/tasks/archive/OTERYN-20260813-immutable-actions-pinning.md
modules:
  - github-actions-ci
  - supply-chain-governance
dependencies:
  - terminal PR #1003
blockers:
  - PR #1024 temporarily owns .github/workflows/build-synology-staging-images.yml; this task will not edit that shared path until #1024 is terminal and current main is refreshed.
cross_repository_tasks: []
```

PR #1003 is merged/terminal and its earlier workflow ownership is released. PR #1024 is the only live overlap observed for this task and is treated as a hard path-level coordination lock for `build-synology-staging-images.yml` only.

## Acceptance

- [x] Inventory every external `uses:` under current-main `.github/workflows/**` and relevant reusable actions/workflows.
- [x] Preserve local `./` references without SHA requirements.
- [x] Resolve every observed mutable dependency from authoritative upstream GitHub tag state without changing its reviewed major version.
- [ ] Pin every external action to a full immutable SHA with human-readable semantic-version comments; all non-overlapping workflow pins are prepared, one shared path waits on PR #1024.
- [x] Keep Dependabot `github-actions` ecosystem enabled; `.github/dependabot.yml` remains unchanged.
- [x] Add deterministic fail-closed validator for mutable tags/branches, short SHAs and malformed external references, including quoted-key and inline-flow bypass coverage.
- [x] Cover valid SHA, tag, branch, short SHA, docker/local/reusable-workflow and malformed forms.
- [x] Wire validator into the unconditional pre-classification section of `.github/workflows/ci.yml`.
- [ ] Exact-head validation, full diff self-review, fresh final review, squash merge, Issue closeout and archive.

## Verified supply-chain mapping

```yaml
actions/checkout: {version: v7.0.1, sha: 3d3c42e5aac5ba805825da76410c181273ba90b1}
actions/upload-artifact: {version: v7.0.1, sha: 043fb46d1a93c77aae656e7c1c64a875d1fc6a0a}
actions/download-artifact: {version: v8.0.1, sha: 3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c}
actions/setup-node: {version: v7.0.0, sha: 820762786026740c76f36085b0efc47a31fe5020}
actions/setup-python: {version: v7.0.0, sha: 5fda3b95a4ea91299a34e894583c3862153e4b97}
actions/setup-go: {version: v7.0.0, sha: b7ad1dad31e06c5925ef5d2fc7ad053ef454303e}
shivammathur/setup-php: {version: 2.37.2, sha: f3e473d116dcccaddc5834248c87452386958240}
docker/setup-buildx-action: {version: v4.2.0, sha: bb05f3f5519dd87d3ba754cc423b652a5edd6d2c}
docker/metadata-action: {version: v6.2.0, sha: dc802804100637a589fabce1cb79ff13a1411302}
docker/login-action: {version: v4.6.0, sha: dbcb813823bdd20940b903addbd779551569679f}
docker/build-push-action: {version: v7.3.0, sha: 53b7df96c91f9c12dcc8a07bcb9ccacbed38856a}
lukka/run-vcpkg: {version: v11.6, sha: b1a0dd252f06b9e25b3c022a9a03bd7a427fb6a2, already_immutable: true}
lukka/run-cmake: {version: v10.9, sha: 5d55ea7949e25f69f0ecb516d8d572297e03a956, already_immutable: true}
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-13T16:52:00+02:00
head: 2f3b9bc1d4f4de7fd800ddc17ecd65c173ae8ec7
branch: ci/immutable-actions-pinning-1008
pr: 1022
status: implementing
phase: implement
execution_mode: github_connector
execution_reason: repository-wide workflow mutation and exact GitHub Actions validation without owner-funded AI
context_routes: [testing, ci-repair]
task_kind: implementation
context_pressure: medium
context_growth: stable
decomposition_decision: phased
validation_level: focused
proven:
  - PR #1003 is terminal and merged.
  - Protected main is 38775e953bd9740df08620482240b483fde69ecc; its post-task-start advancement is documentation-only and is already a parent of the current task head.
  - Initial fail-closed CI inventory found 180 mutable references; deterministic migration found 177 remaining mutable references across 47 workflow files after the first two workflow edits.
  - The migration validator validated the complete transformed inventory and the permanent validator is hardened against quoted-key and inline-flow bypass forms.
  - Dependabot github-actions configuration remains enabled at directory `/` and unchanged.
  - Current head contains every non-overlapping workflow pin and intentionally leaves build-synology-staging-images.yml unchanged while PR #1024 owns it.
  - Temporary migration workflow/helper/generated files are absent from the current PR diff.
conflicts:
  - PR #1024 owns build-synology-staging-images.yml until terminal.
first_failure:
  marker: pull-request validation and trusted push do not both pin checkout
  evidence: Cloudflare Oteryn Endpoint Main Operation run 31712025037 job 94487152299; the workflow successfully executed pinned actions/checkout SHA, then a repository contract test failed because it hard-coded the historical actions/checkout@v7 string.
validation:
  - run_id: 31707680824
    result: EXPECTED_FAIL
    evidence: new validator rejected the pre-migration mutable inventory; fixture tests passed.
  - run_id: 31708212412
    result: PARTIAL_PASS
    evidence: deterministic transform pinned 177 references across 47 files and validator passed 193 uses; branch push was rejected only because GITHUB_TOKEN lacks workflow-write permission.
  - run_id: 31708649280
    result: PASS
    evidence: read-only migration artifact reproduced the transformed workflows and validator pass.
  - run_id: 31712025037
    result: FOCUSED_REPAIR_REQUIRED
    evidence: action checkout by immutable SHA succeeded; only the historical tag-string assertion in check-main-operation-workflow.py failed.
next_action: Update the explicitly claimed Cloudflare workflow contract test to validate two immutable full-SHA checkout references rather than the historical mutable tag string, then inspect the next first relevant partial-head failure while PR #1024 retains the one shared workflow lock.
```

## Safety

CI/repository hardening only. No production deployment, protected-environment approval, secret mutation, live data mutation, or owner-funded AI use is authorized or performed.