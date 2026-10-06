---
task_id: OTERYN-20261005-premium-physical-e2e
governing_issue: 1431
required_reads:
  - docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md
  - docs/agents/programs/OTERYN_PLATFORM_COMPLETION_LEAD_PROGRAMME.md
search_first:
  - premium physical E2E
  - premium-snapshots/read
optional_reads:
  - docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
---

# OTERYN-20261005 Premium physical cross-repository E2E

## Goal

Close the remaining non-production physical PREM-P producer/consumer proof for Issue #1431 without activating Premium in production or mutating Oteryn-Game.

The harness runs the real Platform Premium snapshot endpoint behind a real TLS 1.3 mutual-TLS terminator and compiles the real Oteryn-Game `PremiumSnapshotClient` plus `snapshot::validate` from pinned Game merge `0fceef1f2cc311ed21eda96d168e1f8b2b298f83`. The Game checkout is read-only; the Platform workflow copies an ephemeral integration-test driver into that checkout only for the CI run.

## Acceptance criteria

- [x] No fixture/mock response substitutes for the Platform producer.
- [x] No custom HTTP client substitutes for the Game Premium consumer.
- [x] TLS 1.3 mutual TLS is physically negotiated and the exact client certificate subject reaches the Platform trusted-terminator server variables.
- [x] The test design covers disabled `503`, unknown AccountId `404`, `NONE`, operator-test-grant `ACTIVE`, and `REVOKED`.
- [x] Successful snapshots are parsed and validated by Game `snapshot::validate`, including exact Platform `producer_revision`.
- [x] Platform and Game revisions are pinned and emitted in test evidence.
- [x] No production/staging/protected-environment, secret, certificate-issuance or Game-repository mutation is performed.
- [ ] Exact-head workflow passes after the PR is opened.
- [ ] Whole-diff review has no unresolved material finding.

## Ownership

```yaml
owned_paths:
  - .github/workflows/phase7-production-like-validation.yml
  - tests/e2e/premium_physical_e2e/**
  - docs/agents/tasks/active/OTERYN-20261005-premium-physical-e2e.md
modules:
  - ProductsEntitlements
  - cross-repository qualification
dependencies:
  - Oteryn/Oteryn-Platform#1432
  - Oteryn/Oteryn-Platform#1433
  - Oteryn/Oteryn-Game#1774
blockers:
  - GitHub Actions capacity for exact-head validation
cross_repository_tasks:
  - OTV2-PREMIUM-DELIVERY (Game read-only at 0fceef1f2cc311ed21eda96d168e1f8b2b298f83)
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-05T23:20:00+02:00
status: validating
phase: exact_head_validation
branch: test/1431-premium-physical-e2e
head: 883994de56b247750d77e1e870a5d1e59f0750f3
pr: 1462
context_routes:
  - products-entitlements
  - cross-repository-e2e
owned_paths:
  - .github/workflows/phase7-production-like-validation.yml
  - tests/e2e/premium_physical_e2e/**
  - docs/agents/tasks/active/OTERYN-20261005-premium-physical-e2e.md
proven:
  - Platform PREM-P contract and producer are merged through PRs #1432 and #1433.
  - Game PR #1774 merged the real PremiumSnapshotClient path/TLS reconciliation and exact Platform fixture validation as merge 0fceef1f2cc311ed21eda96d168e1f8b2b298f83.
  - Game activation record keeps the real-endpoint non-production run open and explicitly requires 503, 404, NONE, ACTIVE and REVOKED physical cases before activation.
derived:
  - A Platform-owned ephemeral harness can close the physical producer/consumer evidence without any Game write by compiling the pinned public Game library and calling the real private Platform endpoint.
unknown:
  - Exact-head workflow result until a PR is opened and GitHub-hosted runners execute the candidate.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Treat vendored fixture compatibility as physical Platform-to-Game E2E.
  - Inject synthetic SSL headers into Laravel instead of negotiating mutual TLS.
  - Use a custom curl client as a substitute for the Game PremiumSnapshotClient.
changed_paths:
  - .github/workflows/phase7-production-like-validation.yml
  - tests/e2e/premium_physical_e2e/Dockerfile.platform-fpm
  - tests/e2e/premium_physical_e2e/nginx.conf
  - tests/e2e/premium_physical_e2e/control.php
  - tests/e2e/premium_physical_e2e/game_client.rs
  - tests/e2e/premium_physical_e2e/run.sh
  - docs/agents/tasks/active/OTERYN-20261005-premium-physical-e2e.md
validation:
  - command: initial PR #1462 governance/CI validation on head 7b1eade0e2a73e14bf4e4a9f8b5e60b81f31ab2b
    result: FAIL
    evidence: Agent Governance rejected unsupported checkpoint status prepared and omitted PR identity; CI rejected an unregistered 54th workflow against lifecycle budget 53
  - command: workflow-lifecycle remediation on functional head 883994de56b247750d77e1e870a5d1e59f0750f3
    result: PASS
    evidence: dedicated workflow removed; Premium physical job is routed through existing Phase 7 only for classifier class payment; workflow inventory remains bounded at 53
blockers:
  - exact-head repository and physical E2E validation pending on this remediated candidate
next_action: Hold exact head after checkpoint commit, inspect Phase 7 premium-physical-e2e and repository CI; repair only evidence-backed harness failures, then classify READY_FOR_INTEGRATION.
```
