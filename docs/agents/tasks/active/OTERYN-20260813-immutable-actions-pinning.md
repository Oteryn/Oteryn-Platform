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
  - tools/validation/apply_github_actions_pins.py
  - tests/ci/fixtures/github-actions-pinning/**
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
- [x] Add deterministic fail-closed validator for mutable tags/branches, short SHAs and malformed external references.
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
updated_at: 2026-08-13T16:43:00+02:00
head: 2b46f047fb8964e5522028122d5a67c626f1e3e9
branch: ci/immutable-actions-pinning-1008
pr: 1022
status: implementing
context_routes: [testing, ci-repair]
proven:
  - PR #1003 is terminal and merged.
  - Protected main advanced to 38775e953bd9740df08620482240b483fde69ecc via architecture-doc-only changes; no workflow changed in that main advancement.
  - Initial fail-closed CI inventory found 180 mutable references; after the two already-pinned workflow files, deterministic migration found 177 remaining mutable references across 47 workflow files.
  - The migration validator passed 5 fixture tests and validated 193 total uses references after deterministic transformation.
  - Local reusable workflow references and existing lukka full-SHA references are intentionally preserved.
  - Dependabot github-actions configuration remains enabled at directory `/`.
  - PR #1024 currently overlaps only .github/workflows/build-synology-staging-images.yml within this task's workflow inventory.
unknown: []
conflicts:
  - PR #1024 owns build-synology-staging-images.yml until it becomes terminal; no mutation of that path is permitted meanwhile.
validation:
  - run_id: 31707680824
    result: EXPECTED_FAIL
    evidence: new validator rejected the pre-migration mutable inventory; its fixture tests passed.
  - run_id: 31708212412
    result: PARTIAL_PASS
    evidence: deterministic transform pinned 177 references across 47 files and validator passed 193 uses; branch push was rejected only because GITHUB_TOKEN lacks workflow-write permission.
  - run_id: 31708649280
    result: PASS
    evidence: read-only migration artifact reproduced the same transformed workflows and validator pass.
next_action: Materialize content-addressed transformed workflow blobs without editing shared workflow paths, then assemble a current-main-based Git tree for every non-overlapping pin while waiting for PR #1024 to release the final shared path.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: 20260813T1643+0200
  session_started_at: 2026-08-13T16:27:00+02:00
  checkpointed_at: 2026-08-13T16:43:00+02:00
  last_progress_at: 2026-08-13T16:41:18+02:00
  phase: materialize_non_overlapping_pins
  exact_head: 2b46f047fb8964e5522028122d5a67c626f1e3e9
  pull_request: 1022
  active_operation: Immutable Actions Pin Migration content-addressed blob materialization
  external_run_ids: [31711349327]
  operation_started_at: 2026-08-13T16:41:21+02:00
  wait_deadline_at: 2026-08-13T17:01:21+02:00
  check_generation: materialization-helper
  checks_used: 1
  status: active
  safe_to_resume: true
  resume_condition: workflow run 31711349327 is terminal
  next_action: Inspect run 31711349327 once terminal; if successful, read the generated ordinary-file blob SHAs and assemble the non-overlapping pinned workflow tree without touching build-synology-staging-images.yml.
```

## Safety

CI/repository hardening only. No production deployment, protected-environment approval, secret mutation, live data mutation, or owner-funded AI use is authorized or performed.