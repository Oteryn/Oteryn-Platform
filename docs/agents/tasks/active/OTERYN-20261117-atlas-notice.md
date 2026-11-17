---
task_id: OTERYN-20261117-atlas-notice
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-atlas-notice

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

The atlas viewer wrote spawn loading state and marker/report outcomes to one
shared `notice` node. A spawn load that finished after a refused map report
cleared the node, so the refusal disappeared. The browser qualification failed
with a 30 s wait for the refusal text on CI (PR 1897 and PR 1906). Marker and
report outcomes now use their own `outcome` status node; spawn loading keeps
`notice`.

## Acceptance criteria

- [x] A spawn load held until after the report refusal no longer erases the refusal (browser check red without the change, green with it)
- [x] Spawn loading, marker save/delete and report messages and timeouts unchanged
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-atlas-notice.md
  - portal-rs/static/atlas/viewer.js
  - portal-rs/crates/atlas/src/spawns_browser.cjs
modules:
  - atlas
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-17T05:33:05Z
head: c6361bd0a0d891f3d45d6dbe164a0b448362ac50
branch: claude/happy-davinci-ijz3wy-atlas-notice
pr: 1907
status: validating
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-atlas-notice.md
  - portal-rs/static/atlas/viewer.js
  - portal-rs/crates/atlas/src/spawns_browser.cjs
proven:
  - The browser check holds the spawns response behind the refused report and fails at the 30 s wait without the viewer change
  - No migration, contract, fixture, flag, route or server code change
derived: []
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: "spawns_browser.cjs: waitFor getByText('Unavailable or invalid input. Please try again.') timeout"
  evidence: Portal RS CI browser job 133834523715 on PR 1906 head b75902a6
rejected_hypotheses:
  - A slow report refusal from the mutation budget (PR 1903); the held-spawns check erases the refusal with no database delay, and the same failure occurred on PR 1897 before PR 1903
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-atlas-notice.md
  - portal-rs/static/atlas/viewer.js
  - portal-rs/crates/atlas/src/spawns_browser.cjs
validation:
  - command: PORTAL_ATLAS_BROWSER_NODE=node cargo test -p oteryn-atlas --lib spawns::tests::postgres_spawns_private_markers_and_reports (PostgreSQL 18), without and with the viewer change
    result: PASS
    evidence: red at the post-release refusal wait without the change; three green runs with it
blockers:
  - none
next_action: "open the PR, wait for Portal RS CI, then squash auto-merge"
```
