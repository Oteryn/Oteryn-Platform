# Runtime Diagnostics Hardening Closeout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish PR #1242 truthfully: preserve strict Platform runtime diagnostics, resolve the remaining Wiki media/browser portability failures, pass exact-head CI, merge, and close out the task.

**Architecture:** Keep the diagnostics gate fail-closed. Intentional HTTP failures are explicit bounded allowances by exact status/path/count; genuine console/page/request/CSP failures remain fatal. Product behavior distinguishes missing Wiki media storage (404) from integrity failure (500), while WebKit evidence avoids the Playwright screenshot operation that itself injects CSP-blocked inline style.

**Tech Stack:** Laravel 13 / PHP 8.5, PHPUnit, PHPStan, Node.js 24, Playwright 1.62.1, GitHub Actions.

**Spec:** `docs/agents/tasks/active/OTERYN-20260823-runtime-diagnostics-hardening.md`

## Global Constraints

- Repository writes: `Oteryn/Oteryn-Platform` only.
- Continue the existing branch `test/runtime-diagnostics-hardening-20260823` and PR #1242; do not create a replacement task/PR.
- Do not import Atlas WebGL, canvas, map, geometry, pixel-oracle, rendering, or screenshot-oracle subsystems.
- Do not add blanket console/request/CSP suppression.
- The only navigation-cancellation signatures allowed are Chromium `net::ERR_ABORTED`, Firefox `NS_BINDING_ABORTED`, and WebKit `Load request cancelled`.
- Expected HTTP failures must stay exact and bounded by status, pathname, and occurrence count.
- Genuine CSP violations, page errors, ordinary failed requests, wrong status/path, and surplus expected-failure occurrences remain fatal.
- No production/staging mutation and no owner-funded AI/model/API invocation.
- Work autonomously until a real authority/safety/ownership blocker exists; do not stop merely because CI is running or a draft PR exists.

## Durable Starting State

- Worktree: `C:\Users\barte\Oteryn-Platform-runtime-e2e`.
- Local branch head before this handoff: `740ec45cd07c7b3ec12fb321ead3815ad172d957` (`fix(acceptance): avoid WebKit screenshot CSP mutation`).
- Remote PR #1242 head before this handoff commit: `f5958e8691d8d7ff689680570a902b5f1f57c99d`; verify live state before changing anything.
- Runtime diagnostics focused suite is green at 23/23 after the WebKit evidence regression test.
- Exact-head Editorial Media and Error State workflows were green after the repeated-identical-allowance classifier repair.
- Portability on `f5958e8` still exposed Wiki media 500/browser failure behavior plus WebKit screenshot CSP; the WebKit screenshot repair exists locally in `740ec45`.
- Current WIP changes distinguish missing Wiki media storage as 404 from integrity failure as 500 and alter the admin-Wiki browser scenario accordingly.
- Focused feature test `test_wiki_thumbnail_distinguishes_missing_storage_from_integrity_failure` passed with exit 0 / 10 assertions after the current PHP change.
- PHPStan on the prior candidate failed because `AdminWikiAdministrationTest.php` passed `string|false` from `getContent()` directly into `assertStringNotContainsString()`.
- Untracked `artifacts/` and `scripts/acceptance/node_modules/` are task-local temporary resources; never commit them.

### Task 1: Resume the exact branch and verify ownership

**Files:**
- Read: `docs/agents/tasks/active/OTERYN-20260823-runtime-diagnostics-hardening.md`
- Read: this plan
- Read: `docs/agents/prompts/OTERYN-RUNTIME-DIAGNOSTICS-CLOSEOUT-AGENT-PROMPT.md`

**Interfaces:**
- Consumes: existing task ownership, PR #1242, protected `main`, current worktree.
- Produces: a verified live baseline with no duplicate owner or history divergence.

- [ ] **Step 1:** Run `git status --short`, `git rev-parse HEAD`, fetch `main` and the task branch, then verify PR #1242 live head/state.
- [ ] **Step 2:** Confirm the task remains `status: validating` and no active task/PR owns the same paths.
- [ ] **Step 3:** Compare local and remote history. Do not force-push; reconcile any independent remote advance from evidence first.
- [ ] **Step 4:** Keep `artifacts/` and `scripts/acceptance/node_modules/` untracked and out of commits.

### Task 2: Finish the bounded Wiki media repair and PHPStan fix

**Files:**
- Modify: `app/EditorialMedia/Application/WikiEditorialMediaFileResponse.php`
- Modify: `scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs`
- Modify: `tests/Feature/Wiki/WikiEditorialMediaReferenceSyncTest.php`
- Modify: `tests/Feature/Wiki/AdminWikiAdministrationTest.php`

**Interfaces:**
- Consumes: `WikiEditorialMediaFileResponse::private()`, runtime diagnostics HTTP allowances, Laravel error rendering.
- Produces: missing media storage returns 404; corrupt/integrity failure returns 500; ordinary failed requests remain fatal; conflict HTML assertions are PHPStan-safe.

- [ ] **Step 1:** Keep the feature test that removes the stored object and asserts the Wiki thumbnail route is 404, then corrupts bytes and asserts 500.
- [ ] **Step 2:** Keep `HttpExceptionInterface` passthrough around storage access so the explicit 404 is not wrapped as `WikiEditorialMediaUnavailable`; non-HTTP storage exceptions still fail closed.
- [ ] **Step 3:** In the browser spec, assert corrupt media status 500 through authenticated `page.request.get()` and use a removed-file 404 for browser fallback.
- [ ] **Step 4:** Preserve the exact 404 allowance only for `/admin/wiki/media/<id>/thumbnail` after the browser has observed that expected response.
- [ ] **Step 5:** Fix the PHPStan error in `AdminWikiAdministrationTest.php` by normalizing response content before string assertions:

```php
$content = $conflictResponse->getContent();
self::assertIsString($content);
self::assertStringNotContainsString('<style', $content);
self::assertStringNotContainsString('style=', $content);
```

### Task 3: Re-run focused validation on the completed local candidate

**Files:**
- Test: `scripts/acceptance/unit/runtime-diagnostics.test.mjs`
- Test: `tests/Feature/Wiki/WikiEditorialMediaReferenceSyncTest.php`
- Test: `tests/Feature/Wiki/AdminWikiAdministrationTest.php`
- Test: `tests/Feature/PlayerCompanion/SessionAnalysisFeatureTest.php`

**Interfaces:**
- Consumes: completed Task 2 code.
- Produces: fresh local evidence before the final push.

- [ ] **Step 1:** Run `npm --prefix scripts/acceptance run test:runtime-diagnostics`; expected result is 23/23 PASS or higher if a justified regression test is added.
- [ ] **Step 2:** Run `node --check scripts/acceptance/runtime-diagnostics.mjs`, `node --check scripts/acceptance/tests/helpers.mjs`, and `node --check scripts/acceptance/tests/admin-wiki-editorial-media.spec.mjs`.
- [ ] **Step 3:** Run `git diff --check` and ensure only task-owned files are modified/staged.
- [ ] **Step 4:** Run PHP syntax checks with `docker run --rm -v "${PWD}:/workspace" -w /workspace php:8.5-cli php -l <changed-php-file>` for each changed PHP file.
- [ ] **Step 5:** Run focused PHPUnit using `oteryn-php85-gd-validation:local` and volume `oteryn-runtime-cycle3-vendor:/workspace/vendor` for Wiki media, Wiki stale-conflict, and Player Companion delete behavior.
- [ ] **Step 6:** Run `composer analyse` in the same PHP 8.5 validation container and require exit 0 before push.
- [ ] **Step 7:** Run Playwright `--list` for primary portability, Wiki reconciliation, and Editorial Media configs; parsing/enumeration must be clean.

### Task 4: Push one final candidate and inspect exact-head CI

**Files:**
- Modify: `docs/agents/tasks/active/OTERYN-20260823-runtime-diagnostics-hardening.md`

**Interfaces:**
- Consumes: green focused validation and clean task diff.
- Produces: one exact pushed SHA with an auditable checkpoint and CI evidence.

- [ ] **Step 1:** Update the task checkpoint with PROVEN/DERIVED/UNKNOWN/CONFLICT, exact local validation, current repair-cycle count, and one concrete next action.
- [ ] **Step 2:** Commit the completed repair and checkpoint in coherent commits; do not amend published history and do not force-push.
- [ ] **Step 3:** Push the existing branch and verify GitHub PR #1242 reports exactly the pushed head SHA.
- [ ] **Step 4:** Inspect CI for that exact SHA. Prioritize `platform-gate`, Agent Governance, Wiki Reconciliation, Editorial Media, Error State, Downloads, Portal Acceptance, and the main Acceptance/portability workflow.
- [ ] **Step 5:** If a check fails, classify the first real failure from job logs. Do not add a fourth speculative repair cycle; only continue when the existing cycle-3 change is incomplete or a deterministic test/checkpoint correction is proven.

### Task 5: Self-review, Ready, merge, and closeout

**Files:**
- Modify/archive: `docs/agents/tasks/active/OTERYN-20260823-runtime-diagnostics-hardening.md`
- Verify/remove: task branch and task-local temporary resources.

**Interfaces:**
- Consumes: exact-head green required CI and clean review state.
- Produces: merged PR, archived task, deleted source branch, released ownership, and clean worktree.

- [ ] **Step 1:** Review the full PR diff against `main`; verify no Atlas rendering/map scope, no blanket diagnostics suppression, no secrets, and no unrelated changes.
- [ ] **Step 2:** Verify PR reviews, review threads, requested reviewers, mergeability, protected-branch requirements, and exact-head `platform-gate` success.
- [ ] **Step 3:** Mark PR #1242 Ready only after implementation, focused validation, real browser E2E, self-review, and checkpoint evidence are all complete.
- [ ] **Step 4:** Squash-merge through normal branch protection only; never bypass required checks.
- [ ] **Step 5:** Verify the merge landed on `main`, delete the source branch, and verify the branch is gone remotely.
- [ ] **Step 6:** Archive the task according to repository governance and release owned paths.
- [ ] **Step 7:** Remove only task-owned temporary `artifacts/`, `scripts/acceptance/node_modules/`, test containers, and named vendor volume when no longer needed. Do not blanket-prune Docker resources.
- [ ] **Step 8:** Final response must use the canonical terminal fields from `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md` and must not claim completion unless merge and closeout are proven.

## Expected final evidence

- Runtime diagnostics focused suite green.
- PHP focused tests and PHPStan green.
- Exact-head Wiki, Editorial Media, Error State, Downloads, Portal, Acceptance/portability, Governance, and required `platform-gate` green.
- PR #1242 merged through branch protection.
- Active task archived and branch deleted.
- No task-owned temporary resources or untracked WIP remain.
