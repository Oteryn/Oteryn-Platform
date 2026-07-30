---
task_id: OTERYN-20260730-media-fallback-evidence
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/portal-coverage-manifest.json
search_first:
  - Issue #357, parent #326 and open PRs touching media rendering, previews, thumbnails or fallback behavior
  - rendered views and browser specs that display user-visible images or downloadable media previews
optional_reads:
  - docs/testing/PORTAL_ACCEPTANCE_COVERAGE_MATRIX.md
  - docs/testing/PORTAL_EVIDENCE_DIMENSIONS.json
---

# OTERYN-20260730-media-fallback-evidence

## Goal

Deliver Issue #357 as a bounded fail-closed audit and evidence contract for media-consuming rendered surfaces, without claiming the remaining exhaustive state matrix under parent #326.

## Acceptance criteria

- [ ] Every covered rendered surface is explicitly classified as `media_consumer` or `not_applicable` with rationale.
- [ ] Every applicable normal, missing, broken/integrity-failed and no-image state maps to exact executable evidence.
- [ ] Referenced evidence files, stable markers, Playwright projects and npm profiles exist.
- [ ] Unknown consumers, missing mappings and orphan evidence fail deterministically.
- [ ] Deterministic negative fixtures cover the contract failure modes.
- [ ] Strict Portal Acceptance executes the validator and fixtures.
- [ ] Parent #326 and all production nonclaims remain open.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-media-fallback-evidence.md
  - docs/agents/PROJECT_STATE.md
  - docs/testing/PORTAL_MEDIA_STATE_EVIDENCE.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/validate-portal-media-state-evidence.mjs
  - scripts/acceptance/coverage/test-portal-media-state-evidence.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
modules:
  - Testing
  - AgentGovernance
  - ProductArchitecture
dependencies:
  - Issue #357
  - parent Issue #326
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-30T14:10:00Z
head: eb5736610f4554b196d870d88f4dea2b541db708
branch: test/OTERYN-20260730-media-fallback-evidence
pr: none
status: investigating
context_routes:
  - agent-governance
  - testing
  - architecture
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260730-media-fallback-evidence.md
  - docs/agents/PROJECT_STATE.md
  - docs/testing/PORTAL_MEDIA_STATE_EVIDENCE.json
  - docs/testing/PRODUCT_COMPLETENESS_FRONTEND_AUDIT_2026-07-30.md
  - scripts/acceptance/coverage/validate-portal-media-state-evidence.mjs
  - scripts/acceptance/coverage/test-portal-media-state-evidence.mjs
  - scripts/acceptance/package.json
  - .github/workflows/portal-acceptance-contract.yml
proven:
  - Parent Issue #326 explicitly requires missing/broken media states only on media-consuming surfaces.
  - Current portal coverage metadata records broad states and evidence but does not expose a dedicated fail-closed media-applicability contract.
  - Current main at task start is eb5736610f4554b196d870d88f4dea2b541db708.
derived:
  - A separate applicability ledger can prevent both false completeness claims and artificial media tests on non-media surfaces.
unknown:
  - The authoritative set of rendered media consumers and which fallback states are product-contract applicable to each.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Treat every rendered route as a media consumer.
  - Treat a successful image URL response as proof of visible fallback UX.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260730-media-fallback-evidence.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: discovery task initialized only
blockers:
  - none
next_action: Inventory rendered views, coverage fragments and existing browser markers that consume media, then define the smallest truthful applicability schema before implementation.
```
