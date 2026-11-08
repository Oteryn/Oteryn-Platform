---
task_id: OTERYN-20261108-portal-rs-e2e-full
governing_issue: 530
required_reads:
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
search_first:
  - portal-rs/e2e
optional_reads: []
---

# Full portal shell qualification

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530 (open at admission).
Owner authorizes disposable local services, full browser coverage, minimal fixes,
commits and normal push on codex/portal-rs-e2e-full; no pull request.
Base: 26c767eec36c04b5227a9676ec827ec76c4e82e9, fetched from the requested source branch.
The source PR is #1782; this task extends its shell qualification without modifying that PR.
Jira mapping unknown; no connector is available. Synchronization remains pending.

## Ownership

Owned paths: portal-rs/** and this packet. No workflow or Game repository changes.
Existing source-shell packet records overlapping source ownership; this task is the
owner-requested follow-up on a separate branch and checkout.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-08T13:32:48Z
head: 7bcfe31a6b53ac55115ef642ae37fa23aa0cee0a
branch: codex/portal-rs-e2e-full
pr: none
status: validating
context_routes: []
owned_paths:
  - portal-rs/**
  - docs/agents/tasks/active/OTERYN-20261108-portal-rs-e2e-full.md
proven:
  - requested source branch fetched; newer than minimum source head
  - supplied PostgreSQL 18.6 and Chromium available
derived: []
unknown:
  - final complete composed route coverage and acceptance
conflicts: []
first_failure:
  marker: mobile-dropdown-hidden
  evidence: home.css disables menus on focus-within at phone widths
rejected_hypotheses: []
changed_paths: []
validation:
  - command: pending
    result: NOT_RUN
    evidence: qualification in progress
blockers:
  - none
next_action: complete the expanded browser matrix and PostgreSQL workspace acceptance
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner requests pushed branch without a pull request
source_branch_evidence: current task instruction
```

## Work in progress evidence

- 318 route rows (including prefix aliases), 48 registry sections; one compatibility
  Bazaar item-action adapter is mutually exclusive with the qualified runtime.
  Its 404 absence is executed and it is marked NOT_APPLICABLE with the source reason.
- Fourteen production defects repaired with owning regressions. See e2e/REPORT.md.
- UI, Web, Community, Wiki, Gameauth and Admin focused regressions pass. The
  journal PostgreSQL security regression ran with PORTAL_TEST_DATABASE_URL and
  passed owner writes, missing-token refusal and foreign-origin refusal.
- Strict locked Clippy passes. Full workspace PostgreSQL acceptance remains pending.
- The completed diagnostic phone sweep ran 374 cases: 355 passed and 19 failed.
  All failures were reviewed: source defects above plus corrected fixture inputs,
  signed one-click authorization-token semantics, and redirect diagnostics.
  This diagnostic is not final qualification; a fresh-server rerun is required.
- Current launcher scratch: /tmp/portal-e2e-tRPn5H; the local launcher owns its
  serve/worker children, proxy/mail sinks and two disposable databases.
- Logs: /tmp/portal-e2e-route-diagnostic.log, /tmp/portal-journal-csrf.log,
  /tmp/portal-admin-title.log and /tmp/portal-clippy-latest.log. Raw artifacts
  stay private and ignored. Never publish env.json, mail, traces or cookies.
- Final qualification uses e2e/summarize.mjs, which rejects incomplete four-project
  results, retries, skipped/interrupted cases and uncovered routes/sections.
- For composed Rust tests stop only the launcher's serve/worker children to free
  listener ports, keeping the launcher alive until acceptance has used its test DB.

Confirmation: the repaired-case phone run passed 23 cases; the final journal
preference case then passed after correcting the fixture to assert the documented
204 local preference update and its persisted value (delivery remains gated).
A further geometry probe found the Account phone menu at x=-15; odd-column
alignment is repaired and PL/EN bounds now stay between x=17 and x=373 at 390px.
The initial all-project acceptance attempt was stopped after 41 passes so the
new bounds regression and fix are included in the final immutable candidate.

Combined-flow harness corrections: the closure producer and ownership readers
now share the same authority; privacy checks the live authority before its fixture
writes. World statistics checks distinguish its 200 selector from the exact
503/no-store data query. Healthy ownership feed missing-character aliases require
404. Targeted privacy/world-read passed 12/12 and profile aliases passed 8/8,
covering all four projects, without retries. All three optional Rust browser tests
ran explicitly with native Node and installed Chromium and passed with zero axe
violations; the wrapped Node command omitted required fixture environment, so the
native executable is mandatory. Final matrix remains pending.
