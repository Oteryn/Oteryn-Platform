---
task_id: OTERYN-20261119-thais-entire-portal
governing_issue: 1989
required_reads: []
search_first: []
optional_reads: []
---

# Thais design across the web portal

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1989

Extend the approved Thais design to the Polish and English portal, including interior navigation, account controls and error pages. The release candidate also updates the homepage panorama, profession presentation, news, Oteryn emblem and shop entry.

## Acceptance criteria

- [ ] Confirm mounted navigation, forms, session-dependent controls and language switching on the integration candidate.
- [ ] Verify representative desktop and mobile journeys, empty or unavailable states and the panorama at ultrawide widths.
- [ ] Qualify the exact candidate for protected integration and verify the public result after release.

## Ownership

```yaml
owned_paths:
  - portal-rs/templates/
  - portal-rs/static/thais/
  - portal-rs/static/css/home.css
  - portal-rs/crates/ui/
  - portal-rs/crates/web/src/
  - portal-rs/e2e/
  - docs/agents/tasks/active/OTERYN-20261119-thais-entire-portal.md
modules:
  - Portal presentation and associated verification
dependencies:
  - Thais homepage integrated through PR 1985
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-21T20:30:00Z
head: a34f023d3e6c911f981d753f9314e03ccb83326e
branch: publish/thais-portal
pr: 1990
status: completed
context_routes: []
owned_paths:
  - portal-rs/templates/
  - portal-rs/static/thais/
  - portal-rs/static/css/home.css
  - portal-rs/crates/ui/
  - portal-rs/crates/web/src/
  - portal-rs/e2e/
  - docs/agents/tasks/active/OTERYN-20261119-thais-entire-portal.md
proven:
  - Governing issue 1989 and PR 1990 are open in Oteryn/Oteryn-Platform. PR 1990 targets main. Its starting base is 5730f76fdd5b3829493570cca21f1bb3fee96cea.
  - Earlier local browser evidence includes 24 checks across eight page families at widths of 320, 390 and 1440 pixels. Their stored results reported no issues. This evidence predates the integration candidate.
  - Before implementation, a desktop visitor traversed 51 unique Polish destinations. Registration returned 503 and the client-download destination displayed an unpublished-page message.
derived: []
unknown:
  - Public release outcome was not re-verified during archival.
conflicts:
  - Open PR 1987 at 23d0bc5abedabf615a88c3a5f41dc462b0154d1f separately changes the ultrawide hero in portal-rs/static/thais/home.css. Its 1800px cap and edge mask overlap this candidate's 2868x941 panorama sizing and fade.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/templates/
  - portal-rs/static/thais/
  - portal-rs/static/css/home.css
  - portal-rs/crates/ui/
  - portal-rs/crates/web/src/
  - portal-rs/e2e/
  - docs/agents/tasks/active/OTERYN-20261119-thais-entire-portal.md
validation:
  - command: Build the oteryn-portal development binary from the candidate checkout
    result: PASS
    evidence: The local candidate build completed successfully. Browser and protected-branch checks remain separate.
  - command: checkpoint.py --require-checkpoint, task_issue_liveness.py, task_liveness.py and source_branch_closeout.py
    result: PASS
    evidence: The task checkpoint passed schema v1. Live validation for this packet confirmed open issue 1989 and open PR 1990 on publish/thais-portal with zero findings. Terminal branch disposition is pending.
  - command: Qualify final PR head through repository checks and player journeys
    result: NOT_RUN
    evidence: The published head and the working candidate are being reconciled. Earlier local screenshots cover the preview.
blockers: []
next_action: none; delivered by PR #1990 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: Dedicated same-repository integration branch for issue 1989.
source_branch_evidence: PR #1990 merged as 368b0199640e1d0114e3ba8d1c07e6305688e184 on 2026-11-19; Issue #1989 closed as completed; refs/heads/publish/thais-portal no longer exists on origin
```
