# Validator packet — OTERYN-20260731 portal backend/frontend audit

## Mission

Independently try to disprove the consolidated portal audit and close the two remaining acceptance criteria on the same task and PR:

1. execute a focused zero-retry Issue #365 Wiki reproduction on the frozen audit target;
2. publish a separate validator artifact with exactly one verdict: `VALIDATED`, `VALIDATED_WITH_CORRECTIONS`, or `REJECTED`.

Do not repair application behavior, merge, deploy, mutate Canary, or promote open-PR code into the frozen target.

## Immutable identities

- Repository: `blakinio/Oteryn-Platform`
- Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`
- Audit branch: `audit/OTERYN-20260731-portal-backend-frontend-audit`
- Audit PR: `#381`
- Parent: Issue `#326`
- Focused defect evidence: Issue `#365`
- Current packet authoring head: resolve live from PR `#381`; do not assume a copied SHA is current.

The frozen target remains authoritative even if `main` advances.

## Required reads

Read before execution:

- `AGENTS.md`
- `docs/agents/AGENTS.md`
- `docs/agents/EXECUTION_PROTOCOL.md`
- `docs/agents/BUILD_TEST_MATRIX.md`
- `docs/architecture/TEST_STRATEGY.md`
- `docs/agents/tasks/active/OTERYN-20260731-portal-backend-frontend-audit.md`
- `docs/agents/reports/OTERYN-20260731-portal-backend-frontend-audit.md`
- every file under `docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/`
- Issue `#326`
- Issue `#365`

## Authorized writes

Only these writes are authorized:

- one new independent validator artifact under `docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/`;
- corrections to audit reports/matrices when the validator proves a factual error;
- the active audit checkpoint.

No application, route, view, asset, configuration, migration, model, test, workflow, dependency, product manifest, acceptance ledger, deployment, staging, production, or Canary change is authorized.

An ephemeral local probe may be created only for execution and must be deleted before recording the final working-tree state. It must never be committed.

## A. Exact checkout preflight

Record the literal output of:

```bash
git fetch --prune origin main audit/OTERYN-20260731-portal-backend-frontend-audit
git checkout --detach b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608
test "$(git rev-parse HEAD)" = "b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608"
git status --short
php --version
composer --version
node --version
npm --version
```

Requirements:

- checkout SHA must match exactly;
- initial working tree must be clean;
- record installed tool versions without replacing repository constraints with local assumptions.

Use the repository's current acceptance workflow/bootstrap as the authoritative environment recipe. Preserve the same runtime class used by the existing acceptance evidence: real Laravel HTTP, isolated Platform and Canary MariaDB schemas, Redis ACL, and MailHog.

## B. Re-run deterministic repository contracts

On the frozen target, run and preserve complete output:

```bash
npm --prefix scripts/acceptance run test:coverage-contract:strict
```

Validate independently, rather than copying report totals:

- canonical surfaces: 27;
- manifest named-route assignments: 228;
- capability records: 43;
- capability result: 23 implemented, 3 partial, 14 missing, 3 not applicable;
- route/view/navigation closure has zero orphan views;
- content-scale validator classifies only 18 base-manifest surfaces and omits the nine fragment surfaces listed in the audit;
- media applicability closes all 27 surfaces;
- no user-facing backend-only or frontend-only capability is promoted to implemented.

Any different result requires a correction or rejection with exact evidence.

## C. Focused Issue #365 reproduction

### Existing frozen-target behavior

The frozen test `scripts/acceptance/tests/admin-wiki-administration.spec.mjs`:

- uses `responsive-mobile` at 390×844;
- sets retries to zero;
- waits for approved-media thumbnail traffic to become idle before publication;
- proves durable publication through `Status: Published` and `Unpublish to draft`;
- no longer asserts the transient `role=status` publication message.

The shared diagnostics helper records every response with status `>=500` in the `browser-diagnostics` attachment.

### Required focused run

First execute the unchanged frozen test:

```bash
cd scripts/acceptance
ACCEPTANCE_SHA=b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608 \
ACCEPTANCE_PROFILE=critical \
ACCEPTANCE_ZERO_RETRIES=1 \
npx playwright test tests/admin-wiki-administration.spec.mjs \
  --project=responsive-mobile \
  --workers=1 \
  --retries=0
```

Then execute an ephemeral local probe of the same flow that records, immediately after the publish redirect:

- publish POST status and redirect chain;
- all accessible `role=status` texts;
- durable article status/version and presence of `Unpublish to draft`;
- all `browser-diagnostics.serverErrors` entries;
- all thumbnail request status codes for `/admin/wiki/media/{id}/thumbnail`;
- sanitized Laravel/application log lines covering the publish request and concurrent thumbnail requests;
- sanitized web-server stderr/stdout for the same interval.

The probe must not alter authentication, authorization, MFA, session security, publication state semantics, request ordering, database fixtures, or application/test code. Delete it after execution and prove the working tree is clean.

Run the focused scenario at least three independent times on the same frozen SHA with retries disabled. Do not use Playwright retries as samples.

### Classification rules

Classify the flash and thumbnail symptoms independently:

- `REPRODUCED`: directly observed on the frozen target with exact sanitized evidence;
- `NOT_REPRODUCED`: at least three clean zero-retry executions with the required observer evidence;
- `INCONCLUSIVE`: environment/bootstrap/logging failure or incomplete evidence.

Do not infer a shared cause. Do not classify `NOT_REPRODUCED` from the durable-state assertion alone.

## D. Historical evidence cross-check

Independently inspect artifacts from:

- run `30562698853`, job `90939481510`, artifact `8767657461`, SHA `35f39b48233b186502cbdcc05aec7ffc40e78fc7`;
- run `30578806660`, job `90993603962`, artifact `8773887288`, SHA `fb1bbac96c0dcd0096aef55c2c8c752e453b6ddb`.

Expected evidence to verify, not assume:

- responsive desktop and tablet completed the Wiki test;
- responsive mobile alone failed the transient `role=status` assertion while the redirected page showed `Published`, version 3 and `Unpublish to draft`;
- run 1 and run 2 show the same deterministic thumbnail pattern per viewport:
  - desktop: 9 HTTP 500 responses across media IDs 1, 3 and 5;
  - tablet: 12 HTTP 500 responses across media IDs 1, 3, 5 and 7;
  - mobile: 16 HTTP 500 responses across media IDs 1, 3, 5, 7 and 9;
- both runs contain two console errors caused by the HTML `pattern` value `[a-z0-9]+([._-][a-z0-9]+)*`; this is separate evidence and must not be silently merged into Issue #365 without scope/severity analysis;
- no artifact proves a causal relationship between the flash loss and thumbnail failures.

If any expected historical count or boundary is wrong, correct the audit.

## E. Adversarial validation of audit findings

Try to disprove each finding:

1. `OTERYN-AUDIT-P35-006` — locate exact evidence that thumbnail responses were not HTTP 500, were expected/controlled, or did not affect an administrator media surface.
2. `OTERYN-AUDIT-P35-001` — prove that the content-scale validator actually loads all six manifest fragments or otherwise enforces one record for all 27 surfaces.
3. `OTERYN-AUDIT-P35-002` — prove that HTTP 503 is present in the dedicated EN/PL desktop/tablet/mobile error matrix with the same assertions as 404/419/429/500.
4. `OTERYN-AUDIT-P35-003` — locate a fail-closed per-rendered-surface accessibility applicability/evidence ledger, including reduced-motion classification.
5. `OTERYN-AUDIT-P35-005` — prove that the historical publication did not durably succeed or that the accessible success announcement was present.
6. `OTERYN-AUDIT-P1-001` — reconcile `ACTIVE_WORK.md` with the live PR/task ownership state at the frozen baseline.

Also sample at least five `LOW`/`INFO` or `UNKNOWN` assertions, including Marketplace gating, `home-preview`, open-PR-only capability boundaries, staging identity, and production nonclaim.

## F. Unauthorized-change check

Against the frozen target, list every audit PR path and verify that it is under the task's owned audit-document/evidence paths:

```bash
git diff --name-only b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608...origin/audit/OTERYN-20260731-portal-backend-frontend-audit
git diff --check b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608...origin/audit/OTERYN-20260731-portal-backend-frontend-audit
```

Reject any application/runtime/test/workflow/deployment mutation in PR `#381`.

## G. Exact-head CI check

Resolve the current PR head after validator writes. Require all emitted workflow families to complete successfully on that exact final head. Record run IDs and conclusions. A green predecessor does not prove a later head.

## H. Validator artifact

Create one separate file, recommended name:

`docs/agents/evidence/OTERYN-20260731-portal-backend-frontend-audit/INDEPENDENT_VALIDATION.md`

It must contain:

- validator session identity and role;
- exact frozen target and exact audit-branch head validated;
- clean-checkout and tool-version evidence;
- commands executed and first relevant failure, if any;
- independent counts and sampled assertions;
- focused Issue #365 result for flash and thumbnail separately;
- corrections made, if any;
- unauthorized-change result;
- exact final-head CI runs;
- exactly one final verdict:
  - `VALIDATED`
  - `VALIDATED_WITH_CORRECTIONS`
  - `REJECTED`

`VALIDATED` is forbidden while the focused Issue #365 execution is `INCONCLUSIVE` or not run. Do not convert missing evidence into a pass.
