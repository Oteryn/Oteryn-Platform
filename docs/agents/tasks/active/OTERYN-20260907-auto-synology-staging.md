---
task_id: OTERYN-20260907-auto-synology-staging
governing_issue: 1313
terminal_pr_policy: archive_pending
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - active Synology deployment tasks and open PR ownership
  - existing trusted-main Synology dispatch patterns
optional_reads: []
---

# OTERYN-20260907-auto-synology-staging

## Goal

Governing GitHub Issue: #1313 — make each runtime-affecting protected-main change automatically available on Synology staging after exact-SHA images pass their build/contract checks, while retaining the existing guarded manual deploy/rollback workflow.

## Acceptance criteria

- [x] Runtime-affecting protected-main changes build exact-SHA Platform/Gateway images and dispatch guarded Synology staging deployment.
- [x] Pull requests never deploy and ordinary main pushes do not publish the privileged deploy-runner.
- [x] Pythonless runner validation and canonical public staging origin are integrated.
- [x] Failed candidates retain fail-closed recovery identity and can only resume/finalize after exact schema/image/world proof.
- [x] Duplicate runtime pull was removed and Gateway dependency readiness now has explicit internal TLS probes.
- [x] Drifted recovery Gateway can be reconstructed to the exact candidate image before health proof.
- [x] Run #22 proved Gateway bindings, both internal TLS legs, identity/readiness/login isolation and MFA renderer are healthy for the previous candidate.
- [x] Run #22 isolated a false-negative MFA asset assertion that matched localized English copy instead of stable semantic markers.
- [ ] PR #1326 passes repository-required exact-head checks and protected integration.
- [ ] The resulting protected-main SHA automatically finalizes the previous candidate, deploys current main, and passes the complete Synology staging health contract.
- [ ] Archive this task only after live proof; close Issue #1313 completed afterward.

## Ownership

```yaml
owned_paths:
  - deploy/synology/scripts/health-check.sh
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
modules:
  - Synology staging deployment
  - staging health contract
  - recovery candidate verification
dependencies:
  - protected-main auto-deploy foundation from PR #1314
  - Pythonless validation from PR #1317
  - canonical public origin from PR #1320
  - resumable deployment and dependency diagnostics from PR #1321
  - candidate finalization from PR #1322
  - candidate Gateway reconstruction from PR #1325
  - approved immutable Canary staging digest
  - synology-staging environment and platform-runners execution path
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T21:53:00Z
head: 8d94ca13360913f6464409cb342b7cfb56788879
branch: fix/20260907-synology-mfa-health-contract
pr: 1326
status: validating
context_routes:
  - testing
  - execution-resources
owned_paths:
  - deploy/synology/scripts/health-check.sh
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
proven:
  - Current protected main is 83bb47493c94d2a7cabbad01c32d41549e3ba664 from PR #1325.
  - Deploy Synology Staging run 34163892737 targeted exact main 83bb47493c94d2a7cabbad01c32d41549e3ba664.
  - Run 34163892737 passed runner tools, GHCR, configuration, immutable image resolution and staging environment creation.
  - The recovery path detected Gateway runtime drift and reconstructed the exact previous-candidate Gateway before health proof.
  - The same run verified Platform/Gateway/Canary bindings, Gateway -> Platform internal TLS, Gateway -> Canary session issuer internal TLS, Gateway identity/readiness, bounded invalid login, private no-store headers, port isolation, MFA QR renderer and protected anonymous MFA route.
  - The run failed immediately after the MFA PHP probe because health-check.sh searched the Blade source for literal English copy `Scan with your authenticator app`.
  - The previous candidate Blade uses localization key `portal.identity.mfa_scan` and already contains `mfa-qr-panel` and `mfa-qr-code`; its CSS contains the `mfa-qr` rules.
  - PR #1326 replaces the locale-specific source grep with stable semantic translation-key/structure checks and preserves the live MFA renderer, protected route and CSS assertions.
  - Temporary validation run 34164599934 passed shell syntax and the focused Synology auto-deploy contract before publication.
derived:
  - Run #22 is a false-negative health-contract failure after the substantive Gateway and MFA runtime probes passed; the prior candidate was not rejected for runtime readiness.
unknown:
  - Whether any later health probe after the corrected MFA asset assertion will expose another live defect; the next protected-main deployment will prove this.
conflicts: []
first_failure:
  marker: MFA_LOCALIZED_COPY_FALSE_NEGATIVE
  evidence: Run 34163892737 printed `MFA QR renderer and protected anonymous route verified.` and then exited before the following QR-first asset success message; the first intervening command grepped a literal English sentence that is absent from the localized Blade source.
rejected_hypotheses:
  - Gateway readiness remains broken; both internal TLS legs, Gateway identity and aggregate readiness passed in run #22.
  - QR-first MFA is absent from the previous candidate; its Blade and CSS contain the stable QR-first structure and renderer test passed live.
  - Remove recovery state manually; rejected because candidate ownership must remain fail-closed until the complete corrected health contract passes.
changed_paths:
  - deploy/synology/scripts/health-check.sh
  - tests/ci/test_synology_auto_staging_deploy.py
  - docs/agents/tasks/active/OTERYN-20260907-auto-synology-staging.md
validation:
  - command: Deploy Synology Staging run 34163892737
    result: FAIL
    evidence: false-negative localized-copy asset assertion after Gateway/TLS/MFA runtime probes passed
  - command: Temporary Synology MFA Health Repair run 34164599934
    result: PASS
    evidence: bash syntax and focused auto-staging contract passed
  - command: repository-hosted PR #1326 exact-head checks
    result: NOT_RUN
    evidence: checks start after this checkpoint publication
blockers:
  - none
next_action: Validate PR #1326. After protected integration, verify the corrected automatic Synology staging deployment reaches full health; then archive this task packet.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository protected repair PR path
source_branch_evidence: governing Issue #1313 and PR #1326
```

## Notes

Automatic deployment applies to runtime-affecting changes and Synology deployment/workflow package changes. Documentation-only commits do not rebuild or redeploy unchanged runtime. Production deployment remains excluded.