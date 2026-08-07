# Main-push CI and Acceptance routing audit — 2026-08-07

## Scope

Audit target: `blakinio/Oteryn-Platform@17f4d5a0de3f029c036df61d326e369cc53bb0ef`.

Bounded domain: post-merge `push` routing for core CI and Acceptance E2E, especially documentation/governance-only commits. Audit mode only; no workflow implementation is changed.

## Authority

Completed baseline Issue #452 requires documentation/task/metadata-only changes not to run unrelated heavy application, browser, container or deployment gates without a proven reason. Completed P0 Issue #467 implemented path-aware routing for pull requests across the five affected validation families and proved docs-only PR heavy jobs skip.

## Primary evidence

### Core CI

`.github/workflows/ci.yml` triggers on every push to `main`. Its classifier uses the pull-request base/head range only for `pull_request`; all other events execute `scripts/ci/classify_changes.py --all`.

The classifier itself maps `docs/agents/**` and ordinary `docs/**` to zero heavy gates, but `--all` deliberately forces every gate. Therefore a docs-only merge that correctly skips PR runtime tests becomes runtime-heavy after merge.

### Acceptance E2E

`.github/workflows/acceptance-validation.yml` uses product-oriented path filters for `pull_request`, but its `push: main` trigger is unconditional. Non-PR runs default to `full`, which provisions runtime services, installs PHP/Node/browser dependencies and executes the full browser baseline plus additional profiles.

The workflow concurrency key includes workflow/ref/run suffix and uses `cancel-in-progress: true`; ordinary main pushes therefore share one direct Acceptance generation.

### Live proof

PR #781 was documentation/governance-only and merged as `f72fafd461f6bd2f41c5a58b975a5532f8e426ef`. The resulting main push started Acceptance run `31162272112`; its job progressed through runtime/service/browser setup and into the full Chromium baseline.

PR #782 was another documentation/governance-only closeout and merged as `17f4d5a0de3f029c036df61d326e369cc53bb0ef`. Its main push emitted Acceptance run `31162564522` and CI run `31162564524`.

At `2026-08-07T08:39:06Z`, run `31162272112` became `cancelled`, immediately after the newer documentation-only main generation entered the same concurrency lane. The defect therefore affects both resource economy and continuity of meaningful post-merge acceptance evidence.

## Material finding

### OPA-GOV-0020 — MEDIUM — PROVEN

Expected: documentation/governance-only changes remain lightweight after merge, while product-risk pushes retain required post-merge validation.

Actual: core CI forces all gates on every main push and Acceptance launches a full profile on every main push. A following docs-only merge can also cancel an in-progress earlier main Acceptance generation.

Durable remediation owner: Issue #783.

## Duplicate analysis

Issue #467 is related but not a duplicate. Its goal and acceptance explicitly concern pull-request change classification and five routed workflow families; its terminal evidence proves docs-only PR skipping. It neither governs Acceptance E2E nor proves path-aware `push: main` behavior.

Searches for main-push Acceptance/docs-only/cancellation routing found no other actionable root-cause Issue.

## Required remediation proof

Issue #783 requires exact main-push diff routing, docs-only suppression for runtime-heavy CI and Acceptance, preservation of product/security/dependency gates, concurrency safety, and deterministic regression tests for ambiguous event ranges and trigger boundaries.

## Validation boundary

The live reproduction was incidental to normal audit closeout merges; no synthetic product/runtime or destructive event was created. Runtime E2E for this documentation-only audit deliverable is `NOT_APPLICABLE`.
