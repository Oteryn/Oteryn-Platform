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
  - tests/operations/cloudflare-oteryn-endpoints/check-marker-workflow.py
  - docs/agents/tasks/active/OTERYN-20260813-immutable-actions-pinning.md
  - docs/agents/tasks/archive/OTERYN-20260813-immutable-actions-pinning.md
modules:
  - github-actions-ci
  - supply-chain-governance
dependencies:
  - terminal PR #1003
blockers:
  - PR #1024 currently owns .github/workflows/build-synology-staging-images.yml; this task will not edit that shared path until its live ownership is terminal and released.
cross_repository_tasks: []
```

PR #1003 is merged/terminal and its earlier workflow ownership is released. PR #1024 is the only current live path lock for `build-synology-staging-images.yml`.

## Acceptance

- [x] Inventory every external `uses:` under current-main `.github/workflows/**` and relevant reusable actions/workflows.
- [x] Preserve local `./` references without SHA requirements.
- [x] Resolve every observed mutable dependency from authoritative upstream GitHub tag state without changing its reviewed major version.
- [ ] Pin every external action to a full immutable SHA with human-readable semantic-version comments; all non-overlapping workflow pins are present, one shared path waits on PR #1024.
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
updated_at: 2026-08-13T17:18:00+02:00
head: 79d090ee8cbf57ccc92c518a570fd12cf9997c91
branch: ci/immutable-actions-pinning-1008
pr: 1022
status: implementing
phase: implement
execution_mode: github_connector
execution_reason: repository-wide workflow mutation and exact GitHub Actions validation without owner-funded AI
context_routes:
  - testing
  - ci-repair
task_kind: implementation
context_pressure: medium
context_growth: stable
decomposition_decision: phased
validation_level: focused
owned_paths:
  - .github/workflows/** except the live-owned .github/workflows/build-synology-staging-images.yml
  - .github/dependabot.yml
  - tools/validation/github_actions_pinning.py
  - tools/validation/test_github_actions_pinning.py
  - tests/ci/fixtures/github-actions-pinning/**
  - tests/operations/cloudflare-oteryn-endpoints/check-main-operation-workflow.py
  - tests/operations/cloudflare-oteryn-endpoints/check-marker-workflow.py
  - this task record
blockers:
  - PR #1024 remains live and owns .github/workflows/build-synology-staging-images.yml.
proven:
  - PR #1003 is terminal and merged.
  - Protected main was integrated into this task lineage before the current non-overlapping pin generation.
  - Deterministic migration found and pinned 177 remaining mutable references across 47 workflow files after the first two workflow edits.
  - Dependabot github-actions configuration remains enabled at directory `/` and unchanged.
  - Current task lineage contains every non-overlapping workflow pin and temporary migration/helper/generated files are absent from the PR diff.
  - Cloudflare Oteryn Endpoint Main Operation run 31712699790 passes with immutable checkout pin assertions.
  - tests/operations/cloudflare-oteryn-endpoints/check-marker-workflow.py now requires an immutable full-SHA checkout reference instead of the historical mutable tag literal.
derived:
  - Once PR #1024 releases its single overlapping workflow, only that landed workflow needs a mechanical pin-only edit before final repository-wide validation.
  - The Edge Security Emulation failure on partial head 79d090ee was external Docker Hub infrastructure failure, not an Actions pinning regression, because checkout/setup/upload pinned actions executed successfully before Docker returned HTTP 502 for coredns/coredns:1.12.1.
unknown:
  - terminal exact-head result of the final candidate after the last shared workflow is pinned.
  - final review result after the final candidate exists.
conflicts:
  - live path ownership overlap with PR #1024 on .github/workflows/build-synology-staging-images.yml only.
first_failure:
  marker: pre-migration-mutable-actions
  evidence: CI run 31707680824 correctly failed closed after the new validator detected the repository's pre-migration mutable external uses references.
rejected_hypotheses:
  - local `./` reusable workflow/action references require SHA pinning; they are repository-local and intentionally excluded.
  - docker:// uses values are GitHub action repository references; they are a technically distinct runner form and are classified separately.
  - pinning requires downgrading recent action majors; authoritative tag resolution preserved every reviewed major.
  - Edge Security Emulation failure on partial head was caused by immutable action SHAs; the failure occurred later on a Docker Hub HTTP 502 while pulling CoreDNS.
changed_paths:
  - .github/workflows/** external uses references except live-owned build-synology-staging-images.yml
  - tools/validation/github_actions_pinning.py
  - tools/validation/test_github_actions_pinning.py
  - tests/ci/fixtures/github-actions-pinning/cases.json
  - tests/ci/fixtures/github-actions-pinning/valid-sha.yml
  - tests/operations/cloudflare-oteryn-endpoints/check-main-operation-workflow.py
  - tests/operations/cloudflare-oteryn-endpoints/check-marker-workflow.py
  - this task record
validation:
  - command: CI run 31707680824 pre-migration fail-closed inventory
    result: FAIL
    evidence: fixture tests passed and validator rejected the mutable pre-migration repository inventory as designed.
  - command: Immutable Actions Pin Migration run 31708212412
    result: PASS
    evidence: deterministic transform pinned 177 references across 47 workflows and validated 193 uses references; subsequent branch push was separately rejected by GitHub workflow-write permission policy.
  - command: read-only deterministic migration reproduction run 31708649280
    result: PASS
    evidence: reproduced the same transformed workflow set and validator pass without repository mutation.
  - command: Cloudflare Oteryn Endpoint Main Operation run 31712699790
    result: PASS
    evidence: pinned checkout action executed and the repaired immutable-SHA compatibility contract passed.
  - command: Edge Security Emulation run 31713533471
    result: FAIL
    evidence: immutable checkout/setup-php/upload-artifact actions executed successfully; Docker Hub returned HTTP 502 while pulling coredns/coredns:1.12.1.
  - command: final exact-head repository validation
    result: NOT_RUN
    evidence: final candidate does not exist until PR #1024 releases the one shared workflow path.
next_action: after PR #1024 is terminal and ownership is released, refresh current main, pin only the landed build-synology-staging-images.yml external uses references, synchronize the final candidate, and run exact-final-head validation.
```

## Safety

CI/repository hardening only. No production deployment, protected-environment approval, secret mutation, live data mutation, or owner-funded AI use is authorized or performed.
