---
task_id: OTERYN-20260908-synology-build-v2
governing_issue: 1328
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - Platform Issue #1328
  - Platform PR #1331
  - Platform PR #1335
  - Platform PR #1336
  - Platform PR #1337
  - Platform PR #1338
optional_reads: []
---

# OTERYN-20260908-synology-build-v2

## Goal

Governing GitHub Issue: #1328 — make Synology staging image builds component-aware and proportional while preserving protected-main identity, immutable per-component provenance, rollback/recovery safety and guarded automatic staging deployment.

## Acceptance criteria

- [x] Pull requests build only images whose real build inputs changed; control-plane changes fail closed.
- [x] Protected-main changes rebuild only changed runtime components and reuse unchanged components only by proven source SHA + immutable digest.
- [x] Overall release identity remains separate from Platform/Gateway component identity.
- [x] Candidate/current/last-good/rollback/recovery state validates explicit per-component provenance.
- [x] Platform, Gateway and deploy-runner Docker contexts are bounded to truthful inputs.
- [x] Deployment-package inputs are separated from runtime-image inputs and Synology documentation is outside the image/deploy trigger set.
- [x] Existing migration/schema/rollback safety remains fail closed.
- [x] Exact-head CI including `platform-gate` and Merge Queue passed for the implementation/proof PRs.
- [x] Deployment-package-only and one-runtime-component protected-main behavior is proven live on Synology staging.
- [x] Performance evidence demonstrates proportional image-job reduction.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260908-synology-build-v2.md
  - deploy/synology/BUILD_ROUTING.md
modules:
  - Synology staging image selection, deployment package, component provenance, release state and rollback/recovery validation
dependencies:
  - Platform Issue #1313 completed with live Synology staging proof
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T08:41:00Z
status: completed
phase: terminal_closeout
last_completed_step: protected-main Case B c2e4ff4d035b50a96c88cf99adfb32d60040e317 deployed successfully to Synology staging in run 34204735730 with one Gateway build and immutable Platform reuse
issue: 1328
branch: ci/1328-synology-gateway-proof
head: c2e4ff4d035b50a96c88cf99adfb32d60040e317
base_sha: 0cd7e76c45736e81cc50db24c6af79ba4a819631
pr: 1338
context_routes:
  - testing
  - execution-resources
  - agent-governance
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260908-synology-build-v2.md
  - deploy/synology/BUILD_ROUTING.md
proven:
  - PR #1331 introduced component-aware build/provenance architecture and integrated through Merge Queue as protected main 48565e8c3a31c1d0349a89e7c7b38118fd4cebba.
  - Protected-main run 34195930772 exposed a provenance fan-in shell defect after successful Platform/Gateway publication; PR #1335 repaired the fan-in and narrowed Gateway image inputs to go.mod plus cmd/** plus internal/**.
  - PR #1335 integrated through Merge Queue as protected main cbe2d4ee618019f1884c3f076133469386198f95; Build Synology run 34197410049 and automatic Deploy Synology run 34197488519 both succeeded.
  - Gateway BuildKit context was reduced from 136.13 kB before truthful bounding to 12.53 kB after PR #1335.
  - PR #1336 removed the broad deploy/synology/** trigger/classifier fallback, separated runtime deployment, operator-only and validation-only inputs, and excluded Synology documentation from image/deploy workflow routing.
  - PR #1336 passed exact-head and Merge Queue qualification and integrated as protected main 1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19.
  - Protected-main Build Synology run 34201292818 at 1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19 passed deployment validation, Platform/Gateway publication and exact-provenance dispatch; ordinary main did not publish deploy-runner. Deploy run 34201364919 then succeeded with rollback skipped.
  - Case A PR #1337 pinned tls-init to an immutable Alpine digest, passed exact-head and Merge Queue qualification, and integrated as protected main 0cd7e76c45736e81cc50db24c6af79ba4a819631.
  - Case A protected-main Build Synology run 34202647423 passed with BUILD_PLATFORM=false and BUILD_GATEWAY=false and allocated zero image build jobs. Its exact-provenance dispatch downloaded no current-run component artifact and selected persisted component reuse.
  - Case A Deploy Synology run 34202722679 validated persisted release lineage and both OCI revisions, reused Platform source 1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19 at ghcr.io/oteryn/oteryn-platform@sha256:9a84fa634f33c8ae2cc3a0938da16c2d155c70d3b6cea28205f880d0a6bee0df and Gateway source 1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19 at ghcr.io/oteryn/oteryn-game-gateway@sha256:a91d9e55328e75628a5e7e9433a369c96d2a16302a21add87758a138b6603347, passed the full staging health contract and skipped rollback.
  - Case B PR #1338 pinned the already observed Gateway base-image digests and changed no Platform or deploy-runner image input. Exact-head Build Synology run 34204088536 allocated exactly one game-gateway image job; Agent Governance, CodeQL, DB outage, Edge, Game Auth, full CI platform-gate and Phase 7 all passed.
  - PR #1338 integrated through Merge Queue as protected main c2e4ff4d035b50a96c88cf99adfb32d60040e317.
  - Case B protected-main Build Synology run 34204674858 allocated exactly one game-gateway build, no Platform or deploy-runner build, and published Gateway ghcr.io/oteryn/oteryn-game-gateway@sha256:7532c17f31eb314c03a1e6f0a3c0dff5a59c79314080c7168f2eafd4adad73ab from source c2e4ff4d035b50a96c88cf99adfb32d60040e317.
  - Case B dispatch explicitly sent PLATFORM_CHANGED=false and GATEWAY_CHANGED=true with the exact new Gateway source/digest pair.
  - Case B Deploy Synology run 34204735730 validated persisted release lineage and OCI revisions, reused Platform source 1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19 at ghcr.io/oteryn/oteryn-platform@sha256:9a84fa634f33c8ae2cc3a0938da16c2d155c70d3b6cea28205f880d0a6bee0df, deployed the new Gateway digest, verified loopback/private bindings, both internal TLS dependencies, Gateway identity/readiness, MFA QR, canonical origins, LAN endpoint and World Registry, emitted Oteryn Synology staging deployment is healthy, and skipped rollback.
  - The former fixed workflow allocated three image jobs. Case A reduced allocation 3 to 0 (100 percent); Case B reduced allocation 3 to 1 (66.7 percent).
  - Post-merge Agent Governance run 34204674854 failed only because the now-terminal PR #1338 was still represented by the active task packet; checkpoint schema, central policy, governing-Issue validation and source-branch closeout validation all passed. This archive transition removes that stale active ownership.
  - Repair Synology Autostart run 34202647284 exposed a separate missing oteryn-deploy-runner/runner container assumption before checking staging services. No causal link to component routing was proven, and the separate follow-up is tracked as Issue #1339.
derived:
  - Component build allocation is now proportional to truthful inputs while staging release identity remains exact protected-main and unchanged components cannot be reused without persisted immutable provenance.
  - Deployment-package-only protected-main reconciliation can advance the overall release with zero runtime image builds while preserving per-component source identity.
  - Gateway-only protected-main changes build one Gateway image and combine its current-run provenance with persisted Platform provenance before deployment.
  - Synology documentation can be changed independently without entering the image/deploy workflow because the former broad deployment-root trigger has been removed.
unknown: []
conflicts: []
first_failure:
  marker: historical-synology-component-routing-and-provenance-defects
  evidence: the early protected-main fan-in defect and broad deployment-root invalidation were repaired by PRs #1335 and #1336 and superseded by successful Case A and Case B live evidence.
rejected_hypotheses:
  - Deployment-package-only protected-main changes require rebuilding Platform and Gateway; Case A proved zero image allocation plus immutable reuse.
  - A Gateway-only change requires rebuilding Platform; Case B proved exactly one Gateway build plus persisted Platform reuse.
  - Synology documentation is a runtime deployment input because it lives under deploy/synology; PR #1336 replaced the broad root trigger with explicit runtime/operator/validation path sets.
  - The Gateway base-image digests used in Case B were guessed from mutable tags; the exact digests were first observed and recorded in successful protected-main BuildKit provenance.
  - The Repair Synology Autostart failure was caused by Case A or Case B; it failed at independent deploy-runner container discovery before staging services were inspected and is tracked separately as #1339.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260908-synology-build-v2.md
  - docs/agents/tasks/archive/OTERYN-20260908-synology-build-v2.md
  - deploy/synology/BUILD_ROUTING.md
validation:
  - command: PR #1338 exact-head Build Synology Staging Images run 34204088536
    result: PASS
    evidence: exactly one game-gateway image job; no Platform or deploy-runner build; PR staging dispatch skipped
  - command: PR #1338 exact-head required CI and Phase 7
    result: PASS
    evidence: platform-gate and Phase 7 completed successfully before Merge Queue integration
  - command: protected-main Build Synology Staging Images run 34202647423
    result: PASS
    evidence: Case A allocated zero image build jobs and dispatched both runtime components by persisted immutable reuse
  - command: Deploy Synology Staging run 34202722679
    result: PASS
    evidence: Case A reused exact persisted Platform/Gateway source SHA + digest pairs and ended healthy with rollback skipped
  - command: protected-main Build Synology Staging Images run 34204674858
    result: PASS
    evidence: Case B allocated exactly one Gateway build, published exact Gateway provenance, allocated no Platform/deploy-runner build and dispatched exact main
  - command: Deploy Synology Staging run 34204735730
    result: PASS
    evidence: Case B reused persisted immutable Platform, deployed new immutable Gateway, passed full staging health and skipped rollback
blockers: []
next_action: none
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: archive-only closeout after protected-main zero-build and one-component live staging proofs
source_branch_evidence: Issue #1328; Build runs 34202647423 and 34204674858; Deploy runs 34202722679 and 34204735730
```

## Notes

The final component-routing runtime proven on Synology staging is protected `main@c2e4ff4d035b50a96c88cf99adfb32d60040e317`. This closeout branch is intentionally documentation-only. Production deployment remains excluded. The unrelated autostart runner-topology finding continues separately in Issue #1339.
