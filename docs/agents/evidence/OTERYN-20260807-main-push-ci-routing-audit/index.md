# OTERYN-20260807 main-push CI routing audit evidence

- Repository: `blakinio/Oteryn-Platform`
- Audited main: `17f4d5a0de3f029c036df61d326e369cc53bb0ef`
- Domain: post-merge CI / Acceptance workflow routing
- Finding: `OPA-GOV-0020`
- Remediation Issue: #783
- Severity: medium
- Confidence: high
- Evidence state: PROVEN

## Static proof chain

1. `.github/workflows/ci.yml` triggers on every main push.
2. Non-PR CI classification calls `scripts/ci/classify_changes.py --all`.
3. `--all` forces every gate despite docs/governance paths otherwise mapping to zero heavy gates.
4. `.github/workflows/acceptance-validation.yml` has an unconditional push-to-main trigger and defaults non-PR execution to `full`.
5. `tests/ci/test_workflow_trigger_economy.py` covers the four heavy PR workflow trigger ignores and basic CI classifier presence, but does not assert main-push diff routing or Acceptance trigger economy.

## Live proof chain

1. Docs-only audit merge `f72fafd461f6bd2f41c5a58b975a5532f8e426ef` started full Acceptance run `31162272112`.
2. The job installed services/dependencies/browsers and entered the full Chromium baseline.
3. Docs-only closeout merge `17f4d5a0de3f029c036df61d326e369cc53bb0ef` emitted Acceptance `31162564522` and CI `31162564524`.
4. Earlier Acceptance `31162272112` then completed as `cancelled` at `2026-08-07T08:39:06Z` because the newer main generation shared the workflow concurrency lane.

## Duplicate boundary

Completed Issue #467 / PR #468 is the accepted P0 pull-request routing implementation. Its scope is PR classification across CI, Phase 7, Edge, DB Outage and Game Auth Concurrency. It does not include Acceptance E2E or main-push path classification.

## Safety boundary

No runtime, workflow, production, staging, repository setting or external-repository mutation was performed by this audit.
