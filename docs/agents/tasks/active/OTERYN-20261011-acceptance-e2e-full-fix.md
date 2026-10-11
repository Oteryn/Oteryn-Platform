---
task_id: OTERYN-20261011-acceptance-e2e-full-fix
governing_issue: 1520
required_reads: []
search_first:
  - scripts/acceptance/runtime-diagnostics.mjs
optional_reads: []
---

# OTERYN-20261011 acceptance E2E full profile fix

## Goal

Governing GitHub Issue: #1520 (https://github.com/Oteryn/Oteryn-Platform/issues/1520) — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Make the `full` profile of `Acceptance E2E and Visual UX` green on `main`. Three specs fail on every retained `main` push run (earliest 34719930030, 2026-09-12) because pull requests only run the `critical` profile. Fix the stale specs and the time-dependent fixture; no product change, diagnostics stay strict and no test is skipped.

## Acceptance criteria

- [x] `admin-acceptance.spec.mjs` declares the deliberate 404 on `/news/{slug}` and `/pages/{slug}` with `allowExpectedHttpFailure`.
- [x] The Today news fixture is newer than any earlier fixture in the same run, so it stays in the 3 latest published news.
- [x] `public-game-data-acceptance.spec.mjs` reads `Runtime:` and `Players online:` from the definition list introduced by #1298.
- [x] Diagnostics helpers are unchanged; no test is skipped or retried.
- [ ] A `full` profile run of `Acceptance E2E and Visual UX` on the exact head passes.

## Ownership

```yaml
owned_paths:
  - scripts/acceptance/tests/admin-acceptance.spec.mjs
  - scripts/acceptance/tests/public-game-data-acceptance.spec.mjs
  - scripts/acceptance/seed-homepage-navigation-seo.php
  - docs/agents/tasks/active/OTERYN-20261011-acceptance-e2e-full-fix.md
modules:
  - Acceptance
dependencies:
  - Issue #1520
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T08:30:00Z
head: UNKNOWN
branch: claude/acceptance-e2e-fix-20261011
pr: none
status: validating
context_routes:
  - testing
owned_paths:
  - scripts/acceptance/tests/admin-acceptance.spec.mjs
  - scripts/acceptance/tests/public-game-data-acceptance.spec.mjs
  - scripts/acceptance/seed-homepage-navigation-seo.php
  - docs/agents/tasks/active/OTERYN-20261011-acceptance-e2e-full-fix.md
proven:
  - Every retained main push run (34719930030 at 224e5f07 through 38110180131 at 8006fc8f) fails on the same 3 tests; pull_request runs use the critical profile, which excludes them.
  - Run 38110180131 admin failure is only the afterEach diagnostics assertion (console 404 on the unpublished news and page routes); the test body passed.
  - Run 38110180131 Today snapshot lists long-localized news and Content Scale News 001/002 as the 3 latest news; the content-scale fixture publishes at seed time minus 1 minute, the Today fixture at seed time minus 2 minutes.
  - Run 38110180131 servers snapshot shows Runtime ONLINE and Players online 1 as dt/dd terms, not paragraphs.
derived:
  - Not caused by DECANARY merges #1491 or #1492; the specs drifted after #1242 (diagnostics), #1114 (Today fixture) and #1298 (servers markup).
unknown:
  - Whether later steps of the Today and Flow 6b tests, never reached on main, pass; only a full profile run proves it.
conflicts: []
first_failure:
  marker: earliest retained main run 34719930030
  evidence: same 3 failures as run 38110180131
rejected_hypotheses:
  - DECANARY PUBLIC-OFF-1 (#1491) or CREATE-OFF-1 (#1492) regression; the failures predate both merges.
changed_paths:
  - scripts/acceptance/tests/admin-acceptance.spec.mjs
  - scripts/acceptance/tests/public-game-data-acceptance.spec.mjs
  - scripts/acceptance/seed-homepage-navigation-seo.php
  - docs/agents/tasks/active/OTERYN-20261011-acceptance-e2e-full-fix.md
validation:
  - command: node --check on both changed specs; php -l on the seed
    result: PASS
    evidence: syntax checks clean
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261011-acceptance-e2e-full-fix.md --require-checkpoint
    result: PASS
    evidence: structural checkpoint validation
blockers:
  - none
next_action: Run the full profile on the exact head; archive this record after merge.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: pending merge
```
