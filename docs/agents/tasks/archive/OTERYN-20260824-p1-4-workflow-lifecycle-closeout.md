---
task_id: OTERYN-20260824-p1-4-workflow-lifecycle-closeout
issue: 1255
status: completed
project_lane: oteryn-platform-core
execution_mode: remote_terminal_github_connector
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/AGENTS.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
search_first:
  - Issue #1255
  - current protected main workflow inventory
  - active PR workflow ownership
---

# OTERYN-20260824 P1.4 workflow lifecycle closeout

## Goal

Close only organization audit v3.9 P1.4 by evidence-classifying the current Platform workflow surface and retiring only provably safe obsolete/duplicate/superseded/migration-only workflows.
## Acceptance criteria

- [x] Every workflow on the task-start `main` ref has one P1.4 lifecycle category with evidence.
- [x] Required-check, caller, trigger, and open-ownership contracts are inspected before any workflow mutation.
- [x] Only proven-safe P1.4 workflow retirement/consolidation is performed; uncertain workflows stay kept/UNKNOWN.
- [x] Final diff is limited to `.github/workflows/**` plus this single task record.
- [x] Exact-head workflow validation and protected required checks pass before squash merge.
- [x] Post-merge `main` re-inventory and Issue #1255 reconciliation are terminal live-state actions recorded in Issue/PR evidence immediately after this archive candidate merges.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260824-p1-4-workflow-lifecycle-closeout.md
modules:
  - ci-workflow-lifecycle
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

Workflow files remain discovery-only until a non-overlapping proven-safe retirement candidate is established.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-24T11:53:37Z
head: 0dcace20e6fd59bdf663a0f12c658190f364b77e
branch: audit/issue-1255-p1-4-workflow-lifecycle
pr: 1256
status: completed
context_routes:
  - testing
  - ci-repair
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260824-p1-4-workflow-lifecycle-closeout.md
proven:
  - protected task-start main is d0ffc93855cba744ca5dc654651f528c962970aa
  - task-start main contains exactly 55 workflow YAML files
  - live branch protection requires status context platform-gate
  - open PRs own codeql.yml, build-synology-staging-images.yml, and repair-synology-autostart.yml
  - CI lifecycle registry budget equals the 55-file current inventory and records six previously retired workflow names
  - Issue #1085 previously retired six proven obsolete wrappers while preserving current proving coverage
  - no active main task record claims .github/workflows paths
  - Issue #1255 is the single tracking Issue for this task
  - all 55 current workflows now have one evidence-backed P1.4 lifecycle category and KEEP disposition
  - exact duplicate scan found no byte-identical current workflow definitions
  - no current workflow is proven safe to retire; no workflow mutation is justified
  - exact classification head 0dcace20e6fd59bdf663a0f12c658190f364b77e changes no .github/workflows path and maps exactly 55 unique current workflow paths
  - CI run 32723828907 passed on exact classification head and produced protected required check platform-gate PASS
  - Agent Governance run 32723828899 failed only on pre-existing task OTERYN-20260823-platform-transfer-terminal-reconciliation because terminal PR #1243 remains represented as active without archive-pending transition
  - live classic branch protection requires only platform-gate; rulesets are empty and required approving review count is zero
derived:
  - P1.4 is HEIGHTENED validation because CI/check lifecycle is externally visible
  - workflow mutation must not touch files owned by current open PRs
unknown: []
conflicts: []
first_failure:
  marker: OUT_OF_SCOPE Agent Governance liveness failure for terminal PR #1243
  evidence: run 32723828899; task OTERYN-20260823-platform-transfer-terminal-reconciliation is outside P1.4 and was not modified
rejected_hypotheses:
  - similarity or age alone is sufficient evidence for workflow removal
changed_paths:
  - docs/agents/tasks/archive/OTERYN-20260824-p1-4-workflow-lifecycle-closeout.md
validation:
  - command: python tools/validation/workflow_inventory.py
    result: PASS
    evidence: Classified 55 workflows; lifecycle actual=55 budget=55 retired=6 manual_only=0
  - command: python tools/validation/test_workflow_inventory.py
    result: PASS
    evidence: 18 tests passed
  - command: python tests/ci/test_workflow_trigger_economy.py
    result: PASS
    evidence: storage hygiene and workflow trigger economy contracts PASS
  - command: python tests/ci/test_stable_platform_gate_policy.py
    result: PASS
    evidence: exit 0
  - command: python tests/ci/test_required_test_gate.py
    result: PASS
    evidence: 12 tests passed
  - command: python tools/agents/test_terminal_branch_reusable.py
    result: PASS
    evidence: 5 tests passed
  - command: exact-head classification/path-set self-review
    result: PASS
    evidence: 55 rows, 55 unique paths, exact match to current inventory, no UNKNOWN, no non-KEEP, no workflow diff
  - command: protected required PR check on 0dcace20e6fd59bdf663a0f12c658190f364b77e
    result: PASS
    evidence: CI run 32723828907; platform-gate PASS
  - command: Agent Governance on 0dcace20e6fd59bdf663a0f12c658190f364b77e
    result: BLOCKED
    evidence: run 32723828899 failed on unrelated terminal PR #1243 active-record liveness; not a required protected check and no P1.4 repair authorized
  - command: product/runtime E2E
    result: NOT_APPLICABLE
    evidence: P1.4 makes no workflow or product/runtime behavior changes; GitHub workflow lifecycle validation is the affected integration surface
blockers:
  - none
next_action: merge PR #1256 after the archive-only final head reproduces protected platform-gate PASS; then verify main inventory, source-branch deletion, and close Issue #1255
```

## Workflow classification evidence

Baseline: protected `main` `d0ffc93855cba744ca5dc654651f528c962970aa`; 55/55 workflows classified. Live classic protection requires only `platform-gate`; live rulesets: none. Disposition is `KEEP` for every current workflow because no current definition is proven duplicate/superseded/obsolete with safe removable coverage.

| Path | Workflow name | Triggers | Purpose / jobs | Caller / required-check evidence | Category | Disposition | Decision evidence |
|---|---|---|---|---|---|---|---|
| .github/workflows/acceptance-soak.yml | Acceptance E2E Public Soak | schedule, workflow_dispatch | public-soak | not named by live protection | SCHEDULED_MAINTENANCE | KEEP | registered current lifecycle; `schedule,workflow_dispatch`; distinct jobs: public-soak |
| .github/workflows/acceptance-stability.yml | Acceptance E2E Stability Repeat | schedule, workflow_dispatch | repeated-critical | not named by live protection | SCHEDULED_MAINTENANCE | KEEP | registered current lifecycle; `schedule,workflow_dispatch`; distinct jobs: repeated-critical |
| .github/workflows/acceptance-validation.yml | Acceptance E2E and Visual UX | pull_request, push, workflow_dispatch, workflow_call | acceptance | not named by live protection; called by `acceptance-soak.yml`, `acceptance-stability.yml` | REUSABLE_WORKFLOW | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch,workflow_call`; distinct jobs: acceptance |
| .github/workflows/agent-governance.yml | Agent Governance | pull_request, push, workflow_dispatch | checkpoint-validation | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: checkpoint-validation |
| .github/workflows/announcements-acceptance.yml | Announcements Acceptance | pull_request, push, workflow_dispatch | Complete Announcements lifecycle | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: Complete Announcements lifecycle |
| .github/workflows/branch-lifecycle.yml | Branch Lifecycle | pull_request, push, workflow_dispatch | validate; live-dry-run; apply-reviewed-manifest | not named by live protection | MANUAL_OPERATION | KEEP | ADR 0037 preserves Branch Lifecycle as merged-source fallback while terminal lifecycle adds closed-unmerged cleanup; not a replacement pair |
| .github/workflows/build-synology-staging-images.yml | Build Synology Staging Images | pull_request, push, workflow_dispatch | Validate Synology deployment package; Build ${{ matrix.name }} image | not named by live protection | DEPLOYMENT | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: Validate Synology deployment package; Build ${{ matrix.name }} image; current open PR ownership also forbids mutation in this task |
| .github/workflows/character-bazaar-staging-control.yml | Character Bazaar Staging Control | push, workflow_dispatch | control | not named by live protection | DEPLOYMENT | KEEP | registered current lifecycle; `push,workflow_dispatch`; distinct jobs: control |
| .github/workflows/character-bazaar-staging-validation.yml | Character Bazaar Staging Validation | pull_request, push, workflow_dispatch | Validate Character Bazaar staging package | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: Validate Character Bazaar staging package |
| .github/workflows/ci.yml | CI | push, pull_request | classify-changes; runtime-tests; php-coverage-report; test; platform-gate | branch protection `platform-gate` | REQUIRED_PR_GATE | KEEP | produces exact live required status `platform-gate`; removal/rename forbidden by classic protection; rulesets list is empty |
| .github/workflows/cloudflare-oteryn-edge-audit.yml | Cloudflare Oteryn Edge Audit | pull_request, pull_request_target | validate; live-audit | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,pull_request_target`; distinct jobs: validate; live-audit |
| .github/workflows/cloudflare-oteryn-endpoint-main-operation.yml | Cloudflare Oteryn Endpoint Main Operation | pull_request, push | validate; operate | not named by live protection | DEPLOYMENT | KEEP | registered current lifecycle; `pull_request,push`; distinct jobs: validate; operate |
| .github/workflows/cloudflare-oteryn-endpoints.yml | Cloudflare Oteryn Endpoints | pull_request, pull_request_target, issue_comment, workflow_dispatch | validate; manage; marker-manage; comment-manage | not named by live protection | MANUAL_OPERATION | KEEP | registered current lifecycle; `pull_request,pull_request_target,issue_comment,workflow_dispatch`; distinct jobs: validate; manage; marker-manage; comment-manage |
| .github/workflows/cloudflare-oteryn-hsts-stage1.yml | Cloudflare Oteryn HSTS Stage 1 | pull_request, push | validate; operate | not named by live protection | MIGRATION_TEMPORARY | KEEP | current HSTS stage-1 apply/audit/rollback contract remains active; later longer-duration promotion is only a future possibility |
| .github/workflows/cloudflare-oteryn-public-edge-repair.yml | Cloudflare Oteryn Public Edge Repair | pull_request, push | validate; operate | not named by live protection | MANUAL_OPERATION | KEEP | registered current lifecycle; `pull_request,push`; distinct jobs: validate; operate |
| .github/workflows/cloudflare-zone-edge-audit.yml | Cloudflare Zone Edge Audit | pull_request, push, workflow_dispatch | validate; audit | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: validate; audit |
| .github/workflows/codeql.yml | CodeQL | push, pull_request, schedule, workflow_dispatch | analyze (${{ matrix.language }}) | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `push,pull_request,schedule,workflow_dispatch`; distinct jobs: analyze (${{ matrix.language }}); current open PR ownership also forbids mutation in this task |
| .github/workflows/community-data-acceptance.yml | Community Data Acceptance | pull_request, push, workflow_dispatch | Complete Community Data lifecycle | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: Complete Community Data lifecycle |
| .github/workflows/content-scale-acceptance.yml | Content Scale Acceptance | pull_request, push, workflow_dispatch | Long bilingual content matrix | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: Long bilingual content matrix |
| .github/workflows/deploy-synology-staging.yml | Deploy Synology Staging | workflow_dispatch | deploy | not named by live protection | DEPLOYMENT | KEEP | registered current lifecycle; `workflow_dispatch`; distinct jobs: deploy |
| .github/workflows/downloads-acceptance.yml | Downloads Acceptance | pull_request, push, workflow_dispatch | Complete Downloads lifecycle | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: Complete Downloads lifecycle |
| .github/workflows/edge-security-emulation.yml | Edge Security Emulation | pull_request, workflow_dispatch | classify-changes; validate | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,workflow_dispatch`; distinct jobs: classify-changes; validate |
| .github/workflows/editorial-media-acceptance.yml | Editorial Media Acceptance | pull_request, push, workflow_dispatch | Complete Editorial Media lifecycle | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: Complete Editorial Media lifecycle |
| .github/workflows/error-state-acceptance.yml | Error State Acceptance | pull_request, push, workflow_dispatch | Localized global error lifecycle | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: Localized global error lifecycle |
| .github/workflows/events-acceptance.yml | Events Acceptance | pull_request, push, workflow_dispatch | Complete Events lifecycle | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: Complete Events lifecycle |
| .github/workflows/game-auth-ticket-concurrency.yml | Game Auth Ticket Concurrency | pull_request | classify-changes; concurrency-proof | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request`; distinct jobs: classify-changes; concurrency-proof |
| .github/workflows/game-catalog-contract.yml | Game Catalog Contract | workflow_dispatch, push, pull_request | validate-contract; static-analysis; Canary staging import activation rollback | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `workflow_dispatch,push,pull_request`; distinct jobs: validate-contract; static-analysis; Canary staging import activation rollback |
| .github/workflows/game-gateway-ci.yml | Game Gateway CI | pull_request, push | test-build | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push`; distinct jobs: test-build |
| .github/workflows/github-actions-storage-hygiene.yml | GitHub Actions Storage Hygiene | pull_request, pull_request_target, schedule, workflow_dispatch | static-validation; closed-pr-cache-cleanup; maintenance | not named by live protection | SCHEDULED_MAINTENANCE | KEEP | registered current lifecycle; `pull_request,pull_request_target,schedule,workflow_dispatch`; distinct jobs: static-validation; closed-pr-cache-cleanup; maintenance |
| .github/workflows/historical-branch-audit.yml | Historical Branch Audit | pull_request, pull_request_target, push, schedule, workflow_dispatch | validate; live-dry-run; steady-state-read-only; apply-reviewed-historical-reconciliation | not named by live protection | SCHEDULED_MAINTENANCE | KEEP | registered current lifecycle; `pull_request,pull_request_target,push,schedule,workflow_dispatch`; distinct jobs: validate; live-dry-run; steady-state-read-only; apply-reviewed-historical-reconciliation |
| .github/workflows/native-auth-canary-cache-build.yml | Native Auth Canary Cache Header Build | pull_request, workflow_dispatch | Build exact Canary cache-header revision | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,workflow_dispatch`; distinct jobs: Build exact Canary cache-header revision |
| .github/workflows/native-auth-ephemeral-cutover-rehearsal.yml | Native Auth Ephemeral Cutover Rehearsal | workflow_dispatch, pull_request | Build exact Game Gateway revision; Build exact Canary native-auth revision; Build exact controlled OTClient revision; Full ephemeral production-like native-auth cutover; Required native-auth ephemeral cutover rehearsal | not named by live protection | MIGRATION_TEMPORARY | KEEP | active native-auth production-verification record keeps legacy compatibility Track A conditional; lifecycle is migration/compatibility and is not proven finished |
| .github/workflows/native-protocol-contract-audits.yml | Native protocol contract audits | pull_request, push | Audit 1 - architecture and boundary; Audit 2 - security auth replay downgrade; Audit 3 - parser schema and limits; Audit 4 - tests CI and Canary regression boundary; Audit 5 - integration rollout and rollback | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push`; distinct jobs: Audit 1 - architecture and boundary; Audit 2 - security auth replay downgrade; Audit 3 - parser schema and limits; Audit 4 - tests CI and Canary regression boundary; Audit 5 - integration rollout and rollback |
| .github/workflows/native-protocol-contract.yml | Native protocol contract | pull_request, push | validate | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push`; distinct jobs: validate |
| .github/workflows/oteryn-public-edge-validation.yml | Oteryn Public Edge Validation | pull_request, push | validate; observe | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push`; distinct jobs: validate; observe |
| .github/workflows/parallel-coordinator-prompt-eval.yml | Parallel Coordinator Prompt Eval | pull_request, push, workflow_dispatch | prompt-eval | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: prompt-eval |
| .github/workflows/phase7-production-like-validation.yml | Phase 7 Production-Like Validation | pull_request, workflow_dispatch | classify-changes; validate | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,workflow_dispatch`; distinct jobs: classify-changes; validate |
| .github/workflows/platform-db-outage-validation.yml | Platform DB Outage Validation | pull_request, workflow_dispatch | classify-changes; validate | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,workflow_dispatch`; distinct jobs: classify-changes; validate |
| .github/workflows/playwright-runtime-validation.yml | Playwright PHP 8.5 Runtime | pull_request, push, workflow_dispatch | validate; Refresh Synology Playwright cache | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: validate; Refresh Synology Playwright cache |
| .github/workflows/portal-acceptance-contract.yml | Portal Acceptance Contract | pull_request, push, workflow_dispatch | Strict portal coverage closure; Complete account lifecycle | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: Strict portal coverage closure; Complete account lifecycle |
| .github/workflows/portal-e2e-audit.yml | Portal E2E Audit | pull_request, workflow_dispatch | Exact-head comprehensive portal audit | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,workflow_dispatch`; distinct jobs: Exact-head comprehensive portal audit |
| .github/workflows/recover-synology-staging-schema.yml | Recover Synology Staging Schema | workflow_dispatch | recover | not named by live protection | MANUAL_OPERATION | KEEP | registered current lifecycle; `workflow_dispatch`; distinct jobs: recover |
| .github/workflows/repair-synology-autostart.yml | Repair Synology Autostart | push, workflow_dispatch | Enforce Docker restart policies | not named by live protection | MANUAL_OPERATION | KEEP | registered current lifecycle; `push,workflow_dispatch`; distinct jobs: Enforce Docker restart policies; current open PR ownership also forbids mutation in this task |
| .github/workflows/repair-synology-compose-orphans.yml | Repair Synology Compose Container Names | workflow_dispatch | repair | not named by live protection | MANUAL_OPERATION | KEEP | registered current lifecycle; `workflow_dispatch`; distinct jobs: repair |
| .github/workflows/support-legal-acceptance.yml | Support Legal Acceptance | pull_request, push, workflow_dispatch | Complete Support Legal lifecycle | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: Complete Support Legal lifecycle |
| .github/workflows/support-moderation-acceptance.yml | Support Moderation Acceptance | pull_request, push, workflow_dispatch | Complete Support Moderation lifecycle | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: Complete Support Moderation lifecycle |
| .github/workflows/synology-container-hygiene.yml | Synology Container Hygiene | pull_request, workflow_dispatch | static-validation; live-hygiene | not named by live protection | MANUAL_OPERATION | KEEP | registered current lifecycle; `pull_request,workflow_dispatch`; distinct jobs: static-validation; live-hygiene |
| .github/workflows/synology-diagnostics.yml | Synology Diagnostics | push, workflow_dispatch | Read-only Platform Synology diagnostics | not named by live protection | MANUAL_OPERATION | KEEP | registered current lifecycle; `push,workflow_dispatch`; distinct jobs: Read-only Platform Synology diagnostics |
| .github/workflows/synology-production-target-preflight.yml | Synology Production Target Preflight | pull_request, workflow_dispatch | static-validation; live-preflight | not named by live protection | MANUAL_OPERATION | KEEP | registered current lifecycle; `pull_request,workflow_dispatch`; distinct jobs: static-validation; live-preflight |
| .github/workflows/synology-rollback-contract.yml | Synology Rollback Contract | pull_request, push, workflow_dispatch | contract | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: contract |
| .github/workflows/terminal-branch-lifecycle-read-reusable.yml | Oteryn Terminal Branch Lifecycle Read Reusable | workflow_call | read-only-inventory | not named by live protection | REUSABLE_WORKFLOW | KEEP | `workflow_call` contract; archived #1233 closeout records META/Game/Atlas exact-SHA repins and real caller runs after permission split |
| .github/workflows/terminal-branch-lifecycle-reusable.yml | Oteryn Terminal Branch Lifecycle Reusable | workflow_call | validate-operation; close-event-cleanup; apply-reviewed-manifest | not named by live protection | REUSABLE_WORKFLOW | KEEP | `workflow_call` contract; archived #1233 closeout records META/Game/Atlas exact-SHA repins and real caller runs after permission split |
| .github/workflows/terminal-branch-lifecycle.yml | Terminal Branch Lifecycle | pull_request, pull_request_target, push, workflow_dispatch | validate; live-dry-run; apply-reviewed-manifest; close-event-cleanup | not named by live protection | MANUAL_OPERATION | KEEP | ADR 0037 adds closed-unmerged terminal cleanup and explicitly extends ADR 0024; distinct from merged-source Branch Lifecycle fallback |
| .github/workflows/tibia-linux-live-reference.yml | Tibia Linux Reference Harness | pull_request, workflow_dispatch | synthetic-no-network | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,workflow_dispatch`; distinct jobs: synthetic-no-network |
| .github/workflows/wiki-reconciliation-acceptance.yml | Wiki Reconciliation Acceptance | pull_request, push, workflow_dispatch | Complete Wiki reconciliation lifecycle | not named by live protection | REQUIRED_PR_GATE | KEEP | registered current lifecycle; `pull_request,push,workflow_dispatch`; distinct jobs: Complete Wiki reconciliation lifecycle |

### Consolidation decision

- Exact byte-level duplicate groups across all 55 current workflow files: **NONE**.
- Similar acceptance wrappers are not sufficient duplication evidence: they target different domain jobs/tests/path contracts; similarity alone is explicitly non-authorizing.
- Issue #1085 / PR #1086 already retired six proven obsolete workflows and established the lifecycle registry; current main registry exactly matches the 55-file inventory and budget.
- Since #1085, lifecycle changes were intentional: `codeql.yml` and the two terminal reusable workflows were added, while the obsolete `liquid20-synology-control.yml` was removed; registry tracks those changes.
- `branch-lifecycle.yml` vs `terminal-branch-lifecycle.yml`: **not duplicates**. ADR 0037 says terminal cleanup extends the existing merged-source lifecycle and directs merged-source fallback reconciliation to Branch Lifecycle.
- `native-auth-ephemeral-cutover-rehearsal.yml`: **retain as MIGRATION_TEMPORARY** because the active verification record still permits conditional compatibility Track A; lifecycle completion is not proven.
- `cloudflare-oteryn-hsts-stage1.yml`: **retain as MIGRATION_TEMPORARY** because the repository operation contract defines the currently staged one-month HSTS target with exact apply/audit/rollback; later promotion remains a future decision.
- `terminal-branch-lifecycle-*-reusable.yml`: **retain as REUSABLE_WORKFLOW**; archived #1233 closeout records META/Game/Atlas exact-SHA repins and real caller runs.
- No safe P1.4 workflow mutation is therefore justified. The correct bounded remediation is classification/evidence only; no required-check identity changes and no branch/ruleset mutation.

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  risk: high
  triggers:
    - CI workflow lifecycle / externally visible check contract
  unknown_or_conflict: []
  rationale: classification is read/evidence-only; any workflow mutation would require full compatibility proof, but none is justified
  self_review:
    result: PASS
    exact_head: 0dcace20e6fd59bdf663a0f12c658190f364b77e
    evidence:
      - full PR diff contains only this task record; no workflow paths changed
      - 55 classification rows equal the 55-file current-main inventory exactly
      - required-check contract re-read live: platform-gate only; rulesets none
      - no review submissions or inline review threads existed at self-review
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: same-repository squash PR is the sole delivery path and this source ref has no retention purpose
source_branch_evidence: repository delete_branch_on_merge is enabled; final ref absence must be verified immediately after merge
```

## Out-of-scope finding

OUT_OF_SCOPE_FINDING: Agent Governance run `32723828899` failed because `OTERYN-20260823-platform-transfer-terminal-reconciliation` represents terminal PR #1243 as active without an explicit archive-pending transition. That record/PR is outside P1.4, was not modified, and is not a protected required check for `main`; protected `platform-gate` passed on the same exact classification head.

## Terminal candidate semantics

This archive file is delivered by PR #1256. Before merge, live Issue/PR state remains authoritative. The post-merge inventory, merge SHA, source-ref absence, and Issue close event are recorded in live Issue/PR evidence to avoid a self-referential follow-up PR solely to write its own merge identity.

## Scope boundary

No P1.1/P1.2/P1.3/P1.5/P1.6, product/runtime, dependency, runner, environment, secret, branch-protection/ruleset, deployment-redesign, or cross-repository write work is authorized.
