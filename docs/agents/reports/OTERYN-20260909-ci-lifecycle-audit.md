# Oteryn Platform CI lifecycle audit — 2026-09-09

Issue: #1364  
Admission protected main: `102d29241662d45bb810bca65f03386fcb598dad`  
Prior full workflow baseline: PR #1256 / `557c08e72497ecd6a1b07fe8f282a1754763a4ff`

## Executive result

The current Platform CI surface contains 55 registered GitHub Actions workflows. The fresh verdict is:

| Verdict | Count | Meaning |
|---|---:|---|
| KEEP | 43 | Durable current lifecycle remains justified. |
| REPAIR | 10 | Keep the workflow, remove a dead task-record-only trigger. |
| RETIRE | 2 | Historical/migration proof workflow has no current caller, required check, owner or durable lifecycle. |
| CONSOLIDATE | 0 | No additional consolidation is proven safe in this generation. |
| UNKNOWN | 0 | Every current workflow received an evidence-backed terminal audit verdict. |

Expected executable workflow count after the proven cleanup: **53**.

The cleanup does **not** change branch protection, `platform-gate`, merge-group support, product tests, runtime classification, coverage policy, deployment/rollback semantics, Cloudflare operational safeguards, Agent Governance live checks, or ADR 0039 steady-state branch hygiene.

## Method

This audit does not infer removability from age, naming, skipped jobs, or visual similarity.

1. Read live protected-main branch state, required contexts and rulesets.
2. Read the current workflow directory and machine lifecycle registry.
3. Reuse PR #1256 only as an evidence baseline, then compare that exact merge with current main.
4. Re-evaluate lifecycle context even for byte-lineage-unchanged workflows because task ownership and migration state can change without editing a workflow file.
5. Search current repository callers/references for retirement candidates.
6. Refresh open-PR changed-path ownership before claiming mutation paths.
7. Preserve uncertain or distinct trust/runtime boundaries rather than optimizing them speculatively.

Between the prior 55/55 audit merge and this admission main, only five workflow definitions changed: `agent-governance.yml`, `build-synology-staging-images.yml`, `ci.yml`, `deploy-synology-staging.yml`, and `synology-rollback-contract.yml`. The other 50 definitions retained their workflow lineage, but their lifecycle context was still rechecked.

## Live merge authority and current-main evidence

- protected `main`: `102d29241662d45bb810bca65f03386fcb598dad`;
- required classic branch-protection context: `platform-gate`;
- repository rulesets: none;
- admission lifecycle registry: 55 registered workflows, budget 55;
- current-main CI run `34282843333`: `classify-changes` PASS, aggregate `test` PASS, `platform-gate` job `102251547776` PASS; `runtime-tests` and `php-coverage-report` correctly skipped for the docs-only change;
- admission Agent Governance `checkpoint-validation` job `102251475134` is red, but it is not the protected required context. Its unique live Issue/task/policy/Control-Room checks remain required by repository governance and are not removed by this audit.

## Current ownership exclusions

The writer does not mutate workflow files currently held by other open PRs. At the latest ownership refresh:

- `.github/workflows/build-synology-staging-images.yml` — PR #1251;
- `.github/workflows/codeql.yml` — PRs #1280/#1281;
- `.github/workflows/repair-synology-autostart.yml` — PR #1211.

Those files remain `KEEP`; ownership exclusion is additional safety, not their sole lifecycle justification.

## Full 55-workflow verdict ledger

| # | Workflow | Verdict | Evidence-backed disposition |
|---:|---|---|---|
| 1 | `acceptance-soak.yml` | KEEP | Bounded scheduled/manual public acceptance soak; distinct long-running lifecycle. |
| 2 | `acceptance-stability.yml` | KEEP | Bounded scheduled/manual repeat stability proof; distinct lifecycle. |
| 3 | `acceptance-validation.yml` | KEEP | Reusable/current acceptance engine with `workflow_call` callers; durable browser proof boundary. |
| 4 | `agent-governance.yml` | KEEP | Unique META-policy, live Issue/task ownership, task-liveness and Control-Room enforcement; overlap with deterministic core checks is insufficient consolidation proof. |
| 5 | `announcements-acceptance.yml` | KEEP | Path-scoped complete Announcements browser lifecycle. |
| 6 | `branch-lifecycle.yml` | REPAIR | Durable merged-source branch lifecycle remains; remove only missing historical active-task trigger. |
| 7 | `build-synology-staging-images.yml` | KEEP | Current immutable staging-image build/release boundary; recently changed and owned by open PR #1251. |
| 8 | `character-bazaar-staging-control.yml` | KEEP | Distinct guarded staging control/deployment capability. |
| 9 | `character-bazaar-staging-validation.yml` | KEEP | Distinct path-scoped validation for Bazaar staging package. |
| 10 | `ci.yml` | KEEP | Produces protected `platform-gate`, includes merge-group support and fail-closed classification/aggregate gate. |
| 11 | `cloudflare-oteryn-edge-audit.yml` | REPAIR | Durable edge audit remains; remove only missing historical edge-task trigger. |
| 12 | `cloudflare-oteryn-endpoint-main-operation.yml` | KEEP | Distinct guarded endpoint main operation; no replacement proven. |
| 13 | `cloudflare-oteryn-endpoints.yml` | REPAIR | Durable endpoint management remains; remove only missing historical edge-task trigger. |
| 14 | `cloudflare-oteryn-hsts-stage1.yml` | REPAIR | Exact apply/audit/rollback HSTS stage-1 operation remains current; remove only missing historical task trigger. |
| 15 | `cloudflare-oteryn-public-edge-repair.yml` | REPAIR | Guarded public-edge repair remains; remove only missing historical task trigger. |
| 16 | `cloudflare-zone-edge-audit.yml` | REPAIR | Durable zone audit remains; remove only missing historical zone-task trigger. |
| 17 | `codeql.yml` | KEEP | Current security-analysis lifecycle; open PR #1280/#1281 ownership also excludes mutation. |
| 18 | `community-data-acceptance.yml` | KEEP | Distinct production-like Community Data browser/integration proof. |
| 19 | `content-scale-acceptance.yml` | KEEP | Distinct long bilingual content-scale acceptance matrix. |
| 20 | `deploy-synology-staging.yml` | KEEP | Current manual immutable-component staging deploy/rollback boundary; recently reconciled and live-proven. |
| 21 | `downloads-acceptance.yml` | KEEP | Distinct complete Downloads lifecycle acceptance. |
| 22 | `edge-security-emulation.yml` | KEEP | Distinct edge-security classifier/emulation proof. |
| 23 | `editorial-media-acceptance.yml` | KEEP | Distinct editorial media acceptance lifecycle. |
| 24 | `error-state-acceptance.yml` | KEEP | Distinct localized global error-state browser lifecycle. |
| 25 | `events-acceptance.yml` | KEEP | Distinct complete Events lifecycle acceptance. |
| 26 | `game-auth-ticket-concurrency.yml` | KEEP | Distinct concurrency proof routed by changed risk. |
| 27 | `game-catalog-contract.yml` | REPAIR | Current catalog schema/import contract remains; remove historical active-task trigger references only. |
| 28 | `game-gateway-ci.yml` | KEEP | Distinct Game Gateway build/test lifecycle. |
| 29 | `github-actions-storage-hygiene.yml` | KEEP | Distinct scheduled/closed-PR storage maintenance lifecycle. |
| 30 | `historical-branch-audit.yml` | KEEP | ADR 0039 canonical steady-state enforcement of `NEW_UNEXPLAINED_BRANCHES = 0`; trusted read-only PR/schedule boundary remains mandatory. |
| 31 | `native-auth-canary-cache-build.yml` | RETIRE | Self/manual-triggered historical Canary build at fixed `b15b7d...`; no current caller or consumer of artifact `native-auth-rehearsal-canary-cache-headers`; not protected-required. |
| 32 | `native-auth-ephemeral-cutover-rehearsal.yml` | RETIRE | Prior `MIGRATION_TEMPORARY` KEEP rationale depended on active native-auth production verification, now completed/archived; workflow pins historical Platform/Gateway/Canary/OTClient revisions and has no current caller/required check. |
| 33 | `native-protocol-contract-audits.yml` | REPAIR | Five distinct current native-protocol audit boundaries remain; remove only missing historical producer-task trigger. |
| 34 | `native-protocol-contract.yml` | KEEP | Current native protocol contract validator; no replacement proven. |
| 35 | `oteryn-public-edge-validation.yml` | KEEP | Current public-edge validate/observe contract remains distinct. |
| 36 | `parallel-coordinator-prompt-eval.yml` | KEEP | Explicitly evaluates `oteryn-portal-parallel-coordinator-prompt-v1.json`; default `prompt_eval.py` invocation does not replace this non-default suite. |
| 37 | `phase7-production-like-validation.yml` | KEEP | Distinct risk-routed production-like integration profile. |
| 38 | `platform-db-outage-validation.yml` | KEEP | Distinct database outage/failure-recovery proof. |
| 39 | `playwright-runtime-validation.yml` | KEEP | Distinct PHP/Playwright runtime and Synology browser-cache lifecycle. |
| 40 | `portal-acceptance-contract.yml` | KEEP | Current strict portal acceptance/account lifecycle contract. |
| 41 | `portal-e2e-audit.yml` | KEEP | Explicit comprehensive exact-head portal E2E orchestration; not a default per-PR replacement for focused lanes. |
| 42 | `recover-synology-staging-schema.yml` | KEEP | Documented supported manual schema recovery entry point; cross-checked by rollback recovery tests. |
| 43 | `repair-synology-autostart.yml` | KEEP | Distinct guarded Synology repair capability; open PR #1211 owns current file. |
| 44 | `repair-synology-compose-orphans.yml` | KEEP | Distinct bounded compose-orphan recovery capability. |
| 45 | `support-legal-acceptance.yml` | KEEP | Distinct Support/Legal acceptance lifecycle. |
| 46 | `support-moderation-acceptance.yml` | KEEP | Distinct Support Moderation acceptance lifecycle. |
| 47 | `synology-container-hygiene.yml` | KEEP | Distinct static/live container-hygiene boundary on Synology. |
| 48 | `synology-diagnostics.yml` | KEEP | Distinct read-only Platform Synology diagnostics/manual capability. |
| 49 | `synology-production-target-preflight.yml` | REPAIR | Durable production-target preflight remains; remove only missing historical task trigger. |
| 50 | `synology-rollback-contract.yml` | REPAIR | Current rollback/schema-recovery contract remains; remove only missing historical task trigger. |
| 51 | `terminal-branch-lifecycle-read-reusable.yml` | KEEP | Organization reusable read-only trust boundary; intentionally isolated permissions. |
| 52 | `terminal-branch-lifecycle-reusable.yml` | KEEP | Organization reusable mutation-capable terminal lifecycle; distinct from read-only reusable. |
| 53 | `terminal-branch-lifecycle.yml` | KEEP | Repository terminal lifecycle orchestration/caller; distinct from reusable implementations. |
| 54 | `tibia-linux-live-reference.yml` | KEEP | Distinct external/live reference verification lifecycle. |
| 55 | `wiki-reconciliation-acceptance.yml` | KEEP | Distinct Wiki reconciliation acceptance lifecycle. |

## Retirement proof

### `native-auth-ephemeral-cutover-rehearsal.yml`

The previous P1.4 audit explicitly classified this workflow as `MIGRATION_TEMPORARY / KEEP` because the then-active native-auth production-verification record still permitted conditional compatibility Track A. That dependency is now terminal and archived.

The current workflow still pins historical revisions:

- Platform `b5dd6a7be5c704d5706241240e06f8bb8c4b5efe`;
- Gateway `53158217a6c6017230301cf4daa783b04fcc13d5`;
- Canary `b15b7d544f4795e3a2a65b88de35391b9fd0a20d`;
- OTClient `9189d1063e968a0c2ffab11c5069db192e753397`;
- Canary harness `f46ae126557d4d26043c77fe17968b72fd5bc688`.

Its PR path set also names the no-longer-active `OTERYN-20260723-native-auth-ephemeral-cutover-rehearsal.md`. Current repository search finds the workflow filename only in the workflow itself, lifecycle registry, and historical audit/provenance documents. No current reusable caller or protected required context references it.

Historical evidence remains in the merged PR/task/Git history; retaining an executable historical workflow is not archival policy under `CI_WORKFLOW_LIFECYCLE.md`.

### `native-auth-canary-cache-build.yml`

The workflow triggers manually or when its own YAML changes, binds historical Canary `b15b7d544f4795e3a2a65b88de35391b9fd0a20d`, and publishes `native-auth-rehearsal-canary-cache-headers`.

Current repository search finds the artifact name only inside this workflow and finds the workflow filename only in itself, the lifecycle registry, and historical P1.4 evidence. No caller, consumer, required context, active task, or current deployment path depends on it. Its historical proof remains in Git history.

## Trigger repairs

Ten retained workflows contain direct `paths` entries naming task Markdown that is absent from the current `docs/agents/tasks/active/` control plane. Those entries are task-history coupling, not product/runtime ownership. The repair removes only those dead task-specific entries while leaving all product, scripts, tests, contracts, operational marker and workflow-self triggers intact.

The affected retained workflows are:

- `branch-lifecycle.yml`;
- `cloudflare-oteryn-edge-audit.yml`;
- `cloudflare-oteryn-endpoints.yml`;
- `cloudflare-oteryn-hsts-stage1.yml`;
- `cloudflare-oteryn-public-edge-repair.yml`;
- `cloudflare-zone-edge-audit.yml`;
- `game-catalog-contract.yml`;
- `native-protocol-contract-audits.yml`;
- `synology-production-target-preflight.yml`;
- `synology-rollback-contract.yml`.

A regression is added to `tests/ci/test_workflow_trigger_economy.py`: every direct workflow reference to `docs/agents/tasks/active/*.md` must resolve to a currently existing file. This turns the cleanup from a one-off deletion into a maintained invariant.

## Explicit non-cleanups

### Agent Governance

`agent-governance.yml` overlaps core CI on deterministic checkpoint/prompt validation, but also performs live governing-Issue ownership, live active-task ownership, bound META policy consistency and Control Room rendering/fail-closed enforcement. No equivalent single replacement currently proves those semantics. Verdict: `KEEP`.

### Historical Branch Audit

The one-time historical reconciliation registry is already applied, but ADR 0039 separately requires continuous steady-state branch hygiene and names Historical Branch Audit as the canonical enforcement surface. Removing it would weaken ownerless-branch detection. Verdict: `KEEP`.

### Skipped CI jobs

`runtime-tests` and `php-coverage-report` being skipped for a docs-only change is intended routing, not dead CI. This audit preserves that economy rather than deleting conditional jobs.

## Expected post-cleanup contract

- executable workflows: `55 -> 53`;
- protected required context: unchanged `platform-gate`;
- rulesets: unchanged;
- retired historical workflows recorded in lifecycle registry;
- stale active-task trigger references: zero;
- no test, security, runtime, deployment, rollback, governance or branch-hygiene guarantee intentionally removed;
- historical evidence remains available in Git/PR/task provenance without keeping obsolete executable YAML.

## Validation plan

The coherent candidate must pass at minimum:

- `python tools/validation/test_workflow_inventory.py`;
- `python tools/validation/workflow_inventory.py`;
- `python tests/ci/test_workflow_trigger_economy.py`;
- `python tests/ci/test_classify_changes.py`;
- `python tests/ci/test_push_change_routing.py`;
- `python tests/ci/test_required_test_gate.py`;
- `python tools/validation/test_github_actions_pinning.py`;
- `python tools/validation/github_actions_pinning.py --inventory-json`;
- applicable exact-head workflow validations triggered by changed workflow files;
- Agent Governance exact-head validation;
- protected required `platform-gate` on the PR candidate and Merge Queue candidate;
- resulting-main inventory/protection/source-branch verification after integration.

No production, staging, protected-environment, credential, payment, or external-repository mutation is required for this cleanup.
