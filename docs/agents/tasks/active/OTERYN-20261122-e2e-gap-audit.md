---
task_id: OTERYN-20261122-e2e-gap-audit
governing_issue: 2002
required_reads: []
search_first: []
optional_reads: []
---

# Authenticated site integrity crawl

[Governing issue #2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002)

The audit branch started at `16452208`. The recorded baseline is 48 passed and
96 failed across Polish and English at desktop and mobile widths. The task is
to repair confirmed product defects, qualify designed unavailable views in
the portal-only harness, and publish the existing audit branch.

## Initial acceptance criteria

- Merge the current `origin/main` into `codex/e2e-gap-audit`.
- Cover each product fix with a focused regression.
- Run every crawl actor and section in all four Playwright projects.
- Publish the verified audit branch with a non-force push.

## Findings

- The current main branch serves the decorative panel frame from the portal
  origin. The player setup checks that the asset loads on the strict-CSP
  passkeys page.
- The merged `codex/fix-e2e-small` branch applies `no-store` to unavailable
  portal responses and hides passkey management for accounts with mandatory
  login MFA. The direct passkey refusal remains part of the regression.
- The League read samples its clock while holding the projection lock. An
  expired Game watermark gives the designed localized 503 without result rows.
- Character creation navigation and the router now use the same mount flags.
  The recorded 404 came from enabling the web flag without both command flags.
- Shared chrome reuses the parsed catalogue for each language. The desktop
  player account shard fell from a five-minute timeout to 1 minute 48 seconds.
- The portal-only fixture seeds real accounts through the CLI and uses the
  existing browser sign-in and MFA helpers. The first Polish mobile diagnostic
  completed with 30 passes and six shard timeouts. It exposed the character
  creation link defect and the need for a larger aggregate shard budget.

## Independent review follow-up

- Removed the moderator report-review grant and its migration. The crawl
  companion test now requires a 403 for the moderator's report-list request.
- Reused the panel's Host validation in the shared shell. A PostgreSQL
  regression covers a disabled panel, a different host, a missing port, and
  a matching host.
- Checked protected destinations by actor and required the enforced CSP on
  the passkeys document alongside the successful frame request.
- Limited unavailable responses to exact routes and actors in the empty-Game
  fixture. The crawl asserts localized fallback content and no-store.
- Restored the generated inventory paths to their state on origin/main.
- The final Playwright run passed 160 cases, with 40 in each project. The
  affected Rust tests and the two crawl contract unit tests also passed.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-22T16:08:52Z
head: 998f7acd4192e3f535b692b812cb034a852defa1
branch: codex/e2e-gap-audit
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/e2e/
  - portal-rs/crates/characters/src/create_web/
  - portal-rs/crates/characters/src/navigation.rs
  - portal-rs/crates/minigames/src/league/
  - portal-rs/crates/ui/src/lib.rs
proven:
  - origin/main through 33145d6e and the requested 02980dc2 fix branch are merged
  - Final portal build and disposable PostgreSQL 18.6 startup succeeded
derived: []
unknown:
  - Remote readback is pending
conflicts: []
first_failure:
  marker: recorded baseline
  evidence: 48 passed and 96 failed in the handoff
rejected_hypotheses: []
changed_paths: []
validation:
  - command: node run.mjs .harness/run/secrets/browser.json test specs/site-integrity.spec.mjs specs/fix-small.spec.mjs --workers=2 --trace=off
    result: PASS
    evidence: Review follow-up passed all 160 cases with 40 per project.
  - command: targeted PostgreSQL staff navigation test and two chrome response tests
    result: PASS
    evidence: Three focused Rust tests passed.
  - command: node --test e2e/crawl-contracts.test.mjs
    result: PASS
    evidence: Two contract checks passed.
  - command: node run.mjs .harness/run/secrets/browser.json test specs/site-integrity.spec.mjs specs/fix-small.spec.mjs --workers=3 --trace=off
    result: PASS
    evidence: All 144 site integrity cases passed, with 36 per project. The companion suite had 15 passes in the same run and its Polish desktop passkey case passed separately after a socket hangup.
  - command: focused Rust tests for the shared catalogue, character creation navigation, and League projection timing
    result: PASS
    evidence: Four passed, including both League cases against PostgreSQL 18.6.
  - command: cargo fmt --all --check
    result: PASS
    evidence: The tested source tree is formatted.
blockers: []
next_action: Push the validated audit branch and verify the remote head
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: The existing audit branch is the requested deliverable under issue 2002.
source_branch_evidence: Pending final remote readback.
```
